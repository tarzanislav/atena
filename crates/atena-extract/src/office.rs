//! Word (DOCX) and OpenDocument (ODT) files: paragraphs and core metadata.
//!
//! Both are ZIP archives of XML. Only the parts that are needed are read, each limited in size,
//! so a "zip bomb" cannot exhaust memory. Deleted text of tracked changes is left out.

use std::io::{Cursor, Read};

use jiff::Timestamp;
use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

use crate::meta::parse_date;

/// Largest uncompressed part that is read.
pub const MAX_PART_BYTES: u64 = 50 * 1024 * 1024;

/// Office document format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OfficeFormat {
    Docx,
    Odt,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct OfficeParts {
    pub title: Option<String>,
    pub text: String,
    pub authors: Vec<String>,
    pub published: Option<Timestamp>,
    pub modified: Option<Timestamp>,
    pub declared_language: Option<String>,
}

fn read_part(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Option<String>, String> {
    let Ok(file) = zip.by_name(name) else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    file.take(MAX_PART_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{name}: {e}"))?;
    if bytes.len() as u64 > MAX_PART_BYTES {
        return Err(format!("{name} is larger than {MAX_PART_BYTES} bytes"));
    }
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}

fn reader(xml: &str) -> Reader<&[u8]> {
    let mut reader = Reader::from_str(xml);
    let config = reader.config_mut();
    config.check_end_names = false;
    config.allow_unmatched_ends = true;
    reader
}

fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn push_paragraph(out: &mut String, paragraph: &mut String) {
    let text = paragraph.trim();
    if !text.is_empty() {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(text);
    }
    paragraph.clear();
}

fn entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => {
            let num = name.strip_prefix('#')?;
            let code = match num.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => num.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

/// Elements whose text is not part of the body: ODT notes, comments and tracked deletions.
const ODT_SKIPPED: [&str; 3] = ["note", "annotation", "tracked-changes"];

/// Body text: `w:t` inside `w:p` (DOCX) or all text inside `text:p` / `text:h` (ODT).
fn body_text(xml: &str, format: OfficeFormat) -> Result<String, String> {
    let mut reader = reader(xml);
    let (mut out, mut paragraph) = (String::new(), String::new());
    // DOCX: depth inside <w:t>; ODT: depth inside <text:p>/<text:h>
    let mut collecting = 0usize;
    let mut skipped = 0usize;
    loop {
        let event = reader.read_event().map_err(|e| e.to_string())?;
        match event {
            Event::Start(e) => match (format, local(e.name().as_ref())) {
                (OfficeFormat::Odt, name) if skipped > 0 || ODT_SKIPPED.contains(&name) => {
                    skipped += 1
                }
                (OfficeFormat::Docx, "t") | (OfficeFormat::Odt, "p" | "h") => collecting += 1,
                _ => {}
            },
            Event::Empty(_) if skipped > 0 => {}
            Event::Empty(e) => match (format, local(e.name().as_ref())) {
                (_, "tab") => paragraph.push('\t'),
                (OfficeFormat::Docx, "br" | "cr") | (OfficeFormat::Odt, "line-break") => {
                    paragraph.push('\n');
                }
                (OfficeFormat::Odt, "s") => {
                    let count = e
                        .try_get_attribute("text:c")
                        .ok()
                        .flatten()
                        .and_then(|a| a.value.trim().parse::<usize>().ok())
                        .unwrap_or(1)
                        .min(100);
                    paragraph.push_str(&" ".repeat(count));
                }
                _ => {}
            },
            Event::End(_) if skipped > 0 => skipped -= 1,
            Event::End(e) => match (format, local(e.name().as_ref())) {
                (OfficeFormat::Docx, "t") => collecting = collecting.saturating_sub(1),
                (OfficeFormat::Docx, "p") => push_paragraph(&mut out, &mut paragraph),
                (OfficeFormat::Odt, "p" | "h") => {
                    collecting = collecting.saturating_sub(1);
                    if collecting == 0 {
                        push_paragraph(&mut out, &mut paragraph);
                    }
                }
                _ => {}
            },
            Event::Text(t) if collecting > 0 && skipped == 0 => match format {
                OfficeFormat::Docx => paragraph.push_str(&t.xml10_content()),
                // ODF collapses runs of white space; explicit spaces come as <text:s>
                OfficeFormat::Odt => {
                    for c in t.xml10_content().chars() {
                        if !c.is_whitespace() {
                            paragraph.push(c);
                        } else if !paragraph.ends_with([' ', '\n', '\t']) {
                            paragraph.push(' ');
                        }
                    }
                }
            },
            Event::GeneralRef(r) if collecting > 0 && skipped == 0 => {
                if let Some(c) = entity(&r.xml10_content()) {
                    paragraph.push(c);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    push_paragraph(&mut out, &mut paragraph);
    Ok(out)
}

/// Core metadata from `docProps/core.xml` or `meta.xml`: title, creator, dates, language.
fn metadata(xml: &str, parts: &mut OfficeParts) {
    let mut reader = reader(xml);
    let mut current: Option<String> = None;
    let mut text = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                current = Some(local(e.name().as_ref()).to_string());
                text.clear();
            }
            Ok(Event::Text(t)) => text.push_str(&t.xml10_content()),
            Ok(Event::GeneralRef(r)) => text.extend(entity(&r.xml10_content())),
            Ok(Event::End(_)) => {
                let value = text.split_whitespace().collect::<Vec<_>>().join(" ");
                let set = |slot: &mut Option<String>| {
                    if slot.is_none() && !value.is_empty() {
                        *slot = Some(value.clone());
                    }
                };
                match current.take().as_deref() {
                    Some("title") => set(&mut parts.title),
                    Some("creator" | "initial-creator") => {
                        if !value.is_empty() && !parts.authors.contains(&value) {
                            parts.authors.push(value.clone());
                        }
                    }
                    Some("created" | "creation-date") => {
                        parts.published = parts.published.or_else(|| parse_date(&value));
                    }
                    Some("modified" | "date") => {
                        parts.modified = parts.modified.or_else(|| parse_date(&value))
                    }
                    Some("language") => set(&mut parts.declared_language),
                    _ => {}
                }
                text.clear();
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
}

/// Most parts an office document may have; more is an archive bomb, not a text.
const MAX_PARTS: usize = 5_000;
/// The `mimetype` part of an OpenDocument text.
const ODT_MIMETYPE: &str = "application/vnd.oasis.opendocument.text";

/// The format of a ZIP archive, if it is a supported office document; `Err` names active content
/// (macros, ActiveX, OLE objects, remote templates, scripts), for which the file is refused.
pub(crate) fn detect(bytes: &[u8]) -> Result<Option<OfficeFormat>, &'static str> {
    let Ok(mut zip) = ZipArchive::new(Cursor::new(bytes)) else {
        return Ok(None);
    };
    if zip.len() > MAX_PARTS {
        return Err("too many parts for a document");
    }
    let names: Vec<String> = zip
        .file_names()
        .filter_map(|n| n.ok().map(|n| n.into_owned()))
        .collect();
    let has = |name: &str| names.iter().any(|n| n == name);
    let format = if has("word/document.xml") {
        OfficeFormat::Docx
    } else if has("content.xml")
        && read_part(&mut zip, "mimetype")
            .ok()
            .flatten()
            .is_some_and(|m| m.trim() == ODT_MIMETYPE)
    {
        OfficeFormat::Odt
    } else {
        return Ok(None);
    };
    let mut text_of = |name: &str| read_part(&mut zip, name).ok().flatten();
    match crate::danger::office_active_content(&names, &mut text_of) {
        Some(active) => Err(active),
        None => Ok(Some(format)),
    }
}

/// Extracts a DOCX or ODT file.
pub(crate) fn extract_office(bytes: &[u8], format: OfficeFormat) -> Result<OfficeParts, String> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let (body, meta) = match format {
        OfficeFormat::Docx => ("word/document.xml", "docProps/core.xml"),
        OfficeFormat::Odt => ("content.xml", "meta.xml"),
    };
    let xml = read_part(&mut zip, body)?.ok_or_else(|| format!("{body} is missing"))?;
    let mut parts = OfficeParts {
        text: body_text(&xml, format)?,
        ..OfficeParts::default()
    };
    if let Some(meta_xml) = read_part(&mut zip, meta)? {
        metadata(&meta_xml, &mut parts);
    }
    Ok(parts)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::io::Write;

    use zip::write::{SimpleFileOptions, ZipWriter};

    use super::*;

    /// Builds a ZIP archive from `(name, content)` pairs.
    pub(crate) fn zip_of(files: &[(&str, &str)]) -> Vec<u8> {
        let mut w = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, content) in files {
            w.start_file(*name, SimpleFileOptions::default()).unwrap();
            w.write_all(content.as_bytes()).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    pub(crate) const DOCX_BODY: &str = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>
        <w:p><w:r><w:t>Školský</w:t></w:r><w:r><w:t xml:space="preserve"> vzdelávací program</w:t></w:r></w:p>
        <w:p><w:r><w:t>Učebné osnovy &amp; ciele</w:t><w:tab/><w:t>2026</w:t></w:r>
             <w:del><w:r><w:delText>vymazaný text</w:delText></w:r></w:del></w:p>
        <w:p/>
        <w:p><w:r><w:t>Riadok 1</w:t><w:br/><w:t>Riadok 2</w:t></w:r></w:p>
        </w:body></w:document>"#;

    pub(crate) const DOCX_CORE: &str = r#"<cp:coreProperties xmlns:cp="x" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/">
        <dc:title>ŠkVP 2026</dc:title><dc:creator>Jana Nová</dc:creator><dc:language>sk-SK</dc:language>
        <dcterms:created>2026-09-01T07:00:00Z</dcterms:created><dcterms:modified>2026-10-01T07:00:00Z</dcterms:modified>
        </cp:coreProperties>"#;

    #[test]
    fn docx() {
        let bytes = zip_of(&[
            ("word/document.xml", DOCX_BODY),
            ("docProps/core.xml", DOCX_CORE),
        ]);
        assert_eq!(detect(&bytes), Ok(Some(OfficeFormat::Docx)));
        let p = extract_office(&bytes, OfficeFormat::Docx).unwrap();
        assert_eq!(
            p.text,
            "Školský vzdelávací program\n\nUčebné osnovy & ciele\t2026\n\nRiadok 1\nRiadok 2"
        );
        assert_eq!(p.title.as_deref(), Some("ŠkVP 2026"));
        assert_eq!(p.authors, ["Jana Nová"]);
        assert_eq!(p.declared_language.as_deref(), Some("sk-SK"));
        assert_eq!(p.published.unwrap().to_string(), "2026-09-01T07:00:00Z");
        assert_eq!(p.modified.unwrap().to_string(), "2026-10-01T07:00:00Z");
    }

    #[test]
    fn odt() {
        let content = r#"<office:document-content xmlns:office="o" xmlns:text="t"><office:body><office:text>
            <text:h text:outline-level="1">Hodnotenie žiakov</text:h>
            <text:p>Slovné<text:s text:c="2"/>hodnotenie<text:note><text:note-citation>1</text:note-citation><text:note-body><text:p>poznámka</text:p></text:note-body></text:note> v <text:span>prvom</text:span> ročníku.</text:p>
            </office:text></office:body></office:document-content>"#;
        let meta = r#"<office:document-meta xmlns:office="o" xmlns:meta="m" xmlns:dc="d"><office:meta>
            <dc:title>Hodnotenie</dc:title><meta:initial-creator>Peter</meta:initial-creator>
            <meta:creation-date>2026-02-01T10:00:00</meta:creation-date><dc:language>sk</dc:language></office:meta></office:document-meta>"#;
        let bytes = zip_of(&[
            ("mimetype", "application/vnd.oasis.opendocument.text"),
            ("content.xml", content),
            ("meta.xml", meta),
        ]);
        assert_eq!(detect(&bytes), Ok(Some(OfficeFormat::Odt)));
        let p = extract_office(&bytes, OfficeFormat::Odt).unwrap();
        assert_eq!(
            p.text,
            "Hodnotenie žiakov\n\nSlovné  hodnotenie v prvom ročníku."
        );
        assert_eq!(p.authors, ["Peter"]);
        assert_eq!(p.published.unwrap().to_string(), "2026-02-01T10:00:00Z");
    }

    #[test]
    fn other_archives_and_bombs() {
        assert_eq!(detect(&zip_of(&[("readme.txt", "x")])), Ok(None));
        assert_eq!(detect(b"not a zip"), Ok(None));
        let big = "a".repeat(MAX_PART_BYTES as usize + 10);
        let bytes = zip_of(&[("word/document.xml", &big)]);
        assert!(
            extract_office(&bytes, OfficeFormat::Docx)
                .unwrap_err()
                .contains("larger than")
        );
    }
}
