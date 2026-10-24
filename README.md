# ATENA

*Slovenčina: [README.sk.md](README.sk.md)*

**An open aggregator of verified information about education.** ATENA collects official, research and
practice sources from many countries, tracks every change, classifies content in a multilingual taxonomy
and always links each item back to its original source.

* Code licence **Apache-2.0** (free for private and commercial use), data CC BY 4.0 / CC0, no paid dependencies.
* Multilingual and multi-country by design. **Core languages from the start: English, Slovak, Czech, Russian
  and Chinese (Simplified)**; Slovak is the reference locale, Slovakia the first country pack and Czechia the second – see [docs/INTERNATIONALIZATION.md](docs/INTERNATIONALIZATION.md).
* Stack: **Rust** core, **TypeScript + React** UI, **Tauri 2** desktop and mobile apps, **PostgreSQL**,
  **Tantivy** search – see [docs/TECHNOLOGY.md](docs/TECHNOLOGY.md).
* Design: Windows 11 / Fluent style, calm pastel section colours, light and dark themes – see [docs/DESIGN.md](docs/DESIGN.md).
* Licences of all components and AI models – see [docs/LICENSES.md](docs/LICENSES.md).

## Repository
| Folder | Contents |
|---|---|
| `crates/atena-core/` | text normalisation, taxonomy, classification suggestions, change detection |
| `crates/atena-crawl/` | polite and safe crawler: robots.txt (RFC 9309), SSRF guard, pacing, TDMRep, RSS/Atom, sitemaps – see [docs/CRAWLER.md](docs/CRAWLER.md) |
| `crates/atena-extract/` | main text without menus and footers, metadata and language from HTML (also legacy encodings), PDF, DOCX and ODT |
| `crates/atena-store/` | PostgreSQL schema and queries, host-lease scheduling, content-addressed file store – see [docs/DATABASE.md](docs/DATABASE.md) |
| `crates/atena-ingest/` | the crawler program: register → fetch → extraction → versions → classification – see [docs/RUNNING.md](docs/RUNNING.md) |
| `data/` | taxonomy (296 concepts, 15 facets, 8 languages) and a register of 186 sources |
| `prototyp-v0.1/` | working Python prototype (crawling, versions, classification, search, web, 41 tests) – the executable specification for the Rust core |
| `tools/language-benchmark/` | benchmark of Rust, Go, Java, Node and Python on ATENA's classification task |
| `docs/` | technology, design, licences, internationalisation (English); `docs/sk/` Slovak versions |

## Principles
1. Every item has a direct link to the original, the capture date and a version.
2. Source authority (where it comes from) and strength of evidence (what it proves) are assessed separately.
3. Automatic classification and translation are always marked as suggestions until a human reviews them.
4. Unknown values stay "unknown" – nothing is filled in by guessing.
5. Alternative pedagogical approaches are never ranked down.
6. **Only open sources, lawfully used**: publicly accessible content only; robots.txt, TDM reservations and
   terms of use obeyed; full text shown only for official texts and open licences; contact details removed,
   erasure on request – see [docs/PRIVACY-AND-RIGHTS.md](docs/PRIVACY-AND-RIGHTS.md).

## Contributing
* Code, identifiers, commit messages and technical documentation: **English**.
* User interface: texts never in code, only in locale files (Project Fluent); a new language is added by
  translating a file, without changing the program. **Slovak** is the reference locale, English the pivot for
  translators, Czech, Russian and Chinese are core locales of equal status.
* User guide and communication with the project author: **Slovak** (`docs/sk/`).
