//! Transparent, rule-based classification suggestions.
//!
//! Every suggestion carries the keywords that caused it, so an editor sees *why* it was made.
//! Keywords are matched only against documents in the same language, which keeps meanings apart
//! (Slovak `sen` – a dream – never triggers English `SEN` – special educational needs).
//! A suggestion is never a decision: an editor confirms or rejects it.

use crate::taxonomy::{Concept, Keyword, Phrase, Taxonomy};
use crate::text::{fold, tokens};

/// Scheme whose concepts are mutually exclusive: only the best match is suggested.
const RESOURCE_TYPE: &str = "resource-type";
/// Characters of the body that count as the document's opening (weighted ×2).
const HEAD_CHARS: usize = 1_500;
/// Characters of the body that are examined at all.
const BODY_CHARS: usize = 20_000;
/// Characters examined for the resource type (title and the first lines).
const TYPE_CHARS: usize = 600;
/// No keyword counts more than this many weighted hits, so one repeated word cannot dominate.
const HIT_CAP: u32 = 6;

/// A suggested concept for a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    /// Scheme ID.
    pub scheme: String,
    /// Concept ID.
    pub concept: String,
    /// Confidence between 0 and 1.
    pub score: f64,
    /// Keywords that produced the suggestion, e.g. `sk:čitateľsk×3`.
    pub evidence: Vec<String>,
}

/// Prepared text of one document part.
struct Part {
    tokens: Vec<String>,
    folded: String,
}

impl Part {
    fn new(text: &str) -> Self {
        Self {
            tokens: tokens(text),
            folded: fold(text),
        }
    }

    fn count(&self, phrase: &Phrase) -> u32 {
        match phrase {
            Phrase::Cjk(needle) => self.folded.matches(needle.as_str()).count() as u32,
            Phrase::Words(words) => {
                if words.is_empty() || self.tokens.len() < words.len() {
                    return 0;
                }
                self.tokens
                    .windows(words.len())
                    .filter(|window| {
                        window.iter().zip(words).all(|(tok, kw)| {
                            if kw.prefix {
                                tok.starts_with(&kw.text)
                            } else {
                                *tok == kw.text
                            }
                        })
                    })
                    .count() as u32
            }
        }
    }
}

fn take_chars(text: &str, skip: usize, take: usize) -> String {
    text.chars().skip(skip).take(take).collect()
}

/// Weighted keyword hits of one concept in one language: title ×3, opening ×2, rest ×1.
fn weight(keywords: &[Keyword], parts: &[(&Part, u32)]) -> (u32, Vec<String>) {
    let mut total = 0;
    let mut evidence = Vec::new();
    for kw in keywords {
        let hits: u32 = parts
            .iter()
            .map(|(part, w)| w * part.count(&kw.phrase))
            .sum();
        if hits > 0 {
            total += hits.min(HIT_CAP);
            evidence.push(format!("{}×{hits}", kw.raw));
        }
    }
    (total, evidence)
}

/// Suggests concepts for a document. `lang` is the detected document language; when it is unknown,
/// keywords of all languages are used. `threshold` is the minimal score (the default is 0.45).
pub fn suggest(
    taxonomy: &Taxonomy,
    title: &str,
    body: &str,
    lang: Option<&str>,
    threshold: f64,
) -> Vec<Suggestion> {
    let title_part = Part::new(title);
    let head = Part::new(&take_chars(body, 0, HEAD_CHARS));
    let rest = Part::new(&take_chars(body, HEAD_CHARS, BODY_CHARS - HEAD_CHARS));
    let opening = Part::new(&format!("{title}\n{}", take_chars(body, 0, TYPE_CHARS)));
    let body_parts = [(&title_part, 3), (&head, 2), (&rest, 1)];
    let type_parts = [(&opening, 1)];

    let languages = |c: &Concept| -> Vec<String> {
        match lang {
            Some(l) if c.keywords.contains_key(l) => vec![l.to_string()],
            Some(_) => Vec::new(),
            None => c.keywords.keys().cloned().collect(),
        }
    };

    let mut out = Vec::new();
    let mut best_type: Option<Suggestion> = None;
    for concept in taxonomy.all_concepts() {
        let is_type = concept.scheme == RESOURCE_TYPE;
        let parts: &[(&Part, u32)] = if is_type { &type_parts } else { &body_parts };
        let mut total = 0;
        let mut evidence = Vec::new();
        for l in languages(concept) {
            let (w, ev) = weight(&concept.keywords[&l], parts);
            total += w;
            evidence.extend(ev.into_iter().map(|e| format!("{l}:{e}")));
        }
        if total == 0 {
            continue;
        }
        let score = if is_type {
            0.6
        } else {
            1.0 - (-f64::from(total) / 3.0).exp()
        };
        if score < threshold {
            continue;
        }
        let s = Suggestion {
            scheme: concept.scheme.clone(),
            concept: concept.id.clone(),
            score: (score * 1000.0).round() / 1000.0,
            evidence,
        };
        if is_type {
            // the type with most evidence wins; ties go to the alphabetically first ID (stable output)
            let better = match &best_type {
                None => true,
                Some(b) => {
                    (s.evidence.len(), std::cmp::Reverse(&s.concept))
                        > (b.evidence.len(), std::cmp::Reverse(&b.concept))
                }
            };
            if better {
                best_type = Some(s);
            }
        } else {
            out.push(s);
        }
    }
    out.extend(best_type);
    out.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.scheme.cmp(&b.scheme))
            .then_with(|| a.concept.cmp(&b.concept))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(s: &[Suggestion], scheme: &str) -> Vec<String> {
        s.iter()
            .filter(|x| x.scheme == scheme)
            .map(|x| x.concept.clone())
            .collect()
    }

    fn tax() -> Taxonomy {
        Taxonomy::bundled().unwrap()
    }

    #[test]
    fn slovak_reading_in_kindergarten() {
        let s = suggest(
            &tax(),
            "Rozvoj čitateľskej gramotnosti v materskej škole",
            "Metodika ponúka aktivity na čítanie s porozumením pre deti v materskej škole. \
             Deti pracujú v skupinách a učiteľ využíva aktivizačné metódy.",
            Some("sk"),
            0.45,
        );
        assert!(ids(&s, "topic").contains(&"reading-literacy".into()));
        assert!(ids(&s, "school-type").contains(&"kindergarten".into()));
        assert_eq!(ids(&s, "resource-type"), ["teacher-guide"]);
        let reading = s.iter().find(|x| x.concept == "reading-literacy").unwrap();
        assert!(reading.evidence.iter().any(|e| e.starts_with("sk:")));
    }

    #[test]
    fn other_core_languages() {
        let t = tax();
        let has = |title: &str, body: &str, lang: &str, scheme: &str, id: &str| {
            ids(&suggest(&t, title, body, Some(lang), 0.45), scheme).contains(&id.to_string())
        };
        assert!(has(
            "Kritické myšlení ve výuce",
            "Rozvoj kritického myšlení žáků.",
            "cs",
            "topic",
            "critical-thinking"
        ));
        assert!(has(
            "Читательская грамотность",
            "Задания на читательскую грамотность для 7 класса.",
            "ru",
            "topic",
            "reading-literacy"
        ));
        assert!(has(
            "人工智能教育",
            "人工智能进课堂。人工智能教育指南。",
            "zh",
            "topic",
            "ai"
        ));
        assert!(has(
            "Artificial intelligence in the classroom",
            "How teachers use generative AI and ChatGPT.",
            "en",
            "topic",
            "ai"
        ));
        assert!(has(
            "Künstliche Intelligenz im Unterricht",
            "Wie Lehrkräfte künstliche Intelligenz nutzen.",
            "de",
            "topic",
            "ai"
        ));
    }

    #[test]
    fn languages_do_not_mix() {
        let s = suggest(
            &tax(),
            "Môj sen",
            "Mal som krásny sen o lete a sen o mori.",
            Some("sk"),
            0.45,
        );
        assert!(!ids(&s, "topic").contains(&"inclusion".into()));
    }

    #[test]
    fn legal_act_is_recognised() {
        let s = suggest(
            &tax(),
            "Vyhláška č. 123/2026 Z. z.",
            "Ministerstvo ustanovuje podľa zákona č. 245/2008 Z. z.",
            Some("sk"),
            0.45,
        );
        assert_eq!(ids(&s, "resource-type"), ["legal-act"]);
    }

    #[test]
    fn nothing_is_invented_for_unrelated_text() {
        let s = suggest(
            &tax(),
            "Rozpis plavárne",
            "Plaváreň je otvorená od pondelka do piatku.",
            Some("sk"),
            0.45,
        );
        assert!(s.is_empty(), "{s:?}");
    }
}
