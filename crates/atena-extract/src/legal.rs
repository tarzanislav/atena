//! What a web page says about its own rights: licence links, links to its terms of use and its
//! whole visible text, footer included – the material for a rights assessment.
//!
//! Main-text extraction drops menus and footers on purpose; here they matter, because that is where
//! "All rights reserved", licence badges and the link to the terms of use usually are.

use ego_tree::iter::Edge;
use scraper::{Html, Node};
use url::Url;

use crate::decode;
use crate::html::{BLOCK_TAGS, Link, MAX_LINKS, collapse, limit_nesting};

/// Longest visible text kept, in characters.
pub const MAX_LEGAL_TEXT: usize = 400_000;

/// Elements whose content is never visible text.
const INVISIBLE: [&str; 11] = [
    "script", "style", "noscript", "template", "svg", "math", "canvas", "head", "select",
    "textarea", "iframe",
];

/// Rights-related signals of an HTML page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegalSignals {
    /// Targets of `rel="license"` on `<a>` and `<link>` elements.
    pub licence_links: Vec<Url>,
    /// Every `http(s)` link of the page with its text – menus and footers included.
    pub links: Vec<Link>,
    /// All visible text, blocks separated by blank lines.
    pub text: String,
}

fn hidden(el: &scraper::node::Element) -> bool {
    el.attr("hidden").is_some() || el.attr("aria-hidden") == Some("true")
}

/// Reads the rights-related signals of an HTML page fetched from `url`.
pub fn legal_signals(body: &[u8], content_type: Option<&str>, url: &Url) -> LegalSignals {
    let (html, _) = decode::decode(body, content_type, url.host_str());
    let doc = Html::parse_document(limit_nesting(&html));
    let base = doc
        .tree
        .root()
        .descendants()
        .filter_map(scraper::ElementRef::wrap)
        .find(|e| e.value().name() == "base")
        .and_then(|b| b.value().attr("href"))
        .and_then(|h| url.join(h.trim()).ok())
        .unwrap_or_else(|| url.clone());
    let absolute = |href: &str| {
        base.join(href.trim())
            .ok()
            .filter(|u| matches!(u.scheme(), "http" | "https"))
            .map(|mut u| {
                u.set_fragment(None);
                u
            })
    };

    let mut signals = LegalSignals::default();
    let mut blocks: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut skip = 0usize;
    let mut open_links: Vec<Option<Link>> = Vec::new();
    let mut length = 0usize;
    let flush = |line: &mut String, blocks: &mut Vec<String>, length: &mut usize| {
        let text = collapse(line);
        line.clear();
        if !text.is_empty() && *length < MAX_LEGAL_TEXT {
            *length += text.chars().count();
            blocks.push(text);
        }
    };
    for edge in doc.tree.root().traverse() {
        match edge {
            Edge::Open(node) => match node.value() {
                Node::Element(el) => {
                    if skip > 0 || INVISIBLE.contains(&el.name()) || hidden(el) {
                        skip += 1;
                        continue;
                    }
                    let rel_licence = el.attr("rel").is_some_and(|r| {
                        r.split_whitespace()
                            .any(|t| t.eq_ignore_ascii_case("license"))
                    });
                    if rel_licence
                        && let Some(target) = el.attr("href").and_then(absolute)
                        && !signals.licence_links.contains(&target)
                    {
                        signals.licence_links.push(target);
                    }
                    if BLOCK_TAGS.contains(&el.name()) || el.name() == "br" {
                        flush(&mut line, &mut blocks, &mut length);
                    }
                    if el.name() == "a" {
                        open_links.push(el.attr("href").and_then(absolute).map(|url| Link {
                            url,
                            text: String::new(),
                            nofollow: false,
                        }));
                    }
                }
                Node::Text(text) if skip == 0 => {
                    line.push_str(text);
                    for link in open_links.iter_mut().flatten() {
                        link.text.push_str(text);
                    }
                }
                _ => {}
            },
            Edge::Close(node) => {
                let Node::Element(el) = node.value() else {
                    continue;
                };
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                if el.name() == "a"
                    && let Some(Some(mut link)) = open_links.pop()
                {
                    link.text = collapse(&link.text);
                    if signals.links.len() < MAX_LINKS
                        && !signals.links.iter().any(|l| l.url == link.url)
                    {
                        signals.links.push(link);
                    }
                }
                if BLOCK_TAGS.contains(&el.name()) {
                    flush(&mut line, &mut blocks, &mut length);
                }
            }
        }
    }
    flush(&mut line, &mut blocks, &mut length);
    signals.text = blocks.join("\n\n");
    signals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_licence_and_terms_links_are_found() {
        let page = r#"<html><head><link rel="license" href="https://creativecommons.org/licenses/by/4.0/">
            <script>var x = "All rights reserved";</script></head><body>
            <nav><a href="/kontakt">Kontakt</a></nav>
            <main><h1>Oznamy</h1><p>Text oznamu.</p></main>
            <footer><p>© 2026 Škola. Všetky práva vyhradené.</p>
            <a href="/podmienky-pouzivania#hore">Podmienky používania</a>
            <a rel="license noopener" href="https://creativecommons.org/licenses/by/4.0/">CC BY 4.0</a>
            <div hidden>Skrytý text</div></footer></body></html>"#;
        let url = Url::parse("https://www.skola.sk/oznamy/").unwrap();
        let s = legal_signals(page.as_bytes(), Some("text/html; charset=utf-8"), &url);
        assert_eq!(
            s.licence_links.iter().map(Url::as_str).collect::<Vec<_>>(),
            ["https://creativecommons.org/licenses/by/4.0/"]
        );
        let links: Vec<(&str, &str)> = s
            .links
            .iter()
            .map(|l| (l.url.as_str(), l.text.as_str()))
            .collect();
        assert_eq!(
            links,
            [
                ("https://www.skola.sk/kontakt", "Kontakt"),
                (
                    "https://www.skola.sk/podmienky-pouzivania",
                    "Podmienky používania"
                ),
                ("https://creativecommons.org/licenses/by/4.0/", "CC BY 4.0"),
            ]
        );
        assert!(s.text.contains("Všetky práva vyhradené."), "{}", s.text);
        assert!(s.text.contains("Text oznamu."));
        assert!(!s.text.contains("Skrytý") && !s.text.contains("var x"));
    }
}
