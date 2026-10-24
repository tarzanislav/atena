//! The fetcher: the single, polite and safe way ATENA downloads anything from the web.
//!
//! For every request, including each redirect hop:
//! 1. the URL is checked ([`NetPolicy::check_url`]) and DNS answers are vetted (no private addresses);
//! 2. robots.txt of the origin is obeyed (RFC 9309; cached for 24 hours);
//! 3. the host is paced: one request at a time, `min_delay` or the robots `crawl-delay` apart,
//!    and left alone after `429`/`503` for as long as `Retry-After` asks;
//! 4. only accepted content types are read, never more than `max_body_bytes` (also after gzip);
//! 5. the TDMRep reservation, a refusal of AI crawlers in robots.txt and `X-Robots-Tag` rules are
//!    attached to the page.

use std::error::Error as StdError;
use std::sync::{Arc, Once};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use jiff::Timestamp;
use reqwest::header::{self, HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Response};
use tokio::time::Instant;
use url::Url;

use crate::date::parse_date;
use crate::directives::{IndexRules, Tdm, TdmFile};
use crate::net::{Blocked, NetPolicy, VettingResolver};
use crate::pace::{OriginCache, Pacer, Turn};
use crate::robots::{MAX_ROBOTS_BYTES, MiningRules, RobotsPolicy, RobotsTxt};

/// The name site owners use for ATENA in robots.txt and `X-Robots-Tag`.
pub const PRODUCT_TOKEN: &str = "ATENA";
/// Largest tdmrep.json that is read.
const MAX_TDM_BYTES: usize = 256 * 1024;
/// Waiting time after `429`/`503` without a usable `Retry-After`.
const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(60);
const ACCEPT: &str =
    "text/html,application/xhtml+xml,application/xml;q=0.9,application/pdf;q=0.9,*/*;q=0.8";

/// Content types read by default (compared without parameters such as `charset`).
pub const DEFAULT_TYPES: [&str; 20] = [
    "text/html",
    "application/xhtml+xml",
    "text/plain",
    "text/csv",
    "text/xml",
    "application/xml",
    "application/rss+xml",
    "application/atom+xml",
    "application/rdf+xml",
    "application/json",
    "application/pdf",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.oasis.opendocument.text",
    "application/vnd.ms-excel",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.oasis.opendocument.spreadsheet",
    "application/gzip",
    "application/x-gzip",
    "application/octet-stream",
];

/// Settings of the fetcher.
#[derive(Debug, Clone)]
pub struct Config {
    /// Name matched against robots.txt groups and `X-Robots-Tag` (default `ATENA`).
    pub product_token: String,
    /// The `User-Agent` header; it contains the product token and a link to the crawler page.
    pub user_agent: String,
    /// Which destinations may be contacted.
    pub net: NetPolicy,
    /// Minimal time between two requests to one host (default 5 s).
    pub min_delay: Duration,
    /// Upper bound for a robots.txt `crawl-delay` (default 60 s).
    pub max_crawl_delay: Duration,
    /// Time allowed for connecting (default 10 s).
    pub connect_timeout: Duration,
    /// Time allowed for a whole request including the body (default 60 s).
    pub timeout: Duration,
    /// Redirects followed per fetch (default 5).
    pub max_redirects: usize,
    /// Largest body read, after decompression (default 50 MiB).
    pub max_body_bytes: usize,
    /// How long robots.txt and tdmrep.json are cached (default 24 h).
    pub policy_ttl: Duration,
    /// How soon an unreachable robots.txt is tried again (default 30 min).
    pub unreachable_ttl: Duration,
    /// Upper bound for `Retry-After` (default 6 h).
    pub max_retry_after: Duration,
    /// Content types that are read; others are refused before the body is downloaded.
    pub accepted_types: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            product_token: PRODUCT_TOKEN.to_string(),
            user_agent: format!(
                "Mozilla/5.0 (compatible; {PRODUCT_TOKEN}/{}; +https://github.com/tarzanislav/atena/blob/main/docs/CRAWLER.md)",
                env!("CARGO_PKG_VERSION")
            ),
            net: NetPolicy::default(),
            min_delay: Duration::from_secs(5),
            max_crawl_delay: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(10),
            timeout: Duration::from_secs(60),
            max_redirects: 5,
            max_body_bytes: 50 * 1024 * 1024,
            policy_ttl: Duration::from_secs(24 * 3600),
            unreachable_ttl: Duration::from_secs(30 * 60),
            max_retry_after: Duration::from_secs(6 * 3600),
            accepted_types: DEFAULT_TYPES.iter().map(|t| t.to_string()).collect(),
        }
    }
}

/// Validators of a stored version, sent as `If-None-Match` / `If-Modified-Since`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Validators {
    /// `ETag` of the stored version.
    pub etag: Option<String>,
    /// `Last-Modified` of the stored version.
    pub last_modified: Option<String>,
}

/// A downloaded resource.
#[derive(Debug, Clone)]
pub struct Page {
    /// Final URL after redirects.
    pub url: Url,
    /// HTTP status (2xx).
    pub status: u16,
    /// `Content-Type` as sent.
    pub content_type: Option<String>,
    /// `Content-Language` as sent.
    pub content_language: Option<String>,
    /// Validators to send next time.
    pub validators: Validators,
    /// Text and data mining reservation (tdmrep.json and headers; HTML/PDF metadata is added later).
    pub tdm: Tdm,
    /// `X-Robots-Tag` rules for ATENA.
    pub index: IndexRules,
    /// The body, decompressed.
    pub body: Bytes,
    /// When the response was received.
    pub fetched_at: Timestamp,
}

/// Result of a successful fetch.
#[derive(Debug, Clone)]
pub enum Fetched {
    /// New content.
    Page(Page),
    /// `304 Not Modified`: the stored version is current.
    NotModified {
        /// Final URL after redirects.
        url: Url,
    },
}

/// Why a fetch did not return content.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    /// Refused by the network policy; nothing was sent.
    #[error("{url}: blocked: {reason}")]
    Blocked {
        /// The refused URL.
        url: String,
        /// The reason.
        reason: Blocked,
    },
    /// robots.txt does not allow ATENA to fetch the URL.
    #[error("{url}: disallowed by robots.txt")]
    Robots {
        /// The disallowed URL.
        url: String,
    },
    /// robots.txt could not be fetched (server error, network, DNS), which means nothing may be
    /// fetched from the site for now (RFC 9309).
    #[error("{url}: robots.txt is unreachable, so nothing may be fetched for now")]
    RobotsUnreachable {
        /// The URL.
        url: String,
    },
    /// More redirects than allowed.
    #[error("{url}: too many redirects")]
    TooManyRedirects {
        /// The last URL reached.
        url: String,
    },
    /// The host asked to be left alone (`429`, `503`) for this long.
    #[error("{url}: try again in {after:?}")]
    RetryLater {
        /// The URL.
        url: String,
        /// Time to wait.
        after: Duration,
    },
    /// An HTTP status other than success, redirect or "not modified".
    #[error("{url}: HTTP {status}")]
    Status {
        /// The URL.
        url: String,
        /// The status code.
        status: u16,
    },
    /// The content type is not accepted; the body was not downloaded.
    #[error("{url}: content type {content_type} is not accepted")]
    UnsupportedType {
        /// The URL.
        url: String,
        /// The refused type.
        content_type: String,
    },
    /// The body exceeds the size limit.
    #[error("{url}: larger than {limit} bytes")]
    TooLarge {
        /// The URL.
        url: String,
        /// The limit.
        limit: usize,
    },
    /// No answer in time.
    #[error("{url}: timed out")]
    Timeout {
        /// The URL.
        url: String,
    },
    /// Connection, TLS or protocol failure.
    #[error("{url}: {message}")]
    Network {
        /// The URL.
        url: String,
        /// Description of the failure.
        message: String,
    },
    /// The HTTP client could not be created.
    #[error("HTTP client: {0}")]
    Setup(String),
}

/// robots.txt of one origin.
#[derive(Debug, Clone)]
struct SiteRules {
    robots: RobotsPolicy,
    /// What robots.txt says to crawlers that collect for AI training and mining.
    mining: MiningRules,
    sitemaps: Vec<String>,
    /// `false` when the policy is the "complete disallow" of an unreachable robots.txt.
    reachable: bool,
}

/// Outcome of fetching robots.txt or tdmrep.json.
enum PolicyFile {
    Response { status: u16, body: Bytes },
    TooManyRedirects,
    Unreachable,
}

/// Downloads web resources politely and safely. Share one instance across tasks.
#[derive(Debug)]
pub struct Fetcher {
    client: Client,
    config: Config,
    pacer: Pacer,
    sites: OriginCache<SiteRules>,
    tdm_files: OriginCache<Option<Arc<TdmFile>>>,
}

pub(crate) fn install_crypto_provider() {
    static ONCE: Once = Once::new();
    // an application may have installed its own provider already; that one is kept
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn path_and_query(url: &Url) -> String {
    match url.query() {
        Some(q) => format!("{}?{q}", url.path()),
        None => url.path().to_string(),
    }
}

fn header_text(headers: &HeaderMap, name: HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// The policy refusal hidden in an error chain (the DNS resolver reports it this way).
/// `io::Error` does not list its own payload as a source, so that payload is searched too.
fn blocked_in_chain(error: &(dyn StdError + 'static)) -> Option<Blocked> {
    let mut current = Some(error);
    while let Some(e) = current {
        if let Some(blocked) = e.downcast_ref::<Blocked>() {
            return Some(blocked.clone());
        }
        if let Some(found) = e
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::get_ref)
            .and_then(|inner| blocked_in_chain(inner))
        {
            return Some(found);
        }
        current = e.source();
    }
    None
}

pub(crate) fn classify(url: &Url, error: &reqwest::Error) -> FetchError {
    let url_text = url.to_string();
    if let Some(reason) = blocked_in_chain(error) {
        return FetchError::Blocked {
            url: url_text,
            reason,
        };
    }
    if error.is_timeout() {
        return FetchError::Timeout { url: url_text };
    }
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(e) = source {
        message.push_str(": ");
        message.push_str(&e.to_string());
        source = e.source();
    }
    FetchError::Network {
        url: url_text,
        message,
    }
}

impl Fetcher {
    /// Creates a fetcher.
    pub fn new(config: Config) -> Result<Self, FetchError> {
        install_crypto_provider();
        let client = Client::builder()
            .user_agent(&config.user_agent)
            // no proxies: a proxy would resolve names itself and bypass the address checks
            .no_proxy()
            // redirects are followed by hand so that every hop is checked
            .redirect(reqwest::redirect::Policy::none())
            .dns_resolver(VettingResolver::new(Arc::new(config.net.clone())))
            .connect_timeout(config.connect_timeout)
            .timeout(config.timeout)
            .referer(false)
            .build()
            .map_err(|e| FetchError::Setup(e.to_string()))?;
        Ok(Self {
            client,
            config,
            pacer: Pacer::default(),
            sites: OriginCache::default(),
            tdm_files: OriginCache::default(),
        })
    }

    /// The settings in use.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Fetches `url`, following redirects; `validators` make the request conditional.
    pub async fn fetch(&self, url: &Url, validators: &Validators) -> Result<Fetched, FetchError> {
        let mut current = url.clone();
        for _ in 0..=self.config.max_redirects {
            self.check(&current)?;
            let site = self.site_rules(&current).await;
            if !site.robots.is_allowed(&path_and_query(&current)) {
                let url = current.to_string();
                return Err(if site.reachable {
                    FetchError::Robots { url }
                } else {
                    FetchError::RobotsUnreachable { url }
                });
            }
            let tdm_file = self.tdm_file(&current).await;
            let mut turn = self.turn(&current, self.delay_for(&site.robots)).await?;
            let response = self.send(&current, Some(validators)).await?;
            let status = response.status().as_u16();
            match status {
                304 => return Ok(Fetched::NotModified { url: current }),
                300..=399 => {
                    current = self.redirect_target(&current, &response)?;
                    continue;
                }
                429 | 503 => {
                    let after = self.retry_after(response.headers());
                    turn.back_off(Instant::now() + after);
                    return Err(FetchError::RetryLater {
                        url: current.to_string(),
                        after,
                    });
                }
                200..=299 => {
                    let page = self
                        .read_page(current, response, tdm_file.as_deref(), &site.mining)
                        .await;
                    drop(turn);
                    return page.map(Fetched::Page);
                }
                _ => {
                    return Err(FetchError::Status {
                        url: current.to_string(),
                        status,
                    });
                }
            }
        }
        Err(FetchError::TooManyRedirects {
            url: current.to_string(),
        })
    }

    /// Sitemap URLs that robots.txt of the URL's origin lists.
    pub async fn sitemaps(&self, url: &Url) -> Vec<Url> {
        if self.check(url).is_err() {
            return Vec::new();
        }
        self.site_rules(url)
            .await
            .sitemaps
            .iter()
            .filter_map(|s| url.join(s).ok())
            .filter(|s| matches!(s.scheme(), "http" | "https"))
            .collect()
    }

    fn check(&self, url: &Url) -> Result<(), FetchError> {
        self.config
            .net
            .check_url(url)
            .map_err(|reason| FetchError::Blocked {
                url: url.to_string(),
                reason,
            })
    }

    fn delay_for(&self, robots: &RobotsPolicy) -> Duration {
        let requested = robots
            .crawl_delay()
            .unwrap_or_default()
            .min(self.config.max_crawl_delay);
        self.config.min_delay.max(requested)
    }

    async fn turn(&self, url: &Url, delay: Duration) -> Result<Turn, FetchError> {
        let host = url.host_str().unwrap_or_default();
        self.pacer
            .turn(host, delay)
            .await
            .map_err(|after| FetchError::RetryLater {
                url: url.to_string(),
                after,
            })
    }

    async fn send(
        &self,
        url: &Url,
        validators: Option<&Validators>,
    ) -> Result<Response, FetchError> {
        let mut request = self.client.get(url.clone()).header(header::ACCEPT, ACCEPT);
        if let Some(v) = validators {
            // stored values that are not valid header values are simply not sent
            let valid =
                |text: &Option<String>| text.as_deref().and_then(|t| HeaderValue::from_str(t).ok());
            if let Some(etag) = valid(&v.etag) {
                request = request.header(header::IF_NONE_MATCH, etag);
            }
            if let Some(date) = valid(&v.last_modified) {
                request = request.header(header::IF_MODIFIED_SINCE, date);
            }
        }
        request.send().await.map_err(|e| classify(url, &e))
    }

    fn redirect_target(&self, url: &Url, response: &Response) -> Result<Url, FetchError> {
        let location = header_text(response.headers(), header::LOCATION).ok_or_else(|| {
            FetchError::Status {
                url: url.to_string(),
                status: response.status().as_u16(),
            }
        })?;
        let target = url.join(&location).map_err(|_| FetchError::Network {
            url: url.to_string(),
            message: format!("invalid redirect target {location:?}"),
        })?;
        if url.scheme() == "https" && target.scheme() == "http" {
            return Err(FetchError::Blocked {
                url: target.to_string(),
                reason: Blocked::Downgrade,
            });
        }
        Ok(target)
    }

    fn retry_after(&self, headers: &HeaderMap) -> Duration {
        let wait = header_text(headers, header::RETRY_AFTER).and_then(|v| match v.parse::<u64>() {
            Ok(seconds) => Some(Duration::from_secs(seconds)),
            Err(_) => {
                let at = parse_date(&v)?;
                Some(Duration::try_from(at.duration_since(Timestamp::now())).unwrap_or_default())
            }
        });
        wait.unwrap_or(DEFAULT_RETRY_AFTER)
            .clamp(Duration::from_secs(1), self.config.max_retry_after)
    }

    /// Reads at most `limit` bytes; more is an error, or – with `truncate` – simply cut off.
    async fn read_body(
        url: &Url,
        response: &mut Response,
        limit: usize,
        truncate: bool,
    ) -> Result<Bytes, FetchError> {
        let too_large = || FetchError::TooLarge {
            url: url.to_string(),
            limit,
        };
        if !truncate && response.content_length().is_some_and(|n| n > limit as u64) {
            return Err(too_large());
        }
        let mut body = BytesMut::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| classify(url, &e))? {
            if body.len() + chunk.len() > limit {
                if !truncate {
                    return Err(too_large());
                }
                body.extend_from_slice(&chunk[..limit - body.len()]);
                break;
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body.freeze())
    }

    async fn read_page(
        &self,
        url: Url,
        mut response: Response,
        tdm_file: Option<&TdmFile>,
        mining: &MiningRules,
    ) -> Result<Page, FetchError> {
        let headers = response.headers().clone();
        let content_type = header_text(&headers, header::CONTENT_TYPE);
        if let Some(ct) = &content_type {
            let essence = ct
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            if !self.config.accepted_types.contains(&essence) {
                return Err(FetchError::UnsupportedType {
                    url: url.to_string(),
                    content_type: essence,
                });
            }
        }
        let body = Self::read_body(&url, &mut response, self.config.max_body_bytes, false).await?;

        let mut tdm = tdm_file
            .map(|f| f.lookup(&path_and_query(&url)))
            .unwrap_or_default();
        tdm.apply_headers(
            header_text(&headers, HeaderName::from_static("tdm-reservation")).as_deref(),
            header_text(&headers, HeaderName::from_static("tdm-policy")).as_deref(),
        );
        tdm.refused_to_ai_crawlers = mining.reserves(&path_and_query(&url));
        let mut index = IndexRules::default();
        for value in headers.get_all("x-robots-tag") {
            if let Ok(text) = value.to_str() {
                index.add_header(text, &self.config.product_token);
            }
        }
        Ok(Page {
            status: response.status().as_u16(),
            content_type,
            content_language: header_text(&headers, header::CONTENT_LANGUAGE),
            validators: Validators {
                etag: header_text(&headers, header::ETAG),
                last_modified: header_text(&headers, header::LAST_MODIFIED),
            },
            tdm,
            index,
            body,
            fetched_at: Timestamp::now(),
            url,
        })
    }

    /// GET of robots.txt or tdmrep.json: redirects are followed (each hop checked), the body is
    /// cut off at `limit`, and every problem counts as "unreachable".
    async fn policy_file(&self, url: Url, limit: usize) -> PolicyFile {
        let mut current = url;
        for _ in 0..=self.config.max_redirects {
            if self.check(&current).is_err() {
                return PolicyFile::Unreachable;
            }
            let Ok(mut turn) = self.turn(&current, self.config.min_delay).await else {
                return PolicyFile::Unreachable;
            };
            let Ok(mut response) = self.send(&current, None).await else {
                return PolicyFile::Unreachable;
            };
            let status = response.status().as_u16();
            if matches!(status, 429 | 503) {
                turn.back_off(Instant::now() + self.retry_after(response.headers()));
                return PolicyFile::Response {
                    status,
                    body: Bytes::new(),
                };
            }
            if (300..=399).contains(&status) {
                match self.redirect_target(&current, &response) {
                    Ok(next) => {
                        current = next;
                        continue;
                    }
                    Err(_) => return PolicyFile::Unreachable,
                }
            }
            let body = Self::read_body(&current, &mut response, limit, true).await;
            drop(turn);
            return match body {
                Ok(body) => PolicyFile::Response { status, body },
                Err(_) => PolicyFile::Unreachable,
            };
        }
        PolicyFile::TooManyRedirects
    }

    async fn site_rules(&self, url: &Url) -> SiteRules {
        let origin = url.origin().ascii_serialization();
        let Ok(robots_url) = url.join("/robots.txt") else {
            return SiteRules {
                robots: RobotsPolicy::DisallowAll,
                mining: MiningRules::default(),
                sitemaps: Vec::new(),
                reachable: false,
            };
        };
        let (token, ttl, retry) = (
            self.config.product_token.as_str(),
            self.config.policy_ttl,
            self.config.unreachable_ttl,
        );
        self.sites
            .get(&origin, |stale| async move {
                let fresh = |robots| SiteRules {
                    robots,
                    mining: MiningRules::default(),
                    sitemaps: Vec::new(),
                    reachable: true,
                };
                match self.policy_file(robots_url, MAX_ROBOTS_BYTES).await {
                    PolicyFile::Response { status, body } if (200..=299).contains(&status) => {
                        let parsed = RobotsTxt::parse(&String::from_utf8_lossy(&body));
                        let rules = SiteRules {
                            robots: RobotsPolicy::Rules(parsed.rules_for(token)),
                            mining: parsed.mining_rules(),
                            sitemaps: parsed.sitemaps().to_vec(),
                            reachable: true,
                        };
                        (rules, ttl)
                    }
                    PolicyFile::Response { status, .. }
                        if (400..=499).contains(&status) && status != 429 =>
                    {
                        (fresh(RobotsPolicy::AllowAll), ttl)
                    }
                    // RFC 9309: too many redirects may be treated as "unavailable"
                    PolicyFile::TooManyRedirects => (fresh(RobotsPolicy::AllowAll), ttl),
                    // unreachable (429, 5xx, network): the last good copy, else complete disallow
                    _ => match stale {
                        Some(last) if last.reachable => (last, retry),
                        _ => (
                            SiteRules {
                                robots: RobotsPolicy::DisallowAll,
                                mining: MiningRules::default(),
                                sitemaps: Vec::new(),
                                reachable: false,
                            },
                            retry,
                        ),
                    },
                }
            })
            .await
    }

    async fn tdm_file(&self, url: &Url) -> Option<Arc<TdmFile>> {
        let origin = url.origin().ascii_serialization();
        let file_url = url.join("/.well-known/tdmrep.json").ok()?;
        let (ttl, retry) = (self.config.policy_ttl, self.config.unreachable_ttl);
        self.tdm_files
            .get(&origin, |stale| async move {
                match self.policy_file(file_url, MAX_TDM_BYTES).await {
                    PolicyFile::Response { status, body } if (200..=299).contains(&status) => {
                        (TdmFile::parse(&body).map(Arc::new), ttl)
                    }
                    PolicyFile::Response { .. } | PolicyFile::TooManyRedirects => (None, ttl),
                    PolicyFile::Unreachable => (stale.flatten(), retry),
                }
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_reason_is_found_in_an_error_chain() {
        let inner = std::io::Error::other(Blocked::Address("intranet.example".into()));
        let outer = std::io::Error::other(inner);
        assert_eq!(
            blocked_in_chain(&outer),
            Some(Blocked::Address("intranet.example".into()))
        );
        assert_eq!(blocked_in_chain(&std::io::Error::other("x")), None);
    }

    #[tokio::test]
    async fn the_http_client_reports_a_resolver_refusal() {
        // `localhost` passes no URL check, so the resolver is the last line of defence here
        install_crypto_provider();
        let client = Client::builder()
            .no_proxy()
            .dns_resolver(VettingResolver::new(Arc::new(NetPolicy::default())))
            .build()
            .unwrap();
        let url = Url::parse("http://localhost:8080/").unwrap();
        let error = client.get(url.clone()).send().await.unwrap_err();
        assert!(
            matches!(
                classify(&url, &error),
                FetchError::Blocked {
                    reason: Blocked::Address(_),
                    ..
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    fn retry_after_values() {
        let f = Fetcher::new(Config::default()).unwrap();
        let wait = |v: &str| {
            let mut h = HeaderMap::new();
            h.insert(header::RETRY_AFTER, HeaderValue::from_str(v).unwrap());
            f.retry_after(&h)
        };
        assert_eq!(wait("120"), Duration::from_secs(120));
        assert_eq!(wait("0"), Duration::from_secs(1));
        assert_eq!(wait("999999"), Duration::from_secs(6 * 3600));
        assert_eq!(wait("soon"), DEFAULT_RETRY_AFTER);
        let in_ten_minutes = Timestamp::now() + jiff::SignedDuration::from_secs(600);
        let http_date =
            jiff::fmt::rfc2822::to_string(&in_ten_minutes.to_zoned(jiff::tz::TimeZone::UTC))
                .unwrap();
        let d = wait(&http_date);
        assert!(
            d > Duration::from_secs(590) && d <= Duration::from_secs(600),
            "{d:?}"
        );
        assert_eq!(f.retry_after(&HeaderMap::new()), DEFAULT_RETRY_AFTER);
    }

    #[test]
    fn default_user_agent_names_the_product_token() {
        let c = Config::default();
        assert!(c.user_agent.contains("ATENA/") && c.user_agent.contains("CRAWLER.md"));
        assert_eq!(c.min_delay, Duration::from_secs(5));
    }
}
