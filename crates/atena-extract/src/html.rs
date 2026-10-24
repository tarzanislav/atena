//! HTML: metadata, links and the main text without menus, footers, cookie bars and sidebars.
//!
//! Finding the main text:
//! 1. Elements that are never content are skipped: scripts, styles, form controls, navigation,
//!    hidden elements, ARIA landmarks for navigation, and elements whose `class`/`id` names them
//!    as menus, cookie bars, share buttons… (`header`/`footer` only outside an article).
//! 2. A `<main>` (or `role="main"`) with enough text is the content. One or two `<article>`s are too.
//! 3. Otherwise every paragraph-like element scores its text length reduced by its link density;
//!    the score goes to its parent and half of it to the grandparent. The best element wins and
//!    grows to its parent while the parent's other children hold at least half as much text.
//!
//! The document tree is walked iteratively, so deeply nested markup cannot exhaust the stack.

use std::collections::HashMap;

use ego_tree::iter::Edge;
use ego_tree::{NodeId, NodeRef};
use jiff::Timestamp;
use scraper::node::Element;
use scraper::{ElementRef, Html, Node};
use url::Url;

use crate::meta::parse_date;

/// Text this long in `<main>` or `<article>` makes it the content.
const SEMANTIC_MIN: usize = 140;
/// Elements with less text are not paragraphs.
const PARAGRAPH_MIN: usize = 25;
/// Lowest score of a scored content element.
const SCORE_MIN: f64 = 60.0;
/// Links kept from one page.
pub const MAX_LINKS: usize = 5_000;
/// Nesting depth at which a page is cut before parsing. The HTML parser slows down with the
/// square of the depth, so a hostile page with 100 000 nested elements would stall it for minutes;
/// real pages stay well below 100.
pub const MAX_DEPTH: usize = 512;
/// Largest HTML passed to the parser, in bytes.
pub const MAX_HTML_BYTES: usize = 16 * 1024 * 1024;

/// Elements that never contain others or whose end tag may be left out; they do not count.
const UNCOUNTED: [&str; 34] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr", "p", "li", "dt", "dd", "tr", "td", "th", "option", "optgroup", "thead",
    "tbody", "tfoot", "colgroup", "caption", "rb", "rt", "rp", "html", "head", "body",
];

/// The page up to the point where the (estimated) nesting depth exceeds [`MAX_DEPTH`],
/// and at most [`MAX_HTML_BYTES`] long.
fn limit_nesting(html: &str) -> &str {
    let mut end = html.len().min(MAX_HTML_BYTES);
    while !html.is_char_boundary(end) {
        end -= 1;
    }
    let html = &html[..end];
    let bytes = html.as_bytes();
    let (mut i, mut depth) = (0, 0usize);
    let find = |from: usize, needle: &str| {
        html.get(from..)
            .and_then(|t| t.find(needle))
            .map(|at| from + at)
    };
    while let Some(at) = find(i, "<") {
        let rest = &bytes[at + 1..];
        let name_of = |from: usize| -> String {
            rest[from..]
                .iter()
                .take_while(|b| b.is_ascii_alphanumeric())
                .map(|b| b.to_ascii_lowercase() as char)
                .collect()
        };
        let close = find(at, ">").unwrap_or(html.len());
        match rest.first() {
            Some(b'!') => {
                i = if rest.starts_with(b"!--") {
                    find(at, "-->").map_or(html.len(), |e| e + 3)
                } else {
                    close + 1
                };
                continue;
            }
            Some(b'/') => {
                if !UNCOUNTED.contains(&name_of(1).as_str()) {
                    depth = depth.saturating_sub(1);
                }
            }
            Some(b) if b.is_ascii_alphabetic() => {
                let name = name_of(0);
                if matches!(name.as_str(), "script" | "style") {
                    i = find(at, &format!("</{name}")).unwrap_or(html.len());
                    continue;
                }
                let self_closing = html[at..close.min(html.len())].ends_with('/');
                if !self_closing && !UNCOUNTED.contains(&name.as_str()) {
                    depth += 1;
                    if depth > MAX_DEPTH {
                        return &html[..at];
                    }
                }
            }
            _ => {}
        }
        i = close.max(at + 1);
    }
    html
}

const SKIPPED_TAGS: [&str; 18] = [
    "script", "style", "noscript", "template", "svg", "math", "canvas", "iframe", "object",
    "embed", "button", "select", "input", "textarea", "nav", "aside", "dialog", "head",
];
const SKIPPED_ROLES: [&str; 9] = [
    "navigation",
    "banner",
    "contentinfo",
    "complementary",
    "search",
    "menu",
    "menubar",
    "dialog",
    "alertdialog",
];
/// `class`/`id` words that mark page furniture (whole words: `share` but not `shared`).
const BOILERPLATE_WORDS: [&str; 31] = [
    "nav",
    "navbar",
    "navigation",
    "menu",
    "submenu",
    "footer",
    "sidebar",
    "breadcrumb",
    "breadcrumbs",
    "cookie",
    "cookies",
    "consent",
    "gdpr",
    "banner",
    "share",
    "sharing",
    "social",
    "advert",
    "advertisement",
    "ads",
    "ad",
    "promo",
    "newsletter",
    "popup",
    "modal",
    "related",
    "pagination",
    "pager",
    "skip",
    "toolbar",
    "skiplink",
];
const BLOCK_TAGS: [&str; 36] = [
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
    "caption",
    "dd",
    "details",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "tr",
    "ul",
    "center",
];
const PARAGRAPH_TAGS: [&str; 14] = [
    "p",
    "pre",
    "blockquote",
    "li",
    "dd",
    "td",
    "th",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "figcaption",
];
const INLINE_TAGS: [&str; 18] = [
    "a", "abbr", "b", "bdi", "cite", "code", "em", "font", "i", "kbd", "mark", "q", "s", "small",
    "span", "strong", "sub", "sup",
];

/// A link found in the content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// Absolute `http(s)` URL without fragment.
    pub url: Url,
    /// Anchor text.
    pub text: String,
    /// `rel="nofollow"` was set.
    pub nofollow: bool,
}

/// What the HTML module finds in a page.
#[derive(Debug, Clone, Default)]
pub(crate) struct HtmlParts {
    pub title: Option<String>,
    pub text: String,
    pub description: Option<String>,
    pub declared_language: Option<String>,
    pub authors: Vec<String>,
    pub published: Option<Timestamp>,
    pub modified: Option<Timestamp>,
    pub canonical: Option<Url>,
    pub feeds: Vec<Url>,
    pub links: Vec<Link>,
    pub robots: Vec<String>,
    pub tdm_reservation: Option<String>,
    pub tdm_policy: Option<String>,
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_article(el: &Element) -> bool {
    matches!(el.name(), "article" | "main") || el.attr("role") == Some("main")
}

fn is_skipped(el: &Element, in_article: bool) -> bool {
    let name = el.name();
    if matches!(name, "html" | "body" | "main" | "article") {
        return false;
    }
    if SKIPPED_TAGS.contains(&name) || (matches!(name, "header" | "footer") && !in_article) {
        return true;
    }
    if el.attr("hidden").is_some() || el.attr("aria-hidden") == Some("true") {
        return true;
    }
    if let Some(style) = el.attr("style") {
        let style: String = style
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_lowercase();
        if style.contains("display:none") || style.contains("visibility:hidden") {
            return true;
        }
    }
    if el
        .attr("role")
        .is_some_and(|r| SKIPPED_ROLES.contains(&r.trim().to_ascii_lowercase().as_str()))
    {
        return true;
    }
    [el.attr("class"), el.attr("id")]
        .into_iter()
        .flatten()
        .any(|names| {
            names
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|word| BOILERPLATE_WORDS.contains(&word.to_ascii_lowercase().as_str()))
        })
}

/// Visible text of an element: all of it, inside links, and its own (text and inline children).
#[derive(Debug, Clone, Copy, Default)]
struct Measure {
    text: usize,
    links: usize,
    own: usize,
}

impl Measure {
    fn link_density(&self) -> f64 {
        if self.text == 0 {
            0.0
        } else {
            self.links as f64 / self.text as f64
        }
    }
}

fn visible_chars(text: &str) -> usize {
    text.chars().filter(|c| !c.is_whitespace()).count()
}

/// Measures every element that is not skipped; skipped subtrees are absent from the map.
fn measure(doc: &Html) -> HashMap<NodeId, Measure> {
    let mut out = HashMap::new();
    let mut stack: Vec<(NodeId, Measure, bool)> = Vec::new();
    let (mut skip, mut article, mut in_link) = (0usize, 0usize, 0usize);
    for edge in doc.tree.root().traverse() {
        match edge {
            Edge::Open(node) => match node.value() {
                Node::Element(el) => {
                    if skip > 0 || is_skipped(el, article > 0) {
                        skip += 1;
                        continue;
                    }
                    article += usize::from(is_article(el));
                    let link = el.name() == "a";
                    in_link += usize::from(link);
                    stack.push((node.id(), Measure::default(), link));
                }
                Node::Text(text) if skip == 0 => {
                    let n = visible_chars(text);
                    if let Some((_, m, _)) = stack.last_mut() {
                        m.text += n;
                        m.own += n;
                        if in_link > 0 {
                            m.links += n;
                        }
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
                article -= usize::from(is_article(el));
                let Some((id, m, link)) = stack.pop() else {
                    continue;
                };
                in_link -= usize::from(link);
                if let Some((_, parent, _)) = stack.last_mut() {
                    parent.text += m.text;
                    parent.links += m.links;
                    if INLINE_TAGS.contains(&el.name()) {
                        parent.own += m.text;
                    }
                }
                out.insert(id, m);
            }
        }
    }
    out
}

fn is_paragraph(el: &Element, m: &Measure) -> bool {
    if PARAGRAPH_TAGS.contains(&el.name()) {
        m.text >= PARAGRAPH_MIN
    } else {
        matches!(
            el.name(),
            "div" | "section" | "span" | "font" | "center" | "article" | "td"
        ) && m.own >= PARAGRAPH_MIN
    }
}

/// The element holding the main text (see the module documentation).
fn content_root<'a>(doc: &'a Html, measures: &HashMap<NodeId, Measure>) -> NodeRef<'a, Node> {
    let elements: Vec<ElementRef<'a>> = doc
        .tree
        .root()
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|e| measures.contains_key(&e.id()))
        .collect();
    let text = |e: &ElementRef<'_>| measures[&e.id()].text;
    let body = || {
        elements
            .iter()
            .find(|e| e.value().name() == "body")
            .map_or_else(|| doc.tree.root(), |e| **e)
    };

    let main = elements
        .iter()
        .filter(|e| e.value().name() == "main" || e.value().attr("role") == Some("main"))
        .filter(|e| text(e) >= SEMANTIC_MIN)
        .max_by_key(|e| text(e));
    if let Some(main) = main {
        return **main;
    }
    let articles: Vec<&ElementRef<'a>> = elements
        .iter()
        .filter(|e| e.value().name() == "article" && text(e) >= SEMANTIC_MIN)
        .collect();
    if (1..=2).contains(&articles.len()) {
        return **articles
            .into_iter()
            .max_by_key(|e| text(e))
            .expect("one or two");
    }

    let mut scores: HashMap<NodeId, f64> = HashMap::new();
    for e in &elements {
        let m = measures[&e.id()];
        if !is_paragraph(e.value(), &m) {
            continue;
        }
        let score = m.text as f64 * (1.0 - m.link_density());
        let parent = e.parent();
        if let Some(p) = parent {
            *scores.entry(p.id()).or_default() += score;
            if let Some(g) = p.parent() {
                *scores.entry(g.id()).or_default() += score / 2.0;
            }
        }
    }
    let Some((&best_id, &best)) = scores
        .iter()
        .filter(|(id, _)| measures.contains_key(id))
        .max_by(|a, b| a.1.total_cmp(b.1).then_with(|| b.0.cmp(a.0)))
    else {
        return body();
    };
    if best < SCORE_MIN {
        return body();
    }
    let mut root = doc.tree.get(best_id).expect("scored node exists");
    let mut score = best;
    // grow to the parent while its other children hold at least half as much text
    while let Some(parent) = root.parent() {
        let parent_score = scores.get(&parent.id()).copied().unwrap_or_default();
        if !measures.contains_key(&parent.id()) || parent_score < 0.75 * score {
            break;
        }
        root = parent;
        score = parent_score.max(score);
    }
    root
}

/// Renders an element as plain text: blocks become paragraphs (blank line between them),
/// `<br>` a line break, list items get a bullet and table cells a separator.
struct Writer {
    out: String,
    line: String,
    pre: usize,
}

impl Writer {
    fn text(&mut self, text: &str) {
        if self.pre > 0 {
            self.line.push_str(text);
            return;
        }
        for c in text.chars() {
            if c.is_whitespace() {
                if !self.line.is_empty() && !self.line.ends_with([' ', '\n']) {
                    self.line.push(' ');
                }
            } else {
                self.line.push(c);
            }
        }
    }

    fn flush(&mut self) {
        let paragraph = self
            .line
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && *l != "•")
            .collect::<Vec<_>>()
            .join("\n");
        if !paragraph.is_empty() {
            if !self.out.is_empty() {
                self.out.push_str("\n\n");
            }
            self.out.push_str(&paragraph);
        }
        self.line.clear();
    }
}

/// Text and links of the content root.
fn render(root: NodeRef<'_, Node>, base: &Url) -> (String, Vec<Link>) {
    let mut w = Writer {
        out: String::new(),
        line: String::new(),
        pre: 0,
    };
    let mut links: Vec<Link> = Vec::new();
    let mut open_links: Vec<(usize, Option<Link>)> = Vec::new();
    let in_article_already = root
        .ancestors()
        .chain(std::iter::once(root))
        .filter_map(|n| n.value().as_element())
        .any(is_article);
    let (mut skip, mut article) = (0usize, usize::from(in_article_already));
    for edge in root.traverse() {
        match edge {
            Edge::Open(node) => match node.value() {
                Node::Element(el) => {
                    if skip > 0 || is_skipped(el, article > 0) {
                        skip += 1;
                        continue;
                    }
                    article += usize::from(is_article(el));
                    let name = el.name();
                    if BLOCK_TAGS.contains(&name) {
                        w.flush();
                    }
                    match name {
                        "br" => w.line.push('\n'),
                        "li" => w.line.push_str("• "),
                        "td" | "th" if !w.line.trim().is_empty() => w.line.push_str(" | "),
                        "pre" => w.pre += 1,
                        "a" => {
                            let link = el
                                .attr("href")
                                .and_then(|h| base.join(h.trim()).ok())
                                .filter(|u| matches!(u.scheme(), "http" | "https"));
                            let nofollow = el.attr("rel").is_some_and(|r| {
                                r.split_whitespace()
                                    .any(|t| t.eq_ignore_ascii_case("nofollow"))
                            });
                            open_links.push((
                                w.out.len() + w.line.len(),
                                link.map(|mut url| {
                                    url.set_fragment(None);
                                    Link {
                                        url,
                                        text: String::new(),
                                        nofollow,
                                    }
                                }),
                            ));
                        }
                        _ => {}
                    }
                }
                Node::Text(text) if skip == 0 => {
                    w.text(text);
                    for (_, link) in open_links.iter_mut() {
                        if let Some(link) = link {
                            link.text.push_str(text);
                        }
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
                article -= usize::from(is_article(el));
                match el.name() {
                    "pre" => w.pre = w.pre.saturating_sub(1),
                    "a" => {
                        if let Some((_, Some(mut link))) = open_links.pop() {
                            link.text = collapse(&link.text);
                            if links.len() < MAX_LINKS && !links.iter().any(|l| l.url == link.url) {
                                links.push(link);
                            }
                        }
                    }
                    _ => {}
                }
                if BLOCK_TAGS.contains(&el.name()) {
                    w.flush();
                }
            }
        }
    }
    w.flush();
    (w.out, links)
}

fn element_text(e: &ElementRef<'_>) -> String {
    collapse(&e.text().collect::<String>())
}

fn non_empty(text: String) -> Option<String> {
    (!text.is_empty()).then_some(text)
}

/// Parses an HTML page fetched from `url`.
pub(crate) fn extract_html(html: &str, url: &Url) -> HtmlParts {
    let doc = Html::parse_document(limit_nesting(html));
    let elements = || doc.tree.root().descendants().filter_map(ElementRef::wrap);
    let named = |name: &'static str| elements().filter(move |e| e.value().name() == name);

    let base = named("base")
        .find_map(|b| b.value().attr("href"))
        .and_then(|h| url.join(h).ok())
        .unwrap_or_else(|| url.clone());

    // <meta name|property|itemprop|http-equiv=… content=…>, first value wins
    let mut metas: HashMap<String, String> = HashMap::new();
    let mut robots = Vec::new();
    let mut authors = Vec::new();
    for m in named("meta") {
        let v = m.value();
        let (Some(key), Some(content)) = (
            v.attr("name")
                .or(v.attr("property"))
                .or(v.attr("itemprop"))
                .or(v.attr("http-equiv")),
            v.attr("content"),
        ) else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let content = collapse(content);
        match key.as_str() {
            "robots" | "atena" => robots.push(content.clone()),
            "author" | "dc.creator" | "dcterms.creator"
                if !content.is_empty() && !authors.contains(&content) =>
            {
                authors.push(content.clone());
            }
            _ => {}
        }
        metas.entry(key).or_insert(content);
    }
    let meta = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| metas.get(*k).cloned())
            .and_then(non_empty)
    };
    let date = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| metas.get(*k).and_then(|v| parse_date(v)))
    };

    let mut canonical = None;
    let mut feeds = Vec::new();
    for l in named("link") {
        let v = l.value();
        let rel = v.attr("rel").unwrap_or_default().to_ascii_lowercase();
        let Some(href) = v.attr("href").and_then(|h| base.join(h.trim()).ok()) else {
            continue;
        };
        let rels: Vec<&str> = rel.split_whitespace().collect();
        if rels.contains(&"canonical") && canonical.is_none() {
            canonical = Some(href);
        } else if rels.contains(&"alternate")
            && matches!(
                v.attr("type")
                    .map(|t| t.trim().to_ascii_lowercase())
                    .as_deref(),
                Some("application/rss+xml" | "application/atom+xml")
            )
            && !feeds.contains(&href)
        {
            feeds.push(href);
        }
    }

    let measures = measure(&doc);
    let root = content_root(&doc, &measures);
    let (text, links) = render(root, &base);

    let title = meta(&["og:title", "dc.title", "dcterms.title"])
        .or_else(|| {
            named("h1")
                .map(|h| element_text(&h))
                .find(|t| !t.is_empty())
        })
        .or_else(|| {
            named("title")
                .next()
                .map(|t| element_text(&t))
                .and_then(non_empty)
        });
    let published = date(&[
        "article:published_time",
        "datepublished",
        "dcterms.issued",
        "dc.date.issued",
        "dcterms.created",
        "dc.date",
        "date",
        "pubdate",
    ])
    .or_else(|| {
        ElementRef::wrap(root)
            .into_iter()
            .flat_map(|r| {
                r.descendants()
                    .filter_map(ElementRef::wrap)
                    .collect::<Vec<_>>()
            })
            .filter(|e| e.value().name() == "time")
            .find_map(|t| t.value().attr("datetime").and_then(parse_date))
    });

    HtmlParts {
        title,
        text,
        description: meta(&["description", "og:description", "dc.description"]),
        declared_language: named("html")
            .find_map(|h| {
                h.value()
                    .attr("lang")
                    .or(h.value().attr("xml:lang"))
                    .map(str::to_string)
            })
            .or_else(|| meta(&["content-language", "dc.language"])),
        authors,
        published,
        modified: date(&[
            "article:modified_time",
            "datemodified",
            "dcterms.modified",
            "og:updated_time",
        ]),
        canonical,
        feeds,
        links,
        robots,
        tdm_reservation: meta(&["tdm-reservation"]),
        tdm_policy: meta(&["tdm-policy"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url() -> Url {
        Url::parse("https://www.skola.sk/oznamy/zapis.html").unwrap()
    }

    const PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
  <title>Zápis do 1. ročníka | ZŠ Príkladná</title>
  <meta name="description" content="Informácie o zápise">
  <meta name="robots" content="noarchive">
  <meta name="googlebot" content="noindex">
  <meta name="tdm-reservation" content="1">
  <meta property="article:published_time" content="2026-03-01T08:00:00+01:00">
  <meta name="author" content="Mgr. Jana Nová">
  <link rel="canonical" href="/oznamy/zapis">
  <link rel="alternate" type="application/rss+xml" href="/rss.xml">
</head>
<body>
  <div id="cookie-bar">Používame cookies. <button>Súhlasím</button></div>
  <header class="site-header"><a href="/">Domov</a></header>
  <nav><ul><li><a href="/">Domov</a></li><li><a href="/kontakt">Kontakt</a></li></ul></nav>
  <div class="layout">
    <div class="side-menu"><a href="/a">A</a> <a href="/b">B</a></div>
    <div class="content">
      <h1>Zápis do 1. ročníka</h1>
      <p>Zápis detí do prvého ročníka sa uskutoční 15. a 16. apríla 2026 v budove školy.</p>
      <p>Zákonný zástupca prinesie <a href="/docs/ziadost.pdf#strana2">žiadosť o prijatie</a> a rodný list dieťaťa.</p>
      <ul><li>termín: 15. 4. 2026</li><li>čas: 14:00 – 17:00</li></ul>
      <table><tr><td>Trieda</td><td>Kapacita</td></tr><tr><td>1.A</td><td>24</td></tr></table>
      <p style="display: none">skrytý text</p>
      <script>var tracking = 1;</script>
    </div>
  </div>
  <footer>© 2026 ZŠ Príkladná · <a href="/gdpr">GDPR</a></footer>
</body></html>"#;

    #[test]
    fn main_text_metadata_and_links() {
        let p = extract_html(PAGE, &url());
        assert_eq!(p.title.as_deref(), Some("Zápis do 1. ročníka"));
        assert_eq!(
            p.text,
            "Zápis do 1. ročníka\n\n\
             Zápis detí do prvého ročníka sa uskutoční 15. a 16. apríla 2026 v budove školy.\n\n\
             Zákonný zástupca prinesie žiadosť o prijatie a rodný list dieťaťa.\n\n\
             • termín: 15. 4. 2026\n\n• čas: 14:00 – 17:00\n\n\
             Trieda | Kapacita\n\n1.A | 24"
        );
        assert_eq!(p.links.len(), 1);
        assert_eq!(
            p.links[0].url.as_str(),
            "https://www.skola.sk/docs/ziadost.pdf"
        );
        assert_eq!(p.links[0].text, "žiadosť o prijatie");
        assert_eq!(
            p.canonical.unwrap().as_str(),
            "https://www.skola.sk/oznamy/zapis"
        );
        assert_eq!(p.feeds[0].as_str(), "https://www.skola.sk/rss.xml");
        assert_eq!(p.robots, ["noarchive"]);
        assert_eq!(p.tdm_reservation.as_deref(), Some("1"));
        assert_eq!(p.published.unwrap().to_string(), "2026-03-01T07:00:00Z");
        assert_eq!(p.authors, ["Mgr. Jana Nová"]);
        assert_eq!(p.declared_language.as_deref(), Some("en"));
        assert_eq!(p.description.as_deref(), Some("Informácie o zápise"));
    }

    #[test]
    fn main_element_and_article_header_are_kept() {
        let html = r#"<body><header>Logo a menu školy</header>
          <main><article><header><h1>Nový školský zákon</h1><time datetime="2026-09-01">1. 9. 2026</time></header>
          <p>Od septembra platí nový školský zákon, ktorý mení pravidlá hodnotenia žiakov aj organizáciu vyučovania.</p>
          <footer>Autor: redakcia</footer></article></main>
          <aside><p>Najčítanejšie články tohto týždňa a mnoho ďalšieho textu v bočnom paneli stránky.</p></aside></body>"#;
        let p = extract_html(html, &url());
        assert!(
            p.text
                .starts_with("Nový školský zákon\n\n1. 9. 2026\n\nOd septembra"),
            "{}",
            p.text
        );
        assert!(p.text.ends_with("Autor: redakcia"));
        assert!(!p.text.contains("Logo") && !p.text.contains("Najčítanejšie"));
        assert_eq!(p.published.unwrap().to_string(), "2026-09-01T00:00:00Z");
    }

    #[test]
    fn old_table_layout_with_br_and_no_paragraphs() {
        let html = r#"<html><body><form id="aspnetForm"><table><tr>
          <td class="menu"><a href="/1">Úvod</a><br><a href="/2">Dokumenty</a></td>
          <td><font size="2"><b>Oznam riaditeľa školy</b><br>Vážení rodičia, v piatok 20. marca 2026 bude riaditeľské voľno.<br>
          Školský klub detí bude v prevádzke od 6:30 do 16:00.</font></td></tr></table></form></body></html>"#;
        let p = extract_html(html, &url());
        assert_eq!(
            p.text,
            "Oznam riaditeľa školy\nVážení rodičia, v piatok 20. marca 2026 bude riaditeľské voľno.\n\
             Školský klub detí bude v prevádzke od 6:30 do 16:00."
        );
        assert_eq!(p.title, None);
    }

    #[test]
    fn deep_nesting_is_cut_before_parsing() {
        let hostile = format!(
            "{}<p>hlboko</p>{}",
            "<div>".repeat(100_000),
            "</div>".repeat(100_000)
        );
        let start = std::time::Instant::now();
        let p = extract_html(&hostile, &url());
        assert!(
            start.elapsed() < std::time::Duration::from_secs(20),
            "{:?}",
            start.elapsed()
        );
        assert!(!p.text.contains("hlboko"));

        let normal = format!(
            "{}<p>hlboko</p>{}",
            "<div><ul><li>".repeat(150),
            "</ul></div>".repeat(150)
        );
        assert!(extract_html(&normal, &url()).text.contains("hlboko"));
        assert_eq!(
            limit_nesting("<div><!-- <div><div> --><script>'<div>'</script><br/><img></div>").len(),
            64
        );
        let cut = format!("{}x", "<span>".repeat(MAX_DEPTH + 1));
        assert_eq!(limit_nesting(&cut).len(), MAX_DEPTH * 6);
    }
}
