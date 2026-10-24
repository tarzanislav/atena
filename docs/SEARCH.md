# Search

*Slovenčina: [sk/VYHLADAVANIE.md](sk/VYHLADAVANIE.md)*

The crate `crates/atena-search` keeps a full-text index ([Tantivy](https://github.com/quickwit-oss/tantivy),
MIT) of the documents in the database and searches it. The database stays the source of truth: the index
is derived from it and can be rebuilt at any time.

## What may be found – the legality principle
The index takes a document only as far as [LEGALITY.md](LEGALITY.md) allows
(`atena_search::searchable`, which asks `legality::permission`):

| Document | Found | Text searched | Text shown |
|---|---|---|---|
| gone, of a disabled or rejected source, `noindex`, or of a source ATENA may not collect (terms forbid automated access, country without a legal profile) | **no** | – | – |
| metadata only: terms of use not verified or reserving mining, a TDM reservation (TDMRep, AI-crawler refusal), a country whose data protection law keeps texts at home | by title, labels, source, country, language | no | no |
| copyrighted, terms verified, mining allowed in both countries | yes | yes | **no** – title, source, labels and the link to the original |
| official texts and open licences | yes | yes | a snippet (not with `nosnippet`); the whole text in ATENA (not with `noarchive`) |

Text that may not be shown is indexed but **not stored in the index at all**; only texts that may be shown
are stored there, for snippets.

## Languages
Every language group has its own title and text fields and its own analyzer: words → overlong tokens
dropped → lower case → stemmer → diacritics folded (after stemming, because stemmers need `ä` or `ách`).

| Group | Stemmer |
|---|---|
| Slovak, Czech, Polish | ATENA's own light stemmers (`stem.rs`): the longest inflectional ending that leaves three letters is removed – `školách`, `školy`, `škole` → `škol` |
| English, German, French, Spanish, Italian, Portuguese, Dutch, Swedish, Danish, Norwegian, Finnish, Hungarian, Romanian, Russian, Greek, Turkish, Arabic | Snowball (BSD-3-Clause) |
| Chinese, Japanese, Korean | none: runs of characters become overlapping pairs (`阅读素养` → `阅读 读素 素养`), which find any word without a dictionary |
| other languages, unknown language | none |

So `citatelskej gramotnosti` finds `Čitateľská gramotnosť`, and `skoly` finds `školách`. The words
are looked up in every group; titles weigh three times as much as texts. All words must match; when no
document has them all, documents with some of them are shown and the result says so. `"…"` searches a
phrase, `-word` excludes a word.

## Across languages: concepts
The words are also matched against the keywords of the taxonomy (`atena_core::classify`). Documents
labelled with a recognised concept – or a narrower one – are found in any language: `čitateľská
gramotnosť` finds English and Chinese documents on reading literacy. The result names the concepts it
recognised. Labels an editor rejected do not count.

## Filters
`scheme/concept` (narrower concepts included), `country/SK`, `authority/A1`, `language/sk`,
`source/SK01`. Values of one scheme are alternatives; different schemes must all hold. The result counts
the values of every scheme among the documents found (facets).

## Ranking
Relevance × authority of the source × freshness.

| Authority | A1 | A2 | B1 | B2 | C1 | C2 | D | E |
|---|---|---|---|---|---|---|---|---|
| weight | 1.5 | 1.4 | 1.3 | 1.25 | 1.1 | 1.05 | 1.0 | 0.8 |

Freshness: up to a quarter more for the newest documents, the advantage halving every year (from the
document's modification or publication date, otherwise the last change ATENA saw). Pedagogical
approaches are never ranked – only sources.

## Keeping the index up to date
Database triggers (migration `0006_search_queue.sql`) queue a document in `search_queue` whenever
something search shows may have changed: a new document or version, a text reduced to metadata, the
publisher's rules, a label decision, an erasure, a change of the source's name, country, authority,
licence, terms or state. A check that only confirms a page queues nothing. The indexer takes queued
documents, writes or removes them, commits, and only then removes them from the queue – unless they
were queued again meanwhile. No code path can leave the index behind.

```
atena-ingest index              # bring the index up to date (atena-ingest run does it every minute)
atena-ingest index --rebuild    # build it anew (after an ATENA update that changes the index format)
atena-ingest search citatelska gramotnost --lang sk
atena-ingest search zápis --country SK --label language/sk --limit 20 --page 2
atena-ingest search "školský poriadok" --facets
atena-ingest search inklúzia --json
```

`ATENA_INDEX` is the index directory (default `./atena-index`). ATENA writes a marker file into it and
opens or deletes only directories with that marker (or empty ones) – never a directory named there by
mistake. Searching needs no database.

## Tests
`cargo test -p atena-search`: analyzers and stemmers, the legal limits, concepts across languages,
filters, ranking and the index on disk; with `ATENA_TEST_DATABASE_URL` also the triggers and updates
from a real database.
