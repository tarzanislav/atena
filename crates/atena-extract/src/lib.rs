//! ATENA content extraction: from downloaded bytes to plain text, metadata and language.
//!
//! [`extract`] recognises the format (signature first, then the declared content type), decodes
//! legacy encodings, finds the main text of web pages without menus and footers, reads PDF, DOCX
//! and ODT files, and determines the language. The publisher's rules found inside the document
//! (`<meta name="robots">`, TDMRep in HTML and PDF) are returned raw for the pipeline to merge
//! with those from the HTTP response.
//!
//! Text paragraphs are separated by a blank line, which is what change detection in `atena-core`
//! compares.

mod decode;
mod html;
mod lang;
mod meta;
mod office;
mod pdf;

use jiff::Timestamp;
use url::Url;

pub use html::{Link, MAX_LINKS};
pub use lang::{Language, LanguageSource, detect as detect_language};
pub use office::MAX_PART_BYTES;

/// Longest text kept from one document, in characters.
pub const MAX_TEXT_CHARS: usize = 2_000_000;

/// Format of the source document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// HTML or XHTML page.
    Html,
    /// PDF document.
    Pdf,
    /// Word document (DOCX).
    Docx,
    /// OpenDocument text (ODT).
    Odt,
    /// Plain text.
    Text,
}

/// The content of a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// Recognised format.
    pub format: Format,
    /// Title (page heading or document metadata).
    pub title: Option<String>,
    /// Main text; paragraphs separated by a blank line.
    pub text: String,
    /// `true` when the text was cut at [`MAX_TEXT_CHARS`].
    pub truncated: bool,
    /// Description from metadata.
    pub description: Option<String>,
    /// Language of the text.
    pub language: Option<Language>,
    /// Language declared by the document (`<html lang>`, metadata), as written.
    pub declared_language: Option<String>,
    /// Authors from metadata.
    pub authors: Vec<String>,
    /// Publication or creation time from metadata.
    pub published: Option<Timestamp>,
    /// Modification time from metadata.
    pub modified: Option<Timestamp>,
    /// Canonical URL of a web page.
    pub canonical: Option<Url>,
    /// Links in the main content of a web page.
    pub links: Vec<Link>,
    /// RSS/Atom feeds announced by a web page.
    pub feeds: Vec<Url>,
    /// Contents of `<meta name="robots">` and `<meta name="atena">` (apply with `IndexRules::add_list`).
    pub robots: Vec<String>,
    /// TDMRep reservation from the document (`<meta>` or XMP): `Some(true)` reserved.
    pub tdm_reservation: Option<bool>,
    /// TDMRep policy URL from the document.
    pub tdm_policy: Option<String>,
    /// Encoding the text was decoded from (HTML and plain text).
    pub encoding: Option<&'static str>,
}

/// A document that cannot be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExtractError {
    /// Not a format ATENA reads (images, video, archives…).
    #[error("unsupported format ({0})")]
    Unsupported(String),
    /// A damaged or protected PDF.
    #[error("PDF: {0}")]
    Pdf(String),
    /// A damaged DOCX or ODT.
    #[error("office document: {0}")]
    Office(String),
}

fn tdm_value(value: Option<&str>) -> Option<bool> {
    match value?.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

fn essence(content_type: Option<&str>) -> String {
    content_type
        .and_then(|ct| ct.split(';').next())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

/// Recognises the format: file signatures first (servers often send PDFs as `text/html` or
/// `application/octet-stream`), then the content type, then a look at the text.
fn format_of(
    body: &[u8],
    content_type: Option<&str>,
) -> Result<(Format, Option<office::OfficeFormat>), ExtractError> {
    let head = &body[..body.len().min(1024)];
    if head.windows(5).any(|w| w == b"%PDF-") {
        return Ok((Format::Pdf, None));
    }
    if body.starts_with(b"PK\x03\x04") {
        return match office::detect(body) {
            Some(office::OfficeFormat::Docx) => {
                Ok((Format::Docx, Some(office::OfficeFormat::Docx)))
            }
            Some(office::OfficeFormat::Odt) => Ok((Format::Odt, Some(office::OfficeFormat::Odt))),
            None => Err(ExtractError::Unsupported("ZIP archive".into())),
        };
    }
    let lower = String::from_utf8_lossy(head).to_ascii_lowercase();
    match essence(content_type).as_str() {
        "text/html" | "application/xhtml+xml" => Ok((Format::Html, None)),
        "text/plain" | "text/csv" => Ok((Format::Text, None)),
        _ if lower.contains("<html")
            || lower.contains("<!doctype html")
            || lower.contains("<body") =>
        {
            Ok((Format::Html, None))
        }
        "" | "application/octet-stream" if !head.contains(&0) => Ok((Format::Text, None)),
        other => Err(ExtractError::Unsupported(if other.is_empty() {
            "binary data".into()
        } else {
            other.to_string()
        })),
    }
}

fn cut(text: String) -> (String, bool) {
    match text.char_indices().nth(MAX_TEXT_CHARS) {
        Some((end, _)) => (text[..end].to_string(), true),
        None => (text, false),
    }
}

fn plain_text(text: &str) -> String {
    let mut out = String::new();
    let mut blank = false;
    for line in text.lines().map(str::trim_end) {
        if line.trim().is_empty() {
            blank = !out.is_empty();
            continue;
        }
        if !out.is_empty() {
            out.push_str(if blank { "\n\n" } else { "\n" });
        }
        out.push_str(line);
        blank = false;
    }
    out
}

/// Extracts a document downloaded from `url` with the given `Content-Type`.
pub fn extract(
    body: &[u8],
    content_type: Option<&str>,
    url: &Url,
) -> Result<Document, ExtractError> {
    let (format, office_format) = format_of(body, content_type)?;
    let mut doc = Document {
        format,
        title: None,
        text: String::new(),
        truncated: false,
        description: None,
        language: None,
        declared_language: None,
        authors: Vec::new(),
        published: None,
        modified: None,
        canonical: None,
        links: Vec::new(),
        feeds: Vec::new(),
        robots: Vec::new(),
        tdm_reservation: None,
        tdm_policy: None,
        encoding: None,
    };
    let text = match format {
        Format::Html => {
            let (html, encoding) = decode::decode(body, content_type, url.host_str());
            let p = html::extract_html(&html, url);
            doc.encoding = Some(encoding);
            doc.title = p.title;
            doc.description = p.description;
            doc.declared_language = p.declared_language;
            doc.authors = p.authors;
            doc.published = p.published;
            doc.modified = p.modified;
            doc.canonical = p.canonical;
            doc.links = p.links;
            doc.feeds = p.feeds;
            doc.robots = p.robots;
            doc.tdm_reservation = tdm_value(p.tdm_reservation.as_deref());
            doc.tdm_policy = p.tdm_policy;
            p.text
        }
        Format::Text => {
            let (text, encoding) = decode::decode(body, content_type, url.host_str());
            doc.encoding = Some(encoding);
            plain_text(&text)
        }
        Format::Pdf => {
            let p = pdf::extract_pdf(body).map_err(ExtractError::Pdf)?;
            doc.title = p.title;
            doc.authors = p.authors;
            doc.published = p.published;
            doc.modified = p.modified;
            doc.declared_language = p.declared_language;
            doc.tdm_reservation = tdm_value(p.tdm_reservation.as_deref());
            doc.tdm_policy = p.tdm_policy;
            p.text
        }
        Format::Docx | Format::Odt => {
            let kind = office_format.expect("set for office formats");
            let p = office::extract_office(body, kind).map_err(ExtractError::Office)?;
            doc.title = p.title;
            doc.authors = p.authors;
            doc.published = p.published;
            doc.modified = p.modified;
            doc.declared_language = p.declared_language;
            p.text
        }
    };
    (doc.text, doc.truncated) = cut(text);
    let sample = match &doc.title {
        Some(title) if doc.text.chars().count() < 200 => format!("{title}\n{}", doc.text),
        _ => doc.text.clone(),
    };
    doc.language = lang::decide(&sample, doc.declared_language.as_deref());
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url() -> Url {
        Url::parse("https://www.example.cz/dokumenty/").unwrap()
    }

    #[test]
    fn format_is_recognised_by_signature_before_content_type() {
        let docx = office::tests::zip_of(&[
            ("word/document.xml", office::tests::DOCX_BODY),
            ("docProps/core.xml", office::tests::DOCX_CORE),
        ]);
        let d = extract(&docx, Some("application/octet-stream"), &url()).unwrap();
        assert_eq!(d.format, Format::Docx);
        assert_eq!(d.language.unwrap().code, "sk");
        assert!(matches!(
            extract(b"%PDF-1.7 broken", Some("text/html"), &url()),
            Err(ExtractError::Pdf(_))
        ));
        assert!(matches!(
            extract(b"\x89PNG\r\n\x1a\n\0\0", Some("image/png"), &url()),
            Err(ExtractError::Unsupported(_))
        ));
        assert!(matches!(
            extract(&office::tests::zip_of(&[("a.txt", "x")]), None, &url()),
            Err(ExtractError::Unsupported(_))
        ));
    }

    #[test]
    fn pdf_through_the_public_interface() {
        let bytes =
            pdf::tests::sample_pdf(&["Metodicky pokyn k hodnoteniu", "ziakov zakladnej skoly."]);
        let d = extract(&bytes, Some("application/pdf"), &url()).unwrap();
        assert_eq!(d.format, Format::Pdf);
        assert_eq!(d.tdm_reservation, Some(true));
        assert_eq!(d.language.unwrap().code, "sk");
    }

    #[test]
    fn html_in_a_legacy_encoding() {
        let html = "<html lang=\"cs\"><head><meta charset=\"windows-1250\"><title>Přijímací řízení</title>\
            <meta name=\"robots\" content=\"noindex\"><meta name=\"tdm-reservation\" content=\"1\"></head>\
            <body><p>Přijímací zkoušky na střední školy se konají v dubnu. Uchazeči podávají přihlášky do 1. března.</p></body></html>";
        let (bytes, _, _) = encoding_rs::WINDOWS_1250.encode(html);
        let d = extract(&bytes, Some("text/html"), &url()).unwrap();
        assert_eq!(d.encoding, Some("windows-1250"));
        assert_eq!(d.title.as_deref(), Some("Přijímací řízení"));
        assert!(d.text.starts_with("Přijímací zkoušky na střední školy"));
        let lang = d.language.unwrap();
        assert_eq!(
            (lang.code.as_str(), lang.source),
            ("cs", LanguageSource::Detected)
        );
        assert_eq!(d.robots, ["noindex"]);
        assert_eq!(d.tdm_reservation, Some(true));
    }

    #[test]
    fn plain_text_and_truncation() {
        let d = extract(
            "Учебный план\r\n\r\n\r\nПредметы и часы для 5 класса основной школы.  \n".as_bytes(),
            Some("text/plain; charset=utf-8"),
            &url(),
        )
        .unwrap();
        assert_eq!(
            d.text,
            "Учебный план\n\nПредметы и часы для 5 класса основной школы."
        );
        assert_eq!(d.language.unwrap().code, "ru");
        let long = "a".repeat(MAX_TEXT_CHARS + 5);
        let d = extract(long.as_bytes(), Some("text/plain"), &url()).unwrap();
        assert!(d.truncated && d.text.len() == MAX_TEXT_CHARS);
    }
}
