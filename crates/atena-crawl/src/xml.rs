//! A small, lenient XML walker for feeds and sitemaps.
//!
//! Element names are reduced to lower-case local names (`dc:date` → `date`), the text of each
//! element is collected with entities resolved, and child text is also added to the parent so that
//! XHTML content (`<content type="xhtml"><div>…</div></content>`) is not lost. Custom entities
//! declared in a DTD are never expanded, which rules out "billion laughs" attacks.

use std::borrow::Cow;

use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

/// Maximal collected text per element; the rest is dropped.
const MAX_TEXT: usize = 64 * 1024;

/// An opening tag.
pub(crate) struct Start<'a> {
    /// Lower-case local name.
    pub name: &'a str,
    /// Lower-case local name of the parent element.
    pub parent: Option<&'a str>,
    /// Nesting depth (the root element has depth 0).
    pub depth: usize,
    tag: &'a BytesStart<'a>,
}

impl Start<'_> {
    /// The value of an attribute by lower-case local name, entities resolved.
    pub fn attr(&self, name: &str) -> Option<String> {
        self.tag.attributes().flatten().find_map(|a| {
            (local(a.key.local_name().as_ref()) == name)
                .then(|| {
                    a.normalized_value(XmlVersion::Implicit1_0)
                        .map(Cow::into_owned)
                        .ok()
                })
                .flatten()
        })
    }
}

/// A closing tag with the trimmed text of the element.
pub(crate) struct End<'a> {
    /// Lower-case local name.
    pub name: &'a str,
    /// Lower-case local name of the parent element.
    pub parent: Option<&'a str>,
    /// Text content, trimmed.
    pub text: &'a str,
}

/// An event passed to the handler of [`walk`].
pub(crate) enum Node<'a> {
    /// An opening tag.
    Start(Start<'a>),
    /// A closing tag.
    End(End<'a>),
}

/// XML that could not be read at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid XML: {0}")]
pub struct XmlError(pub String);

fn local(name: &str) -> String {
    name.to_ascii_lowercase()
}

fn push_limited(buf: &mut String, text: &str) {
    let room = MAX_TEXT.saturating_sub(buf.len());
    if room == 0 {
        return;
    }
    let mut end = text.len().min(room);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    buf.push_str(&text[..end]);
}

/// Walks the document. An error after the root element was seen ends the walk quietly
/// (feeds in the wild are often slightly broken); an error before it is reported.
pub(crate) fn walk(text: &str, mut handle: impl FnMut(Node<'_>)) -> Result<(), XmlError> {
    let mut reader = Reader::from_str(text);
    let config = reader.config_mut();
    config.expand_empty_elements = true;
    config.check_end_names = false;
    config.allow_unmatched_ends = true;
    config.allow_dangling_amp = true;

    let mut names: Vec<String> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    let mut seen_root = false;
    loop {
        let event = match reader.read_event() {
            Ok(event) => event,
            Err(e) if !seen_root => return Err(XmlError(e.to_string())),
            Err(_) => break,
        };
        match event {
            Event::Start(tag) => {
                seen_root = true;
                let name = local(tag.local_name().as_ref());
                handle(Node::Start(Start {
                    name: &name,
                    parent: names.last().map(String::as_str),
                    depth: names.len(),
                    tag: &tag,
                }));
                names.push(name);
                texts.push(String::new());
            }
            Event::End(_) => {
                let (Some(name), Some(text)) = (names.pop(), texts.pop()) else {
                    continue;
                };
                let trimmed = text.trim();
                handle(Node::End(End {
                    name: &name,
                    parent: names.last().map(String::as_str),
                    text: trimmed,
                }));
                if let Some(parent) = texts.last_mut()
                    && !trimmed.is_empty()
                {
                    if !parent.is_empty() {
                        push_limited(parent, " ");
                    }
                    push_limited(parent, trimmed);
                }
            }
            Event::Text(t) => {
                if let Some(buf) = texts.last_mut() {
                    push_limited(buf, &t.xml10_content());
                }
            }
            Event::CData(c) => {
                if let Some(buf) = texts.last_mut() {
                    push_limited(buf, &c);
                }
            }
            Event::GeneralRef(r) => {
                if let Some(buf) = texts.last_mut() {
                    let name = r.xml10_content();
                    match (r.resolve_char_ref(), resolve_predefined_entity(&name)) {
                        (Ok(Some(c)), _) => buf.push(c),
                        (_, Some(s)) => push_limited(buf, s),
                        // unknown entity (often HTML's &nbsp; in feeds): keep it as written
                        _ => push_limited(buf, &format!("&{name};")),
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if seen_root {
        Ok(())
    } else {
        Err(XmlError("no root element".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_text_names_and_attributes() {
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        walk(
            r#"<?xml version="1.0"?><!DOCTYPE x [<!ENTITY lol "lol">]>
               <a:Root xmlns:a="urn:x"><b href="/x?a=1&amp;b=2">T &amp; &#x10D;&nbsp;<![CDATA[<i>c</i>]]></b><c/></a:Root>"#,
            |node| match node {
                Node::Start(s) => starts.push((s.name.to_string(), s.depth, s.attr("href"))),
                Node::End(e) => ends.push((e.name.to_string(), e.text.to_string())),
            },
        )
        .unwrap();
        assert_eq!(starts[0], ("root".to_string(), 0, None));
        assert_eq!(starts[1].2.as_deref(), Some("/x?a=1&b=2"));
        assert_eq!(
            ends[0],
            ("b".to_string(), "T & č&nbsp;<i>c</i>".to_string())
        );
        assert_eq!(ends[1], ("c".to_string(), String::new()));
        assert_eq!(ends[2].1, "T & č&nbsp;<i>c</i>");
    }

    #[test]
    fn broken_tail_is_tolerated_but_garbage_is_not() {
        let mut n = 0;
        let ok = walk("<rss><item>a</item><item>b</ite", |node| {
            if let Node::End(_) = node {
                n += 1;
            }
        });
        assert!(ok.is_ok() && n >= 1);
        assert!(walk("plain text", |_| {}).is_err());
    }
}
