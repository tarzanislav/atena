# Meranie jazykov na úlohe ATENA (24. 10. 2026)

Rovnaký algoritmus zaraďovania (úprava textu bez diakritiky, delenie na slová, porovnanie
s kľúčovými výrazmi 296 pojmov taxonómie) v piatich jazykoch. Všetky dali rovnaký výsledok:
38 352 návrhov, kontrolný súčet 22151.042.

| Jazyk | ms na dokument | dokumentov/s |
|---|---|---|
| Rust 1.97 | 1,494 | 669 |
| Go | 1,902 | 526 |
| Java (JVM) | 3,150 | 317 |
| Node 22 (JavaScript/TypeScript) | 4,059 | 246 |
| Python 3.13 (kód ATENA v0.1) | 67,18 | 15 |

Spustenie: `python3 gen.py` (vytvorí docs.json, kw.json; 4 000 dokumentov, ~5 700 znakov)
→ `python3 bench_py.py docs.json`, `cd rs && cargo run --release -- ../docs.json ../kw.json`,
`cd go && go run . ../docs.json ../kw.json`, `node js/bench.mjs docs.json kw.json`,
Java číta TSV (pozri skript v histórii relácie).
