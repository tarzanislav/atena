//! RSS 2.0, RSS 1.0 (RDF) and Atom feeds – the cheapest way to notice new items on a source.

use jiff::Timestamp;
use url::Url;

use crate::date::parse_date;
use crate::xml::{Node, XmlError, walk};

/// Items kept from one feed.
pub const MAX_ITEMS: usize = 10_000;
/// Characters kept from an item summary.
const MAX_SUMMARY: usize = 4_000;

/// Feed format, recognised by the root element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedFormat {
    /// `<rss>`
    Rss2,
    /// `<rdf:RDF>`
    Rss1,
    /// `<feed>`
    Atom,
}

/// One entry of a feed. Links are absolute `http(s)` URLs; the summary may contain HTML.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeedItem {
    /// Title.
    pub title: Option<String>,
    /// Link to the item.
    pub link: Option<Url>,
    /// Stable identifier (`guid`, Atom `id`).
    pub id: Option<String>,
    /// Publication time.
    pub published: Option<Timestamp>,
    /// Time of the last update.
    pub updated: Option<Timestamp>,
    /// Summary or description.
    pub summary: Option<String>,
}

/// A parsed feed.
#[derive(Debug, Clone, PartialEq)]
pub struct Feed {
    /// Format of the feed.
    pub format: FeedFormat,
    /// Title of the feed.
    pub title: Option<String>,
    /// Link to the website.
    pub link: Option<Url>,
    /// Items in document order.
    pub items: Vec<FeedItem>,
}

/// A document that is not a usable feed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeedError {
    /// Not well-formed XML.
    #[error(transparent)]
    Xml(#[from] XmlError),
    /// Well-formed, but the root element is not `rss`, `RDF` or `feed`.
    #[error("not an RSS or Atom feed")]
    NotAFeed,
}

/// An absolute http(s) URL, resolved against the feed address.
pub(crate) fn absolute(base: &Url, href: &str) -> Option<Url> {
    let url = base.join(href.trim()).ok()?;
    matches!(url.scheme(), "http" | "https").then_some(url)
}

fn truncated(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((end, _)) => text[..end].to_string(),
        None => text.to_string(),
    }
}

fn set_once<T>(slot: &mut Option<T>, value: Option<T>) {
    if slot.is_none() {
        *slot = value;
    }
}

/// Parses a feed fetched from `base`.
pub fn parse_feed(data: &[u8], base: &Url) -> Result<Feed, FeedError> {
    let text = String::from_utf8_lossy(data);
    let mut format = None;
    let mut not_a_feed = false;
    let mut feed_title = None;
    let mut feed_link = None;
    let mut items: Vec<FeedItem> = Vec::new();
    let mut item: Option<FeedItem> = None;

    walk(&text, |node| match node {
        Node::Start(start) => {
            if start.depth == 0 {
                format = match start.name {
                    "rss" => Some(FeedFormat::Rss2),
                    "rdf" => Some(FeedFormat::Rss1),
                    "feed" => Some(FeedFormat::Atom),
                    _ => {
                        not_a_feed = true;
                        None
                    }
                };
            }
            if format.is_none() {
                return;
            }
            match (start.name, start.parent) {
                ("item" | "entry", _) if items.len() < MAX_ITEMS => {
                    item = Some(FeedItem::default())
                }
                // Atom: <link href="…" rel="alternate"/>; other relations (self, edit…) are skipped
                ("link", parent) if start.attr("href").is_some() => {
                    let rel = start.attr("rel").unwrap_or_else(|| "alternate".into());
                    if rel != "alternate" {
                        return;
                    }
                    let link = start.attr("href").and_then(|h| absolute(base, &h));
                    match (parent, item.as_mut()) {
                        (Some("entry"), Some(it)) => set_once(&mut it.link, link),
                        (Some("feed"), _) => set_once(&mut feed_link, link),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Node::End(end) => {
            if format.is_none() {
                return;
            }
            let text = end.text;
            let value = (!text.is_empty()).then(|| text.to_string());
            if matches!(end.name, "item" | "entry") {
                if let Some(done) = item.take() {
                    items.push(done);
                }
                return;
            }
            let in_item = matches!(end.parent, Some("item" | "entry"));
            match (in_item, item.as_mut()) {
                (true, Some(it)) => match end.name {
                    "title" => set_once(&mut it.title, value),
                    "link" => set_once(&mut it.link, value.and_then(|v| absolute(base, &v))),
                    "guid" | "id" => set_once(&mut it.id, value),
                    "pubdate" | "published" | "issued" | "date" => {
                        set_once(&mut it.published, parse_date(text));
                    }
                    "updated" | "modified" => set_once(&mut it.updated, parse_date(text)),
                    "description" | "summary" | "encoded" | "content" => {
                        set_once(&mut it.summary, value.map(|v| truncated(&v, MAX_SUMMARY)));
                    }
                    _ => {}
                },
                (false, _) if matches!(end.parent, Some("channel" | "feed")) => match end.name {
                    "title" => set_once(&mut feed_title, value),
                    "link" => set_once(&mut feed_link, value.and_then(|v| absolute(base, &v))),
                    _ => {}
                },
                _ => {}
            }
        }
    })?;

    match format {
        Some(format) if !not_a_feed => Ok(Feed {
            format,
            title: feed_title,
            link: feed_link,
            items,
        }),
        _ => Err(FeedError::NotAFeed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://www.example.sk/rss/").unwrap()
    }

    #[test]
    fn rss2() {
        let feed = parse_feed(
            br#"<?xml version="1.0" encoding="UTF-8"?>
            <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
              <channel>
                <title>Novinky &amp; oznamy</title>
                <link>https://www.example.sk/</link>
                <image><title>Logo</title><url>https://www.example.sk/logo.png</url></image>
                <item>
                  <title>Nov&#xe1; vyhl&#xe1;&#x161;ka</title>
                  <link>/clanok/123</link>
                  <guid isPermaLink="false">id-123</guid>
                  <pubDate>Sat, 24 Oct 2026 08:00:00 +0200</pubDate>
                  <description><![CDATA[<p>Text&nbsp;oznamu</p>]]></description>
                </item>
                <item><title>Bez d&#xe1;tumu</title><link>javascript:alert(1)</link></item>
              </channel>
            </rss>"#,
            &base(),
        )
        .unwrap();
        assert_eq!(feed.format, FeedFormat::Rss2);
        assert_eq!(feed.title.as_deref(), Some("Novinky & oznamy"));
        assert_eq!(feed.items.len(), 2);
        let first = &feed.items[0];
        assert_eq!(first.title.as_deref(), Some("Nová vyhláška"));
        assert_eq!(
            first.link.as_ref().unwrap().as_str(),
            "https://www.example.sk/clanok/123"
        );
        assert_eq!(first.id.as_deref(), Some("id-123"));
        assert_eq!(first.published.unwrap().to_string(), "2026-10-24T06:00:00Z");
        assert_eq!(first.summary.as_deref(), Some("<p>Text&nbsp;oznamu</p>"));
        assert_eq!(feed.items[1].link, None);
    }

    #[test]
    fn atom() {
        let feed = parse_feed(
            br#"<feed xmlns="http://www.w3.org/2005/Atom">
              <title>Blog</title>
              <link rel="self" href="https://blog.example.cz/atom.xml"/>
              <link href="https://blog.example.cz/"/>
              <entry>
                <title type="html">Kritick&#233; my&#353;len&#237;</title>
                <link rel="edit" href="/edit/1"/>
                <link rel="alternate" href="/2026/kriticke-mysleni"/>
                <id>tag:blog.example.cz,2026:1</id>
                <published>2026-10-20T10:00:00Z</published>
                <updated>2026-10-21T09:30:00+01:00</updated>
                <content type="xhtml"><div xmlns="http://www.w3.org/1999/xhtml"><p>Prvn&#237;</p><p>druh&#253;</p></div></content>
              </entry>
            </feed>"#,
            &Url::parse("https://blog.example.cz/atom.xml").unwrap(),
        )
        .unwrap();
        assert_eq!(feed.format, FeedFormat::Atom);
        assert_eq!(feed.link.unwrap().as_str(), "https://blog.example.cz/");
        let e = &feed.items[0];
        assert_eq!(e.title.as_deref(), Some("Kritické myšlení"));
        assert_eq!(
            e.link.as_ref().unwrap().as_str(),
            "https://blog.example.cz/2026/kriticke-mysleni"
        );
        assert_eq!(e.updated.unwrap().to_string(), "2026-10-21T08:30:00Z");
        assert_eq!(e.summary.as_deref(), Some("První druhý"));
    }

    #[test]
    fn rss1_with_dublin_core_date() {
        let feed = parse_feed(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns="http://purl.org/rss/1.0/" xmlns:dc="http://purl.org/dc/elements/1.1/">
              <channel><title>Ведомости</title><link>https://edu.example.ru/</link></channel>
              <item><title>Читательская грамотность</title><link>https://edu.example.ru/1</link><dc:date>2026-10-01</dc:date></item>
            </rdf:RDF>"#
                .as_bytes(),
            &base(),
        )
        .unwrap();
        assert_eq!(feed.format, FeedFormat::Rss1);
        assert_eq!(feed.title.as_deref(), Some("Ведомости"));
        assert_eq!(
            feed.items[0].published.unwrap().to_string(),
            "2026-10-01T00:00:00Z"
        );
    }

    #[test]
    fn other_documents_are_rejected() {
        assert_eq!(
            parse_feed(b"<html><body>x</body></html>", &base()),
            Err(FeedError::NotAFeed)
        );
        assert!(matches!(
            parse_feed(b"not xml", &base()),
            Err(FeedError::Xml(_))
        ));
    }
}
