//! The multilingual taxonomy (`data/taxonomy.yaml`): schemes (facets) of concepts with stable English IDs.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Deserialize;

use crate::text::{fold, has_cjk, tokens};

/// The taxonomy shipped with ATENA, embedded at build time.
pub const BUNDLED_TAXONOMY: &str = include_str!("../../../data/taxonomy.yaml");

/// Errors found while loading or validating a taxonomy.
#[derive(Debug, thiserror::Error)]
pub enum TaxonomyError {
    /// The YAML could not be parsed.
    #[error("invalid taxonomy YAML: {0}")]
    Yaml(#[from] serde_saphyr::Error),
    /// A concept ID occurs twice within one scheme.
    #[error("duplicate concept {scheme}:{id}")]
    Duplicate {
        /// Scheme ID.
        scheme: String,
        /// Concept ID.
        id: String,
    },
    /// A concept points to a broader concept that does not exist.
    #[error("concept {scheme}:{id} has unknown broader concept {broader}")]
    UnknownBroader {
        /// Scheme ID.
        scheme: String,
        /// Concept ID.
        id: String,
        /// The missing broader ID.
        broader: String,
    },
    /// A concept has no label in the reference language.
    #[error("concept {scheme}:{id} has no label in the reference language {lang}")]
    MissingReferenceLabel {
        /// Scheme ID.
        scheme: String,
        /// Concept ID.
        id: String,
        /// Reference language code.
        lang: String,
    },
}

#[derive(Debug, Deserialize)]
struct RawTaxonomy {
    version: u32,
    reference_language: String,
    core_languages: Vec<String>,
    languages: Vec<String>,
    schemes: BTreeMap<String, RawScheme>,
}

#[derive(Debug, Deserialize)]
struct RawScheme {
    labels: BTreeMap<String, String>,
    #[serde(default)]
    note: BTreeMap<String, String>,
    concepts: Vec<RawConcept>,
}

#[derive(Debug, Deserialize)]
struct RawConcept {
    id: String,
    #[serde(default)]
    broader: Option<String>,
    labels: BTreeMap<String, String>,
    #[serde(default)]
    note: BTreeMap<String, String>,
    #[serde(default)]
    isced: Vec<String>,
    #[serde(default)]
    keywords: BTreeMap<String, Vec<String>>,
}

/// One token of a keyword phrase; tokens of four or more characters match as word prefixes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordToken {
    /// Folded text (for long tokens already shortened to a stem).
    pub text: String,
    /// `true` when the token matches the beginning of a word.
    pub prefix: bool,
}

/// A keyword phrase prepared for matching.
#[derive(Debug, Clone)]
pub enum Phrase {
    /// A sequence of word tokens (Latin and Cyrillic scripts).
    Words(Vec<KeywordToken>),
    /// A folded substring (Chinese has no word boundaries).
    Cjk(String),
}

/// A keyword as written in the taxonomy plus its prepared form.
#[derive(Debug, Clone)]
pub struct Keyword {
    /// The keyword as written in `data/taxonomy.yaml` (used as evidence for a suggestion).
    pub raw: String,
    /// Prepared phrase.
    pub phrase: Phrase,
}

/// A concept of one scheme.
#[derive(Debug, Clone)]
pub struct Concept {
    /// Scheme (facet) ID, e.g. `topic`.
    pub scheme: String,
    /// Stable English ID, e.g. `reading-literacy`.
    pub id: String,
    /// Broader concept in the same scheme.
    pub broader: Option<String>,
    /// Labels by language code.
    pub labels: BTreeMap<String, String>,
    /// Editorial notes by language code.
    pub note: BTreeMap<String, String>,
    /// ISCED 2011 levels (school types only).
    pub isced: Vec<String>,
    /// Keywords by language code.
    pub keywords: BTreeMap<String, Vec<Keyword>>,
}

impl Concept {
    /// Label in `lang`, falling back to English and then to any label.
    pub fn label(&self, lang: &str) -> &str {
        self.labels
            .get(lang)
            .or_else(|| self.labels.get("en"))
            .or_else(|| self.labels.values().next())
            .map(String::as_str)
            .unwrap_or(&self.id)
    }
}

/// A scheme (facet) of the taxonomy.
#[derive(Debug, Clone)]
pub struct Scheme {
    /// Scheme ID.
    pub id: String,
    /// Labels by language code.
    pub labels: BTreeMap<String, String>,
    /// Notes by language code.
    pub note: BTreeMap<String, String>,
    /// Concept IDs in the order of the file.
    pub concept_ids: Vec<String>,
}

/// The loaded and validated taxonomy.
#[derive(Debug, Clone)]
pub struct Taxonomy {
    /// Format version of the file.
    pub version: u32,
    /// The single reference language (`sk`).
    pub reference_language: String,
    /// Languages that must be complete before a release.
    pub core_languages: Vec<String>,
    /// All languages the taxonomy may contain.
    pub languages: Vec<String>,
    /// Schemes in file order of their IDs.
    pub schemes: BTreeMap<String, Scheme>,
    concepts: HashMap<(String, String), Concept>,
}

/// Words shorter than 7 characters are matched whole as prefixes; longer ones lose their last two
/// characters, which absorbs case endings (`čítanie` ~ `čítania`, `понимание` ~ `понимания`).
fn keyword_token(token: String) -> KeywordToken {
    let n = token.chars().count();
    if n >= 7 {
        KeywordToken {
            text: token.chars().take(n - 2).collect(),
            prefix: true,
        }
    } else {
        KeywordToken {
            prefix: n >= 4,
            text: token,
        }
    }
}

fn prepare(raw: &str) -> Option<Keyword> {
    let phrase = if has_cjk(raw) {
        Phrase::Cjk(fold(raw))
    } else {
        let toks: Vec<KeywordToken> = tokens(raw).into_iter().map(keyword_token).collect();
        if toks.is_empty() {
            return None;
        }
        Phrase::Words(toks)
    };
    Some(Keyword {
        raw: raw.to_string(),
        phrase,
    })
}

impl Taxonomy {
    /// The taxonomy bundled with ATENA.
    pub fn bundled() -> Result<Self, TaxonomyError> {
        Self::from_yaml(BUNDLED_TAXONOMY)
    }

    /// Parses and validates a taxonomy in the v2 YAML format.
    pub fn from_yaml(text: &str) -> Result<Self, TaxonomyError> {
        let raw: RawTaxonomy = serde_saphyr::from_str(text)?;
        let mut schemes = BTreeMap::new();
        let mut concepts = HashMap::new();
        for (sid, rs) in raw.schemes {
            let mut ids = Vec::new();
            let mut seen = HashSet::new();
            for rc in &rs.concepts {
                if !seen.insert(rc.id.clone()) {
                    return Err(TaxonomyError::Duplicate {
                        scheme: sid.clone(),
                        id: rc.id.clone(),
                    });
                }
            }
            for rc in rs.concepts {
                if let Some(b) = &rc.broader
                    && !seen.contains(b)
                {
                    return Err(TaxonomyError::UnknownBroader {
                        scheme: sid.clone(),
                        id: rc.id.clone(),
                        broader: b.clone(),
                    });
                }
                if !rc.labels.contains_key(&raw.reference_language) {
                    return Err(TaxonomyError::MissingReferenceLabel {
                        scheme: sid.clone(),
                        id: rc.id.clone(),
                        lang: raw.reference_language.clone(),
                    });
                }
                let keywords = rc
                    .keywords
                    .into_iter()
                    .map(|(lang, list)| (lang, list.iter().filter_map(|k| prepare(k)).collect()))
                    .collect();
                ids.push(rc.id.clone());
                concepts.insert(
                    (sid.clone(), rc.id.clone()),
                    Concept {
                        scheme: sid.clone(),
                        id: rc.id,
                        broader: rc.broader,
                        labels: rc.labels,
                        note: rc.note,
                        isced: rc.isced,
                        keywords,
                    },
                );
            }
            schemes.insert(
                sid.clone(),
                Scheme {
                    id: sid,
                    labels: rs.labels,
                    note: rs.note,
                    concept_ids: ids,
                },
            );
        }
        Ok(Self {
            version: raw.version,
            reference_language: raw.reference_language,
            core_languages: raw.core_languages,
            languages: raw.languages,
            schemes,
            concepts,
        })
    }

    /// A concept by scheme and ID.
    pub fn get(&self, scheme: &str, id: &str) -> Option<&Concept> {
        self.concepts.get(&(scheme.to_string(), id.to_string()))
    }

    /// Concepts of a scheme in file order.
    pub fn concepts_of<'a>(&'a self, scheme: &str) -> impl Iterator<Item = &'a Concept> + 'a {
        let ids = self
            .schemes
            .get(scheme)
            .map(|s| s.concept_ids.clone())
            .unwrap_or_default();
        let scheme = scheme.to_string();
        ids.into_iter()
            .filter_map(move |id| self.concepts.get(&(scheme.clone(), id)))
    }

    /// All concepts (any order).
    pub fn all_concepts(&self) -> impl Iterator<Item = &Concept> {
        self.concepts.values()
    }

    /// Direct children of a concept (`None` = top-level concepts of the scheme).
    pub fn children(&self, scheme: &str, id: Option<&str>) -> Vec<&Concept> {
        self.concepts_of(scheme)
            .filter(|c| c.broader.as_deref() == id)
            .collect()
    }

    /// All descendants of a concept.
    pub fn descendants(&self, scheme: &str, id: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![id.to_string()];
        while let Some(cur) = stack.pop() {
            for c in self.children(scheme, Some(&cur)) {
                out.push(c.id.clone());
                stack.push(c.id.clone());
            }
        }
        out
    }

    /// Concepts without a label in one of the core languages: `(scheme, id, language)`.
    /// A release requires this list to be empty.
    pub fn missing_core_labels(&self) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for scheme in self.schemes.values() {
            for id in &scheme.concept_ids {
                let c = &self.concepts[&(scheme.id.clone(), id.clone())];
                for lang in &self.core_languages {
                    if !c.labels.contains_key(lang) {
                        out.push((scheme.id.clone(), id.clone(), lang.clone()));
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_taxonomy_is_valid_and_complete_for_the_brief() {
        let t = Taxonomy::bundled().expect("valid taxonomy");
        assert_eq!(t.reference_language, "sk");
        assert_eq!(t.core_languages, ["sk", "en", "cs", "ru", "zh"]);
        assert!(t.all_concepts().count() >= 296);
        let top: Vec<&str> = t
            .children("school-type", None)
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        for id in [
            "early-childhood",
            "primary-lower-secondary",
            "upper-secondary",
            "special-education",
            "merged-schools",
            "lifelong-learning",
        ] {
            assert!(top.contains(&id), "school-type {id}");
        }
        for id in [
            "trends",
            "changes",
            "active-learning-methods",
            "lesson-demonstrations",
            "teaching-methods",
            "teaching-strategies",
            "non-formal",
            "digital-learning",
            "inclusion",
            "curriculum-reform",
            "learning-areas",
            "reading-literacy",
            "ai",
            "comprehension-levels",
            "critical-thinking",
            "lesson-phases",
            "legislation",
            "terminology",
            "educational-games",
        ] {
            assert!(t.get("topic", id).is_some(), "topic {id}");
        }
        assert!(t.concepts_of("literacy").count() >= 10);
    }

    #[test]
    fn labels_fall_back_to_english() {
        let t = Taxonomy::bundled().unwrap();
        let c = t.get("method", "jigsaw").unwrap();
        assert_eq!(c.label("sk"), "Skladačka (jigsaw)");
        assert_eq!(c.label("xx"), "Jigsaw");
    }

    #[test]
    fn hierarchy_queries() {
        let t = Taxonomy::bundled().unwrap();
        let d = t.descendants("school-type", "early-childhood");
        assert!(d.contains(&"kindergarten".to_string()) && d.contains(&"nursery".to_string()));
    }

    #[test]
    fn rejects_duplicates_and_unknown_broader() {
        let head = "version: 2\nreference_language: sk\ncore_languages: [sk]\nlanguages: [sk]\nschemes:\n  a:\n    labels: {sk: A}\n    concepts:\n";
        let dup = format!(
            "{head}    - {{id: x, labels: {{sk: X}}}}\n    - {{id: x, labels: {{sk: Y}}}}\n"
        );
        assert!(matches!(
            Taxonomy::from_yaml(&dup),
            Err(TaxonomyError::Duplicate { .. })
        ));
        let orphan = format!("{head}    - {{id: x, broader: y, labels: {{sk: X}}}}\n");
        assert!(matches!(
            Taxonomy::from_yaml(&orphan),
            Err(TaxonomyError::UnknownBroader { .. })
        ));
        let no_ref = format!("{head}    - {{id: x, labels: {{en: X}}}}\n");
        assert!(matches!(
            Taxonomy::from_yaml(&no_ref),
            Err(TaxonomyError::MissingReferenceLabel { .. })
        ));
    }

    #[test]
    fn long_keywords_become_stems() {
        assert_eq!(
            keyword_token("citanie".into()),
            KeywordToken {
                text: "citan".into(),
                prefix: true
            }
        );
        assert_eq!(
            keyword_token("metod".into()),
            KeywordToken {
                text: "metod".into(),
                prefix: true
            }
        );
        assert_eq!(
            keyword_token("sen".into()),
            KeywordToken {
                text: "sen".into(),
                prefix: false
            }
        );
    }
}
