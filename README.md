# ATENA

*Slovenčina: [README.sk.md](README.sk.md)*

**An open aggregator of verified information about education.** ATENA collects official, research and
practice sources from many countries, tracks every change, classifies content in a multilingual taxonomy
and always links each item back to its original source.

## The four pillars
1. **Seriousness, openness and moving human knowledge forward** – the knowledge of every person.
2. **Anonymity and respect for privacy** – ours and everyone else's.
3. **Security and protection** against dangerous people, robots, code and processes.
4. **Respect for the law** – no law may be broken, anywhere.

The principles below and the rules written into the core (`atena-core`) put the pillars into practice.

* Code licence **Apache-2.0** (free for private and commercial use), data CC BY 4.0 / CC0, no paid dependencies.
* Multilingual and multi-country by design. **Core languages from the start: English, Slovak, Czech, Russian
  and Chinese (Simplified)**; Slovak is the reference locale, Slovakia the first country pack and Czechia the second – see [docs/INTERNATIONALIZATION.md](docs/INTERNATIONALIZATION.md).
* Stack: **Rust** core, **TypeScript + React** UI, **Tauri 2** desktop and mobile apps, **PostgreSQL**,
  **Tantivy** search – see [docs/TECHNOLOGY.md](docs/TECHNOLOGY.md).
* Design: Windows 11 / Fluent style, calm pastel section colours, light and dark themes – see [docs/DESIGN.md](docs/DESIGN.md).
* Licences of all components and AI models – see [docs/LICENSES.md](docs/LICENSES.md).
* The desktop app: user guide (Slovak) – see [docs/sk/PRIRUCKA.md](docs/sk/PRIRUCKA.md).

## Repository
| Folder | Contents |
|---|---|
| `crates/atena-core/` | text normalisation, taxonomy, classification suggestions, change detection |
| `crates/atena-crawl/` | polite and safe crawler: robots.txt (RFC 9309), SSRF guard, pacing, TDMRep, RSS/Atom, sitemaps – see [docs/CRAWLER.md](docs/CRAWLER.md) |
| `crates/atena-extract/` | main text without menus and footers, metadata and language from HTML (also legacy encodings), PDF, DOCX and ODT |
| `crates/atena-store/` | PostgreSQL schema and queries, host-lease scheduling; original files only in the archive – see [docs/DATABASE.md](docs/DATABASE.md) |
| `crates/atena-assess/` | automatic rights assessment: input lines → licence, terms of use and evidence – see [docs/ASSESSMENT.md](docs/ASSESSMENT.md) |
| `crates/atena-search/` | multilingual full-text search (Tantivy) of what the legality principle allows: own stemmers for Slovak, Czech and Polish, concepts across languages – see [docs/SEARCH.md](docs/SEARCH.md) |
| `crates/atena-ingest/` | the crawler program: register → fetch → extraction → versions → classification → search index; imports and shared proposals – see [docs/RUNNING.md](docs/RUNNING.md), [docs/IMPORT-AND-SHARING.md](docs/IMPORT-AND-SHARING.md) |
| `crates/atena-app/` | the desktop app without its window: the database inside the app (a private PostgreSQL for this computer only), the operations of the interface, errors as translatable codes – see [docs/DESKTOP.md](docs/DESKTOP.md) |
| `apps/desktop/` | the desktop app: Tauri 2 window, React interface in five languages (Fluent), Windows installer with PostgreSQL inside – see [apps/desktop/README.md](apps/desktop/README.md) |
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
6. **The legality principle stands above every other rule**: only open sources; nothing may infringe the
   law, misuse data or breach data protection – in Slovakia, the EU or any other country; the operator's
   law and each source country's law apply and the stricter wins; what is uncertain is not done.
   Written into the core (`atena-core/src/legality.rs`) – see [docs/LEGALITY.md](docs/LEGALITY.md) and
   [docs/PRIVACY-AND-RIGHTS.md](docs/PRIVACY-AND-RIGHTS.md).
7. **Security beside legality**: ATENA sends nothing about its users to anyone and uses no third-party
   services; by itself it connects only to its sources, as an anonymous visitor, and to the operator's
   own server, which stores no IP addresses (`legality::CONNECTIONS`).

## Contributing
* Code, identifiers, commit messages and technical documentation: **English**.
* User interface: texts never in code, only in locale files (Project Fluent); a new language is added by
  translating a file, without changing the program. **Slovak** is the reference locale, English the pivot for
  translators, Czech, Russian and Chinese are core locales of equal status.
* User guide and communication with the project author: **Slovak** (`docs/sk/`).
