# Technológie ATENA (rozhodnuté 24. 10. 2026)

Zásada: všetko otvorené, bez licenčných poplatkov, voľne súkromne aj komerčne.
Licencie každej súčasti stráži v CI `cargo-deny` (Rust) a kontrola npm licencií – rovnako ako v ALTAIS.

## Prehľad

| Vrstva | Technológia | Licencia | Prečo |
|---|---|---|---|
| Jadro (server, zberač, spracovanie, API) | **Rust**: Axum, Tokio, sqlx, reqwest | MIT / Apache-2.0 | najrýchlejší v našom meraní (669 dok./s vs. Python 15), jeden program, málo pamäte, chyby odhalí kompilátor |
| Databáza | **PostgreSQL** + pgvector | PostgreSQL License | zdroj pravdy, zvláda stovky GB až TB |
| Fulltextové vyhľadávanie | **Tantivy** (v programe) | MIT | rýchle, bez ďalšej služby, dá sa znovu zostaviť z DB |
| Súbory (PDF, Word) | úložisko podľa odtlačku BLAKE3; neskôr SeaweedFS | Apache-2.0 / CC0 | jedno PDF sa uloží raz |
| Jazyk dokumentu | lingua | Apache-2.0 | aj sk vs. cs |
| PDF | PDFium (pdfium-render) | BSD / MIT | text so stranami |
| AI modely | ONNX Runtime (BGE-M3, E5), llama.cpp (EuroLLM, Qwen3) | MIT / Apache-2.0 | lokálne, bez posielania textov von |
| Zložité PDF s tabuľkami (voliteľne) | Python + Docling | MIT | jediné miesto, kde je Python najlepší |
| Rozhranie | **TypeScript + React** | MIT | web, PC aj mobil z jedného kódu |
| Program pre PC a mobil | **Tauri 2** | MIT / Apache-2.0 | inštalátor pár MB, Windows, Mac, Linux, Android, iOS |
| Lokálna knižnica v programe | SQLite | verejná doména | okamžité hľadanie aj bez internetu |
| Preklady rozhrania | Project Fluent | Apache-2.0 | ako ALTAIS |

## Nepoužívať
MongoDB (SSPL), Elasticsearch (AGPL/SSPL/Elastic), MinIO (AGPL), CockroachDB (vlastná licencia),
modely NLLB-200, SeamlessM4T, TowerInstruct (CC BY-NC), Llama a Gemma (obmedzujúce podmienky).
Podrobne: [LICENCIE.md](LICENCIE.md).

## Štruktúra repozitára (cieľ)
```
crates/atena-core     model, taxonómia, verzie, zaradenie (spoločné pre server aj program)
crates/atena-crawl    slušný zberač: robots RFC 9309, SSRF, TDMRep, RSS/Atom, sitemap
crates/atena-extract  HTML, PDF, DOCX, jazyk
crates/atena-search   Tantivy + PostgreSQL
crates/atena-server   API a web (Axum)
apps/web              rozhranie React + TypeScript (dizajn ŠkVP)
apps/desktop          Tauri 2 (Windows, neskôr Mac, Linux, mobil)
data/                 taxonomia.yaml, zdroje.yaml
prototyp-v0.1/        Python prototyp – špecifikácia a testovacie scenáre
nastroje/             meranie jazykov a ďalšie nástroje
```

## Meranie
`nastroje/meranie-jazykov` – rovnaký algoritmus zaraďovania v 5 jazykoch, rovnaký výsledok:
Rust 669, Go 526, Java 317, Node 246, Python 15 dokumentov za sekundu.
