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
    /// A language marked as draft is not a taxonomy language, or is the reference language.
    #[error("draft label language {0} is unknown or is the reference language")]
    InvalidDraftLanguage(String),
    /// A concept includes an unknown scheme or concept, or one of its own scheme.
    #[error("concept {scheme}:{id} includes {include}, which is unknown or in its own scheme")]
    InvalidInclude {
        /// Scheme ID.
        scheme: String,
        /// Concept ID.
        id: String,
        /// The include as written.
        include: String,
    },
}

/// A scheme or concept without a label in a core language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingLabel {
    /// Scheme ID.
    pub scheme: String,
    /// Concept ID; `None` when the scheme's own label is missing.
    pub concept: Option<String>,
    /// Language code.
    pub lang: String,
}

#[derive(Debug, Deserialize)]
struct RawTaxonomy {
    version: u32,
    reference_language: String,
    core_languages: Vec<String>,
    #[serde(default)]
    draft_label_languages: Vec<String>,
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
    #[serde(default)]
    includes: Vec<String>,
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
    /// Concepts of other schemes this concept gathers (`scheme/concept`, or a whole `scheme`):
    /// a section of the overview is a view of the facets that make it up.
    pub includes: Vec<Include>,
}

/// A concept of another scheme gathered by a concept – or the whole scheme.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Include {
    /// Scheme ID.
    pub scheme: String,
    /// Concept ID; `None` for every concept of the scheme.
    pub concept: Option<String>,
}

impl Include {
    fn parse(text: &str) -> Self {
        match text.trim().split_once('/') {
            Some((scheme, concept)) => Self {
                scheme: scheme.to_string(),
                concept: Some(concept.to_string()),
            },
            None => Self {
                scheme: text.trim().to_string(),
                concept: None,
            },
        }
    }
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
    /// Languages whose labels are machine-drafted and await review by a native speaker;
    /// the UI marks them as "translation – suggestion".
    pub draft_label_languages: Vec<String>,
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
        if let Some(lang) = raw
            .draft_label_languages
            .iter()
            .find(|l| **l == raw.reference_language || !raw.languages.contains(l))
        {
            return Err(TaxonomyError::InvalidDraftLanguage(lang.clone()));
        }
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
                let includes = rc.includes.iter().map(|i| Include::parse(i)).collect();
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
                        includes,
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
        for c in concepts.values() {
            for include in &c.includes {
                let known = include.scheme != c.scheme
                    && schemes.contains_key(&include.scheme)
                    && include.concept.as_ref().is_none_or(|id| {
                        concepts.contains_key(&(include.scheme.clone(), id.clone()))
                    });
                if !known {
                    return Err(TaxonomyError::InvalidInclude {
                        scheme: c.scheme.clone(),
                        id: c.id.clone(),
                        include: match &include.concept {
                            Some(id) => format!("{}/{id}", include.scheme),
                            None => include.scheme.clone(),
                        },
                    });
                }
            }
        }
        Ok(Self {
            version: raw.version,
            reference_language: raw.reference_language,
            core_languages: raw.core_languages,
            draft_label_languages: raw.draft_label_languages,
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

    /// Everything a document may be labelled with to belong to a concept: the concept and its
    /// descendants, and what they include (with descendants; `None` for a whole scheme).
    /// Empty for an unknown concept.
    pub fn members(&self, scheme: &str, id: &str) -> Vec<Include> {
        if self.get(scheme, id).is_none() {
            return Vec::new();
        }
        let own = std::iter::once(id.to_string()).chain(self.descendants(scheme, id));
        let mut out = std::collections::BTreeSet::new();
        for concept in own {
            let Some(c) = self.get(scheme, &concept) else {
                continue;
            };
            for include in &c.includes {
                if let Some(target) = &include.concept {
                    for d in self.descendants(&include.scheme, target) {
                        out.insert(Include {
                            scheme: include.scheme.clone(),
                            concept: Some(d),
                        });
                    }
                }
                out.insert(include.clone());
            }
            out.insert(Include {
                scheme: scheme.to_string(),
                concept: Some(concept),
            });
        }
        out.into_iter().collect()
    }

    /// `true` when labels in `lang` are drafts awaiting review.
    pub fn is_draft_language(&self, lang: &str) -> bool {
        self.draft_label_languages.iter().any(|l| l == lang)
    }

    /// Schemes and concepts without a label in a core language. A release requires this list to be empty.
    pub fn missing_core_labels(&self) -> Vec<MissingLabel> {
        let mut out = Vec::new();
        for scheme in self.schemes.values() {
            let labelled = std::iter::once((None, &scheme.labels)).chain(
                self.concepts_of(&scheme.id)
                    .map(|c| (Some(c.id.clone()), &c.labels)),
            );
            for (concept, labels) in labelled {
                for lang in &self.core_languages {
                    if !labels.contains_key(lang) {
                        out.push(MissingLabel {
                            scheme: scheme.id.clone(),
                            concept: concept.clone(),
                            lang: lang.clone(),
                        });
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
    fn core_languages_are_complete() {
        let t = Taxonomy::bundled().unwrap();
        assert_eq!(t.missing_core_labels(), []);
        assert!(t.is_draft_language("zh") && !t.is_draft_language("sk"));
        let c = t.get("school-type", "special-education").unwrap();
        assert_eq!(c.label("cs"), "Školy pro děti a žáky se SVP");
        assert_eq!(c.label("ru"), "Школы для детей с ОВЗ");
        assert_eq!(c.label("zh"), "特殊教育学校");
    }

    #[test]
    fn reports_missing_labels_and_rejects_a_draft_reference() {
        let yaml = |draft: &str| {
            format!(
                "version: 2\nreference_language: sk\ncore_languages: [sk, cs]\n\
                 draft_label_languages: [{draft}]\nlanguages: [sk, cs]\nschemes:\n  \
                 topic:\n    labels: {{sk: Témy}}\n    concepts:\n    \
                 - id: ai\n      labels: {{sk: AI, cs: AI}}\n"
            )
        };
        let t = Taxonomy::from_yaml(&yaml("cs")).unwrap();
        assert_eq!(
            t.missing_core_labels(),
            [MissingLabel {
                scheme: "topic".into(),
                concept: None,
                lang: "cs".into()
            }]
        );
        assert!(matches!(
            Taxonomy::from_yaml(&yaml("sk")),
            Err(TaxonomyError::InvalidDraftLanguage(l)) if l == "sk"
        ));
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
    fn sections_gather_concepts_of_other_schemes() {
        let t = Taxonomy::bundled().unwrap();
        let include = |scheme: &str, concept: Option<&str>| Include {
            scheme: scheme.into(),
            concept: concept.map(Into::into),
        };
        let practice = t.members("section", "practice");
        for expected in [
            include("section", Some("practice")),
            include("section", Some("teaching-methods")),
            include("method", None),
            include("strategy", Some("project-based-learning")),
            include("resource-type", Some("worksheet")),
        ] {
            assert!(practice.contains(&expected), "{expected:?}");
        }
        // an included concept brings its narrower concepts along
        let inclusion = t.members("section", "inclusion");
        assert!(inclusion.contains(&include("topic", Some("gifted"))));
        assert!(inclusion.contains(&include("school-type", Some("special-primary"))));
        // electronic testing sits in the digital section, question types under it
        let digital = t.members("section", "digital");
        assert!(digital.contains(&include("section", Some("e-testing"))));
        assert!(digital.contains(&include("section", Some("question-types"))));
        assert!(digital.contains(&include("resource-type", Some("test-items"))));
        assert!(t.members("section", "nothing").is_empty());
        // a concept without includes is just itself and its descendants
        assert_eq!(t.members("topic", "ai"), [include("topic", Some("ai"))]);
    }

    #[test]
    fn rejects_unknown_includes_and_includes_of_the_own_scheme() {
        let yaml = |include: &str| {
            format!(
                "version: 2\nreference_language: sk\ncore_languages: [sk]\nlanguages: [sk]\nschemes:\n  \
                 a:\n    labels: {{sk: A}}\n    concepts:\n    - {{id: x, labels: {{sk: X}}, includes: [{include}]}}\n  \
                 b:\n    labels: {{sk: B}}\n    concepts:\n    - {{id: y, labels: {{sk: Y}}}}\n"
            )
        };
        assert!(Taxonomy::from_yaml(&yaml("b/y")).is_ok());
        assert!(Taxonomy::from_yaml(&yaml("b")).is_ok());
        for bad in ["b/z", "c", "a", "a/x"] {
            assert!(
                matches!(
                    Taxonomy::from_yaml(&yaml(bad)),
                    Err(TaxonomyError::InvalidInclude { .. })
                ),
                "{bad}"
            );
        }
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
