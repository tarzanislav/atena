//! One check of one endpoint: fetch → (feed | sitemap | page) → store.
//!
//! The order is from cheap to expensive: a conditional request first (`304` costs nothing), then
//! the text fingerprint (same text = no new version), only then the change analysis, the file
//! store, classification and link discovery. Extraction runs on a worker thread with a time limit,
//! so a hostile PDF cannot stall the crawler. A failure of one endpoint never stops the others.

use std::sync::Arc;
use std::time::{Duration, Instant};

use atena_core::legality::{self, Permission, SourceFacts};
use atena_core::{ChangeKind, Licence, Taxonomy, Terms, compare, suggest};
use atena_crawl::{FetchError, Fetched, Fetcher, Page, Validators, parse_feed, parse_sitemap};
use atena_extract::{Document, ExtractError, Format, LanguageSource, extract};
use atena_store::{
    Capture, Check, Endpoint, EndpointKind, LabelSuggestion, NewEndpoint, NewVersion, Origin,
    Rights, Store, StoreError, hash, parse_hex,
};
use url::Url;

use crate::privacy;
use crate::urls::{MAX_LINKS_PER_PAGE, follow, host, normalize};

/// Settings of the pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Check interval of endpoints found in feeds, sitemaps and links (default 30 days).
    pub discovered_interval: Duration,
    /// Time limit for extracting one document (default 60 s).
    pub extract_timeout: Duration,
    /// Failed checks in a row after which a discovered endpoint is no longer checked (default 6);
    /// register endpoints are never switched off.
    pub disable_after_failures: i32,
    /// Lowest score of a classification suggestion (default 0.45).
    pub classify_threshold: f64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            discovered_interval: Duration::from_secs(30 * 86_400),
            extract_timeout: Duration::from_secs(60),
            disable_after_failures: 6,
            classify_threshold: 0.45,
        }
    }
}

/// What a check did, as recorded in the check log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Short outcome: `new`, `changed`, `unchanged`, `not_modified`, `feed`, `sitemap`, `robots`, …
    pub outcome: String,
    /// Detail for the log.
    pub detail: Option<String>,
    /// The host asked to be left alone for this long.
    pub backoff: Option<Duration>,
}

/// The result of processing a response.
struct Processed {
    outcome: &'static str,
    detail: Option<String>,
    failed: bool,
    disable: bool,
}

impl Processed {
    fn ok(outcome: &'static str, detail: Option<String>) -> Self {
        Self {
            outcome,
            detail,
            failed: false,
            disable: false,
        }
    }

    fn failed(outcome: &'static str, detail: String) -> Self {
        Self {
            outcome,
            detail: Some(detail),
            failed: true,
            disable: false,
        }
    }
}

/// The ingestion pipeline; share one instance between workers.
pub struct Pipeline {
    store: Store,
    fetcher: Arc<Fetcher>,
    taxonomy: Arc<Taxonomy>,
    config: PipelineConfig,
}

fn format_name(format: Format) -> &'static str {
    match format {
        Format::Html => "html",
        Format::Pdf => "pdf",
        Format::Docx => "docx",
        Format::Odt => "odt",
        Format::Text => "text",
    }
}

/// The publisher's rules: HTTP headers and tdmrep.json first, then the document's own metadata.
fn rights(page: &Page, doc: &Document) -> (Rights, bool) {
    let mut tdm = page.tdm.clone();
    tdm.apply(doc.tdm_reservation, doc.tdm_policy.clone());
    let mut index = page.index;
    for list in &doc.robots {
        index.add_list(list);
    }
    let rights = Rights {
        tdm_reserved: tdm.reserved == Some(true),
        tdm_policy: tdm.policy,
        noindex: index.noindex,
        noarchive: index.noarchive,
        nosnippet: index.nosnippet,
        notranslate: index.notranslate,
    };
    (rights, index.nofollow)
}

/// What the legality principle allows for this endpoint's source (and document).
/// Values read back from the database that are not understood count as the strictest.
fn permitted(ep: &Endpoint, tdm_reserved: bool, file_document: bool) -> Permission {
    let licence = Licence::parse(&ep.source_licence);
    legality::permission(
        SourceFacts {
            country: &ep.source_country,
            licence: &licence,
            terms: Terms::parse(&ep.source_terms),
        },
        tdm_reserved,
        file_document,
    )
}

/// Runs CPU-bound work on a worker thread with a time limit.
async fn blocking<T: Send + 'static>(
    limit: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, &'static str> {
    match tokio::time::timeout(limit, tokio::task::spawn_blocking(work)).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_)) => Err("the processing thread failed"),
        Err(_) => Err("processing took too long"),
    }
}

impl Pipeline {
    /// A pipeline over a store and a fetcher.
    pub fn new(
        store: Store,
        fetcher: Arc<Fetcher>,
        taxonomy: Arc<Taxonomy>,
        config: PipelineConfig,
    ) -> Self {
        Self {
            store,
            fetcher,
            taxonomy,
            config,
        }
    }

    /// The store.
    pub fn store(&self) -> &Store {
        &self.store
    }

    fn discovered_interval(&self, ep: &Endpoint) -> i32 {
        let secs = i32::try_from(self.config.discovered_interval.as_secs()).unwrap_or(i32::MAX);
        secs.max(ep.source_interval_s)
    }

    fn new_endpoint(
        &self,
        ep: &Endpoint,
        url: &Url,
        kind: EndpointKind,
        origin: Origin,
    ) -> Option<NewEndpoint> {
        let url = normalize(url);
        Some(NewEndpoint {
            host: host(&url)?,
            url: url.to_string(),
            kind,
            origin,
            follow_links: false,
            interval_s: self.discovered_interval(ep),
        })
    }

    /// When to check again after a failure: 1 h, 4 h, 16 h… but never later than the normal interval.
    fn retry_in(ep: &Endpoint) -> Duration {
        let interval = u64::try_from(ep.interval_s).unwrap_or(86_400).max(3_600);
        let exponent = u32::try_from(ep.fail_count.clamp(0, 10)).unwrap_or(10);
        Duration::from_secs(
            3_600u64
                .saturating_mul(4u64.saturating_pow(exponent))
                .min(interval),
        )
    }

    /// Checks one endpoint and records the result.
    pub async fn check(&self, ep: &Endpoint) -> Result<Outcome, StoreError> {
        let started = Instant::now();
        let interval = Duration::from_secs(u64::try_from(ep.interval_s).unwrap_or(86_400));
        let mut check = Check {
            next_in: interval,
            ..Check::default()
        };
        let mut backoff = None;

        // the legality principle comes first: a source that may not be crawled is not contacted
        let allowed = permitted(ep, false, false);
        if !allowed.crawl {
            check.outcome = "not_permitted".into();
            check.detail = Some(allowed.reasons.join("; "));
            self.store.record_check(ep.id, &check).await?;
            return Ok(Outcome {
                outcome: check.outcome,
                detail: check.detail,
                backoff: None,
            });
        }
        let Ok(url) = Url::parse(&ep.url) else {
            check.outcome = "invalid_url".into();
            check.disable = true;
            check.failed = true;
            self.store.record_check(ep.id, &check).await?;
            return Ok(Outcome {
                outcome: check.outcome,
                detail: None,
                backoff: None,
            });
        };
        let validators = Validators {
            etag: ep.etag.clone(),
            last_modified: ep.last_modified.clone(),
        };
        match self.fetcher.fetch(&url, &validators).await {
            Ok(Fetched::NotModified { .. }) => {
                self.store.mark_seen(ep.id).await?;
                check.outcome = "not_modified".into();
                check.status = Some(304);
            }
            Ok(Fetched::Page(page)) => {
                check.status = Some(page.status);
                check.bytes = Some(page.body.len() as u64);
                let processed = match ep.kind {
                    EndpointKind::Feed => self.feed(ep, &page).await?,
                    EndpointKind::Sitemap => self.sitemap(ep, &page).await?,
                    EndpointKind::Page => self.page(ep, &page).await?,
                };
                check.outcome = processed.outcome.into();
                check.detail = processed.detail;
                check.failed = processed.failed;
                check.disable = processed.disable;
                if processed.failed {
                    check.next_in = Self::retry_in(ep);
                } else {
                    // validators are kept only for content that was processed completely
                    check.validators = Some((
                        page.validators.etag.clone(),
                        page.validators.last_modified.clone(),
                    ));
                }
            }
            Err(error) => {
                check.detail = Some(error.to_string());
                let (outcome, failed, disable) = match &error {
                    FetchError::Robots { .. } => ("robots", false, false),
                    FetchError::RobotsUnreachable { .. } => ("robots_unreachable", true, false),
                    FetchError::RetryLater { after, .. } => {
                        check.next_in = *after;
                        backoff = Some(*after);
                        ("retry_later", false, false)
                    }
                    FetchError::Status {
                        status: s @ (404 | 410),
                        ..
                    } => {
                        check.status = Some(*s);
                        self.store.mark_gone(ep.id).await?;
                        ("gone", true, false)
                    }
                    FetchError::Status { status, .. } => {
                        check.status = Some(*status);
                        ("http_error", true, false)
                    }
                    FetchError::Blocked { .. } => ("blocked", true, false),
                    FetchError::TooLarge { .. } => ("too_large", true, true),
                    FetchError::UnsupportedType { .. } => ("unsupported", true, true),
                    FetchError::TooManyRedirects { .. } => ("redirects", true, false),
                    FetchError::Timeout { .. } => ("timeout", true, false),
                    FetchError::Network { .. } | FetchError::Setup(_) => ("network", true, false),
                };
                check.outcome = outcome.into();
                check.failed = failed;
                check.disable = disable;
                if failed {
                    check.next_in = Self::retry_in(ep);
                }
            }
        }
        if check.failed && ep.fail_count + 1 >= self.config.disable_after_failures {
            check.disable = true;
        }
        // register endpoints are chosen by people: they back off, but are never switched off
        if ep.origin == Origin::Register {
            check.disable = false;
        }
        check.elapsed = started.elapsed();
        self.store.record_check(ep.id, &check).await?;
        tracing::info!(endpoint = ep.id, url = %ep.url, outcome = %check.outcome, detail = ?check.detail, "checked");
        Ok(Outcome {
            outcome: check.outcome,
            detail: check.detail,
            backoff,
        })
    }

    async fn feed(&self, ep: &Endpoint, page: &Page) -> Result<Processed, StoreError> {
        let (body, base) = (page.body.clone(), page.url.clone());
        let feed = match blocking(self.config.extract_timeout, move || {
            parse_feed(&body, &base)
        })
        .await
        {
            Ok(Ok(feed)) => feed,
            Ok(Err(e)) => return Ok(Processed::failed("feed_error", e.to_string())),
            Err(e) => return Ok(Processed::failed("feed_error", e.to_string())),
        };
        let found: Vec<NewEndpoint> = feed
            .items
            .iter()
            .filter_map(|item| item.link.as_ref())
            .filter_map(|link| self.new_endpoint(ep, link, EndpointKind::Page, Origin::Feed))
            .collect();
        let added = self
            .store
            .add_endpoints(&ep.source_id, Some(ep.id), &found)
            .await?;
        Ok(Processed::ok(
            "feed",
            Some(format!("{} items, {added} new", feed.items.len())),
        ))
    }

    async fn sitemap(&self, ep: &Endpoint, page: &Page) -> Result<Processed, StoreError> {
        let (body, base) = (page.body.clone(), page.url.clone());
        let map = match blocking(self.config.extract_timeout, move || {
            parse_sitemap(&body, &base)
        })
        .await
        {
            Ok(Ok(map)) => map,
            Ok(Err(e)) => return Ok(Processed::failed("sitemap_error", e.to_string())),
            Err(e) => return Ok(Processed::failed("sitemap_error", e.to_string())),
        };
        let children = map
            .sitemaps
            .iter()
            .filter_map(|s| self.new_endpoint(ep, &s.loc, EndpointKind::Sitemap, Origin::Sitemap));
        let pages = map
            .urls
            .iter()
            .filter_map(|u| self.new_endpoint(ep, &u.loc, EndpointKind::Page, Origin::Sitemap));
        let found: Vec<NewEndpoint> = children.chain(pages).collect();
        let added = self
            .store
            .add_endpoints(&ep.source_id, Some(ep.id), &found)
            .await?;
        Ok(Processed::ok(
            "sitemap",
            Some(format!(
                "{} entries, {added} new, {} skipped",
                found.len(),
                map.skipped
            )),
        ))
    }

    async fn page(&self, ep: &Endpoint, page: &Page) -> Result<Processed, StoreError> {
        let (body, content_type, url) = (
            page.body.clone(),
            page.content_type.clone(),
            page.url.clone(),
        );
        let doc = match blocking(self.config.extract_timeout, move || {
            extract(&body, content_type.as_deref(), &url)
        })
        .await
        {
            Ok(Ok(doc)) => doc,
            Ok(Err(ExtractError::Unsupported(what))) => {
                return Ok(Processed {
                    disable: true,
                    ..Processed::failed("unsupported", what)
                });
            }
            Ok(Err(e)) => return Ok(Processed::failed("extract_error", e.to_string())),
            Err(e) => return Ok(Processed::failed("extract_error", e.to_string())),
        };

        // data minimisation before anything is fingerprinted or stored
        let mut doc = doc;
        let mut redacted = 0;
        let mut clean = |text: &mut String| {
            let r = privacy::redact(text);
            redacted += r.emails + r.phones;
            *text = r.text;
        };
        clean(&mut doc.text);
        doc.title.iter_mut().for_each(&mut clean);
        doc.description.iter_mut().for_each(&mut clean);
        doc.links.iter_mut().for_each(|l| clean(&mut l.text));
        // a web page's "author" is usually its webmaster; authors are kept for documents only
        if matches!(doc.format, Format::Html | Format::Text) {
            doc.authors.clear();
        }

        let (rights, nofollow) = rights(page, &doc);
        // the legality principle decides what may be kept (and later shown)
        let permission = permitted(
            ep,
            rights.tdm_reserved,
            matches!(doc.format, Format::Pdf | Format::Docx | Format::Odt),
        );
        let keep_text = permission.store_text;
        let fingerprint = parse_hex(&atena_core::text::fingerprint(&doc.text))
            .expect("fingerprints are 64 hex digits");
        let capture = Capture {
            endpoint_id: ep.id,
            source_id: ep.source_id.clone(),
            url: page.url.to_string(),
            format: format_name(doc.format).to_string(),
            raw_hash: hash(&page.body),
            fingerprint,
            fetched_at: page.fetched_at,
            rights,
        };

        let current = self.store.current_document(ep.id).await?;
        if current
            .as_ref()
            .is_some_and(|c| c.last_fingerprint == Some(fingerprint))
        {
            self.store.touch_document(&capture).await?;
            return Ok(Processed::ok("unchanged", None));
        }
        let (change_kind, added, removed, numbers_changed) = match (&current, keep_text) {
            (None, _) => ("initial", 0, 0, false),
            (Some(c), true) if c.body.is_some() => {
                let change = compare(c.body.as_deref(), &doc.text);
                let kind = match change.kind {
                    ChangeKind::Initial => "initial",
                    ChangeKind::Minor => "minor",
                    ChangeKind::Substantive => "substantive",
                    ChangeKind::Unchanged => {
                        // equal after normalisation although the fingerprint differs: not a new version
                        self.store.touch_document(&capture).await?;
                        return Ok(Processed::ok("unchanged", None));
                    }
                };
                let count = |n: usize| i32::try_from(n).unwrap_or(i32::MAX);
                (
                    kind,
                    count(change.added.len()),
                    count(change.removed.len()),
                    change.numbers_changed,
                )
            }
            // the previous text was not kept, so only "it changed" is known
            (Some(_), _) => ("changed", 0, 0, false),
        };

        if permission.keep_original {
            self.store
                .put_blob(page.body.to_vec(), page.content_type.as_deref())
                .await?;
        }
        let language = doc.language.as_ref();
        let title = doc.title.clone().unwrap_or_default();
        let classified_text = if keep_text { doc.text.as_str() } else { "" };
        let suggestions: Vec<LabelSuggestion> = suggest(
            &self.taxonomy,
            &title,
            classified_text,
            language.map(|l| l.code.as_str()),
            self.config.classify_threshold,
        )
        .into_iter()
        .map(|s| LabelSuggestion {
            scheme: s.scheme,
            concept: s.concept,
            score: s.score as f32,
            evidence: s.evidence,
        })
        .collect();
        let links: Vec<(String, String)> = if keep_text {
            doc.links
                .iter()
                .map(|l| (normalize(&l.url).to_string(), l.text.clone()))
                .collect()
        } else {
            Vec::new()
        };
        let version = NewVersion {
            change_kind: change_kind.into(),
            paragraphs_added: added,
            paragraphs_removed: removed,
            numbers_changed,
            content_type: page.content_type.clone(),
            encoding: doc.encoding.map(str::to_string),
            title: doc.title.clone(),
            description: doc.description.clone().filter(|_| keep_text),
            body: keep_text.then(|| doc.text.clone()),
            body_truncated: doc.truncated,
            language: language.map(|l| l.code.clone()),
            language_source: language.map(|l| {
                match l.source {
                    LanguageSource::Detected => "detected",
                    LanguageSource::Declared => "declared",
                }
                .to_string()
            }),
            language_confidence: language.map(|l| l.confidence as f32),
            authors: doc.authors.clone(),
            published_at: doc.published,
            modified_at: doc.modified,
            canonical_url: doc.canonical.as_ref().map(Url::to_string),
        };
        let saved = self
            .store
            .save_version(&capture, &version, &links, &suggestions)
            .await?;

        let mut discovered = 0;
        if keep_text && ep.follow_links && !nofollow {
            let page_url = normalize(&page.url);
            let mut found: Vec<NewEndpoint> = doc
                .links
                .iter()
                .filter(|l| !l.nofollow)
                .filter(|l| follow(&page_url, &normalize(&l.url)).is_some())
                .filter_map(|l| self.new_endpoint(ep, &l.url, EndpointKind::Page, Origin::Link))
                .take(MAX_LINKS_PER_PAGE)
                .collect();
            // feeds the page announces on the publisher's own site
            found.extend(
                doc.feeds
                    .iter()
                    .filter(|f| host(f) == host(&page_url))
                    .filter_map(|f| self.new_endpoint(ep, f, EndpointKind::Feed, Origin::Link)),
            );
            discovered = self
                .store
                .add_endpoints(&ep.source_id, Some(ep.id), &found)
                .await?;
        }
        let outcome = if current.is_none() { "new" } else { "changed" };
        Ok(Processed::ok(
            outcome,
            Some(format!(
                "version {} ({change_kind}), {discovered} new endpoints, {redacted} contact details removed{}",
                saved.version_no,
                if keep_text {
                    String::new()
                } else {
                    format!("; metadata only: {}", permission.reasons.join("; "))
                }
            )),
        ))
    }
}
