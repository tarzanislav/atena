"""RSS/Atom kanály, sitemapy a objavovanie kanálov na stránke."""
from __future__ import annotations

import re
from dataclasses import dataclass
from datetime import datetime, timezone
from html.parser import HTMLParser
from urllib.parse import urljoin, urlsplit

import feedparser
from defusedxml import ElementTree as ET


@dataclass
class FeedItem:
    url: str
    title: str
    published: datetime | None
    summary: str
    guid: str | None = None


def parse_feed(content: bytes, base_url: str) -> tuple[str, list[FeedItem]]:
    d = feedparser.parse(content)
    title = d.feed.get("title", "") if d.get("feed") else ""
    items: list[FeedItem] = []
    for e in d.entries:
        link = e.get("link")
        if not link:
            continue
        link = urljoin(base_url, link)
        pub = None
        t = e.get("published_parsed") or e.get("updated_parsed")
        if t:
            pub = datetime(*t[:6], tzinfo=timezone.utc)
        summary = re.sub(r"<[^>]+>", " ", e.get("summary", "") or "")
        items.append(FeedItem(link, (e.get("title") or "").strip(), pub, re.sub(r"\s+", " ", summary).strip(), e.get("id")))
    return title, items


def looks_like_feed(content: bytes, content_type: str = "") -> bool:
    if any(t in content_type for t in ("rss", "atom")):
        return True
    head = content[:600].lstrip().lower()
    return head.startswith(b"<?xml") and (b"<rss" in head or b"<feed" in head) or head.startswith(b"<rss")


_SM_NS = "{http://www.sitemaps.org/schemas/sitemap/0.9}"


def parse_sitemap(content: bytes) -> tuple[str, list[tuple[str, str | None]]]:
    """Vráti ('index'|'urlset', [(loc, lastmod)]). XML sa parsuje bezpečne (defusedxml)."""
    root = ET.fromstring(content)
    kind = "index" if root.tag.endswith("sitemapindex") else "urlset"
    out = []
    for child in root:
        loc = child.find(f"{_SM_NS}loc")
        if loc is None:
            loc = child.find("loc")
        if loc is None or not (loc.text or "").strip():
            continue
        lm = child.find(f"{_SM_NS}lastmod")
        if lm is None:
            lm = child.find("lastmod")
        out.append((loc.text.strip(), lm.text.strip() if lm is not None and lm.text else None))
    return kind, out


class _LinkParser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.feeds: list[str] = []
        self.links: list[tuple[str, str]] = []
        self._a: str | None = None
        self._txt: list[str] = []
        self.generator = ""

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "link" and (a.get("rel") or "").lower() == "alternate":
            t = (a.get("type") or "").lower()
            if ("rss" in t or "atom" in t) and a.get("href"):
                self.feeds.append(a["href"])
        elif tag == "meta" and (a.get("name") or "").lower() == "generator":
            self.generator = a.get("content") or ""
        elif tag == "a" and a.get("href"):
            self._a = a["href"]
            self._txt = []

    def handle_data(self, data):
        if self._a is not None:
            self._txt.append(data)

    def handle_endtag(self, tag):
        if tag == "a" and self._a is not None:
            self.links.append((self._a, " ".join("".join(self._txt).split())))
            self._a = None


@dataclass
class PageLinks:
    feeds: list[str]
    links: list[tuple[str, str]]
    is_wordpress: bool


def discover(html: bytes | str, base_url: str) -> PageLinks:
    if isinstance(html, bytes):
        html = html.decode("utf-8", errors="replace")
    p = _LinkParser()
    try:
        p.feed(html)
    except Exception:   # poškodené HTML nesmie zastaviť zber
        pass
    feeds = [urljoin(base_url, f) for f in p.feeds]
    links = [(urljoin(base_url, h).split("#")[0], t) for h, t in p.links
             if not h.startswith(("mailto:", "tel:", "javascript:"))]
    wp = "wordpress" in p.generator.lower() or "/wp-content/" in html
    return PageLinks(feeds, links, wp)


_SKIP_EXT = re.compile(r"\.(jpe?g|png|gif|svg|webp|mp3|mp4|avi|mov|zip|rar|7z|exe|css|js|ico|woff2?)($|\?)", re.I)


def child_links(page: PageLinks, base_url: str, limit: int) -> list[tuple[str, str]]:
    """Odkazy na podstránky v rovnakej sekcii webu (rovnaký hostiteľ a začiatok cesty)."""
    b = urlsplit(base_url)
    prefix = b.path.rsplit("/", 1)[0] + "/" if not b.path.endswith("/") else b.path
    seen, out = {base_url.rstrip("/")}, []
    for url, text in page.links:
        u = urlsplit(url)
        if u.scheme not in ("http", "https") or u.netloc != b.netloc or _SKIP_EXT.search(u.path):
            continue
        if not u.path.startswith(prefix):
            continue
        key = url.rstrip("/")
        if key in seen:
            continue
        seen.add(key)
        out.append((url, text))
        if len(out) >= limit:
            break
    return out
