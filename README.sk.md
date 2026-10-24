# ATENA

*English: [README.md](README.md)*

**Otvorený agregátor overených informácií o vzdelávaní** – zbiera úradné, výskumné a praktické zdroje
zo Slovenska a zo sveta, sleduje ich zmeny, triedi ich vo viacjazyčnej taxonómii a každú informáciu
vedie späť na pôvodný zdroj.

* Licencia kódu **Apache-2.0** (voľne súkromne aj komerčne), dáta CC BY 4.0 / CC0, žiadne platené závislosti.
* Technológie: jadro **Rust**, rozhranie **TypeScript + React**, program pre PC a mobil **Tauri 2**,
  databáza **PostgreSQL**, vyhľadávanie **Tantivy** – [docs/TECHNOLOGIE.md](docs/sk/TECHNOLOGIE.md).
* Dizajn v štýle Windows 11 / Fluent, pastelové farby sekcií, svetlý a tmavý vzhľad – [docs/DIZAJN.md](docs/sk/DIZAJN.md).
* Nosné jazyky od začiatku: **angličtina, slovenčina, čeština, ruština a čínština (zjednodušená)**; slovenčina je referenčný jazyk, Slovensko prvý balík krajiny, Česko druhý.
* Medzinárodnosť a balíky krajín (anglicky) – [docs/INTERNATIONALIZATION.md](docs/INTERNATIONALIZATION.md).
* Licencie súčastí a modelov AI – [docs/LICENCIE.md](docs/sk/LICENCIE.md).

## Obsah repozitára
| Priečinok | Čo obsahuje |
|---|---|
| `data/` | taxonómia (296 pojmov, 15 osí, 8 jazykov) a register 186 zdrojov |
| `prototyp-v0.1/` | funkčný prototyp v Pythone (zber, verzie, zaradenie, vyhľadávanie, web, 41 testov) – špecifikácia pre jadro v Ruste |
| `tools/language-benchmark/` | meranie rýchlosti Rust, Go, Java, Node, Python na úlohe ATENA |
| `docs/` | technológie, dizajn, licencie |

## Zásady
1. Každá položka má priamy odkaz na originál, dátum zachytenia a verziu.
2. Autorita zdroja (odkiaľ to je) a sila dôkazov (čo to dokazuje) sa hodnotia oddelene.
3. Automatické zaradenie a preklad sú vždy označené ako návrh, kým ich neskontroluje človek.
4. Neznámy údaj ostáva „nezistený“ – nič sa nedopĺňa odhadom.
5. Alternatívne pedagogické smery sa nepenalizujú.

## Jazyky v projekte
* **Anglicky:** zdrojový kód, názvy v kóde, popisy zmien (commity) a technická dokumentácia – aby mohli prispievať aj zahraniční vývojári.
* **Slovensky:** prvý a referenčný jazyk rozhrania, používateľská príručka a komunikácia s autorom; slovenské verzie dokumentov sú v `docs/sk/`.
* **Texty rozhrania** nie sú v kóde, ale v súboroch prekladov (Project Fluent, ako v ALTAIS); nový jazyk sa pridá preložením súboru, bez zmeny programu.
* **Nosné jazyky** s rovnakým postavením: slovenčina (referenčná), angličtina (sprostredkujúca pre ďalších prekladateľov), čeština, ruština, čínština.
