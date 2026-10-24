//! PDF: text, document information, language and the TDMRep reservation in XMP metadata.
//!
//! The PDF library can panic on malformed files; the panic is caught and reported as an error.
//! Text extraction of a hostile file can still take long, so callers run it on a worker thread
//! with a time limit.

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::danger;
use jiff::Timestamp;
use pdf_extract::{Document, Object, PlainTextOutput, decode_text_string, output_doc};

use crate::meta::parse_pdf_date;

/// Namespace of the TDMRep properties in XMP.
const TDMREP_NS: &str = "http://www.w3.org/ns/tdmrep/";

#[derive(Debug, Clone, Default)]
pub(crate) struct PdfParts {
    pub title: Option<String>,
    pub text: String,
    pub authors: Vec<String>,
    pub published: Option<Timestamp>,
    pub modified: Option<Timestamp>,
    pub declared_language: Option<String>,
    pub tdm_reservation: Option<String>,
    pub tdm_policy: Option<String>,
}

fn text_of(doc: &Document, object: Option<&Object>) -> Option<String> {
    let (_, object) = doc.dereference(object?).ok()?;
    let text = decode_text_string(object).ok()?;
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}

/// The value of `prefix:name` as an attribute (`tdm:reservation="1"`) or element.
fn xmp_value(xmp: &str, prefix: &str, name: &str) -> Option<String> {
    let qualified = format!("{prefix}:{name}");
    let mut from = 0;
    while let Some(at) = xmp[from..].find(&qualified) {
        let start = from + at;
        let after = &xmp[start + qualified.len()..];
        let before = xmp[..start].chars().next_back();
        if let Some(rest) = after.trim_start().strip_prefix('=') {
            let rest = rest.trim_start();
            if let Some(quote) = rest.chars().next().filter(|q| matches!(q, '"' | '\'')) {
                return rest[1..].split(quote).next().map(|v| v.trim().to_string());
            }
        } else if before == Some('<')
            && let Some(rest) = after.strip_prefix('>')
        {
            return rest.split('<').next().map(|v| v.trim().to_string());
        }
        from = start + qualified.len();
    }
    None
}

/// `tdm:reservation` and `tdm:policy`, whatever prefix the file binds to the TDMRep namespace.
fn xmp_tdm(xmp: &str) -> (Option<String>, Option<String>) {
    let prefix = xmp.match_indices("xmlns:").find_map(|(at, _)| {
        let rest = &xmp[at + "xmlns:".len()..];
        let (prefix, value) = rest.split_once('=')?;
        let value = value.trim_start();
        let quote = value.chars().next()?;
        let uri = value.get(1..)?.split(quote).next()?;
        (uri == TDMREP_NS).then(|| prefix.trim().to_string())
    });
    match prefix {
        Some(p) => (
            xmp_value(xmp, &p, "reservation"),
            xmp_value(xmp, &p, "policy"),
        ),
        None => (None, None),
    }
}

/// Tidies extracted text: no trailing spaces or form feeds, at most one blank line, and words
/// hyphenated at a line end joined again (`vzdelá-` + `vanie` → `vzdelávanie`).
fn tidy(text: &str) -> String {
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim_end_matches(|c: char| c.is_whitespace() || c == '\u{c}'))
        .collect();
    let mut out = String::with_capacity(text.len());
    let mut blank = 0;
    let mut joined = false;
    for (i, line) in lines.iter().enumerate() {
        let line = line.trim_start_matches('\u{c}');
        if line.trim().is_empty() {
            blank += 1;
            continue;
        }
        if !out.is_empty() && !joined {
            out.push_str(if blank > 0 { "\n\n" } else { "\n" });
        }
        blank = 0;
        let next_starts_lower = lines
            .get(i + 1)
            .and_then(|n| n.trim_start().chars().next())
            .is_some_and(char::is_lowercase);
        let hyphenated = line.ends_with('-')
            && line.chars().rev().nth(1).is_some_and(char::is_alphabetic)
            && next_starts_lower;
        if hyphenated {
            out.push_str(&line[..line.len() - 1]);
        } else {
            out.push_str(line);
        }
        joined = hyphenated;
        if joined {
            // the next line continues the word
            continue;
        }
    }
    out
}

/// Why a PDF is not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PdfError {
    /// Damaged or protected.
    Broken(String),
    /// Carries active content (see `danger`).
    Dangerous(&'static str),
}

fn extract(bytes: &[u8]) -> Result<PdfParts, PdfError> {
    let broken = |e: &dyn std::fmt::Display| PdfError::Broken(e.to_string());
    let mut doc = Document::load_mem(bytes).map_err(|e| broken(&e))?;
    if doc.is_encrypted() {
        // documents with an owner password only are readable with the empty user password
        doc.decrypt("")
            .map_err(|_| broken(&"the PDF is protected by a password"))?;
    }
    if let Some(active) = danger::pdf_active_content(&doc) {
        return Err(PdfError::Dangerous(active));
    }
    let mut raw = String::new();
    output_doc(&doc, &mut PlainTextOutput::new(&mut raw)).map_err(|e| broken(&e))?;

    let info = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|o| doc.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok());
    let field = |key: &[u8]| text_of(&doc, info.and_then(|d| d.get(key).ok()));
    let catalog = doc.catalog().ok();
    let xmp = catalog
        .and_then(|c| c.get(b"Metadata").ok())
        .and_then(|o| doc.dereference(o).ok())
        .and_then(|(_, o)| o.as_stream().ok())
        .map(|s| {
            s.decompressed_content()
                .unwrap_or_else(|_| s.content.clone())
        })
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    let (tdm_reservation, tdm_policy) = xmp.as_deref().map(xmp_tdm).unwrap_or_default();

    Ok(PdfParts {
        title: field(b"Title"),
        text: tidy(&raw),
        authors: field(b"Author").into_iter().collect(),
        published: field(b"CreationDate").as_deref().and_then(parse_pdf_date),
        modified: field(b"ModDate").as_deref().and_then(parse_pdf_date),
        declared_language: text_of(&doc, catalog.and_then(|c| c.get(b"Lang").ok())),
        tdm_reservation,
        tdm_policy,
    })
}

/// Extracts a PDF; a panic inside the PDF library becomes an error.
pub(crate) fn extract_pdf(bytes: &[u8]) -> Result<PdfParts, PdfError> {
    catch_unwind(AssertUnwindSafe(|| extract(bytes))).unwrap_or_else(|_| {
        Err(PdfError::Broken(
            "the PDF library failed on this file".to_string(),
        ))
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn tdm_in_xmp_with_any_prefix() {
        let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF><rdf:Description
            xmlns:rights="http://www.w3.org/ns/tdmrep/" rights:policy='https://pub.example/policy.json'>
            <rights:reservation>1</rights:reservation></rdf:Description></rdf:RDF></x:xmpmeta>"#;
        assert_eq!(
            xmp_tdm(xmp),
            (
                Some("1".into()),
                Some("https://pub.example/policy.json".into())
            )
        );
        assert_eq!(
            xmp_tdm(r#"<x xmlns:tdm="urn:other"><tdm:reservation>1</tdm:reservation></x>"#),
            (None, None)
        );
    }

    #[test]
    fn tidy_text() {
        assert_eq!(
            tidy("Celoživotné vzdelá-\nvanie učiteľov  \n\u{c}\n\n\nČesko-\nSlovensko\n"),
            "Celoživotné vzdelávanie učiteľov\n\nČesko-\nSlovensko"
        );
    }

    #[test]
    fn active_content_is_refused() {
        use pdf_extract::{Dictionary, StringFormat};

        let mut doc = Document::load_mem(&sample_pdf(&["Oznam riaditela"])).unwrap();
        let mut action = Dictionary::new();
        action.set("S", Object::Name(b"JavaScript".to_vec()));
        action.set(
            "JS",
            Object::String(b"app.alert(1)".to_vec(), StringFormat::Literal),
        );
        let action_id = doc.add_object(action);
        let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        doc.get_object_mut(catalog_id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("OpenAction", Object::Reference(action_id));
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        assert!(matches!(
            extract_pdf(&bytes),
            Err(PdfError::Dangerous("JavaScript"))
        ));
        // the same document without the action is read
        assert!(extract_pdf(&sample_pdf(&["Oznam riaditela"])).is_ok());
    }

    /// A one-page PDF with Helvetica text, document information and XMP metadata.
    pub(crate) fn sample_pdf(lines: &[&str]) -> Vec<u8> {
        use pdf_extract::content::{Content, Operation};
        use pdf_extract::{Dictionary, Stream, StringFormat};

        let mut doc = Document::with_version("1.7");
        let mut font = Dictionary::new();
        font.set("Type", Object::Name(b"Font".to_vec()));
        font.set("Subtype", Object::Name(b"Type1".to_vec()));
        font.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
        font.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
        let font_id = doc.add_object(font);
        let mut fonts = Dictionary::new();
        fonts.set("F1", Object::Reference(font_id));
        let mut resources = Dictionary::new();
        resources.set("Font", Object::Dictionary(fonts));

        let mut ops = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
            Operation::new("TL", vec![16.into()]),
            Operation::new("Td", vec![72.into(), 720.into()]),
        ];
        for line in lines {
            ops.push(Operation::new(
                "Tj",
                vec![Object::String(
                    line.as_bytes().to_vec(),
                    StringFormat::Literal,
                )],
            ));
            ops.push(Operation::new("T*", vec![]));
        }
        ops.push(Operation::new("ET", vec![]));
        let content = Content { operations: ops }.encode().unwrap();
        let content_id = doc.add_object(Stream::new(Dictionary::new(), content));

        let pages_id = doc.new_object_id();
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set("Contents", Object::Reference(content_id));
        page.set("Resources", Object::Dictionary(resources));
        page.set("MediaBox", vec![0.into(), 0.into(), 595.into(), 842.into()]);
        let page_id = doc.add_object(page);
        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name(b"Pages".to_vec()));
        pages.set("Kids", vec![Object::Reference(page_id)]);
        pages.set("Count", 1);
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let xmp = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:tdm="http://www.w3.org/ns/tdmrep/" tdm:reservation="1" tdm:policy="https://pub.example/p.json"/>
            </rdf:RDF></x:xmpmeta>"#;
        let mut meta = Dictionary::new();
        meta.set("Type", Object::Name(b"Metadata".to_vec()));
        meta.set("Subtype", Object::Name(b"XML".to_vec()));
        let meta_id = doc.add_object(Stream::new(meta, xmp.to_vec()));

        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        catalog.set(
            "Lang",
            Object::String(b"sk-SK".to_vec(), StringFormat::Literal),
        );
        catalog.set("Metadata", Object::Reference(meta_id));
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut info = Dictionary::new();
        info.set(
            "Title",
            Object::String(b"Metodicky pokyn".to_vec(), StringFormat::Literal),
        );
        info.set(
            "Author",
            Object::String(b"Ministerstvo".to_vec(), StringFormat::Literal),
        );
        info.set(
            "CreationDate",
            Object::String(b"D:20260901080000+02'00'".to_vec(), StringFormat::Literal),
        );
        let info_id = doc.add_object(info);
        doc.trailer.set("Info", Object::Reference(info_id));

        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn text_and_metadata_of_a_real_pdf() {
        let bytes = sample_pdf(&["Metodicky pokyn k hodnoteniu", "ziakov zakladnej skoly."]);
        let p = extract_pdf(&bytes).unwrap();
        assert!(
            p.text.contains("Metodicky pokyn k hodnoteniu"),
            "{:?}",
            p.text
        );
        assert!(p.text.contains("ziakov zakladnej skoly."), "{:?}", p.text);
        assert_eq!(p.title.as_deref(), Some("Metodicky pokyn"));
        assert_eq!(p.authors, ["Ministerstvo"]);
        assert_eq!(p.published.unwrap().to_string(), "2026-09-01T06:00:00Z");
        assert_eq!(p.declared_language.as_deref(), Some("sk-SK"));
        assert_eq!(p.tdm_reservation.as_deref(), Some("1"));
        assert_eq!(p.tdm_policy.as_deref(), Some("https://pub.example/p.json"));
    }

    #[test]
    fn broken_files_are_errors() {
        assert!(extract_pdf(b"%PDF-1.4\n garbage").is_err());
        assert!(extract_pdf(b"").is_err());
    }
}
