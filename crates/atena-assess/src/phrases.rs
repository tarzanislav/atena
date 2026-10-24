//! The phrases of `data/legal-phrases.yaml` and how they are matched.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

/// The phrase file shipped with ATENA, embedded at build time.
pub const BUNDLED_PHRASES: &str = include_str!("../../../data/legal-phrases.yaml");

/// Phrases of one category, by language.
type ByLanguage = BTreeMap<String, Vec<String>>;

#[derive(Debug, Deserialize)]
struct RawPair {
    subjects: ByLanguage,
    markers: ByLanguage,
}

#[derive(Debug, Deserialize)]
struct RawPhrases {
    legal_links: ByLanguage,
    ignore_links: Vec<String>,
    legal_context: Vec<String>,
    automation: RawPair,
    mining: RawPair,
    personal_use: ByLanguage,
    rights_reserved: ByLanguage,
    non_commercial: ByLanguage,
    reuse_permitted: ByLanguage,
}

/// A subject that counts only together with a marker in the same sentence.
#[derive(Debug, Clone)]
pub struct Pair {
    /// What the sentence is about (robots, text and data mining …).
    pub subjects: Vec<String>,
    /// Prohibition or reservation words.
    pub markers: Vec<String>,
}

/// All phrases, normalised, languages merged (a page may mix languages).
#[derive(Debug, Clone)]
pub struct Phrases {
    /// Words of links to terms of use and legal notices.
    pub legal_links: Vec<String>,
    /// Words of links that are never about content rights.
    pub ignore_links: Vec<String>,
    /// Words that make a block of an ordinary page a legal statement.
    pub legal_context: Vec<String>,
    /// A ban on automated access.
    pub automation: Pair,
    /// A reservation of text and data mining or AI use.
    pub mining: Pair,
    /// Access only for personal use.
    pub personal_use: Vec<String>,
    /// A plain copyright notice.
    pub rights_reserved: Vec<String>,
    /// Non-commercial reuse only.
    pub non_commercial: Vec<String>,
    /// An explicit permission to reuse.
    pub reuse_permitted: Vec<String>,
}

/// Errors in the phrase file.
#[derive(Debug)]
pub struct PhraseError(pub String);

impl std::fmt::Display for PhraseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid legal phrase file: {}", self.0)
    }
}

impl std::error::Error for PhraseError {}

/// Lower case, NFKC, typographic apostrophes as `'`, whitespace collapsed.
pub fn normalize(text: &str) -> String {
    let folded: String = text
        .nfkc()
        .flat_map(char::to_lowercase)
        .map(|c| {
            if matches!(c, '’' | '‘' | '`') {
                '\''
            } else {
                c
            }
        })
        .collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn flatten(by_language: ByLanguage) -> Vec<String> {
    let mut all: Vec<String> = by_language
        .into_values()
        .flatten()
        .map(|p| normalize(&p))
        .filter(|p| !p.is_empty())
        .collect();
    all.sort();
    all.dedup();
    all
}

fn pair(raw: RawPair) -> Pair {
    Pair {
        subjects: flatten(raw.subjects),
        markers: flatten(raw.markers),
    }
}

impl Phrases {
    /// Parses a phrase file.
    pub fn parse(yaml: &str) -> Result<Self, PhraseError> {
        let raw: RawPhrases =
            serde_saphyr::from_str(yaml).map_err(|e| PhraseError(e.to_string()))?;
        let list = |l: Vec<String>| {
            l.iter()
                .map(|p| normalize(p))
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
        };
        let phrases = Self {
            legal_links: flatten(raw.legal_links),
            ignore_links: list(raw.ignore_links),
            legal_context: list(raw.legal_context),
            automation: pair(raw.automation),
            mining: pair(raw.mining),
            personal_use: flatten(raw.personal_use),
            rights_reserved: flatten(raw.rights_reserved),
            non_commercial: flatten(raw.non_commercial),
            reuse_permitted: flatten(raw.reuse_permitted),
        };
        for (name, empty) in [
            ("legal_links", phrases.legal_links.is_empty()),
            (
                "automation",
                phrases.automation.subjects.is_empty() || phrases.automation.markers.is_empty(),
            ),
            (
                "mining",
                phrases.mining.subjects.is_empty() || phrases.mining.markers.is_empty(),
            ),
            ("personal_use", phrases.personal_use.is_empty()),
        ] {
            if empty {
                return Err(PhraseError(format!("{name} has no phrases")));
            }
        }
        Ok(phrases)
    }

    /// The bundled phrases, parsed once.
    pub fn bundled() -> &'static Self {
        static PHRASES: OnceLock<Phrases> = OnceLock::new();
        PHRASES.get_or_init(|| Self::parse(BUNDLED_PHRASES).expect("valid bundled legal phrases"))
    }
}

/// A script written without spaces between words.
fn is_unspaced(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF   // Hiragana, Katakana
        | 0x3400..=0x4DBF // CJK extension A
        | 0x4E00..=0x9FFF // CJK unified ideographs
        | 0xAC00..=0xD7AF // Hangul
    )
}

/// Byte offset of the first match of `phrase` in `text` (both normalised): the phrase must start a
/// word; one of up to three letters must also end it. Unspaced scripts match anywhere.
pub fn find(text: &str, phrase: &str) -> Option<usize> {
    if phrase.is_empty() {
        return None;
    }
    let unspaced = phrase.chars().next().is_some_and(is_unspaced);
    let whole_word =
        !unspaced && phrase.chars().count() <= 3 && phrase.chars().all(char::is_alphanumeric);
    let mut from = 0;
    while let Some(at) = text[from..].find(phrase).map(|i| i + from) {
        let starts_word = unspaced
            || !phrase.chars().next().is_some_and(char::is_alphanumeric)
            || !text[..at]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric);
        let end = at + phrase.len();
        let ends_word = !whole_word
            || !text[end..]
                .chars()
                .next()
                .is_some_and(char::is_alphanumeric);
        if starts_word && ends_word {
            return Some(at);
        }
        from = at + phrase.chars().next().map_or(1, char::len_utf8);
    }
    None
}

/// The first of `phrases` found in `text`.
pub fn first<'a>(text: &str, phrases: &'a [String]) -> Option<&'a str> {
    phrases
        .iter()
        .find(|p| find(text, p).is_some())
        .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_file_is_valid() {
        let p = Phrases::bundled();
        assert!(p.automation.subjects.iter().any(|s| s == "robot"));
        assert!(p.legal_links.iter().any(|s| s == "podmienky"));
        assert!(
            p.rights_reserved
                .iter()
                .any(|s| s == "všetky práva vyhradené")
        );
    }

    #[test]
    fn words_start_and_short_words_are_whole() {
        let t = normalize("Používanie robotov je ZAKÁZANÉ. Notice: no bots.");
        assert!(find(&t, "robot").is_some());
        assert!(find(&t, "zakáz").is_some());
        assert!(find(&t, "no").is_some(), "the word 'no'");
        assert!(find("notice", "no").is_none(), "not inside a longer word");
        assert!(find("chatbots", "bots").is_none(), "must start a word");
        assert!(find("未经许可禁止转载", "禁止").is_some());
        assert_eq!(
            normalize("Ne\u{a0}pas  l’autorisation"),
            "ne pas l'autorisation"
        );
    }
}
