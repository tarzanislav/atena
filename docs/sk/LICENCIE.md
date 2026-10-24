# Licencie: čo ATENA smie použiť

**Pravidlo projektu:** žiadna súčasť nesmie vyžadovať poplatok ani zakazovať komerčné použitie.
Kód ATENA je pod **Apache-2.0** (permisívna licencia s patentovou doložkou – voľné súkromné aj komerčné použitie).
Novú závislosť pridaj až po doplnení do tohto zoznamu; celý strom knižníc jadra kontroluje `cargo deny` pri každom pull requeste (`deny.toml`).

## Jadro v Ruste (priame závislosti)

| Súčasť | Licencia | Na čo |
|---|---|---|
| serde, serde_json, thiserror | MIT OR Apache-2.0 | dátové štruktúry, JSON, chyby |
| serde-saphyr | MIT OR Apache-2.0 | YAML (taxonómia, register) |
| unicode-normalization | (MIT OR Apache-2.0) AND Unicode-3.0 | odstránenie diakritiky pri hľadaní |
| blake3 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | odtlačky obsahu |
| similar | Apache-2.0 | rozdiely medzi verziami |
| tokio | MIT | súbežné spracovanie |
| reqwest (bez predvolených funkcií) | MIT OR Apache-2.0 | sťahovanie (HTTP) |
| rustls s ring | Apache-2.0 OR ISC OR MIT; ring: Apache-2.0 AND ISC | šifrovanie (TLS); aws-lc (predvolené v reqwest) sa nepoužíva pre licenciu OpenSSL |
| url | MIT OR Apache-2.0 | rozbor adries (WHATWG) |
| quick-xml | MIT | RSS/Atom, sitemapy |
| jiff | Unlicense OR MIT | dátumy |
| flate2 | MIT OR Apache-2.0 | zbalené sitemapy (gzip) |
| bytes | MIT | obsah odpovedí |
| scraper, ego-tree | ISC | strom dokumentu HTML (nad html5ever, MIT OR Apache-2.0) |
| encoding_rs | (Apache-2.0 OR MIT) AND BSD-3-Clause | staré kódovania (windows-1250, ISO-8859-2, windows-1251, KOI8-R…) |
| chardetng | Apache-2.0 OR MIT | odhad neuvedeného kódovania |
| pdf-extract (s lopdf) | MIT | text a metadáta z PDF; neudržiavaný ttf-parser v lopdf sa používa len pri tvorbe PDF, preto má výnimku v `deny.toml` (neskôr ho môže nahradiť PDFium) |
| zip | MIT | archívy DOCX a ODT |
| lingua (len 9 jazykov) | Apache-2.0 | rozpoznanie jazyka |
| sqlx 0.8 (PostgreSQL, migrácie) | MIT OR Apache-2.0 | prístup k databáze (0.9 vyžaduje Rust 1.94) |
| jiff-sqlx | Unlicense OR MIT | dátumy v databáze |
| tracing, tracing-subscriber | MIT | záznam o behu zberača |
| tantivy 0.27 (bez predvolených funkcií: pamäťovo mapované súbory, stemmery Snowball, LZ4; bez zstd) | MIT; stemmery Snowball (frostem) BSD-3-Clause | fulltextové vyhľadávanie |

## Prototyp v0.1 (Python)

| Súčasť | Licencia | Poznámka |
|---|---|---|
| Python | PSF | |
| PostgreSQL, pg_trgm, unaccent | PostgreSQL License | |
| FastAPI | MIT | |
| Uvicorn, Jinja2, httpx, pypdf, py3langid | BSD-3-Clause | |
| feedparser | BSD-2-Clause | |
| trafilatura | Apache-2.0 | od verzie 1.8.0 (staršie verzie GPL) |
| python-docx, PyYAML | MIT | |
| defusedxml | PSF-2.0 | |
| psycopg 3 | LGPL-3.0 | použitie ako knižnice bez úprav je povolené aj komerčne |

## Plánované a voliteľné súčasti

| Súčasť | Licencia | Verdikt |
|---|---|---|
| pgvector | PostgreSQL License | ✅ významové vyhľadávanie (etapa 2) |
| Docling (PDF/DOCX so štruktúrou a tabuľkami) | MIT | ✅ |
| Tesseract OCR | Apache-2.0 | ✅ pre skeny |
| OpenSearch | Apache-2.0 | ✅ len ak PostgreSQL nebude stačiť |
| Elasticsearch | AGPL / SSPL / Elastic License | ⚠️ neodporúčané |
| Meilisearch | MIT (Community) / BUSL (Enterprise) | ⚠️ len Community edícia |
| llama.cpp | MIT | ✅ lokálny server jazykových modelov |
| vLLM | Apache-2.0 | ✅ |
| CTranslate2 | MIT | ✅ rýchly preklad modelmi MADLAD/OPUS-MT |
| Keycloak (prihlasovanie škôl) | Apache-2.0 | ✅ až pri viacerých organizáciách |

## Modely umelej inteligencie

| Model | Licencia | Verdikt |
|---|---|---|
| EuroLLM-9B / 22B (35 jazykov vrátane SK, CS, PL, HU, DE, RU, ZH) | Apache-2.0 | ✅ odporúčaný na preklad a anotácie |
| Qwen3 | Apache-2.0 | ✅ silná čínština |
| Mistral Small 3.x | Apache-2.0 | ✅ |
| OLMo 2 | Apache-2.0 | ✅ úplne otvorený (aj trénovacie dáta) |
| MADLAD-400 (preklad, 400+ jazykov) | Apache-2.0 | ✅ |
| OPUS-MT (Helsinki-NLP) | CC BY 4.0 | ✅ s uvedením autorstva |
| BGE-M3, multilingual-E5 (embeddingy) | MIT | ✅ |
| Whisper (prepis zvuku) | MIT | ✅ len pri oprávnenom obsahu |
| **NLLB-200** | **CC BY-NC 4.0** | ❌ zakazuje komerčné použitie |
| **SeamlessM4T, TowerInstruct** | **CC BY-NC** | ❌ |
| Llama | vlastná licencia s obmedzeniami | ❌ nie je otvorená licencia |
| Gemma | vlastné podmienky použitia | ⚠️ nie je otvorená licencia – neodporúčané |

## Externé služby (len voliteľné adaptéry, predvolene vypnuté)

| Služba | Podmienky | Verdikt |
|---|---|---|
| OpenAlex | dáta CC0; API cez bezplatný účet s denným limitom, snapshot na stiahnutie | ✅ |
| Crossref, DOAJ, Zenodo | otvorené metadáta | ✅ |
| CORE | bezplatne s limitom rýchlosti; rýchlejší prístup platený | ✅ len bezplatný režim |
| YouTube Data API | bezplatná kvóta, podmienky služby | ✅ len metadáta a vloženie |
| DeepL, Google Translate, platené API jazykových modelov | platené | ❌ nie v jadre |
| eTranslation (Európska komisia) | len pre oprávnené inštitúcie | ⚠️ voliteľne |

## Obsah zbieraných webov

* Verejne dostupný ≠ voľne použiteľný. Predvolene: **metadáta + odkaz + krátky výňatok**.
* Interné indexovanie plného textu sa opiera o výnimku na vyťažovanie textov a dát (smernica EÚ 2019/790, čl. 4; v SR § 51c autorského zákona 185/2015 Z. z.) – **iba ak si autor použitie výslovne nevyhradil**. ATENA preto číta robots.txt, hlavičku a meta značku `tdm-reservation` a pri výhrade ukladá len metadáta.
* Celý text alebo celý preklad sa zverejní len pri otvorenej licencii (CC BY, CC BY-SA, CC0, Open Government Licence) – nastavenie `show_fulltext` / `translate_allowed` pri zdroji.
* Toto nie je právne stanovisko; pred verejným spustením odporúčam konzultáciu s právnikom pre autorské právo.
