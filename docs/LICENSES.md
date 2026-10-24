# Licences: what ATENA may use

**Project rule:** no component may require a fee or forbid commercial use.
ATENA code is **Apache-2.0** (permissive, with a patent grant – free for private and commercial use).
Add a new dependency only after adding it to this list and to the licence check in CI.

## Rust core (direct dependencies)

The full tree is checked by `cargo deny` on every pull request (`deny.toml`).

| Component | Licence | Used for |
|---|---|---|
| serde, serde_json, thiserror | MIT OR Apache-2.0 | data structures, JSON, errors |
| serde-saphyr | MIT OR Apache-2.0 | YAML (taxonomy, register) |
| unicode-normalization | (MIT OR Apache-2.0) AND Unicode-3.0 | folding diacritics |
| blake3 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | content fingerprints |
| similar | Apache-2.0 | text differences between versions |
| tokio | MIT | asynchronous runtime |
| reqwest (without default features) | MIT OR Apache-2.0 | HTTP client |
| rustls with the ring provider | Apache-2.0 OR ISC OR MIT; ring: Apache-2.0 AND ISC | TLS; aws-lc (reqwest's default) is not used because of its OpenSSL licence |
| url | MIT OR Apache-2.0 | URL parsing (WHATWG) |
| quick-xml | MIT | RSS/Atom, sitemaps |
| jiff | Unlicense OR MIT | dates |
| flate2 | MIT OR Apache-2.0 | gzip sitemaps |
| bytes | MIT | response bodies |
| scraper, ego-tree | ISC | HTML document tree (built on html5ever, MIT OR Apache-2.0) |
| encoding_rs | (Apache-2.0 OR MIT) AND BSD-3-Clause | legacy encodings (windows-1250, ISO-8859-2, windows-1251, KOI8-R…) |
| chardetng | Apache-2.0 OR MIT | guessing an undeclared encoding |
| pdf-extract (with lopdf) | MIT | PDF text and metadata; lopdf's unmaintained ttf-parser is only used for writing PDFs and is excepted in `deny.toml` (PDFium may replace it later) |
| zip | MIT | DOCX and ODT archives |
| lingua (9 languages only) | Apache-2.0 | language detection |
| sqlx 0.8 (PostgreSQL, migrations) | MIT OR Apache-2.0 | database access (0.9 needs Rust 1.94) |
| jiff-sqlx | Unlicense OR MIT | dates in the database |
| tracing, tracing-subscriber | MIT | log of the crawler |
| cap | MIT OR Apache-2.0 | memory limit of the parsing worker |
| psl | MIT OR Apache-2.0 | public suffix list: which hosts belong to one site |
| tantivy 0.27 (without default features: memory-mapped files, Snowball stemmers, LZ4; no zstd) | MIT; Snowball stemmers (frostem) BSD-3-Clause | full-text search |
| getrandom | MIT OR Apache-2.0 | the random password of the database inside the desktop app |

## Desktop app (`apps/desktop`)

The window crate has its own `Cargo.lock`; CI checks it with the same `deny.toml`. The installer
carries the licence texts of every part (`notices/` in the app's folder, written by
`apps/desktop/scripts/notices.mjs` and `postgres-notices.mjs`).

| Component | Licence | Used for |
|---|---|---|
| Tauri 2 (wry, tao and the system web view: WebView2 on Windows, WebKitGTK on Linux) | Apache-2.0 OR MIT | the window |
| open | MIT | opening the original of a document in the user's browser |
| React, React DOM, scheduler | MIT | the interface |
| @fluent/bundle (Project Fluent) | Apache-2.0 | texts of the interface in every language |
| @tauri-apps/api | Apache-2.0 OR MIT | calling the commands of the window |
| Fluent UI System Icons (@fluentui/svg-icons) | MIT | icons |
| Open Sans, Noto Sans SC (@fontsource) | OFL-1.1 (fonts), MIT (packaging) | type in Latin, Cyrillic and Chinese |
| Vite, TypeScript, @vitejs/plugin-react, @tauri-apps/cli | MIT, Apache-2.0 | building only – not part of the app |
| PostgreSQL 17 (official Windows binaries of EnterpriseDB) | PostgreSQL License | the database inside the app |
| libraries of those binaries: OpenSSL, ICU, libxml2, libxslt, zlib, LZ4, Zstandard, MIT Kerberos, mingw-w64 winpthreads, Microsoft Visual C++ runtime | Apache-2.0, Unicode-3.0, MIT, Zlib, BSD, redistribution terms | used by PostgreSQL |
| GNU libiconv, GNU gettext (libintl) | LGPL-2.1-or-later | used by PostgreSQL unmodified, as separate libraries; licence text and source links in the notices |

The build stops when the PostgreSQL binaries bring a library that `postgres-notices.mjs` does not
know, so that a person checks its licence first. `proc-macro-error` (unmaintained) comes with the
Linux GTK bindings of Tauri as a build-time macro and is excepted in `deny.toml`.

## Prototype v0.1 (Python)

| Component | Licence | Note |
|---|---|---|
| Python | PSF | |
| PostgreSQL, pg_trgm, unaccent | PostgreSQL License | |
| FastAPI | MIT | |
| Uvicorn, Jinja2, httpx, pypdf, py3langid | BSD-3-Clause | |
| feedparser | BSD-2-Clause | |
| trafilatura | Apache-2.0 | from 1.8.0 (older versions GPL) |
| python-docx, PyYAML | MIT | |
| defusedxml | PSF-2.0 | |
| psycopg 3 | LGPL-3.0 | use as an unmodified library is allowed commercially |

## Planned and optional components

| Component | Licence | Verdict |
|---|---|---|
| pgvector | PostgreSQL License | ✅ semantic search |
| Docling (PDF/DOCX with structure and tables) | MIT | ✅ |
| Tesseract OCR | Apache-2.0 | ✅ for scans |
| OpenSearch | Apache-2.0 | ✅ only if PostgreSQL is not enough |
| Elasticsearch | AGPL / SSPL / Elastic License | ⚠️ not recommended |
| Meilisearch | MIT (Community) / BUSL (Enterprise) | ⚠️ Community edition only |
| llama.cpp | MIT | ✅ local model server |
| vLLM | Apache-2.0 | ✅ |
| CTranslate2 | MIT | ✅ fast translation with MADLAD/OPUS-MT |
| Keycloak (school sign-in) | Apache-2.0 | ✅ when there are several organisations |

## AI models

| Model | Licence | Verdict |
|---|---|---|
| EuroLLM-9B / 22B (35 languages incl. SK, CS, PL, HU, DE, RU, ZH) | Apache-2.0 | ✅ recommended for translation and summaries |
| Qwen3 | Apache-2.0 | ✅ strong Chinese |
| Mistral Small 3.x | Apache-2.0 | ✅ |
| OLMo 2 | Apache-2.0 | ✅ fully open (training data too) |
| MADLAD-400 (translation, 400+ languages) | Apache-2.0 | ✅ |
| OPUS-MT (Helsinki-NLP) | CC BY 4.0 | ✅ with attribution |
| BGE-M3, multilingual-E5 (embeddings) | MIT | ✅ |
| Whisper (speech to text) | MIT | ✅ only for content we may use |
| **NLLB-200** | **CC BY-NC 4.0** | ❌ forbids commercial use |
| **SeamlessM4T, TowerInstruct** | **CC BY-NC** | ❌ |
| Llama | custom licence with restrictions | ❌ not an open licence |
| Gemma | custom terms of use | ⚠️ not an open licence – not recommended |

## External services (optional adapters, off by default)

| Service | Terms | Verdict |
|---|---|---|
| OpenAlex | data CC0; API with a free account and daily budget, downloadable snapshot | ✅ |
| Crossref, DOAJ, Zenodo | open metadata | ✅ |
| CORE | free with rate limits; faster access is paid | ✅ free tier only |
| YouTube Data API | free quota, terms of service | ✅ metadata and embedding only |
| DeepL, Google Translate, paid LLM APIs | paid | ❌ not in the core |
| eTranslation (European Commission) | eligible institutions only | ⚠️ optional |

## Content of crawled websites

* Publicly available ≠ free to reuse. Default: **metadata + link + short excerpt**.
* Internal full-text indexing relies on the text and data mining exception (EU Directive 2019/790, Art. 4; in Slovakia § 51c of Act 185/2015) – **only if the rightholder has not reserved it**. ATENA reads robots.txt and the `tdm-reservation` header and meta tag and stores only metadata when a reservation exists. Each country pack documents its national implementation.
* Full text or a full translation is published only under an open licence (CC BY, CC BY-SA, CC0, Open Government Licence) – per-source settings `show_fulltext` / `translate_allowed`.
* This is not legal advice; consult a copyright lawyer before a public launch.
