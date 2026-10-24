//! URL identity and the scope of link following.
//!
//! One document per URL, so equal pages must get equal URLs: the fragment and tracking parameters
//! (`utm_*`, `fbclid`…) are removed. Links are followed only within the publisher's site and only
//! to attachments (PDF, office files) or to subpages below the page itself, never across the whole
//! site from its home page.

use url::Url;

/// Query parameters that only track visitors.
const TRACKING: [&str; 10] = [
    "gclid", "fbclid", "msclkid", "yclid", "dclid", "mc_cid", "mc_eid", "_ga", "_gl", "igshid",
];

/// File extensions of documents worth keeping (attachments).
const DOCUMENTS: [&str; 10] = [
    "pdf", "doc", "docx", "odt", "rtf", "xls", "xlsx", "ods", "csv", "txt",
];

/// File extensions that are never followed (media, archives, programs, styles).
const SKIPPED: [&str; 24] = [
    "jpg", "jpeg", "png", "gif", "webp", "svg", "ico", "bmp", "tif", "tiff", "mp3", "mp4", "avi",
    "mov", "wmv", "webm", "zip", "rar", "7z", "gz", "exe", "msi", "css", "js",
];

/// `true` for URLs of pages that are mostly about people (see [`atena_core::pages`]).
pub fn is_personal_page(url: &Url) -> bool {
    atena_core::pages::is_personal_path(url.path())
}

/// Links taken from one page at most.
pub const MAX_LINKS_PER_PAGE: usize = 200;

/// The canonical form of a URL used as its identity.
pub fn normalize(url: &Url) -> Url {
    let mut url = url.clone();
    url.set_fragment(None);
    let tracked = |key: &str| {
        let key = key.to_ascii_lowercase();
        key.starts_with("utm_") || TRACKING.contains(&key.as_str())
    };
    if let Some(query) = url.query() {
        if query.is_empty() {
            url.set_query(None);
        } else if url.query_pairs().any(|(k, _)| tracked(&k)) {
            let kept: Vec<(String, String)> = url
                .query_pairs()
                .filter(|(k, _)| !tracked(k))
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect();
            if kept.is_empty() {
                url.set_query(None);
            } else {
                url.query_pairs_mut().clear().extend_pairs(kept);
            }
        }
    }
    url
}

/// The host name of a URL (lower-case), or `None` for URLs without a host.
pub fn host(url: &Url) -> Option<String> {
    url.host_str().map(str::to_ascii_lowercase)
}

/// Lower-case file extension of the last path segment.
fn extension(url: &Url) -> Option<String> {
    let last = url.path_segments()?.next_back()?;
    let (_, ext) = last.rsplit_once('.')?;
    (!ext.is_empty() && ext.len() <= 5).then(|| ext.to_ascii_lowercase())
}

/// `true` when the URL points to a document file (PDF, office formats).
pub fn is_document_file(url: &Url) -> bool {
    extension(url).is_some_and(|e| DOCUMENTS.contains(&e.as_str()))
}

/// `true` when two hosts belong to one site: the same host, or the same registrable domain by
/// the public suffix list – `static.minedu.sk` and `www.minedu.sk` do, `www.gov.uk` and
/// `assets.service.gov.uk` do not (`gov.uk` is a public suffix shared by many publishers), and an IP
/// address is a site of its own.
pub fn same_site(a: &str, b: &str) -> bool {
    if a.eq_ignore_ascii_case(b) {
        return true;
    }
    let is_name =
        |h: &str| h.contains('.') && h.parse::<std::net::IpAddr>().is_err() && !h.starts_with('[');
    if !is_name(a) || !is_name(b) {
        return false;
    }
    match (psl::domain_str(a), psl::domain_str(b)) {
        (Some(x), Some(y)) => x.eq_ignore_ascii_case(y),
        _ => false,
    }
}

/// `true` when `link` is on the site of `page`.
pub fn on_site(page: &Url, link: &Url) -> bool {
    match (host(page), host(link)) {
        (Some(a), Some(b)) => same_site(&a, &b),
        _ => false,
    }
}

/// The directory of a path: `/a/b/page.html` → `/a/b/`, `/a/b/` → `/a/b/`.
fn directory(path: &str) -> &str {
    match path.rfind('/') {
        Some(at) => &path[..=at],
        None => "/",
    }
}

/// Whether a link on `page` should be followed, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Follow {
    /// An attachment (PDF, office file) on the publisher's site.
    Attachment,
    /// A page below the page's own directory.
    Subpage,
}

/// Decides whether to follow `link` found on `page` (both normalised).
pub fn follow(page: &Url, link: &Url) -> Option<Follow> {
    if !matches!(link.scheme(), "http" | "https") || link == page {
        return None;
    }
    let (page_host, link_host) = (host(page)?, host(link)?);
    if !same_site(&page_host, &link_host) {
        return None;
    }
    if extension(link).is_some_and(|e| SKIPPED.contains(&e.as_str())) || is_personal_page(link) {
        return None;
    }
    if is_document_file(link) {
        return Some(Follow::Attachment);
    }
    // subpages only below a real section, never the whole site from its home page
    let dir = directory(page.path());
    let below = link_host == page_host
        && dir != "/"
        && link.path().starts_with(dir)
        && link.path().len() > dir.len();
    below.then_some(Follow::Subpage)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    #[test]
    fn normalisation() {
        assert_eq!(
            normalize(&u("HTTPS://WWW.Minedu.SK/a/#top")).as_str(),
            "https://www.minedu.sk/a/"
        );
        assert_eq!(normalize(&u("https://x.sk/a?")).as_str(), "https://x.sk/a");
        assert_eq!(
            normalize(&u(
                "https://x.sk/a?id=5&utm_source=fb&UTM_Medium=x&fbclid=abc"
            ))
            .as_str(),
            "https://x.sk/a?id=5"
        );
        assert_eq!(
            normalize(&u("https://x.sk/a?utm_source=fb")).as_str(),
            "https://x.sk/a"
        );
        // queries without tracking parameters are kept exactly as written
        assert_eq!(
            normalize(&u("https://x.sk/a?q=a+b&flag")).as_str(),
            "https://x.sk/a?q=a+b&flag"
        );
        assert_eq!(
            host(&u("https://WWW.Skola.sk/")).as_deref(),
            Some("www.skola.sk")
        );
    }

    #[test]
    fn one_site_by_the_public_suffix_list() {
        assert!(same_site("www.minedu.sk", "static.minedu.sk"));
        assert!(same_site("minedu.sk", "www.minedu.sk"));
        assert!(same_site("www.bbc.co.uk", "news.bbc.co.uk"));
        // gov.uk and gov.pl are shared by many publishers
        assert!(!same_site("www.gov.uk", "assets.publishing.service.gov.uk"));
        assert!(!same_site("www.gov.pl", "www.oke.gov.pl"));
        assert!(!same_site("www.minedu.sk", "evil-minedu.sk"));
        assert!(!same_site("www.minedu.sk", "minedu.sk.evil.com"));
        // addresses and single names are sites of their own
        assert!(same_site("127.0.0.1", "127.0.0.1"));
        assert!(!same_site("127.0.0.1", "127.0.0.2"));
        assert!(!same_site("localhost", "intranet"));
    }

    #[test]
    fn following_scope() {
        let page = u("https://www.minedu.sk/38037-sk/organizacia-skolskeho-roka/");
        let f = |link: &str| follow(&page, &u(link));
        assert_eq!(
            f("https://www.minedu.sk/data/att/123.pdf"),
            Some(Follow::Attachment)
        );
        assert_eq!(
            f("https://static.minedu.sk/files/kalendar.DOCX"),
            Some(Follow::Attachment)
        );
        assert_eq!(f("https://minedu.sk/x.xlsx"), Some(Follow::Attachment));
        assert_eq!(
            f("https://www.minedu.sk/38037-sk/organizacia-skolskeho-roka/2026-2027/"),
            Some(Follow::Subpage)
        );
        assert_eq!(
            f("https://www.minedu.sk/kontakt/"),
            None,
            "outside the section"
        );
        assert_eq!(
            f("https://www.minedu.sk/38037-sk/organizacia-skolskeho-roka/"),
            None,
            "itself"
        );
        assert_eq!(f("https://evil-minedu.sk/a.pdf"), None);
        assert_eq!(f("https://www.example.com/a.pdf"), None);
        assert_eq!(f("https://www.minedu.sk/logo.png"), None);
        assert_eq!(f("mailto:info@minedu.sk"), None);

        let home = u("https://www.skola.sk/");
        assert_eq!(
            follow(&home, &u("https://www.skola.sk/oznamy/")),
            None,
            "no site-wide crawl"
        );
        assert_eq!(
            follow(&home, &u("https://www.skola.sk/dokumenty/skvp.pdf")),
            Some(Follow::Attachment)
        );
        let page = u("https://www.skola.sk/oznamy/zapis.html");
        assert_eq!(
            follow(&page, &u("https://www.skola.sk/oznamy/zapis-2.html")),
            Some(Follow::Subpage)
        );
        let ip = u("http://127.0.0.1:8080/docs/");
        assert_eq!(
            follow(&ip, &u("http://127.0.0.1:8080/docs/a.pdf")),
            Some(Follow::Attachment)
        );
        assert!(
            is_document_file(&u("https://x.sk/a/b.PDF"))
                && !is_document_file(&u("https://x.sk/pdf"))
        );

        // pages about people are not followed
        let section = u("https://www.skola.sk/o-skole/");
        for personal in [
            "https://www.skola.sk/o-skole/zamestnanci/",
            "https://www.skola.sk/o-skole/fotogaleria/vylet-2026.html",
            "https://www.skola.sk/o-skole/galeria/",
            "https://www.skola.sk/o-skole/kontakty.html",
            "https://www.skola.sk/o-skole/ucitelia/jan-novak.html",
            "https://www.skola.sk/o-skole/zoznam-zamestnancov.pdf",
        ] {
            assert_eq!(follow(&section, &u(personal)), None, "{personal}");
        }
        assert_eq!(
            follow(
                &section,
                &u("https://www.skola.sk/o-skole/metodiky-pre-ucitelov.pdf")
            ),
            Some(Follow::Attachment)
        );
        assert_eq!(
            follow(
                &section,
                &u("https://www.skola.sk/o-skole/skolsky-poriadok/")
            ),
            Some(Follow::Subpage)
        );
    }
}
