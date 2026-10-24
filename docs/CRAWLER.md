# The ATENA crawler

*Slovenčina: [sk/ZBERAC.md](sk/ZBERAC.md)*

ATENA collects official, research and practice information about education and always links back to
the original. This page is for **website owners** (what the crawler does and how to control it) and for
**operators** of an ATENA installation (what it guarantees).

## Identification
* Product token: **`ATENA`** – use it in `robots.txt` and `X-Robots-Tag`.
* User-Agent: `Mozilla/5.0 (compatible; ATENA/<version>; +https://github.com/tarzanislav/atena/blob/main/docs/CRAWLER.md)`
* Questions or problems: [GitHub issues](https://github.com/tarzanislav/atena/issues).

## What the crawler respects
| Rule | Behaviour |
|---|---|
| **robots.txt** ([RFC 9309](https://www.rfc-editor.org/rfc/rfc9309)) | Obeys the `ATENA` group, otherwise the `*` group; the longest matching rule wins, `Allow` wins a tie; `*` and `$` patterns. Read before any other request to an origin and cached for 24 hours. |
| robots.txt missing (4xx) | Everything may be fetched. |
| robots.txt unreachable (5xx, 429, network error) | **Nothing** is fetched; the last good copy is used if there is one. Retried after 30 minutes. |
| `Crawl-delay` | Honoured, up to 60 seconds. |
| Pace | One request at a time per host and at least 5 seconds apart (installation default). |
| `429` / `503` with `Retry-After` | The host is left alone for the requested time (up to 6 hours; 60 seconds when the header is missing). |
| Conditional requests | `If-None-Match` / `If-Modified-Since` – unchanged pages cost a `304`. |
| **TDMRep** ([W3C](https://www.w3.org/community/reports/tdmrep/CG-FINAL-tdmrep-20240510/)) | `/.well-known/tdmrep.json` (cached 24 h), `tdm-reservation` / `tdm-policy` headers, and later HTML/PDF metadata. A reservation is recorded with every page and respected by the processing pipeline (text and data mining under Art. 4 of Directive (EU) 2019/790). |
| **AI crawlers in robots.txt** | A group naming a crawler that collects for AI training or mining (`MINING_AGENTS`: GPTBot, CCBot, ClaudeBot, Google-Extended …) that disallows the page counts as a reservation of mining: ATENA, which follows only its own group or `*`, may fetch the page, but keeps metadata only. No later TDMRep value can lift it. |
| `X-Robots-Tag` / `<meta name="robots">` | `noindex`, `nofollow`, `noarchive` (also `nocache`), `nosnippet`, `notranslate`, `none`; values addressed to other crawlers (`otherbot: noindex`) are ignored. |

Examples:

```text
# keep ATENA out completely
User-agent: ATENA
Disallow: /

# slow it down and keep it out of one folder
User-agent: ATENA
Crawl-delay: 30
Disallow: /internal/
```

## Personal data and erasure
ATENA removes e-mail addresses and telephone numbers before storing texts, keeps no originals of web
pages, does not follow galleries, staff lists or contact pages, and erases a page or a whole section on
request (it is then never fetched again). Requests: [GitHub issues](https://github.com/tarzanislav/atena/issues)
or the contact of the installation's operator. Details: [PRIVACY-AND-RIGHTS.md](PRIVACY-AND-RIGHTS.md).

## What it downloads
Web pages, PDF and office documents, RSS/Atom feeds and sitemaps (XML, text, gzip) – at most 50 MiB per
resource (after decompression), only the content types it can process. It sends no cookies, no
`Referer` and no credentials, and does not run JavaScript.

## Guarantees for operators
* **Public Internet only.** Every URL – including every redirect hop and every link found in a feed or
  sitemap – is checked before a connection is made: only `http`/`https`, no user names or passwords in
  URLs, allowed ports only (80, 443, 8080, 8443), no single-label or reserved names (`localhost`,
  `*.local`, `*.internal`, `*.home.arpa`…). DNS answers are filtered and only public addresses are
  connected to, so a name that resolves to `127.0.0.1`, `10.x`, `169.254.169.254` (cloud metadata) or
  IPv6 private ranges is never contacted, not even after DNS rebinding. IPv6 is limited to global unicast
  `2000::/3`. An intranet deployment can allow specific ranges explicitly (`NetPolicy::allowed_ranges`).
* **No proxies.** System and environment proxies are ignored, because a proxy would resolve names
  itself and bypass the address checks.
* **Bounded work.** Timeouts (10 s connect, 60 s per request), 5 redirects, size limits checked while
  streaming, robots.txt read up to 500 KiB, sitemaps up to 50,000 entries / 50 MiB, feeds up to 10,000
  items. XML entities declared in a DTD are never expanded.
* **TLS** via rustls with the operating system's certificate store.

Known limits of this version: per-host state is kept in memory without pruning (fine for thousands of
hosts); feeds in encodings other than UTF-8 are read with replacement characters. HTML `<meta>` rules and
TDMRep in HTML and PDF are read by `atena-extract` and merged with the HTTP rules in the processing
pipeline; EPUB is not read yet.
