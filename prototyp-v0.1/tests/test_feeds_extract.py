from atena.crawl.feeds import child_links, discover, looks_like_feed, parse_feed, parse_sitemap
from atena.extract import detect_language, extract

RSS = b"""<?xml version="1.0" encoding="UTF-8"?><rss version="2.0"><channel><title>NIVaM</title>
<item><title>Nov\xc3\xa9 testovanie</title><link>/novinka-1</link><pubDate>Fri, 16 Oct 2026 10:33:15 +0000</pubDate>
<description>&lt;p&gt;Text novinky&lt;/p&gt;</description></item></channel></rss>"""


def test_feed():
    assert looks_like_feed(RSS)
    title, items = parse_feed(RSS, "https://nivam.sk/feed/")
    assert title == "NIVaM" and items[0].url == "https://nivam.sk/novinka-1"
    assert items[0].published.year == 2026 and items[0].summary == "Text novinky"


def test_sitemap_is_parsed_safely():
    sm = b"""<?xml version="1.0"?><sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
    <sitemap><loc>https://x.sk/a.xml</loc><lastmod>2024-10-21</lastmod></sitemap></sitemapindex>"""
    assert parse_sitemap(sm) == ("index", [("https://x.sk/a.xml", "2024-10-21")])
    import pytest
    bomb = b'<?xml version="1.0"?><!DOCTYPE l [<!ENTITY a "aaaa"><!ENTITY b "&a;&a;">]><urlset>&b;</urlset>'
    with pytest.raises(Exception):
        parse_sitemap(bomb)


def test_discover_and_children():
    html = b"""<html><head><link rel="alternate" type="application/rss+xml" href="/feed/">
    <meta name="generator" content="WordPress 6.8"></head><body>
    <a href="/pre-skoly/a">A</a><a href="/pre-skoly/b#x">B</a><a href="/ine/c">C</a><a href="/pre-skoly/x.jpg">img</a>
    <a href="mailto:x@y.sk">m</a></body></html>"""
    p = discover(html, "https://nivam.sk/pre-skoly/")
    assert p.feeds == ["https://nivam.sk/feed/"] and p.is_wordpress
    assert [u for u, _ in child_links(p, "https://nivam.sk/pre-skoly/", 10)] == [
        "https://nivam.sk/pre-skoly/a", "https://nivam.sk/pre-skoly/b"]


SK = ("Ministerstvo školstva zverejnilo nové pedagogicko-organizačné pokyny pre školský rok. Riaditelia škôl "
      "dostanú podrobné usmernenie k hodnoteniu žiakov a k tvorbe školského vzdelávacieho programu.")
CS = ("Ministerstvo školství zveřejnilo nové pokyny pro školní rok. Ředitelé škol dostanou podrobné doporučení "
      "k hodnocení žáků a k tvorbě školního vzdělávacího programu, které je třeba projednat.")


def test_language_sk_vs_cs():
    assert detect_language(SK) == "sk"
    assert detect_language(CS) == "cs"


def test_html_extraction():
    html = f"<html lang='sk'><head><title>Pokyny | Ministerstvo</title></head><body><nav>Menu Domov Kontakt</nav>" \
           f"<article><h1>Pokyny na školský rok</h1><p>{SK}</p><p>{SK}</p></article><footer>© 2026</footer></body></html>"
    ex = extract(html.encode(), "text/html", "https://minedu.sk/pokyny")
    assert "pedagogicko-organizačné" in ex.text and ex.language == "sk" and "Menu Domov" not in ex.text


def _pdf(text: str) -> bytes:
    stream = f"BT /F1 12 Tf 72 720 Td ({text}) Tj ET".encode()
    objs = [b"<< /Type /Catalog /Pages 2 0 R >>", b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
            b"<< /Length %d >>stream\n" % len(stream) + stream + b"\nendstream",
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"]
    out, offs = b"%PDF-1.4\n", []
    for i, o in enumerate(objs, 1):
        offs.append(len(out))
        out += b"%d 0 obj\n" % i + o + b"\nendobj\n"
    x = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1) + b"".join(b"%010d 00000 n \n" % o for o in offs)
    out += b"trailer << /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF" % (len(objs) + 1, x)
    return out


def test_pdf_extraction_keeps_page_numbers():
    ex = extract(_pdf("Reading literacy guidance for teachers"), "application/pdf", "https://x.org/a.pdf")
    assert "[strana 1]" in ex.text and "Reading literacy" in ex.text and ex.pages == 1
