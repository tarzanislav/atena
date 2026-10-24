from atena.crawl import robots

TXT = """
User-agent: *
Disallow: /private/
Allow: /private/public-page
Disallow: /*.pdf$
Crawl-delay: 3

User-agent: ATENA-bot
User-agent: other
Disallow: /only-for-atena
Allow: /private/
"""


def test_specific_group_wins_over_star():
    p = robots.parse(TXT)
    assert p.can_fetch("ATENA-bot", "https://x.sk/private/doc")      # vlastná skupina povoľuje
    assert not p.can_fetch("ATENA-bot", "https://x.sk/only-for-atena/a")
    assert p.can_fetch("ATENA-bot", "https://x.sk/file.pdf")           # pravidlá * sa na ňu nevzťahujú


def test_longest_match_and_allow_tie():
    p = robots.parse(TXT)
    assert not p.can_fetch("Googlebot", "https://x.sk/private/x")
    assert p.can_fetch("Googlebot", "https://x.sk/private/public-page")
    tie = robots.parse("User-agent: *\nDisallow: /a\nAllow: /a\n")
    assert tie.can_fetch("x", "https://x.sk/a")


def test_wildcards():
    p = robots.parse(TXT)
    assert not p.can_fetch("Googlebot", "https://x.sk/docs/file.pdf")
    assert p.can_fetch("Googlebot", "https://x.sk/docs/file.pdf?v=2")


def test_crawl_delay_and_robots_always_allowed():
    p = robots.parse(TXT)
    assert p.crawl_delay("Googlebot") == 3
    assert p.can_fetch("Googlebot", "https://x.sk/robots.txt")


def test_status_rules():
    assert robots.from_status(404, None).can_fetch("a", "https://x.sk/a")
    assert not robots.from_status(503, None).can_fetch("a", "https://x.sk/a")


def test_empty_disallow_means_allow_all():
    p = robots.parse("User-agent: *\nDisallow:\n\nSitemap: https://x.sk/sitemap.xml")
    assert p.can_fetch("x", "https://x.sk/whatever")
    assert p.sitemaps == ["https://x.sk/sitemap.xml"]
