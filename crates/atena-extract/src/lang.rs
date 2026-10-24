//! Language of a document: detected from the text, with the declared language (`<html lang>`,
//! `Content-Language`, document metadata) as a fallback. Declarations are often wrong – a Slovak
//! school site built on an English template says `lang="en"` – so a confident detection wins.

use std::sync::OnceLock;

use lingua::{Language as Lingua, LanguageDetector, LanguageDetectorBuilder};

/// Characters of text used for detection; more does not improve the result much.
const SAMPLE_CHARS: usize = 3_000;
/// Shorter texts are not detected at all.
const MIN_CHARS: usize = 20;
/// Detections at least this confident override a declaration.
const CONFIDENT: f64 = 0.6;

/// Languages ATENA works with (taxonomy languages; Ukrainian so that it is not taken for Russian).
const LANGUAGES: [(Lingua, &str); 9] = [
    (Lingua::Slovak, "sk"),
    (Lingua::Czech, "cs"),
    (Lingua::English, "en"),
    (Lingua::Russian, "ru"),
    (Lingua::Chinese, "zh"),
    (Lingua::German, "de"),
    (Lingua::Polish, "pl"),
    (Lingua::Hungarian, "hu"),
    (Lingua::Ukrainian, "uk"),
];

/// How the language was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LanguageSource {
    /// Detected from the text.
    Detected,
    /// Taken from the document's declaration.
    Declared,
}

/// The language of a document.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Language {
    /// ISO 639-1 code (`sk`, `cs`, `zh`…).
    pub code: String,
    /// Where the code comes from.
    pub source: LanguageSource,
    /// Confidence of the detection (0–1); 0 for a declaration that was not confirmed.
    pub confidence: f64,
}

fn detector() -> &'static LanguageDetector {
    static DETECTOR: OnceLock<LanguageDetector> = OnceLock::new();
    DETECTOR.get_or_init(|| {
        let languages: Vec<Lingua> = LANGUAGES.iter().map(|(l, _)| *l).collect();
        LanguageDetectorBuilder::from_languages(&languages).build()
    })
}

/// The most likely language of `text` with its confidence.
pub fn detect(text: &str) -> Option<(&'static str, f64)> {
    let sample: String = text.chars().take(SAMPLE_CHARS).collect();
    if sample.chars().filter(|c| c.is_alphabetic()).count() < MIN_CHARS {
        return None;
    }
    let (best, confidence) = detector()
        .compute_language_confidence_values(sample)
        .into_iter()
        .next()?;
    let code = LANGUAGES.iter().find(|(l, _)| *l == best)?.1;
    Some((code, confidence))
}

/// Primary subtag of a language tag: `sk-SK` → `sk`, `zh-Hans-CN` → `zh`.
pub(crate) fn primary_subtag(tag: &str) -> Option<String> {
    let primary = tag.trim().split(['-', '_']).next()?.to_ascii_lowercase();
    (primary.len() >= 2 && primary.len() <= 3 && primary.chars().all(|c| c.is_ascii_alphabetic()))
        .then_some(primary)
}

/// Combines detection and declaration (see the module documentation).
pub(crate) fn decide(text: &str, declared: Option<&str>) -> Option<Language> {
    let detected = detect(text);
    let declared = declared.and_then(primary_subtag);
    match (detected, declared) {
        (Some((code, confidence)), _) if confidence >= CONFIDENT => Some(Language {
            code: code.to_string(),
            source: LanguageSource::Detected,
            confidence,
        }),
        (detected, Some(code)) => Some(Language {
            confidence: detected.filter(|(d, _)| *d == code).map_or(0.0, |(_, c)| c),
            code,
            source: LanguageSource::Declared,
        }),
        (Some((code, confidence)), None) => Some(Language {
            code: code.to_string(),
            source: LanguageSource::Detected,
            confidence,
        }),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_languages_are_told_apart() {
        for (text, code) in [
            (
                "Žiaci rozvíjajú čitateľskú gramotnosť a kritické myslenie na hodinách slovenského jazyka.",
                "sk",
            ),
            (
                "Žáci rozvíjejí čtenářskou gramotnost a kritické myšlení v hodinách českého jazyka.",
                "cs",
            ),
            (
                "Pupils develop reading literacy and critical thinking in their language lessons.",
                "en",
            ),
            (
                "Учащиеся развивают читательскую грамотность и критическое мышление на уроках.",
                "ru",
            ),
            (
                "Учні розвивають читацьку грамотність і критичне мислення на уроках мови.",
                "uk",
            ),
            ("学生在语文课上培养阅读素养和批判性思维能力。", "zh"),
        ] {
            assert_eq!(detect(text).map(|d| d.0), Some(code), "{text}");
        }
    }

    #[test]
    fn a_confident_detection_beats_a_wrong_declaration() {
        let sk =
            "Riaditeľ školy oznamuje rodičom, že zápis do prvého ročníka sa uskutoční v apríli.";
        let lang = decide(sk, Some("en-US")).unwrap();
        assert_eq!(
            (lang.code.as_str(), lang.source),
            ("sk", LanguageSource::Detected)
        );
        let short = decide("OK", Some("cs-CZ")).unwrap();
        assert_eq!(
            (short.code.as_str(), short.source),
            ("cs", LanguageSource::Declared)
        );
        assert_eq!(decide("123 456", None), None);
        assert_eq!(primary_subtag("zh-Hans-CN").as_deref(), Some("zh"));
        assert_eq!(primary_subtag("x"), None);
    }
}
