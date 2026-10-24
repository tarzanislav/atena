//! Turning bytes into text: the encoding is taken from the byte-order mark, the HTTP header, the
//! document's own declaration, or – when nothing is declared or the declaration is clearly wrong –
//! guessed from the bytes. Older Slovak, Czech and Russian sites still use windows-1250,
//! ISO-8859-2, windows-1251 or KOI8-R.

use chardetng::{EncodingDetector, Iso2022JpDetection, Utf8Detection};
use encoding_rs::{Encoding, UTF_8};

/// Bytes examined for an in-document declaration (the HTML prescan length).
const PRESCAN: usize = 1024;

/// The encoding named by `charset=` in a `Content-Type` value.
pub(crate) fn charset_param(content_type: &str) -> Option<&'static Encoding> {
    content_type.split(';').skip(1).find_map(|param| {
        let (key, value) = param.split_once('=')?;
        key.trim()
            .eq_ignore_ascii_case("charset")
            .then(|| Encoding::for_label(value.trim().trim_matches(['"', '\'']).as_bytes()))
            .flatten()
    })
}

/// The value after `name=` (optionally quoted) in an ASCII-lower-cased tag.
fn attribute_value<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let at = tag.find(name)?;
    let rest = tag[at + name.len()..]
        .trim_start()
        .strip_prefix('=')?
        .trim_start();
    let rest = rest.trim_start_matches(['"', '\'']);
    let end = rest
        .find(|c: char| matches!(c, '"' | '\'' | ';' | '>' | '/') || c.is_whitespace())
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// `<meta charset>`, `<meta http-equiv="content-type" content="…charset=…">` or `<?xml encoding>`.
fn declared(bytes: &[u8]) -> Option<&'static Encoding> {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(PRESCAN)]).to_ascii_lowercase();
    let mut found = None;
    if head.trim_start().starts_with("<?xml") {
        let decl = &head[..head.find("?>").unwrap_or(head.len())];
        found = attribute_value(decl, "encoding");
    }
    for (at, _) in head.match_indices("<meta") {
        if found.is_some() {
            break;
        }
        let tag = &head[at..head[at..].find('>').map_or(head.len(), |end| at + end)];
        found = attribute_value(tag, "charset");
    }
    let encoding = Encoding::for_label(found?.as_bytes())?;
    // a UTF-16 declaration in an ASCII-compatible byte stream means UTF-8 (WHATWG rule)
    Some(if encoding.is_ascii_compatible() {
        encoding
    } else {
        UTF_8
    })
}

/// The top-level domain of a host, a hint for guessing (`sk`, `cz`, `ru`).
fn tld(host: Option<&str>) -> Option<&str> {
    host?
        .rsplit('.')
        .next()
        .filter(|t| t.chars().all(|c| c.is_ascii_alphabetic()))
}

/// Decodes `bytes`; returns the text and the name of the encoding used.
pub(crate) fn decode(
    bytes: &[u8],
    content_type: Option<&str>,
    host: Option<&str>,
) -> (String, &'static str) {
    let valid_utf8 = std::str::from_utf8(bytes).is_ok();
    let encoding = if let Some((bom, _)) = Encoding::for_bom(bytes) {
        bom
    } else {
        let claimed = content_type
            .and_then(charset_param)
            .or_else(|| declared(bytes));
        match claimed {
            // valid UTF-8 with non-ASCII characters is UTF-8, whatever the page claims
            _ if valid_utf8 && !bytes.is_ascii() => UTF_8,
            Some(enc) if enc != UTF_8 || valid_utf8 => enc,
            // no declaration, or a UTF-8 declaration that the bytes contradict
            _ => {
                let mut detector = EncodingDetector::new(Iso2022JpDetection::Deny);
                detector.feed(bytes, true);
                detector.guess(tld(host).map(str::as_bytes), Utf8Detection::Allow)
            }
        }
    };
    let (text, used, _) = encoding.decode(bytes);
    (text.into_owned(), used.name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declarations_are_found() {
        let html = br#"<!doctype html><html><head><META http-equiv="Content-Type" content="text/html; charset=windows-1250"></head>"#;
        assert_eq!(declared(html).map(Encoding::name), Some("windows-1250"));
        assert_eq!(
            declared(b"<meta charset='koi8-r'>").map(Encoding::name),
            Some("KOI8-R")
        );
        assert_eq!(
            declared(b"<?xml version=\"1.0\" encoding=\"ISO-8859-2\"?><rss/>").map(Encoding::name),
            Some("ISO-8859-2")
        );
        assert_eq!(
            declared(b"<meta charset=utf-16>").map(Encoding::name),
            Some("UTF-8")
        );
        assert_eq!(declared(b"<p>no declaration</p>"), None);
        assert_eq!(
            charset_param("text/html; Charset=\"UTF-8\"").map(Encoding::name),
            Some("UTF-8")
        );
        assert_eq!(charset_param("text/html"), None);
    }

    #[test]
    fn central_european_and_cyrillic_pages() {
        let (sk_1250, _, _) =
            encoding_rs::WINDOWS_1250.encode("Čitateľská gramotnosť žiakov v základnej škole");
        let (text, used) = decode(&sk_1250, Some("text/html; charset=windows-1250"), None);
        assert_eq!(
            (text.as_str(), used),
            (
                "Čitateľská gramotnosť žiakov v základnej škole",
                "windows-1250"
            )
        );

        // no declaration at all: guessed with the help of the domain
        let (text, _) = decode(&sk_1250, None, Some("www.skola.sk"));
        assert_eq!(text, "Čitateľská gramotnosť žiakov v základnej škole");

        let (ru, _, _) =
            encoding_rs::WINDOWS_1251.encode("Читательская грамотность учащихся средней школы");
        let (text, used) = decode(
            &ru,
            Some("text/html; charset=utf-8"),
            Some("edu.example.ru"),
        );
        assert_eq!(
            (text.as_str(), used),
            (
                "Читательская грамотность учащихся средней школы",
                "windows-1251"
            )
        );
    }

    #[test]
    fn utf8_wins_over_a_wrong_latin_declaration_and_boms_win_over_everything() {
        let (text, used) = decode(
            "Škola".as_bytes(),
            Some("text/html; charset=iso-8859-1"),
            None,
        );
        assert_eq!((text.as_str(), used), ("Škola", "UTF-8"));
        let mut bom = vec![0xEF, 0xBB, 0xBF];
        bom.extend_from_slice("Žiak".as_bytes());
        assert_eq!(
            decode(&bom, Some("text/plain; charset=windows-1250"), None).0,
            "Žiak"
        );
        assert_eq!(
            decode(
                b"plain ASCII",
                Some("text/plain; charset=windows-1250"),
                None
            )
            .1,
            "windows-1250"
        );
    }
}
