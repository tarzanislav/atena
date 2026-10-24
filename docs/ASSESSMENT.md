# Automatic rights assessment

*Slovenčina: [sk/HODNOTENIE.md](sk/HODNOTENIE.md)*

`crates/atena-assess` does for any address what the rights review of the register did by hand
([SOURCE-RIGHTS.md](SOURCE-RIGHTS.md)): it reads the page, its licence and its terms of use and
proposes `licence`, `terms` and a `rights` record with quotes. It is used by `atena-ingest assess`
and by the import of user proposals.

## Input
One item per line, optionally followed by a country code (`SK`, `GB-ENG`, `WORLD` …); empty lines
and lines starting with `#` are skipped. At most 500 items.

| Item | Examples |
|---|---|
| web page or site | `https://www.minedu.sk/`, `www.npi.cz/aktuality` |
| feed or sitemap | `https://eduscol.education.gouv.fr/feed`, `https://example.org/sitemap.xml` |
| document | `https://example.org/osnovy.pdf` |
| OAI-PMH interface | `https://repository.example.org/oai?verb=Identify` |
| DOI | `10.1002/cl2.1180`, `doi:10.1002/cl2.1180`, `https://doi.org/10.1002/cl2.1180` |

Addresses with a user name or password and schemes other than `http(s)` are refused. Every fetch
goes through the crawler: robots.txt, politeness, size limits and the refusal of private addresses
apply to the assessment too.

## What is read
1. **The item.** Its kind (page, feed, sitemap, document, OAI-PMH, DOI), title, TDMRep reservation
   and whether robots.txt refuses AI crawlers. For a DOI, its Crossref record (licences of the
   version of record) and then the publisher's landing page.
2. **The page that speaks for the site** – the item itself for web pages, the site's home page for
   feeds, sitemaps, documents and interfaces: `rel="license"` links, licence badges and the
   footer (`©`, "all rights reserved", licence notes). The rest of an ordinary page is not read as
   legal text, because articles may well discuss robots or AI.
3. **Up to three terms-of-use pages** linked from it (terms, legal notice, copyright, Impressum,
   podmienky, mentions légales … in 16 languages); privacy, cookie and accessibility pages are
   skipped. If an inner page links none, the home page's links are tried.

The phrases are data, not code: `data/legal-phrases.yaml`. A finding needs a subject and a marker in
the same sentence ("robot" + "may not"; "text and data mining" + "reserved").

## The proposal – always the strictest the evidence calls for
| Evidence | `terms` | `licence` |
|---|---|---|
| the item could not be read | `unknown` | `copyrighted` |
| a page about people (galleries, staff, contacts) | `forbidden` | `copyrighted` |
| terms ban robots, scraping, crawling, indexing, or allow access only for personal use | `forbidden` | `copyrighted` |
| terms, TDMRep or robots.txt reserve mining or AI use | `reserved` | `copyrighted` |
| terms found but none could be read | `unknown` | `copyrighted` |
| terms read or none found, nothing of the above | `allowed` | one accepted open licence if that is all that was found, else `copyrighted` |

`official` is never proposed: whether a text is an official work is for a person to judge. The
country comes from the user or from the host name (`.sk`, `europa.eu`, `.gov` …); generic domains
(`.org`, `.com`) tell nothing, and without a country the legality principle allows nothing.

**A proposal is never applied by itself.** A machine can misread terms of use; an imported source
is not crawled until a person has confirmed (or corrected) its proposal.
