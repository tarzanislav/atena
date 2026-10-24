//! Defences against hostile content, shared by every part of ATENA (pillar 3: security and
//! protection against dangerous people, robots, code and processes).
//!
//! Text from the web may carry characters that hide or disguise what it says:
//! * control characters – NUL is refused by PostgreSQL, ESC and BEL drive terminals;
//! * bidirectional embeddings, overrides and isolates, which make text read differently than it
//!   is stored ("Trojan Source");
//! * invisible characters that split words for search or smuggle hidden instructions (zero-width
//!   space, word joiner, soft hyphen, Unicode tag characters).
//!
//! They are removed where text enters ATENA. Addresses are stored only when they are plain
//! absolute `http(s)` URLs of a sensible length – never `javascript:`, `data:` or `file:`.

use std::borrow::Cow;

/// Longest address ATENA fetches or stores (browsers commonly stop near 2,000 characters, and
/// database indexes cannot hold much longer values).
pub const MAX_URL_LEN: usize = 2_000;

/// `true` for characters removed from every text. Tabs and line breaks are kept; the joiners
/// needed by Arabic, Persian, Indic scripts and emoji (U+200C, U+200D) and the plain direction
/// marks (U+200E, U+200F, U+061C) are kept too – they do not hide anything.
pub fn is_hidden(c: char) -> bool {
    matches!(c,
        '\u{0}'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' // C0 controls
        | '\u{7f}'..='\u{9f}'                                       // DEL and C1 controls
        | '\u{ad}'                                                  // soft hyphen
        | '\u{180e}'                                                // Mongolian vowel separator
        | '\u{200b}'                                                // zero-width space
        | '\u{202a}'..='\u{202e}'                                   // embeddings and overrides
        | '\u{2060}'..='\u{2064}'                                   // word joiner, invisible operators
        | '\u{2066}'..='\u{2069}'                                   // isolates
        | '\u{feff}'                                                // zero-width no-break space
        | '\u{fff9}'..='\u{fffb}'                                   // interlinear annotations
        | '\u{e0000}'..='\u{e007f}'                                 // tag characters
    )
}

/// The text without hidden characters; `\r\n` and lone `\r` become `\n`.
pub fn clean_text(text: &str) -> Cow<'_, str> {
    if !text.chars().any(|c| c == '\r' || is_hidden(c)) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' if chars.peek() == Some(&'\n') => {}
            '\r' => out.push('\n'),
            c if is_hidden(c) => {}
            c => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// One line: hidden characters removed, tabs and line breaks turned into spaces, runs of white
/// space collapsed, ends trimmed. For titles, names, link texts and quotes.
pub fn clean_line(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for word in clean_text(text).split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// `true` for an address ATENA may store or follow: an absolute `http` or `https` URL of at most
/// [`MAX_URL_LEN`] characters, without white space, hidden characters or credentials.
pub fn is_storable_url(url: &str) -> bool {
    let lower = url.get(..8).unwrap_or(url).to_ascii_lowercase();
    let rest = if lower.starts_with("https://") {
        &url[8..]
    } else if lower.starts_with("http://") {
        &url[7..]
    } else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    url.len() <= MAX_URL_LEN
        && !authority.is_empty()
        && !authority.contains('@')
        && !url
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || is_hidden(c))
}

/// Text that is safe to print on a terminal: control and hidden characters other than line
/// breaks and tabs are shown as `�`, so a hostile value cannot move the cursor, erase lines or
/// disguise itself.
pub fn for_terminal(text: &str) -> Cow<'_, str> {
    let unsafe_char = |c: char| (c.is_control() || is_hidden(c)) && c != '\n' && c != '\t';
    if !text.chars().any(unsafe_char) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        text.chars()
            .map(|c| if unsafe_char(c) { '\u{fffd}' } else { c })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_characters_are_removed() {
        assert_eq!(
            clean_text("Škola\u{0} a\u{202e}gnp.exe\u{202c}"),
            "Škola agnp.exe"
        );
        assert_eq!(clean_text("vzde\u{ad}lá\u{200b}vanie"), "vzdelávanie");
        assert_eq!(clean_text("a\r\nb\rc\td"), "a\nb\nc\td");
        assert_eq!(clean_text("hidden\u{e0041}\u{e0042}"), "hidden");
        // nothing to remove: the same text, not a copy
        assert!(matches!(clean_text("bežný text"), Cow::Borrowed(_)));
        // joiners and direction marks that scripts need stay
        assert_eq!(
            clean_text("می\u{200c}خواهم \u{200f}"),
            "می\u{200c}خواهم \u{200f}"
        );
    }

    #[test]
    fn lines_are_single_and_tidy() {
        assert_eq!(
            clean_line("  Zápis\n\tdo  1.\u{1b}[2J ročníka "),
            "Zápis do 1.[2J ročníka"
        );
        assert_eq!(clean_line("\u{200b}"), "");
    }

    #[test]
    fn only_plain_web_addresses_are_storable() {
        assert!(is_storable_url("https://www.minedu.sk/zapis?rok=2026#a"));
        assert!(is_storable_url("HTTP://example.org"));
        for bad in [
            "javascript:alert(document.domain)",
            "data:text/html,<script>",
            "file:///etc/passwd",
            "https://user:secret@example.org/",
            "https://example.org/a b",
            "https://example.org/\u{202e}",
            "https://",
            "//example.org/",
        ] {
            assert!(!is_storable_url(bad), "{bad}");
        }
        let long = format!("https://example.org/{}", "a".repeat(MAX_URL_LEN));
        assert!(!is_storable_url(&long));
    }

    #[test]
    fn terminal_output_cannot_be_driven() {
        assert_eq!(for_terminal("ok\u{1b}[2J\u{7}"), "ok\u{fffd}[2J\u{fffd}");
        assert_eq!(for_terminal("riadok\nďalší"), "riadok\nďalší");
    }
}
