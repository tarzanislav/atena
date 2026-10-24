//! The tokenizer every language shares: runs of letters and digits, with the scripts that do not
//! put spaces between words – Chinese, Japanese and Korean – cut into overlapping pairs of
//! characters. Pairs need no dictionary and find any word of two or more characters, the approach
//! of Lucene's CJK analyzer. A lone character stays a token of its own.

use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

/// Han ideographs, kana and Hangul.
pub(crate) fn is_paired_script(c: char) -> bool {
    atena_core::text::is_cjk(c)
        || matches!(c as u32,
            0x3040..=0x30FF   // Hiragana, Katakana
            | 0x31F0..=0x31FF // Katakana phonetic extensions
            | 0xFF66..=0xFF9F // half-width Katakana
            | 0x1100..=0x11FF // Hangul Jamo
            | 0x3130..=0x318F // Hangul compatibility Jamo
            | 0xAC00..=0xD7AF // Hangul syllables
        )
}

/// Words and pairs of CJK characters.
#[derive(Debug, Clone, Copy, Default)]
pub struct Words;

impl Tokenizer for Words {
    type TokenStream<'a> = WordStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> WordStream<'a> {
        WordStream {
            text,
            next: 0,
            run: Vec::new(),
            pair: 0,
            token: Token::default(),
        }
    }
}

/// The token stream of [`Words`].
pub struct WordStream<'a> {
    text: &'a str,
    /// Byte offset where reading continues.
    next: usize,
    /// Byte ranges of the characters of the current CJK run.
    run: Vec<(usize, usize)>,
    /// Index of the next pair of the run.
    pair: usize,
    token: Token,
}

impl WordStream<'_> {
    fn emit(&mut self, from: usize, to: usize) {
        self.token.text.clear();
        self.token.text.push_str(&self.text[from..to]);
        self.token.offset_from = from;
        self.token.offset_to = to;
        self.token.position = self.token.position.wrapping_add(1);
        self.token.position_length = 1;
    }

    /// The next pair (or the lone character) of the current run.
    fn next_pair(&mut self) -> bool {
        let Some(last) = self.run.len().checked_sub(1) else {
            return false;
        };
        if self.pair > last || (self.pair == last && last > 0) {
            return false;
        }
        let (from, end) = self.run[self.pair];
        let to = if last == 0 {
            end
        } else {
            self.run[self.pair + 1].1
        };
        self.pair += 1;
        self.emit(from, to);
        true
    }
}

impl TokenStream for WordStream<'_> {
    fn advance(&mut self) -> bool {
        if self.next_pair() {
            return true;
        }
        let rest = &self.text[self.next..];
        let Some((skip, first)) = rest.char_indices().find(|(_, c)| c.is_alphanumeric()) else {
            self.next = self.text.len();
            return false;
        };
        let start = self.next + skip;
        let paired = is_paired_script(first);
        let mut end = start;
        self.run.clear();
        self.pair = 0;
        for (i, c) in self.text[start..].char_indices() {
            if !c.is_alphanumeric() || is_paired_script(c) != paired {
                break;
            }
            end = start + i + c.len_utf8();
            if paired {
                self.run.push((start + i, end));
            }
        }
        self.next = end;
        if paired {
            self.next_pair()
        } else {
            self.emit(start, end);
            true
        }
    }

    fn token(&self) -> &Token {
        &self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<(String, usize, usize, usize)> {
        let mut words = Words;
        let mut stream = words.token_stream(text);
        let mut out = Vec::new();
        while stream.advance() {
            let t = stream.token();
            out.push((t.text.clone(), t.offset_from, t.offset_to, t.position));
        }
        out
    }

    fn texts(text: &str) -> Vec<String> {
        tokens(text).into_iter().map(|t| t.0).collect()
    }

    #[test]
    fn words_of_spaced_scripts() {
        assert_eq!(
            texts("Čitateľská gramotnosť, 2026!"),
            ["Čitateľská", "gramotnosť", "2026"]
        );
        assert_eq!(texts("e-learning d'école"), ["e", "learning", "d", "école"]);
        assert_eq!(texts("Чтение и письмо"), ["Чтение", "и", "письмо"]);
        assert!(texts("  ,.;  ").is_empty());
    }

    #[test]
    fn pairs_of_cjk_characters() {
        assert_eq!(texts("阅读素养"), ["阅读", "读素", "素养"]);
        assert_eq!(texts("学"), ["学"]);
        assert_eq!(
            texts("PISA 2022年的阅读"),
            ["PISA", "2022", "年的", "的阅", "阅读"]
        );
        assert_eq!(
            texts("読解力テスト"),
            ["読解", "解力", "力テ", "テス", "スト"]
        );
        assert_eq!(texts("읽기 능력"), ["읽기", "능력"]);
    }

    #[test]
    fn offsets_and_positions() {
        let t = tokens("ab 教育部");
        assert_eq!(t[0], ("ab".to_string(), 0, 2, 0));
        assert_eq!(t[1], ("教育".to_string(), 3, 9, 1));
        assert_eq!(t[2], ("育部".to_string(), 6, 12, 2));
    }
}
