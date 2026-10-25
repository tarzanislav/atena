//! The source register (`data/sources.yaml`): who publishes, where and how often ATENA looks.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::taxonomy::Taxonomy;

/// The register shipped with ATENA, embedded at build time.
pub const BUNDLED_SOURCES: &str = include_str!("../../../data/sources.yaml");

/// How the review of the v0.1 catalogue treated a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Review {
    /// Kept as proposed.
    Keep,
    /// Kept with a changed connector, address or priority.
    Adjusted,
    /// Another entry point of a publisher listed elsewhere.
    MergedEndpoint,
    /// Not updated any more: captured once, not monitored.
    Archive,
    /// Added in the review.
    New,
}

/// How far the crawler goes from a source's entry page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrawlScope {
    /// The entry page, its attachments and the pages below it (one level).
    #[default]
    Section,
    /// Any page of the publisher's site up to [`SITE_DEPTH`] links from the entry page – only for
    /// official sources (A1, A2) whose terms allow it.
    Site,
}

/// Links followed from the entry page of a source crawled site-wide.
pub const SITE_DEPTH: i16 = 3;

/// The connector of a tool: a dictionary, corpus or application that is linked and described,
/// never crawled (its content is behind a search form or an application).
pub const TOOL: &str = "tool";

/// Open licences accepted in the register (SPDX identifiers and their IGO variants). Each allows
/// copying, adaptation and commercial use with attribution at most.
pub const OPEN_LICENCES: [&str; 10] = [
    "CC0-1.0",
    "CC-BY-4.0",
    "CC-BY-SA-4.0",
    "CC-BY-3.0-IGO",
    "CC-BY-SA-3.0-IGO",
    "PDDL-1.0",
    "ODC-By-1.0",
    "ODbL-1.0",
    "OGL-UK-3.0",
    "etalab-2.0",
];

/// Under what terms a source's content may be shown.
///
/// ATENA may analyse publicly accessible content unless the publisher reserves text and data
/// mining (Art. 4 of Directive (EU) 2019/790). Showing it is a different matter: the full text is
/// shown only for official texts (laws and other official documents, which copyright does not
/// protect, e.g. § 5 of the Slovak Copyright Act) and for openly licensed content; everything else
/// is shown as title, metadata, ATENA's classification and a link to the original.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Licence {
    /// Official texts (legal acts, official documents).
    Official,
    /// Protected by copyright – the default until the licence is verified.
    #[default]
    Copyrighted,
    /// An open licence from [`OPEN_LICENCES`].
    Open(String),
}

impl TryFrom<String> for Licence {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        match text.as_str() {
            "official" => Ok(Self::Official),
            "copyrighted" => Ok(Self::Copyrighted),
            open if OPEN_LICENCES.contains(&open) => Ok(Self::Open(text)),
            other => Err(format!(
                "unknown licence {other} (official, copyrighted or one of {OPEN_LICENCES:?})"
            )),
        }
    }
}

impl Licence {
    /// A value read back from the database; anything unexpected is `Copyrighted`, the strictest.
    pub fn parse(text: &str) -> Self {
        Self::try_from(text.to_string()).unwrap_or_default()
    }

    /// The value stored in the register and the database.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Official => "official",
            Self::Copyrighted => "copyrighted",
            Self::Open(id) => id,
        }
    }

    /// `true` when the full text may be shown to users.
    pub fn allows_full_text(&self) -> bool {
        !matches!(self, Self::Copyrighted)
    }
}

/// What the publisher's terms of use say about automated access and text and data mining.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Terms {
    /// The terms were read: they neither forbid automated access nor reserve text and data mining.
    /// Silent terms and plain copyright notices ("all rights reserved") are not a reservation in
    /// the sense of Art. 4(3) of Directive (EU) 2019/790.
    Allowed,
    /// The terms forbid automated access (robots, scrapers, indexing) or allow access only for
    /// personal use: the source is not crawled at all.
    Forbidden,
    /// The terms reserve text and data mining or AI use: metadata only.
    Reserved,
    /// Not verified; robots.txt and TDMRep decide, and only metadata is kept.
    #[default]
    Unknown,
}

impl Terms {
    /// A value read back from the database; anything unexpected is `Unknown`.
    pub fn parse(text: &str) -> Self {
        match text {
            "allowed" => Self::Allowed,
            "forbidden" => Self::Forbidden,
            "reserved" => Self::Reserved,
            _ => Self::Unknown,
        }
    }

    /// The value stored in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Forbidden => "forbidden",
            Self::Reserved => "reserved",
            Self::Unknown => "unknown",
        }
    }
}

/// The record of a rights review: where and when the licence and terms of a source were read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rights {
    /// Day of the review, `YYYY-MM-DD`.
    pub checked: String,
    /// The page that states the licence or terms (for official texts, the law that frees them).
    pub page: String,
    /// What it says, briefly and with quotes where they matter.
    pub finding: String,
}

/// One source.
#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    /// Stable ID, e.g. `SK07`.
    pub id: String,
    /// Publisher name in its own language.
    pub name: String,
    /// Entry address.
    pub url: String,
    /// Country or region (`SK`, `EU`, `WORLD` …) – a concept of the `country` scheme.
    pub country: String,
    /// Languages of the content.
    pub languages: Vec<String>,
    /// Authority level `A1`…`E` – a concept of the `authority` scheme.
    pub authority: String,
    /// Rollout priority `P0`…`P3`.
    pub priority: String,
    /// Connector kind (`rss`, `wp`, `html`, `api`, `oai`, `slovlex`, …).
    pub connector: String,
    /// Check interval in seconds.
    pub interval_s: u64,
    /// Outcome of the catalogue review.
    pub review: Review,
    /// Verified feed address, if any.
    #[serde(default)]
    pub feed: Option<String>,
    /// Licence of the content (default: copyrighted).
    #[serde(default)]
    pub licence: Licence,
    /// The terms of use on automated access and mining (default: unknown).
    #[serde(default)]
    pub terms: Terms,
    /// The rights review behind `licence` and `terms`; required unless both are the defaults.
    #[serde(default)]
    pub rights: Option<Rights>,
    /// How far the crawler goes from the entry page.
    #[serde(default)]
    pub scope: CrawlScope,
    /// What the source is, by language (shown with tools; written by ATENA, CC0).
    #[serde(default)]
    pub note: BTreeMap<String, String>,
}

impl Source {
    /// A tool is linked and described, never crawled.
    pub fn is_tool(&self) -> bool {
        self.connector == TOOL
    }
}

#[derive(Deserialize)]
struct RawRegister {
    sources: Vec<Source>,
}

/// Errors in the register.
#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    /// The YAML could not be parsed.
    #[error("invalid source register YAML: {0}")]
    Yaml(#[from] serde_saphyr::Error),
    /// Two sources share an ID.
    #[error("duplicate source ID {0}")]
    Duplicate(String),
    /// A field refers to a concept the taxonomy does not know.
    #[error("source {id}: unknown {scheme} {value}")]
    Unknown {
        /// Source ID.
        id: String,
        /// Taxonomy scheme.
        scheme: &'static str,
        /// The unknown value.
        value: String,
    },
    /// The address is not http(s).
    #[error("source {0}: the address must start with http:// or https://")]
    Address(String),
    /// The entry conflicts with the legality principle (see `legality`).
    #[error("{0}")]
    Legality(String),
    /// Only official sources whose terms allow it are crawled site-wide.
    #[error(
        "source {0}: only official sources (A1, A2) with terms `allowed` may be crawled site-wide"
    )]
    Scope(String),
    /// A tool needs a Slovak description and cannot be crawled site-wide.
    #[error("source {0}: a tool needs a description (note.sk) and is never crawled")]
    Tool(String),
}

/// Loads the register and checks it against the taxonomy.
pub fn load_sources(yaml: &str, taxonomy: &Taxonomy) -> Result<Vec<Source>, RegisterError> {
    let raw: RawRegister = serde_saphyr::from_str(yaml)?;
    let mut seen = HashSet::new();
    for s in &raw.sources {
        if !seen.insert(s.id.clone()) {
            return Err(RegisterError::Duplicate(s.id.clone()));
        }
        for (scheme, value) in [("authority", &s.authority), ("country", &s.country)] {
            if taxonomy.get(scheme, value).is_none() {
                return Err(RegisterError::Unknown {
                    id: s.id.clone(),
                    scheme,
                    value: value.clone(),
                });
            }
        }
        let ok = |u: &str| u.starts_with("https://") || u.starts_with("http://");
        if !ok(&s.url) || s.feed.as_deref().is_some_and(|f| !ok(f)) {
            return Err(RegisterError::Address(s.id.clone()));
        }
        crate::legality::check_source(s).map_err(RegisterError::Legality)?;
        let official = matches!(s.authority.as_str(), "A1" | "A2");
        if s.scope == CrawlScope::Site && (!official || s.terms != Terms::Allowed || s.is_tool()) {
            return Err(RegisterError::Scope(s.id.clone()));
        }
        if s.is_tool() && (s.note.get("sk").is_none_or(|n| n.trim().is_empty()) || s.feed.is_some())
        {
            return Err(RegisterError::Tool(s.id.clone()));
        }
    }
    Ok(raw.sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_register_is_consistent() {
        let t = Taxonomy::bundled().unwrap();
        let sources = load_sources(BUNDLED_SOURCES, &t).expect("valid register");
        assert_eq!(sources.len(), 218);
        let nivam = sources.iter().find(|s| s.id == "SK07").unwrap();
        assert_eq!(nivam.feed.as_deref(), Some("https://nivam.sk/feed/"));
        assert_eq!(nivam.interval_s, 3600);
        assert!(sources.iter().filter(|s| s.feed.is_some()).count() >= 10);
        let slovlex = sources.iter().find(|s| s.id == "SK03").unwrap();
        assert!(slovlex.licence.allows_full_text());
        assert_eq!(nivam.licence, Licence::Copyrighted);
        assert_eq!(nivam.terms, Terms::Allowed);
        let youtube = sources.iter().find(|s| s.id == "N39").unwrap();
        assert_eq!(youtube.terms, Terms::Forbidden);
        // every source went through the rights review
        assert!(sources.iter().all(|s| s.rights.is_some()));
        // official sources crawled site-wide; tools are described, never crawled
        let site: Vec<&str> = sources
            .iter()
            .filter(|s| s.scope == CrawlScope::Site)
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(site, ["SK04", "SK07", "SK10"]);
        let tools: Vec<&Source> = sources.iter().filter(|s| s.is_tool()).collect();
        assert_eq!(tools.len(), 9);
        assert!(
            tools
                .iter()
                .all(|t| t.note.contains_key("sk") && t.note.contains_key("en"))
        );
    }

    #[test]
    fn site_wide_scope_and_tools_are_checked() {
        let t = Taxonomy::bundled().unwrap();
        let entry = |authority: &str, connector: &str, terms: &str, extra: &str| {
            format!(
                "sources:\n- id: X1\n  name: X\n  url: https://x.example/\n  country: SK\n  languages: [sk]\n  \
                 authority: {authority}\n  priority: P0\n  connector: {connector}\n  interval_s: 3600\n  review: new\n  \
                 terms: {terms}\n{extra}  rights:\n    checked: 2026-10-25\n    page: https://x.example/\n    finding: Read.\n"
            )
        };
        let site = "  scope: site\n";
        assert!(load_sources(&entry("A2", "html", "allowed", site), &t).is_ok());
        for (authority, terms) in [("C1", "allowed"), ("A2", "unknown"), ("A1", "reserved")] {
            assert!(matches!(
                load_sources(&entry(authority, "html", terms, site), &t),
                Err(RegisterError::Scope(_))
            ));
        }
        let described = "  note:\n    sk: Slovník.\n";
        assert!(load_sources(&entry("B2", "tool", "unknown", described), &t).unwrap()[0].is_tool());
        assert!(matches!(
            load_sources(&entry("B2", "tool", "unknown", ""), &t),
            Err(RegisterError::Tool(_))
        ));
    }

    #[test]
    fn licences_are_validated() {
        let t = Taxonomy::bundled().unwrap();
        let entry = |licence: &str| {
            format!(
                "sources:\n- id: X1\n  name: X\n  url: https://x.example/\n  country: SK\n  languages: [sk]\n  \
                 authority: A2\n  priority: P0\n  connector: html\n  interval_s: 3600\n  review: new\n  licence: {licence}\n  \
                 rights:\n    checked: 2026-10-24\n    page: https://x.example/licence\n    finding: Open licence.\n"
            )
        };
        let open = load_sources(&entry("CC-BY-4.0"), &t).unwrap();
        assert_eq!(open[0].licence, Licence::Open("CC-BY-4.0".into()));
        assert_eq!(open[0].licence.as_str(), "CC-BY-4.0");
        assert!(load_sources(&entry("all-rights-mine"), &t).is_err());
        assert!(!Licence::Copyrighted.allows_full_text());
    }
}
