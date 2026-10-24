//! Path patterns shared by robots.txt (RFC 9309) and TDMRep.
//!
//! `*` matches any sequence of characters and a trailing `$` anchors the end of the path.
//! Paths and patterns are normalised the same way before comparison, so `/a%62c`, `/abc`
//! and `/ab%63` are equal while `/a%2Fb` (an encoded slash) stays different from `/a/b`.

/// A compiled path pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pattern {
    /// Literal parts between the `*` wildcards; the first one must match at the start.
    parts: Vec<String>,
    /// `true` when the pattern ended with `$`.
    anchored: bool,
    /// Length of the normalised pattern in octets – its specificity.
    len: usize,
}

const HEX: &[u8; 16] = b"0123456789ABCDEF";

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

fn hex_value(b: u8) -> Option<u8> {
    (b as char).to_digit(16).map(|d| d as u8)
}

fn push_escaped(out: &mut String, b: u8) {
    out.push('%');
    out.push(HEX[usize::from(b >> 4)] as char);
    out.push(HEX[usize::from(b & 0x0F)] as char);
}

/// Normalises a path: escapes of unreserved characters are decoded, other escapes upper-cased,
/// and bytes outside printable ASCII escaped. In a URL path `*` and `$` are ordinary characters,
/// so they are escaped too (`literal_specials`); in a pattern they keep their meaning.
fn normalize(text: &str, literal_specials: bool) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%'
            && let (Some(h), Some(l)) = (
                bytes.get(i + 1).copied().and_then(hex_value),
                bytes.get(i + 2).copied().and_then(hex_value),
            )
        {
            let value = h << 4 | l;
            if is_unreserved(value) {
                out.push(value as char);
            } else {
                push_escaped(&mut out, value);
            }
            i += 3;
            continue;
        }
        if b <= b' ' || b >= 0x7F || (literal_specials && matches!(b, b'*' | b'$')) {
            push_escaped(&mut out, b);
        } else {
            out.push(b as char);
        }
        i += 1;
    }
    out
}

/// Normalises the path and query of a URL for matching (`/a/b?c=d`).
pub(crate) fn normalize_path(path_and_query: &str) -> String {
    normalize(path_and_query, true)
}

impl Pattern {
    /// Compiles a rule value. A value that starts with neither `/` nor `*` gets a leading `/`,
    /// so a careless `Disallow: private/` still protects `/private/`.
    pub(crate) fn new(value: &str) -> Self {
        let mut text = normalize(value, false);
        if !text.starts_with('/') && !text.starts_with('*') {
            text.insert(0, '/');
        }
        let len = text.len();
        let anchored = text.ends_with('$');
        if anchored {
            text.pop();
        }
        // only the final `$` is special; any other is an ordinary character
        let text = text.replace('$', "%24");
        Self {
            parts: text.split('*').map(str::to_string).collect(),
            anchored,
            len,
        }
    }

    /// Specificity of the pattern: the number of octets in its normalised form.
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// `true` when a normalised path (see [`normalize_path`]) matches the pattern.
    pub(crate) fn matches(&self, path: &str) -> bool {
        let (first, rest) = self.parts.split_first().expect("split yields one part");
        let Some(mut tail) = path.strip_prefix(first.as_str()) else {
            return false;
        };
        let Some((last, middle)) = rest.split_last() else {
            return !self.anchored || tail.is_empty();
        };
        // leftmost placement of every middle part leaves the most room for the rest
        for part in middle {
            match tail.find(part.as_str()) {
                Some(at) => tail = &tail[at + part.len()..],
                None => return false,
            }
        }
        if self.anchored {
            tail.ends_with(last.as_str())
        } else {
            tail.contains(last.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(pattern: &str, path: &str) -> bool {
        Pattern::new(pattern).matches(&normalize_path(path))
    }

    #[test]
    fn prefix_wildcard_and_anchor() {
        assert!(m("/fish", "/fish.html"));
        assert!(m("/fish", "/fish/salmon?id=1"));
        assert!(!m("/fish", "/Fish.asp"));
        assert!(m("/*.php", "/folder/index.php?x=1"));
        assert!(m("/*.php$", "/index.php"));
        assert!(!m("/*.php$", "/index.php?x=1"));
        assert!(m("/fish*.php", "/fishheads/catfish.php"));
        assert!(m("/a*b*c$", "/axxbyybzzc"));
        assert!(!m("/a*b*c$", "/axxbyybzzcd"));
        assert!(m("/$", "/"));
        assert!(!m("/$", "/page"));
        assert!(m("*", "/anything"));
        assert!(m("/**", "/x"));
    }

    #[test]
    fn rfc_9309_percent_encoding_table() {
        assert!(m("/foo/bar?baz=quz", "/foo/bar?baz=quz"));
        assert!(m("/foo/bar/ツ", "/foo/bar/%E3%83%84"));
        assert!(m("/foo/bar/%E3%83%84", "/foo/bar/%e3%83%84"));
        assert!(m("/foo/bar/%62%61%7A", "/foo/bar/baz"));
        assert!(m("/foo/bar/baz", "/foo/bar/%62%61%7A"));
        // an encoded slash is not a path separator
        assert!(!m("/a/b", "/a%2Fb"));
    }

    #[test]
    fn specials_in_urls_are_literal() {
        assert!(m("/price%24", "/price$"));
        assert!(!m("/a$b", "/a"));
        assert!(m("/a$b", "/a$b"));
        assert!(m("/star%2A", "/star*"));
    }

    #[test]
    fn missing_slash_is_added_and_length_counts_octets() {
        assert!(m("private/", "/private/x"));
        assert_eq!(Pattern::new("/ab*$").len(), 5);
        assert_eq!(Pattern::new("/ツ").len(), "/%E3%83%84".len());
    }
}
