//! What a publisher allows ATENA to do with a page, beyond whether it may be fetched at all.
//!
//! * **TDMRep** (W3C TDM Reservation Protocol): the rights holder may reserve text and data
//!   mining (EU Directive 2019/790, Art. 4). Sources in processing order – later ones override
//!   earlier ones, and a missing value never resets one already found: `/.well-known/tdmrep.json`,
//!   the HTTP headers `tdm-reservation` / `tdm-policy`, then HTML `<meta>` and PDF/EPUB metadata
//!   (applied by the extractor through [`Tdm::apply`]).
//! * **Indexing rules** from `X-Robots-Tag` headers (and later `<meta name="robots">`):
//!   `noindex`, `nofollow`, `noarchive`, `nosnippet`, `notranslate`.

use serde::Deserialize;

use crate::pattern::{Pattern, normalize_path};

/// The TDM reservation of one resource. `reserved: None` means "unset" – the protocol is not used.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tdm {
    /// `Some(true)` when text and data mining is reserved by the rights holder.
    pub reserved: Option<bool>,
    /// URL of the licensing policy, if one is published.
    pub policy: Option<String>,
    /// robots.txt refuses the page to crawlers that collect for AI training and mining
    /// ([`crate::robots::MINING_AGENTS`]). No later source can lift this.
    pub refused_to_ai_crawlers: bool,
}

impl Tdm {
    /// `true` when mining is reserved by any means: TDMRep or robots.txt.
    pub fn is_reserved(&self) -> bool {
        self.reserved == Some(true) || self.refused_to_ai_crawlers
    }

    /// Applies values from a later source; absent values keep the current ones.
    pub fn apply(&mut self, reserved: Option<bool>, policy: Option<String>) {
        if reserved.is_some() {
            self.reserved = reserved;
        }
        if policy.is_some() {
            self.policy = policy;
        }
    }

    /// Applies the `tdm-reservation` and `tdm-policy` HTTP header values.
    pub fn apply_headers(&mut self, reservation: Option<&str>, policy: Option<&str>) {
        self.apply(
            reservation.and_then(parse_reservation),
            policy
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_string),
        );
    }
}

/// `1` reserves, `0` does not; any other value is a protocol error and counts as unset.
pub fn parse_reservation(value: &str) -> Option<bool> {
    match value.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

#[derive(Deserialize)]
struct RawRule {
    location: String,
    #[serde(rename = "tdm-reservation")]
    reservation: serde_json::Value,
    #[serde(rename = "tdm-policy", default)]
    policy: Option<String>,
}

#[derive(Debug, Clone)]
struct TdmRule {
    location: Pattern,
    reserved: Option<bool>,
    policy: Option<String>,
}

/// The rules of an origin's `/.well-known/tdmrep.json`.
#[derive(Debug, Clone, Default)]
pub struct TdmFile {
    rules: Vec<TdmRule>,
}

impl TdmFile {
    /// Parses the file; `None` when it is not a valid TDMRep file (the origin then does not
    /// implement the protocol). Individual malformed rules are skipped.
    pub fn parse(json: &[u8]) -> Option<Self> {
        let raw: Vec<serde_json::Value> = serde_json::from_slice(json).ok()?;
        let rules = raw
            .into_iter()
            .filter_map(|v| serde_json::from_value::<RawRule>(v).ok())
            .map(|r| TdmRule {
                location: Pattern::new(&r.location),
                reserved: match r.reservation {
                    serde_json::Value::Number(n) => parse_reservation(&n.to_string()),
                    serde_json::Value::Bool(b) => Some(b),
                    _ => None,
                },
                policy: r.policy,
            })
            .collect();
        Some(Self { rules })
    }

    /// The reservation for a path with query: the first matching rule applies, none means unset.
    pub fn lookup(&self, path_and_query: &str) -> Tdm {
        let path = normalize_path(path_and_query);
        self.rules
            .iter()
            .find(|r| r.location.matches(&path))
            .map(|r| Tdm {
                reserved: r.reserved,
                policy: r.reserved.and(r.policy.clone()),
                refused_to_ai_crawlers: false,
            })
            .unwrap_or_default()
    }
}

/// Indexing rules a publisher set for crawlers (union of all applicable directives).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexRules {
    /// Do not show the page in search results.
    pub noindex: bool,
    /// Do not follow links on the page.
    pub nofollow: bool,
    /// Do not keep or show a stored copy.
    pub noarchive: bool,
    /// Do not show text excerpts.
    pub nosnippet: bool,
    /// Do not offer a translation.
    pub notranslate: bool,
}

/// Directives that take a value after a colon and must not be mistaken for a crawler name.
const VALUED: [&str; 4] = [
    "unavailable_after",
    "max-snippet",
    "max-image-preview",
    "max-video-preview",
];

impl IndexRules {
    /// Adds a comma-separated directive list such as `noindex, nofollow` (also a `<meta>` content).
    pub fn add_list(&mut self, list: &str) {
        for directive in list.split(',') {
            match directive.trim().to_ascii_lowercase().as_str() {
                "noindex" => self.noindex = true,
                "nofollow" => self.nofollow = true,
                "none" => {
                    self.noindex = true;
                    self.nofollow = true;
                }
                // `nocache` is Bing's name for `noarchive`
                "noarchive" | "nocache" => self.noarchive = true,
                "nosnippet" => self.nosnippet = true,
                "notranslate" => self.notranslate = true,
                _ => {}
            }
        }
    }

    /// Adds one `X-Robots-Tag` header value. A value may be addressed to one crawler
    /// (`otherbot: noindex`); it then applies only when that is our product token.
    pub fn add_header(&mut self, value: &str, product_token: &str) {
        if let Some((name, rest)) = value.split_once(':') {
            let name = name.trim();
            let is_crawler = !name.is_empty()
                && !VALUED.contains(&name.to_ascii_lowercase().as_str())
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ' '));
            if is_crawler {
                if name.eq_ignore_ascii_case(product_token) {
                    self.add_list(rest);
                }
                return;
            }
        }
        self.add_list(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tdm_file_first_matching_rule_applies() {
        let file = TdmFile::parse(
            br#"[
              {"location": "/images/*", "tdm-reservation": 1, "tdm-policy": "https://pub.example/policy.json"},
              {"location": "/open/", "tdm-reservation": 0},
              {"location": "/broken/", "tdm-reservation": 7},
              {"location": "/*", "tdm-reservation": 1},
              {"no-location": true}
            ]"#,
        )
        .unwrap();
        let images = file.lookup("/images/cat.png");
        assert_eq!(images.reserved, Some(true));
        assert_eq!(
            images.policy.as_deref(),
            Some("https://pub.example/policy.json")
        );
        assert_eq!(file.lookup("/open/doc.html").reserved, Some(false));
        assert_eq!(file.lookup("/broken/x").reserved, None);
        assert_eq!(file.lookup("/other").reserved, Some(true));
        assert!(TdmFile::parse(b"<html>not json</html>").is_none());
        assert!(TdmFile::parse(b"{}").is_none());
        assert_eq!(TdmFile::parse(b"[]").unwrap().lookup("/x"), Tdm::default());
    }

    #[test]
    fn later_sources_override_but_never_reset() {
        let mut tdm = TdmFile::parse(br#"[{"location": "/*", "tdm-reservation": 0}]"#)
            .unwrap()
            .lookup("/a");
        tdm.apply_headers(Some(" 1 "), Some("https://pub.example/p"));
        assert_eq!(tdm.reserved, Some(true));
        tdm.apply_headers(None, None);
        tdm.apply_headers(Some("yes"), None);
        assert_eq!(tdm.reserved, Some(true));
        assert_eq!(tdm.policy.as_deref(), Some("https://pub.example/p"));
        tdm.apply(Some(false), None);
        assert_eq!(tdm.reserved, Some(false));
    }

    #[test]
    fn x_robots_tag() {
        let mut r = IndexRules::default();
        r.add_header("noarchive, nosnippet", "ATENA");
        r.add_header("googlebot: noindex", "ATENA");
        r.add_header("unavailable_after: 25 Jun 2030 15:00:00 PST", "ATENA");
        assert_eq!(
            r,
            IndexRules {
                noarchive: true,
                nosnippet: true,
                ..IndexRules::default()
            }
        );
        r.add_header("atena: none", "ATENA");
        r.add_list("NoTranslate");
        assert!(r.noindex && r.nofollow && r.notranslate);
    }
}
