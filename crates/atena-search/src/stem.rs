//! Light stemmers for Slovak, Czech and Polish, which the Snowball project does not cover.
//!
//! A light stemmer removes inflectional endings – cases, numbers, persons – and a few very
//! productive derivational ones (`-ovanie`, `-osť` …), and nothing else. It is deliberately
//! modest: it must turn `školy`, `škole`, `školách` into one term without merging unrelated words.
//! One ending is removed, the longest that leaves a stem of at least three letters.

use std::sync::LazyLock;

use tantivy::tokenizer::{Token, TokenFilter, TokenStream, Tokenizer};

/// Languages with a light stemmer of ATENA's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slavic {
    /// Slovak.
    Slovak,
    /// Czech.
    Czech,
    /// Polish.
    Polish,
}

/// Shortest stem left.
const MIN_STEM: usize = 3;

// Endings separated by white space, grouped for the reader; the order does not matter.
const SLOVAK: &[&str] = &[
    // nouns of action and abstract nouns
    "ovaniami ovaniach ovaniam ovanie ovania ovaniu ovaní",
    "aniami aniach aniam anie ania aniu aní eniami eniach eniam enie enia eniu ení",
    "osťami ostiach ostiam osťou osti ostí osť",
    // verbs
    "ovať ujeme ujete ujem uješ ujú uje ovali ovala oval áme áte ajú íme íte ať iť ieť",
    // adjectives
    "ieho iemu ého ému ými ých ími ích ým ím ej",
    // nouns
    "ovia iami iach iam ami ách ach ovi och om ov ou mi ia ie iu ii",
    "á é í ú ý ä a e i o u y",
];

const CZECH: &[&str] = &[
    // nouns of action and abstract nouns
    "ováními ováních ováním ování áními áních áním ání eními eních ením ení ění",
    "ostmi ostech ostem ostí osti ost",
    // verbs
    "ovat ujeme ujete ujou ují uje ovali ovala oval áme áte ají íme íte at it et ět",
    // adjectives
    "ého ému ými ých ími ích ým ím ové ová ový ovou",
    // nouns
    "ovi ami ách ech ům ám em ou",
    "á é ě í ů ý a e i o u y",
];

const POLISH: &[&str] = &[
    // nouns of action and abstract nouns
    "owaniami owaniach owaniem owania owanie owaniu",
    "aniami aniach aniem ania anie aniu eniami eniach eniem enia enie eniu",
    "ościami ościach ością ości ość",
    // verbs
    "ować ujemy ujecie ują uje ać ić yć",
    // adjectives
    "ego emu ych ymi ich imi ej",
    // nouns
    "ami ach owi ów om em",
    "ą ę a e i o u y",
];

/// The endings of a list, longest first.
fn endings(list: &[&'static str]) -> Vec<&'static str> {
    let mut all: Vec<&str> = list.iter().flat_map(|l| l.split_whitespace()).collect();
    all.sort_by_key(|e| std::cmp::Reverse(e.chars().count()));
    all
}

static ENDINGS: LazyLock<[Vec<&'static str>; 3]> =
    LazyLock::new(|| [endings(SLOVAK), endings(CZECH), endings(POLISH)]);

impl Slavic {
    fn endings(self) -> &'static [&'static str] {
        let index = match self {
            Self::Slovak => 0,
            Self::Czech => 1,
            Self::Polish => 2,
        };
        &ENDINGS[index]
    }

    /// The stem of a lower-case word: without the longest ending that leaves three letters.
    pub fn stem(self, word: &str) -> &str {
        let length = word.chars().count();
        self.endings()
            .iter()
            .find(|e| word.ends_with(*e) && length - e.chars().count() >= MIN_STEM)
            .map_or(word, |e| &word[..word.len() - e.len()])
    }
}

/// A token filter applying a [`Slavic`] stemmer to lower-case tokens.
#[derive(Debug, Clone, Copy)]
pub struct SlavicStemmer(pub Slavic);

impl TokenFilter for SlavicStemmer {
    type Tokenizer<T: Tokenizer> = SlavicStemmerFilter<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> Self::Tokenizer<T> {
        SlavicStemmerFilter {
            language: self.0,
            tokenizer,
        }
    }
}

/// The tokenizer [`SlavicStemmer`] produces.
#[derive(Debug, Clone)]
pub struct SlavicStemmerFilter<T> {
    language: Slavic,
    tokenizer: T,
}

impl<T: Tokenizer> Tokenizer for SlavicStemmerFilter<T> {
    type TokenStream<'a> = SlavicStemmerStream<T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        SlavicStemmerStream {
            language: self.language,
            tail: self.tokenizer.token_stream(text),
        }
    }
}

/// The token stream of [`SlavicStemmerFilter`].
pub struct SlavicStemmerStream<T> {
    language: Slavic,
    tail: T,
}

impl<T: TokenStream> TokenStream for SlavicStemmerStream<T> {
    fn advance(&mut self) -> bool {
        if !self.tail.advance() {
            return false;
        }
        let token = self.tail.token_mut();
        let stem_len = self.language.stem(&token.text).len();
        token.text.truncate(stem_len);
        true
    }

    fn token(&self) -> &Token {
        self.tail.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.tail.token_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(language: Slavic, words: &[&str]) {
        let stems: Vec<&str> = words.iter().map(|w| language.stem(w)).collect();
        assert!(
            stems.windows(2).all(|w| w[0] == w[1]),
            "{language:?}: {words:?} → {stems:?}"
        );
    }

    #[test]
    fn slovak_forms_meet() {
        same(
            Slavic::Slovak,
            &[
                "škola",
                "školy",
                "škole",
                "školu",
                "školou",
                "školách",
                "školami",
            ],
        );
        same(
            Slavic::Slovak,
            &["žiak", "žiaka", "žiakovi", "žiakom", "žiakov"],
        );
        same(
            Slavic::Slovak,
            &["vzdelávanie", "vzdelávania", "vzdelávaní", "vzdelávaniu"],
        );
        same(Slavic::Slovak, &["testovanie", "testovania", "testovať"]);
        same(
            Slavic::Slovak,
            &["matematika", "matematiky", "matematike", "matematiku"],
        );
        same(
            Slavic::Slovak,
            &["kurikulárna", "kurikulárnej", "kurikulárnu"],
        );
        same(
            Slavic::Slovak,
            &["gramotnosť", "gramotnosti", "gramotnosťou"],
        );
    }

    #[test]
    fn czech_and_polish_forms_meet() {
        same(
            Slavic::Czech,
            &[
                "škola",
                "školy",
                "škole",
                "školu",
                "školou",
                "školách",
                "školami",
            ],
        );
        same(Slavic::Czech, &["vzdělávání", "vzděláváním", "vzdělávat"]);
        same(Slavic::Czech, &["gramotnost", "gramotnosti", "gramotností"]);
        same(
            Slavic::Polish,
            &[
                "szkoła",
                "szkoły",
                "szkołę",
                "szkołą",
                "szkołami",
                "szkołach",
            ],
        );
        same(Slavic::Polish, &["nauczanie", "nauczania", "nauczaniu"]);
    }

    #[test]
    fn short_words_stay() {
        assert_eq!(Slavic::Slovak.stem("pes"), "pes");
        assert_eq!(Slavic::Slovak.stem("ja"), "ja");
        assert_eq!(Slavic::Czech.stem("pan"), "pan");
        assert_eq!(Slavic::Slovak.stem("123"), "123");
    }
}
