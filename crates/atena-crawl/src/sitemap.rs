//! Sitemaps (sitemaps.org): XML `urlset` and `sitemapindex`, plain-text lists, gzip-compressed files.

use std::io::Read;

use flate2::read::GzDecoder;
use jiff::Timestamp;
use url::Url;

use crate::date::parse_date;
use crate::feed::absolute;
use crate::xml::{Node, XmlError, walk};

/// Entries kept from one sitemap (the protocol limit).
pub const MAX_SITEMAP_URLS: usize = 50_000;
/// Largest uncompressed sitemap (the protocol limit).
pub const MAX_SITEMAP_BYTES: usize = 50 * 1024 * 1024;

/// How often a page is expected to change, as declared by the site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeFreq {
    /// Changes on every access.
    Always,
    /// Hourly.
    Hourly,
    /// Daily.
    Daily,
    /// Weekly.
    Weekly,
    /// Monthly.
    Monthly,
    /// Yearly.
    Yearly,
    /// Archived, never changes.
    Never,
}

impl ChangeFreq {
    fn parse(text: &str) -> Option<Self> {
        Some(match text.trim().to_ascii_lowercase().as_str() {
            "always" => Self::Always,
            "hourly" => Self::Hourly,
            "daily" => Self::Daily,
            "weekly" => Self::Weekly,
            "monthly" => Self::Monthly,
            "yearly" => Self::Yearly,
            "never" => Self::Never,
            _ => return None,
        })
    }
}

/// A page listed in a sitemap.
#[derive(Debug, Clone, PartialEq)]
pub struct SitemapUrl {
    /// Address of the page.
    pub loc: Url,
    /// Last modification declared by the site.
    pub lastmod: Option<Timestamp>,
    /// Expected change frequency.
    pub changefreq: Option<ChangeFreq>,
}

/// A child sitemap listed in a sitemap index.
#[derive(Debug, Clone, PartialEq)]
pub struct SitemapRef {
    /// Address of the child sitemap.
    pub loc: Url,
    /// Last modification declared by the site.
    pub lastmod: Option<Timestamp>,
}

/// The content of one sitemap file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sitemap {
    /// Pages (from a `urlset` or a text sitemap).
    pub urls: Vec<SitemapUrl>,
    /// Child sitemaps (from a `sitemapindex`).
    pub sitemaps: Vec<SitemapRef>,
    /// Entries left out: on another host, not http(s), or over the limit.
    pub skipped: usize,
}

/// A sitemap that cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SitemapError {
    /// Larger than [`MAX_SITEMAP_BYTES`] after decompression.
    #[error("sitemap is larger than {MAX_SITEMAP_BYTES} bytes")]
    TooLarge,
    /// Broken gzip data.
    #[error("invalid gzip data")]
    Gzip,
    /// Not well-formed XML.
    #[error(transparent)]
    Xml(#[from] XmlError),
    /// XML whose root is neither `urlset` nor `sitemapindex`.
    #[error("not a sitemap")]
    NotASitemap,
}

/// Decompresses gzip data (recognised by its magic bytes), refusing more than the size limit.
fn inflate(data: &[u8]) -> Result<Vec<u8>, SitemapError> {
    let mut out = Vec::new();
    GzDecoder::new(data)
        .take(MAX_SITEMAP_BYTES as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|_| SitemapError::Gzip)?;
    if out.len() > MAX_SITEMAP_BYTES {
        return Err(SitemapError::TooLarge);
    }
    Ok(out)
}

/// Entries must stay on the sitemap's host and within the limit.
struct Collector<'a> {
    base: &'a Url,
    sitemap: Sitemap,
}

impl Collector<'_> {
    fn accept(&mut self, loc: &str) -> Option<Url> {
        let full = self.sitemap.urls.len() + self.sitemap.sitemaps.len() >= MAX_SITEMAP_URLS;
        let url = absolute(self.base, loc).filter(|u| u.host_str() == self.base.host_str());
        if full || url.is_none() {
            self.sitemap.skipped += 1;
            return None;
        }
        url
    }
}

/// Parses a sitemap fetched from `sitemap_url`.
pub fn parse_sitemap(data: &[u8], sitemap_url: &Url) -> Result<Sitemap, SitemapError> {
    let inflated;
    let data = if data.starts_with(&[0x1f, 0x8b]) {
        inflated = inflate(data)?;
        &inflated[..]
    } else if data.len() > MAX_SITEMAP_BYTES {
        return Err(SitemapError::TooLarge);
    } else {
        data
    };
    let decoded = String::from_utf8_lossy(data);
    let text = decoded.trim_start_matches('\u{feff}');
    let mut c = Collector {
        base: sitemap_url,
        sitemap: Sitemap::default(),
    };

    if !text.trim_start().starts_with('<') {
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
            // a text sitemap lists absolute URLs only
            if Url::parse(line).is_err() {
                c.sitemap.skipped += 1;
                continue;
            }
            if let Some(loc) = c.accept(line) {
                c.sitemap.urls.push(SitemapUrl {
                    loc,
                    lastmod: None,
                    changefreq: None,
                });
            }
        }
        return Ok(c.sitemap);
    }

    let mut root_ok = false;
    let (mut loc, mut lastmod, mut changefreq) = (None, None, None);
    walk(text, |node| match node {
        Node::Start(start) => {
            if start.depth == 0 {
                root_ok = matches!(start.name, "urlset" | "sitemapindex");
            }
            if matches!(start.name, "url" | "sitemap") {
                (loc, lastmod, changefreq) = (None, None, None);
            }
        }
        Node::End(end) if root_ok => match (end.name, end.parent) {
            // `parent` keeps extension elements such as <image:loc> out
            ("loc", Some("url" | "sitemap")) => loc = Some(end.text.to_string()),
            ("lastmod", Some("url" | "sitemap")) => lastmod = parse_date(end.text),
            ("changefreq", Some("url")) => changefreq = ChangeFreq::parse(end.text),
            ("url", _) => {
                if let Some(url) = loc.take().and_then(|l| c.accept(&l)) {
                    c.sitemap.urls.push(SitemapUrl {
                        loc: url,
                        lastmod: lastmod.take(),
                        changefreq: changefreq.take(),
                    });
                }
            }
            ("sitemap", _) => {
                if let Some(url) = loc.take().and_then(|l| c.accept(&l)) {
                    c.sitemap.sitemaps.push(SitemapRef {
                        loc: url,
                        lastmod: lastmod.take(),
                    });
                }
            }
            _ => {}
        },
        Node::End(_) => {}
    })?;
    if root_ok {
        Ok(c.sitemap)
    } else {
        Err(SitemapError::NotASitemap)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::{Compression, write::GzEncoder};

    use super::*;

    fn base() -> Url {
        Url::parse("https://www.example.sk/sitemap.xml").unwrap()
    }

    #[test]
    fn urlset_with_extensions_and_foreign_hosts() {
        let s = parse_sitemap(
            br#"<?xml version="1.0" encoding="UTF-8"?>
            <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:image="http://www.google.com/schemas/sitemap-image/1.1">
              <url><loc>https://www.example.sk/a?x=1&amp;y=2</loc><lastmod>2026-10-01</lastmod><changefreq>weekly</changefreq>
                   <image:image><image:loc>https://cdn.example.net/i.png</image:loc></image:image></url>
              <url><loc>/relative</loc></url>
              <url><loc>https://evil.example.com/</loc></url>
              <url><loc>mailto:x@example.sk</loc></url>
            </urlset>"#,
            &base(),
        )
        .unwrap();
        assert_eq!(s.urls.len(), 2);
        assert_eq!(s.urls[0].loc.as_str(), "https://www.example.sk/a?x=1&y=2");
        assert_eq!(
            s.urls[0].lastmod.unwrap().to_string(),
            "2026-10-01T00:00:00Z"
        );
        assert_eq!(s.urls[0].changefreq, Some(ChangeFreq::Weekly));
        assert_eq!(s.urls[1].loc.as_str(), "https://www.example.sk/relative");
        assert_eq!(s.skipped, 2);
    }

    #[test]
    fn index_text_and_gzip() {
        let index = br#"<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
            <sitemap><loc>https://www.example.sk/s1.xml.gz</loc><lastmod>2026-10-20T10:00:00+02:00</lastmod></sitemap>
            </sitemapindex>"#;
        let s = parse_sitemap(index, &base()).unwrap();
        assert_eq!(
            s.sitemaps[0].loc.as_str(),
            "https://www.example.sk/s1.xml.gz"
        );
        assert!(s.urls.is_empty());

        let text = "\u{feff}https://www.example.sk/1\n\n  https://www.example.sk/2  \nnot a url\n";
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        gz.write_all(text.as_bytes()).unwrap();
        let s = parse_sitemap(&gz.finish().unwrap(), &base()).unwrap();
        assert_eq!(s.urls.len(), 2);
        assert_eq!(s.skipped, 1);
    }

    #[test]
    fn limits_and_wrong_documents() {
        let mut gz = GzEncoder::new(Vec::new(), Compression::best());
        gz.write_all(&vec![b' '; MAX_SITEMAP_BYTES + 10]).unwrap();
        assert_eq!(
            parse_sitemap(&gz.finish().unwrap(), &base()),
            Err(SitemapError::TooLarge)
        );
        assert_eq!(
            parse_sitemap(&[0x1f, 0x8b, 1, 2, 3], &base()),
            Err(SitemapError::Gzip)
        );
        assert_eq!(
            parse_sitemap(b"<rss><channel/></rss>", &base()),
            Err(SitemapError::NotASitemap)
        );
        let many: String = (0..MAX_SITEMAP_URLS + 5)
            .map(|i| format!("https://www.example.sk/{i}\n"))
            .collect();
        let s = parse_sitemap(many.as_bytes(), &base()).unwrap();
        assert_eq!((s.urls.len(), s.skipped), (MAX_SITEMAP_URLS, 5));
    }
}
