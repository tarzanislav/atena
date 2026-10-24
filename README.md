# ATENA

*Slovenčina: [README.sk.md](README.sk.md)*

**An open aggregator of verified information about education.** ATENA collects official, research and
practice sources from many countries, tracks every change, classifies content in a multilingual taxonomy
and always links each item back to its original source.

* Code licence **Apache-2.0** (free for private and commercial use), data CC BY 4.0 / CC0, no paid dependencies.
* Multilingual and multi-country by design; **Slovak is the first locale and Slovakia the first country pack**,
  Czech follows – see [docs/INTERNATIONALIZATION.md](docs/INTERNATIONALIZATION.md).
* Stack: **Rust** core, **TypeScript + React** UI, **Tauri 2** desktop and mobile apps, **PostgreSQL**,
  **Tantivy** search – see [docs/TECHNOLOGY.md](docs/TECHNOLOGY.md).
* Design: Windows 11 / Fluent style, calm pastel section colours, light and dark themes – see [docs/DESIGN.md](docs/DESIGN.md).
* Licences of all components and AI models – see [docs/LICENSES.md](docs/LICENSES.md).

## Repository
| Folder | Contents |
|---|---|
| `data/` | taxonomy (296 concepts, 15 facets, 8 languages) and a register of 186 sources |
| `prototyp-v0.1/` | working Python prototype (crawling, versions, classification, search, web, 41 tests) – the executable specification for the Rust core |
| `nastroje/meranie-jazykov/` | benchmark of Rust, Go, Java, Node and Python on ATENA's classification task |
| `docs/` | technology, design, licences, internationalisation (English); `docs/sk/` Slovak versions |

## Principles
1. Every item has a direct link to the original, the capture date and a version.
2. Source authority (where it comes from) and strength of evidence (what it proves) are assessed separately.
3. Automatic classification and translation are always marked as suggestions until a human reviews them.
4. Unknown values stay "unknown" – nothing is filled in by guessing.
5. Alternative pedagogical approaches are never ranked down.

## Contributing
Code, identifiers, commit messages and technical documentation are in English.
User interface texts live in locale files (Project Fluent); the Slovak locale is the reference.
