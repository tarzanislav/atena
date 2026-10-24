//! The automatic rights assessment of one item: read the page, its licence and its terms of use,
//! and propose a licence and terms for the register – with the evidence.
//!
//! The proposal follows the legality principle (`atena_core::legality`) and the rules of the
//! rights review (`docs/SOURCE-RIGHTS.md`):
//! * a ban on automated access, or access only for personal use → `forbidden`;
//! * a reservation of text and data mining (terms, TDMRep, robots.txt refusing AI crawlers) →
//!   `reserved`;
//! * one accepted open licence and nothing against it → that licence, `allowed`;
//! * terms read (or none found) without either → `copyrighted`, `allowed`;
//! * anything that could not be read → `unknown`.
//!
//! A machine reading terms of use can be wrong, so a proposal is never applied by itself: a source
//! from an import is not crawled until a person confirms it.

use std::sync::Arc;

use atena_core::source::Rights;
use atena_crawl::{FetchError, Fetched, Fetcher, Page, Validators, parse_feed, parse_sitemap};
use atena_extract::{Format, LegalSignals, extract, legal_signals};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::country::country_of_host;
use crate::input::{Item, Target};
use crate::licence::{LicenceHint, licence_of_url, licences_in_text};
use crate::phrases::{Phrases, first, normalize};
use crate::terms::{Finding, FindingKind, analyse, is_legal_block};

/// Terms-of-use pages read per item at most.
pub const MAX_LEGAL_PAGES: usize = 3;

/// What the item turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A web page.
    Page,
    /// An RSS or Atom feed.
    Feed,
    /// A sitemap.
    Sitemap,
    /// A PDF, DOCX, ODT or text document.
    Document,
    /// An OAI-PMH interface of a repository.
    Oai,
    /// A publication identified by its DOI.
    Doi,
}

impl Kind {
    /// The connector the register uses for it.
    pub fn connector(self) -> &'static str {
        match self {
            Self::Page | Self::Document => "html",
            Self::Feed => "rss",
            Self::Sitemap => "sitemap",
            Self::Oai => "oai",
            Self::Doi => "doi",
        }
    }
}

/// Why the item itself could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Problem {
    /// robots.txt does not allow ATENA to fetch it.
    Robots,
    /// robots.txt could not be fetched, so nothing may be fetched for now.
    RobotsUnreachable,
    /// The address is not allowed (private network, unusual port …).
    Blocked,
    /// The server answered with an error status.
    Status,
    /// Too many redirects, timeout or connection failure.
    Unreachable,
    /// The server asked to be left alone for now.
    RetryLater,
    /// A content type ATENA does not read.
    UnsupportedType,
    /// Larger than ATENA reads.
    TooLarge,
}

impl Problem {
    /// A short description for the rights record.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Robots => "robots.txt does not allow ATENA to fetch it",
            Self::RobotsUnreachable => "robots.txt could not be fetched",
            Self::Blocked => "the address is not allowed",
            Self::Status => "the server answered with an error",
            Self::Unreachable => "the server could not be reached",
            Self::RetryLater => "the server asked to be left alone for now",
            Self::UnsupportedType => "ATENA does not read this content type",
            Self::TooLarge => "the content is larger than ATENA reads",
        }
    }
}

impl From<&FetchError> for Problem {
    fn from(e: &FetchError) -> Self {
        match e {
            FetchError::Robots { .. } => Self::Robots,
            FetchError::RobotsUnreachable { .. } => Self::RobotsUnreachable,
            FetchError::Blocked { .. } => Self::Blocked,
            FetchError::Status { .. } => Self::Status,
            FetchError::RetryLater { .. } => Self::RetryLater,
            FetchError::UnsupportedType { .. } => Self::UnsupportedType,
            FetchError::TooLarge { .. } => Self::TooLarge,
            _ => Self::Unreachable,
        }
    }
}

/// Why the proposal is what it is – codes for the user interface and for reviewers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reason {
    /// The item could not be read.
    CannotRead,
    /// The page is about people (galleries, staff, contacts).
    PersonalPage,
    /// The terms forbid automated access.
    AutomationBan,
    /// The terms allow access only for personal use.
    PersonalUseOnly,
    /// The terms reserve text and data mining or AI use.
    MiningReserved,
    /// TDMRep reserves text and data mining.
    TdmReserved,
    /// robots.txt refuses crawlers that collect for AI training.
    AiCrawlersRefused,
    /// An accepted open licence.
    OpenLicence,
    /// Licences that contradict each other, or one ATENA does not accept as open.
    LicenceNotOpen,
    /// Reuse only for non-commercial purposes.
    NonCommercial,
    /// A plain copyright notice.
    RightsReserved,
    /// Terms of use were read.
    TermsRead,
    /// No terms of use were found.
    NoTermsFound,
    /// Terms of use were found but could not be read.
    TermsUnreadable,
    /// The country could not be told from the address; a person must name it.
    UnknownCountry,
}

/// What the assessment proposes for the register.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    /// `licence` value: `copyrighted`, or an accepted open licence (never `official`: that is a
    /// person's call).
    pub licence: String,
    /// `terms` value: `allowed`, `forbidden`, `reserved` or `unknown`.
    pub terms: String,
    /// The rights record to write with it.
    pub rights: Rights,
    /// Why.
    pub reasons: Vec<Reason>,
}

/// The result of assessing one item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assessment {
    /// Line of the input.
    pub line: usize,
    /// What the line pointed to.
    pub target: Target,
    /// The address actually read (after redirects).
    pub url: Url,
    /// What it is, when it could be read.
    pub kind: Option<Kind>,
    /// Its title, if it has one.
    pub title: Option<String>,
    /// Country whose law applies: as given by the user, or told by the host name.
    pub country: Option<String>,
    /// Day of the assessment, `YYYY-MM-DD`.
    pub checked: String,
    /// Why the item could not be read.
    pub problem: Option<Problem>,
    /// The page is about people.
    pub personal_page: bool,
    /// TDMRep reserves mining for it.
    pub tdm_reserved: bool,
    /// robots.txt refuses AI crawlers for it.
    pub refused_to_ai_crawlers: bool,
    /// Licences found.
    pub licences: Vec<LicenceHint>,
    /// Terms-of-use pages read.
    pub legal_pages: Vec<Url>,
    /// Terms-of-use pages that could not be read.
    pub unreadable_legal_pages: Vec<Url>,
    /// What the pages say.
    pub findings: Vec<Finding>,
    /// The proposal.
    pub proposal: Proposal,
}

/// Today in UTC, `YYYY-MM-DD`.
fn today() -> String {
    Timestamp::now().to_zoned(TimeZone::UTC).date().to_string()
}

/// Reads items and proposes their licence and terms. Cheap to clone; share one fetcher.
#[derive(Debug, Clone)]
pub struct Assessor {
    fetcher: Arc<Fetcher>,
    phrases: &'static Phrases,
}

/// What was read from the item before the decision.
#[derive(Default)]
struct Collected {
    url: Option<Url>,
    kind: Option<Kind>,
    title: Option<String>,
    problem: Option<Problem>,
    tdm_reserved: bool,
    refused_to_ai_crawlers: bool,
    licences: Vec<LicenceHint>,
    legal_pages: Vec<Url>,
    unreadable_legal_pages: Vec<Url>,
    findings: Vec<Finding>,
}

impl Collected {
    fn add_licences(&mut self, hints: impl IntoIterator<Item = LicenceHint>) {
        for hint in hints {
            if !self.licences.iter().any(|h| h.id == hint.id) {
                self.licences.push(hint);
            }
        }
    }
}

/// A link that may lead to terms of use, with how likely that is.
fn legal_link_score(link: &atena_extract::Link, entry: &Url, p: &Phrases) -> u8 {
    if licence_of_url(&link.url).is_some() || link.url == *entry {
        return 0;
    }
    let text = normalize(&link.text);
    let path = normalize(&link.url.path().replace(['-', '_', '/', '.'], " "));
    if first(&text, &p.ignore_links).is_some() || first(&path, &p.ignore_links).is_some() {
        return 0;
    }
    let by_text = first(&text, &p.legal_links).is_some();
    let by_path = first(&path, &p.legal_links).is_some();
    if !by_text && !by_path {
        return 0;
    }
    let same_host = link.url.host_str() == entry.host_str();
    2 * u8::from(by_text) + u8::from(by_path) + u8::from(same_host)
}

impl Assessor {
    /// An assessor using `fetcher` (robots.txt, politeness and address checks included) and the
    /// bundled phrases.
    pub fn new(fetcher: Arc<Fetcher>) -> Self {
        Self {
            fetcher,
            phrases: Phrases::bundled(),
        }
    }

    async fn page(&self, url: &Url) -> Result<Page, FetchError> {
        match self.fetcher.fetch(url, &Validators::default()).await? {
            Fetched::Page(page) => Ok(page),
            // no validators were sent, so this cannot happen; treat it as unreachable
            Fetched::NotModified { url } => Err(FetchError::Status {
                url: url.to_string(),
                status: 304,
            }),
        }
    }

    /// Licence hints and legal findings of one HTML page read as an ordinary page.
    fn read_ordinary(&self, signals: &LegalSignals, page: &Url, c: &mut Collected) {
        c.add_licences(signals.licence_links.iter().filter_map(licence_of_url));
        c.add_licences(signals.links.iter().filter_map(|l| licence_of_url(&l.url)));
        let legal_text: String = signals
            .text
            .split("\n\n")
            .filter(|b| is_legal_block(b, self.phrases))
            .collect::<Vec<_>>()
            .join("\n\n");
        c.add_licences(licences_in_text(&legal_text));
        c.findings
            .extend(analyse(&signals.text, page, self.phrases, true));
    }

    /// Finds the terms-of-use pages linked from `signals` and reads them.
    async fn read_terms(&self, signals: &LegalSignals, entry: &Url, c: &mut Collected) {
        let mut candidates: Vec<(u8, &atena_extract::Link)> = signals
            .links
            .iter()
            .map(|l| (legal_link_score(l, entry, self.phrases), l))
            .filter(|(score, _)| *score > 0)
            .collect();
        candidates.sort_by_key(|c| std::cmp::Reverse(c.0));
        for (_, link) in candidates.into_iter().take(MAX_LEGAL_PAGES) {
            let page = match self.page(&link.url).await {
                Ok(page) => page,
                Err(_) => {
                    c.unreadable_legal_pages.push(link.url.clone());
                    continue;
                }
            };
            let ct = page.content_type.as_deref();
            let text = match extract(&page.body, ct, &page.url) {
                Ok(doc) if doc.format == Format::Html => {
                    let s = legal_signals(&page.body, ct, &page.url);
                    c.add_licences(s.licence_links.iter().filter_map(licence_of_url));
                    s.text
                }
                Ok(doc) => doc.text,
                Err(_) => {
                    c.unreadable_legal_pages.push(link.url.clone());
                    continue;
                }
            };
            c.add_licences(licences_in_text(&text));
            c.findings
                .extend(analyse(&text, &page.url, self.phrases, false));
            c.legal_pages.push(page.url);
        }
    }

    /// Reads a web address: the item itself, then the page that carries the site's legal links
    /// (the item when it is a web page, otherwise the site's home page), then the terms.
    async fn read_url(&self, url: &Url, c: &mut Collected) {
        let entry = match self.page(url).await {
            Ok(page) => page,
            Err(e) => {
                c.problem = Some(Problem::from(&e));
                return;
            }
        };
        c.url = Some(entry.url.clone());
        c.tdm_reserved |= entry.tdm.reserved == Some(true);
        c.refused_to_ai_crawlers |= entry.tdm.refused_to_ai_crawlers;
        let ct = entry.content_type.as_deref();
        let head: String = String::from_utf8_lossy(&entry.body[..entry.body.len().min(1024)])
            .trim_start_matches('\u{feff}')
            .trim_start()
            .to_ascii_lowercase();
        let xmlish = ct.is_some_and(|t| t.contains("xml"))
            || ["<?xml", "<rss", "<feed", "<urlset", "<sitemapindex"]
                .iter()
                .any(|s| head.starts_with(s));
        let mut html_entry = false;
        if head.contains("<oai-pmh") {
            c.kind = Some(Kind::Oai);
        } else if xmlish && let Ok(feed) = parse_feed(&entry.body, &entry.url) {
            c.kind = Some(Kind::Feed);
            c.title = feed.title;
        } else if xmlish
            && let Ok(map) = parse_sitemap(&entry.body, &entry.url)
            && !(map.urls.is_empty() && map.sitemaps.is_empty())
        {
            c.kind = Some(Kind::Sitemap);
        } else {
            match extract(&entry.body, ct, &entry.url) {
                Ok(doc) => {
                    c.tdm_reserved |= doc.tdm_reservation == Some(true);
                    c.title = doc.title.clone();
                    html_entry = doc.format == Format::Html;
                    c.kind = Some(if html_entry {
                        Kind::Page
                    } else {
                        Kind::Document
                    });
                    if !html_entry {
                        c.add_licences(licences_in_text(
                            &doc.text.chars().take(20_000).collect::<String>(),
                        ));
                    }
                }
                Err(_) => {
                    c.problem = Some(Problem::UnsupportedType);
                    return;
                }
            }
        }

        let signals = if html_entry {
            Some(legal_signals(&entry.body, ct, &entry.url))
        } else {
            // feeds, sitemaps, interfaces and documents: the site's home page speaks for them
            let mut home = entry.url.clone();
            home.set_path("/");
            home.set_query(None);
            match self.page(&home).await {
                Ok(page) => Some(legal_signals(
                    &page.body,
                    page.content_type.as_deref(),
                    &page.url,
                )),
                Err(_) => None,
            }
        };
        let Some(signals) = signals else {
            return;
        };
        self.read_ordinary(&signals, &entry.url, c);
        self.read_terms(&signals, &entry.url, c).await;
        // an inner page without legal links: try the home page's
        if c.legal_pages.is_empty()
            && c.unreadable_legal_pages.is_empty()
            && entry.url.path() != "/"
            && html_entry
        {
            let mut home = entry.url.clone();
            home.set_path("/");
            home.set_query(None);
            if let Ok(page) = self.page(&home).await {
                let s = legal_signals(&page.body, page.content_type.as_deref(), &page.url);
                self.read_ordinary(&s, &page.url, c);
                self.read_terms(&s, &page.url, c).await;
            }
        }
    }

    /// Reads a DOI: its Crossref record (licences of the version of record), then the
    /// publisher's landing page like any web page.
    async fn read_doi(&self, doi: &str, c: &mut Collected) {
        if let Ok(api) = Url::parse(&format!("https://api.crossref.org/works/{doi}"))
            && let Ok(page) = self.page(&api).await
            && let Some((title, licences)) = crossref_record(&page.body)
        {
            c.title = title;
            c.add_licences(licences);
        }
        let title = c.title.take();
        let landing = Target::Doi(doi.to_string()).url();
        self.read_url(&landing, c).await;
        if title.is_some() {
            c.title = title;
        }
        if c.problem.is_none() {
            c.kind = Some(Kind::Doi);
        }
    }

    /// Assesses one item.
    pub async fn assess(&self, item: &Item) -> Assessment {
        let mut c = Collected::default();
        match &item.target {
            Target::Url(url) => self.read_url(url, &mut c).await,
            Target::Doi(doi) => self.read_doi(doi, &mut c).await,
        }
        let url = c.url.clone().unwrap_or_else(|| item.target.url());
        let country = item
            .country
            .clone()
            .filter(|code| atena_core::legality::profile(code).is_some())
            .or_else(|| url.host_str().and_then(country_of_host).map(str::to_string));
        let mut assessment = Assessment {
            line: item.line,
            target: item.target.clone(),
            personal_page: atena_core::pages::is_personal_path(url.path()),
            url,
            kind: c.kind,
            title: c.title,
            country,
            checked: today(),
            problem: c.problem,
            tdm_reserved: c.tdm_reserved,
            refused_to_ai_crawlers: c.refused_to_ai_crawlers,
            licences: c.licences,
            legal_pages: c.legal_pages,
            unreadable_legal_pages: c.unreadable_legal_pages,
            findings: c.findings,
            proposal: Proposal {
                licence: String::new(),
                terms: String::new(),
                rights: Rights {
                    checked: String::new(),
                    page: String::new(),
                    finding: String::new(),
                },
                reasons: Vec::new(),
            },
        };
        assessment.proposal = propose(&assessment);
        assessment
    }

    /// Assesses items, `concurrency` at a time (each host is still asked one request at a time),
    /// in input order.
    pub async fn assess_all(&self, items: &[Item], concurrency: usize) -> Vec<Assessment> {
        let mut out = Vec::with_capacity(items.len());
        for chunk in items.chunks(concurrency.max(1)) {
            let mut set = tokio::task::JoinSet::new();
            for item in chunk {
                let (me, item) = (self.clone(), item.clone());
                set.spawn(async move { me.assess(&item).await });
            }
            while let Some(done) = set.join_next().await {
                if let Ok(a) = done {
                    out.push(a);
                }
            }
        }
        out.sort_by_key(|a| a.line);
        out
    }
}

/// Title and licences of the version of record from a Crossref `works` response.
pub fn crossref_record(body: &[u8]) -> Option<(Option<String>, Vec<LicenceHint>)> {
    let record: serde_json::Value = serde_json::from_slice(body).ok()?;
    let message = record.get("message")?;
    let title = message["title"][0].as_str().map(str::to_string);
    let licences = message["license"]
        .as_array()
        .map(|all| {
            all.iter()
                .filter(|l| {
                    matches!(
                        l["content-version"].as_str().unwrap_or("unspecified"),
                        "vor" | "unspecified"
                    )
                })
                .filter_map(|l| Url::parse(l["URL"].as_str()?).ok())
                .filter_map(|u| licence_of_url(&u))
                .collect()
        })
        .unwrap_or_default();
    Some((title, licences))
}

/// The first finding of a kind.
fn quote_of(a: &Assessment, kind: FindingKind) -> Option<&Finding> {
    a.findings.iter().find(|f| f.kind == kind)
}

fn quoted(f: &Finding) -> String {
    format!("\"{}\" ({})", f.quote, f.page)
}

/// The proposal for an assessment: the strictest outcome its evidence calls for.
pub fn propose(a: &Assessment) -> Proposal {
    let mut reasons = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let page = a.legal_pages.first().unwrap_or(&a.url).to_string();
    let has = |kind| a.findings.iter().any(|f| f.kind == kind);
    let mut licence = "copyrighted".to_string();

    let terms =
        if let Some(problem) = a.problem {
            reasons.push(Reason::CannotRead);
            notes.push(format!(
                "the address could not be read: {}",
                problem.describe()
            ));
            "unknown"
        } else if a.personal_page {
            reasons.push(Reason::PersonalPage);
            notes.push("the page is about people; ATENA does not take such pages".into());
            "forbidden"
        } else if has(FindingKind::AutomationBan) || has(FindingKind::PersonalUseOnly) {
            let kind = if has(FindingKind::AutomationBan) {
                reasons.push(Reason::AutomationBan);
                FindingKind::AutomationBan
            } else {
                FindingKind::PersonalUseOnly
            };
            if has(FindingKind::PersonalUseOnly) {
                reasons.push(Reason::PersonalUseOnly);
            }
            if let Some(f) = quote_of(a, kind) {
                notes.push(format!(
                    "the terms do not allow ATENA's access: {}",
                    quoted(f)
                ));
            }
            "forbidden"
        } else if has(FindingKind::MiningReserved) || a.tdm_reserved || a.refused_to_ai_crawlers {
            if let Some(f) = quote_of(a, FindingKind::MiningReserved) {
                reasons.push(Reason::MiningReserved);
                notes.push(format!("text and data mining is reserved: {}", quoted(f)));
            }
            if a.tdm_reserved {
                reasons.push(Reason::TdmReserved);
                notes.push("TDMRep reserves text and data mining".into());
            }
            if a.refused_to_ai_crawlers {
                reasons.push(Reason::AiCrawlersRefused);
                notes.push("robots.txt refuses crawlers that collect for AI training".into());
            }
            "reserved"
        } else if a.legal_pages.is_empty() && !a.unreadable_legal_pages.is_empty() {
            reasons.push(Reason::TermsUnreadable);
            notes.push(format!(
                "the terms of use could not be read ({})",
                a.unreadable_legal_pages
                    .iter()
                    .map(Url::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            "unknown"
        } else {
            if a.legal_pages.is_empty() {
                reasons.push(Reason::NoTermsFound);
                notes.push("no terms of use found".into());
            } else {
                reasons.push(Reason::TermsRead);
                notes.push(format!(
                "terms read ({}); no ban on automated access and no reservation of mining found",
                a.legal_pages.iter().map(Url::as_str).collect::<Vec<_>>().join(", ")
            ));
            }
            let open: Vec<&LicenceHint> = a.licences.iter().filter(|h| h.open).collect();
            let not_open = a.licences.iter().any(|h| !h.open) || has(FindingKind::NonCommercial);
            if open.len() == 1 && !not_open {
                licence = open[0].id.clone();
                reasons.push(Reason::OpenLicence);
                notes.push(format!("licence {} ({})", open[0].id, open[0].evidence));
            } else if !a.licences.is_empty() {
                reasons.push(Reason::LicenceNotOpen);
                notes.push(format!(
                    "licences found: {} – not one accepted open licence",
                    a.licences
                        .iter()
                        .map(|h| h.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if has(FindingKind::NonCommercial) {
                reasons.push(Reason::NonCommercial);
                notes.push("reuse only for non-commercial purposes".into());
            }
            if let Some(f) = quote_of(a, FindingKind::RightsReserved) {
                reasons.push(Reason::RightsReserved);
                notes.push(format!("copyright notice {}", quoted(f)));
            }
            "allowed"
        };
    if a.country.is_none() {
        reasons.push(Reason::UnknownCountry);
        notes.push("the country could not be told from the address".into());
    }
    let mut finding = format!("Automatic assessment: {}.", notes.join("; "));
    if finding.chars().count() > 900 {
        finding = finding.chars().take(899).collect::<String>() + "…";
    }
    Proposal {
        licence,
        terms: terms.to_string(),
        rights: Rights {
            checked: a.checked.clone(),
            page,
            finding,
        },
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossref_licences_of_the_version_of_record_only() {
        let body = br#"{"status":"ok","message":{"title":["Montessori education"],
            "license":[
              {"URL":"http://creativecommons.org/licenses/by/4.0/","content-version":"vor"},
              {"URL":"http://creativecommons.org/licenses/by-nc/4.0/","content-version":"am"},
              {"URL":"https://www.elsevier.com/tdm/userlicense/1.0/","content-version":"tdm"}]}}"#;
        let (title, licences) = crossref_record(body).unwrap();
        assert_eq!(title.as_deref(), Some("Montessori education"));
        assert_eq!(
            licences.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(),
            ["CC-BY-4.0"]
        );
        assert!(crossref_record(b"not json").is_none());
    }
}
