//! How texts become terms, per language.
//!
//! Every language group has its own analyzer and its own title and text fields, so that a Slovak
//! word is stemmed as Slovak and a German one as German. The chain is the same for all:
//! words ([`Words`]) → overlong tokens dropped → lower case → stemmer → diacritics folded.
//! Folding comes after stemming (stemmers need `ä` or `ách`), so `skola` finds `školách`.

use tantivy::tokenizer::{
    AsciiFoldingFilter, Language, LowerCaser, RemoveLongFilter, Stemmer, TextAnalyzer,
    TokenizerManager,
};

use crate::stem::{Slavic, SlavicStemmer};
use crate::words::Words;

/// Longer tokens (in bytes) are dropped: they are hashes, encoded data or run-together junk.
const MAX_TOKEN_BYTES: usize = 64;

/// How a group reduces words to stems.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Stemming {
    /// ATENA's own light stemmer.
    Own(Slavic),
    /// A Snowball stemmer.
    Snowball(Language),
    /// None: words are only lower-cased and folded (CJK pairs, languages without a stemmer).
    None,
}

/// A language group: its ID (used in field and analyzer names), its language codes and stemmer.
#[derive(Debug)]
pub(crate) struct Group {
    pub id: &'static str,
    languages: &'static [&'static str],
    stemming: Stemming,
}

/// The groups; the last one takes every other language and documents of unknown language.
pub(crate) const GROUPS: &[Group] = &[
    group("sk", &["sk"], Stemming::Own(Slavic::Slovak)),
    group("cs", &["cs"], Stemming::Own(Slavic::Czech)),
    group("pl", &["pl"], Stemming::Own(Slavic::Polish)),
    group("en", &["en"], Stemming::Snowball(Language::English)),
    group("de", &["de"], Stemming::Snowball(Language::German)),
    group("fr", &["fr"], Stemming::Snowball(Language::French)),
    group("es", &["es"], Stemming::Snowball(Language::Spanish)),
    group("it", &["it"], Stemming::Snowball(Language::Italian)),
    group("pt", &["pt"], Stemming::Snowball(Language::Portuguese)),
    group("nl", &["nl"], Stemming::Snowball(Language::Dutch)),
    group("sv", &["sv"], Stemming::Snowball(Language::Swedish)),
    group("da", &["da"], Stemming::Snowball(Language::Danish)),
    group(
        "no",
        &["no", "nb", "nn"],
        Stemming::Snowball(Language::Norwegian),
    ),
    group("fi", &["fi"], Stemming::Snowball(Language::Finnish)),
    group("hu", &["hu"], Stemming::Snowball(Language::Hungarian)),
    group("ro", &["ro"], Stemming::Snowball(Language::Romanian)),
    group("ru", &["ru"], Stemming::Snowball(Language::Russian)),
    group("el", &["el"], Stemming::Snowball(Language::Greek)),
    group("tr", &["tr"], Stemming::Snowball(Language::Turkish)),
    group("ar", &["ar"], Stemming::Snowball(Language::Arabic)),
    group("other", &[], Stemming::None),
];

const fn group(id: &'static str, languages: &'static [&'static str], stemming: Stemming) -> Group {
    Group {
        id,
        languages,
        stemming,
    }
}

impl Group {
    /// Name of the analyzer in the index's tokenizer manager.
    pub(crate) fn analyzer_name(&self) -> String {
        format!("atena_{}", self.id)
    }

    fn analyzer(&self) -> TextAnalyzer {
        let words = TextAnalyzer::builder(Words)
            .filter(RemoveLongFilter::limit(MAX_TOKEN_BYTES))
            .filter(LowerCaser);
        match self.stemming {
            Stemming::Own(language) => words
                .filter(SlavicStemmer(language))
                .filter(AsciiFoldingFilter)
                .build(),
            Stemming::Snowball(language) => words
                .filter(Stemmer::new(language))
                .filter(AsciiFoldingFilter)
                .build(),
            Stemming::None => words.filter(AsciiFoldingFilter).build(),
        }
    }
}

/// Index of the group of a language code (`sk`, `sk-SK`, `nb` …); unknown codes go to `other`.
pub(crate) fn group_of(language: Option<&str>) -> usize {
    let primary = language
        .and_then(|l| l.trim().split(['-', '_']).next())
        .map(str::to_ascii_lowercase);
    primary
        .and_then(|p| {
            GROUPS
                .iter()
                .position(|g| g.languages.contains(&p.as_str()))
        })
        .unwrap_or(GROUPS.len() - 1)
}

/// Registers the analyzers of all groups (needed every time an index is opened).
pub(crate) fn register(manager: &TokenizerManager) {
    for group in GROUPS {
        manager.register(&group.analyzer_name(), group.analyzer());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms(language: &str, text: &str) -> Vec<String> {
        let mut analyzer = GROUPS[group_of(Some(language))].analyzer();
        let mut stream = analyzer.token_stream(text);
        let mut out = Vec::new();
        while stream.advance() {
            out.push(stream.token().text.clone());
        }
        out
    }

    #[test]
    fn languages_find_their_groups() {
        assert_eq!(GROUPS[group_of(Some("sk-SK"))].id, "sk");
        assert_eq!(GROUPS[group_of(Some("NB"))].id, "no");
        assert_eq!(GROUPS[group_of(Some("zh-Hans"))].id, "other");
        assert_eq!(GROUPS[group_of(None)].id, "other");
    }

    #[test]
    fn inflected_forms_and_missing_diacritics_meet() {
        assert_eq!(terms("sk", "Školách"), terms("sk", "skoly"));
        assert_eq!(
            terms("sk", "čitateľská gramotnosť"),
            terms("sk", "citatelskej gramotnosti")
        );
        assert_eq!(terms("cs", "vzdělávání"), terms("cs", "vzdelavat"));
        assert_eq!(terms("de", "Schulen"), terms("de", "Schule"));
        assert_eq!(terms("en", "reading"), terms("en", "reads"));
        assert_eq!(terms("ru", "чтения"), terms("ru", "чтение"));
    }

    #[test]
    fn chinese_is_paired_and_long_tokens_dropped() {
        assert_eq!(terms("zh", "阅读素养"), ["阅读", "读素", "素养"]);
        assert!(terms("en", &"x".repeat(MAX_TOKEN_BYTES + 1)).is_empty());
    }
}
