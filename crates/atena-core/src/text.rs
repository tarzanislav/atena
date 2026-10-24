//! Text normalisation shared by search, classification and change detection.
//!
//! The same folding is applied to documents and to keywords, so matching works across
//! diacritics (Slovak, Czech, Hungarian, Polish), `ё`/`е` in Russian and German `ß`.

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// Lower-case and strip diacritics: `Čitateľská` → `citatelska`, `ёлка` → `елка`, `Straße` → `strasse`.
pub fn fold(text: &str) -> String {
    let mut lower = String::with_capacity(text.len());
    for c in text.chars() {
        if c == 'ß' || c == 'ẞ' {
            lower.push_str("ss");
        } else {
            lower.extend(c.to_lowercase());
        }
    }
    lower.nfkd().filter(|c| !is_combining_mark(*c)).collect()
}

/// `true` for characters of the CJK Unified Ideographs blocks (Chinese text has no spaces between words).
pub fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2A6DF)
}

/// `true` when the text contains at least one CJK character.
pub fn has_cjk(text: &str) -> bool {
    text.chars().any(is_cjk)
}

/// Folded word tokens: runs of letters and digits, joined across `-` and `'` (`e-learning`, `d'école`).
pub fn tokens(text: &str) -> Vec<String> {
    let folded: Vec<char> = fold(text).chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < folded.len() {
        if !folded[i].is_alphanumeric() {
            i += 1;
            continue;
        }
        let start = i;
        while i < folded.len() && folded[i].is_alphanumeric() {
            i += 1;
        }
        while i + 1 < folded.len()
            && matches!(folded[i], '-' | '\'')
            && folded[i + 1].is_alphanumeric()
        {
            i += 1;
            while i < folded.len() && folded[i].is_alphanumeric() {
                i += 1;
            }
        }
        out.push(folded[start..i].iter().collect());
    }
    out
}

/// Removes differences that are not content changes: spacing, line breaks, typographic quotes and dashes.
pub fn normalize_for_hash(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.nfkc() {
        let c = match c {
            '\u{00a0}' | '\u{2009}' | '\u{202f}' => ' ',
            '\u{201e}' | '\u{201c}' | '\u{201d}' => '"',
            '\u{2013}' | '\u{2014}' => '-',
            other => other,
        };
        if c.is_whitespace() {
            space = !out.is_empty();
        } else {
            if space {
                out.push(' ');
                space = false;
            }
            out.push(c);
        }
    }
    out
}

/// BLAKE3 fingerprint of the normalised text (hex); equal fingerprints mean no content change.
pub fn fingerprint(text: &str) -> String {
    blake3::hash(normalize_for_hash(text).as_bytes())
        .to_hex()
        .to_string()
}

/// Non-empty paragraphs (split on line breaks), each with normalised spacing.
pub fn paragraphs(text: &str) -> Vec<String> {
    text.lines()
        .map(normalize_for_hash)
        .filter(|p| !p.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_diacritics_in_core_languages() {
        assert_eq!(fold("Čitateľská gramotnosť"), "citatelska gramotnost");
        assert_eq!(fold("Čtenářská gramotnost"), "ctenarska gramotnost");
        assert_eq!(fold("Szövegértés ő ű"), "szovegertes o u");
        assert_eq!(fold("Ёлка"), "елка");
        assert_eq!(fold("Straße"), "strasse");
        assert_eq!(fold("阅读素养"), "阅读素养");
    }

    #[test]
    fn tokenises_with_joiners() {
        assert_eq!(
            tokens("E-learning v 1. ročníku!"),
            ["e-learning", "v", "1", "rocniku"]
        );
        assert_eq!(tokens("  "), Vec::<String>::new());
    }

    #[test]
    fn detects_cjk() {
        assert!(has_cjk("人工智能教育"));
        assert!(!has_cjk("Читательская"));
    }

    #[test]
    fn ignores_spacing_and_typography_in_fingerprints() {
        assert_eq!(
            fingerprint("„Termín“ je 20. 3.\n\n  2026 – piatok"),
            fingerprint("\"Termín\" je 20. 3. 2026 - piatok")
        );
        assert_ne!(fingerprint("20. 3. 2026"), fingerprint("27. 3. 2026"));
    }
}
