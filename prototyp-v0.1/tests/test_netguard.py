import pytest

from atena.crawl.netguard import BlockedURL, check_url


@pytest.mark.parametrize("url", [
    "http://127.0.0.1/", "http://10.0.0.5/x", "http://192.168.1.1/", "http://169.254.169.254/latest/meta-data",
    "http://[::1]/", "http://localhost/", "http://intranet.local/", "file:///etc/passwd", "ftp://x.sk/",
    "http://user:pass@example.org/", "http://8.8.8.8:22/", "http://[::ffff:127.0.0.1]/",
])
def test_blocked(url):
    with pytest.raises(BlockedURL):
        check_url(url)


def test_public_ip_allowed():
    check_url("https://8.8.8.8/")


def test_dns_resolution_checked():
    fake = lambda host, port, proto=0: [(2, 1, 6, "", ("10.1.2.3", port))]
    with pytest.raises(BlockedURL):
        check_url("https://evil.example/", resolver=fake)
    ok = lambda host, port, proto=0: [(2, 1, 6, "", ("93.184.215.14", port))]
    check_url("https://example.org/", resolver=ok)
