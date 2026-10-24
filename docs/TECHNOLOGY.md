# ATENA technology (decided 24 Oct 2026)

Rule: everything open, no licence fees, free for private and commercial use.
Licences are enforced in CI with `cargo-deny` (Rust) and an npm licence check, as in ALTAIS.

| Layer | Technology | Licence | Why |
|---|---|---|---|
| Core (server, crawler, processing, API) | **Rust**: Axum, Tokio, sqlx, reqwest | MIT / Apache-2.0 | fastest in our benchmark (669 docs/s vs Python 15), single binary, low memory, compile-time safety |
| Database | **PostgreSQL** + pgvector | PostgreSQL License | source of truth, scales to hundreds of GB and beyond |
| Full-text search | **Tantivy** (embedded) | MIT | fast, no extra service, rebuildable from the DB |
| Files (PDF, Word) | content-addressed store (BLAKE3); later SeaweedFS | CC0 / Apache-2.0 | each file stored once |
| Language detection | lingua | Apache-2.0 | handles Slovak vs Czech |
| PDF | PDFium (pdfium-render) | BSD / MIT | text with page numbers |
| AI models | ONNX Runtime (BGE-M3, E5), llama.cpp (EuroLLM, Qwen3) | MIT / Apache-2.0 | local, texts never leave the server |
| Complex PDFs with tables (optional) | Python + Docling | MIT | the one place where Python is best |
| User interface | **TypeScript + React** | MIT | web, desktop and mobile from one code base |
| Desktop and mobile app | **Tauri 2** | MIT / Apache-2.0 | small installers; Windows, macOS, Linux, Android, iOS |
| Local library in the app | SQLite | public domain | instant offline search |
| UI translations, formatting | Project Fluent, ICU4X | Apache-2.0 / Unicode | see INTERNATIONALIZATION.md |

**Do not use:** MongoDB (SSPL), Elasticsearch (AGPL/SSPL/Elastic), MinIO (AGPL), CockroachDB (proprietary),
models NLLB-200, SeamlessM4T, TowerInstruct (CC BY-NC), Llama and Gemma (restrictive terms). Details: [LICENSES.md](LICENSES.md).

## Target repository layout
```
crates/atena-core     model, taxonomy, versions, classification (shared by server and app)
crates/atena-crawl    polite crawler: robots.txt RFC 9309, SSRF guard, TDMRep, RSS/Atom, sitemaps
crates/atena-extract  HTML, PDF, DOCX, language
crates/atena-search   Tantivy + PostgreSQL
crates/atena-server   API and web (Axum)
apps/web              React + TypeScript UI
apps/desktop          Tauri 2
data/                 taxonomy, source register, country packs
prototyp-v0.1/        Python prototype – executable specification and test scenarios
```

## Database size estimate (2 million documents)
About 85 GB of data and indexes (vectors are the largest part; binary quantisation shrinks them to a few GB)
and about 600 GB of original files outside the database. A global deployment with 20 million documents is
roughly ten times that – still a single PostgreSQL server with partitioning and replicas.

## Benchmark
`tools/language-benchmark`: the same classification algorithm in five languages, identical results:
Rust 669, Go 526, Java 317, Node 246, Python 15 documents per second (one core).
