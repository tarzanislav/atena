//! The source register (`data/sources.yaml`): who publishes, where and how often ATENA looks.

use std::collections::HashSet;

use serde::Deserialize;

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
        assert_eq!(sources.len(), 186);
        let nivam = sources.iter().find(|s| s.id == "SK07").unwrap();
        assert_eq!(nivam.feed.as_deref(), Some("https://nivam.sk/feed/"));
        assert_eq!(nivam.interval_s, 3600);
        assert!(sources.iter().filter(|s| s.feed.is_some()).count() >= 10);
    }
}
