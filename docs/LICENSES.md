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
