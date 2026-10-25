# ATENA

*English: [README.md](README.md)*

**Otvorený agregátor overených informácií o vzdelávaní** – zbiera úradné, výskumné a praktické zdroje
zo Slovenska a zo sveta, sleduje ich zmeny, triedi ich vo viacjazyčnej taxonómii a každú informáciu
vedie späť na pôvodný zdroj.

## Štyri piliere
1. **Serióznosť, otvorenosť a snaha posunúť ľudské vedenie dopredu** – vedenie každého človeka.
2. **Anonymita a rešpekt k súkromiu** – nášmu aj iných.
3. **Bezpečnosť a ochrana** pred nebezpečnými ľuďmi, robotmi, kódmi a procesmi.
4. **Dodržiavanie zákona** – žiaden zákon nemôže byť porušený; rešpekt pred zákonom.

Zásady nižšie a pravidlá zapísané v jadre (`atena-core`) tieto piliere uvádzajú do praxe.

* Licencia kódu **Apache-2.0** (voľne súkromne aj komerčne), dáta CC BY 4.0 / CC0, žiadne platené závislosti.
* Technológie: jadro **Rust**, rozhranie **TypeScript + React**, program pre PC a mobil **Tauri 2**,
  databáza **PostgreSQL**, vyhľadávanie **Tantivy** – [docs/TECHNOLOGIE.md](docs/sk/TECHNOLOGIE.md).
* Dizajn v štýle Windows 11 / Fluent, pastelové farby sekcií, svetlý a tmavý vzhľad – [docs/DIZAJN.md](docs/sk/DIZAJN.md).
* Nosné jazyky od začiatku: **angličtina, slovenčina, čeština, ruština a čínština (zjednodušená)**; slovenčina je referenčný jazyk, Slovensko prvý balík krajiny, Česko druhý.
* Medzinárodnosť a balíky krajín (anglicky) – [docs/INTERNATIONALIZATION.md](docs/INTERNATIONALIZATION.md).
* Licencie súčastí a modelov AI – [docs/LICENCIE.md](docs/sk/LICENCIE.md).
* Aplikácia pre počítač: príručka pre používateľa – [docs/sk/PRIRUCKA.md](docs/sk/PRIRUCKA.md).

## Obsah repozitára
| Priečinok | Čo obsahuje |
|---|---|
| `crates/atena-core/` | úprava textu, taxonómia, návrhy zaradenia, rozpoznanie zmien |
| `crates/atena-crawl/` | slušný a bezpečný zberač: robots.txt (RFC 9309), ochrana vnútornej siete, tempo, TDMRep, RSS/Atom, sitemapy – [docs/sk/ZBERAC.md](docs/sk/ZBERAC.md) |
| `crates/atena-extract/` | hlavný text bez menu a pätičky, metadáta a jazyk z HTML (aj staré kódovania), PDF, DOCX a ODT |
| `crates/atena-store/` | databáza PostgreSQL, plánovanie kontrol; pôvodné súbory len v archíve – [docs/sk/DATABAZA.md](docs/sk/DATABAZA.md) |
| `crates/atena-assess/` | automatické právne vyhodnotenie: riadky vstupu → licencia, podmienky a doklady – [docs/sk/HODNOTENIE.md](docs/sk/HODNOTENIE.md) |
| `crates/atena-search/` | viacjazyčné fulltextové vyhľadávanie (Tantivy) v tom, čo dovoľuje zásada zákonnosti: vlastné stemmery pre slovenčinu, češtinu a poľštinu, pojmy naprieč jazykmi – [docs/sk/VYHLADAVANIE.md](docs/sk/VYHLADAVANIE.md) |
| `crates/atena-ingest/` | program zberača: register → sťahovanie → spracovanie → verzie → zaradenie → index vyhľadávania; import a zdieľané návrhy – [docs/sk/SPUSTENIE.md](docs/sk/SPUSTENIE.md), [docs/sk/IMPORT-A-ZDIELANIE.md](docs/sk/IMPORT-A-ZDIELANIE.md) |
| `crates/atena-app/` | aplikácia pre počítač bez okna: databáza vo vnútri aplikácie (súkromný PostgreSQL len pre tento počítač), operácie rozhrania, chyby ako preložiteľné kódy – [docs/sk/APLIKACIA.md](docs/sk/APLIKACIA.md) |
| `apps/desktop/` | aplikácia pre počítač: okno Tauri 2, rozhranie v Reacte v piatich jazykoch (Fluent), inštalátor pre Windows s PostgreSQL vo vnútri – [docs/sk/APLIKACIA.md](docs/sk/APLIKACIA.md) |
| `data/` | taxonómia (296 pojmov, 15 osí, 8 jazykov) a register 186 zdrojov |
| `prototyp-v0.1/` | funkčný prototyp v Pythone (zber, verzie, zaradenie, vyhľadávanie, web, 41 testov) – špecifikácia pre jadro v Ruste |
| `tools/language-benchmark/` | meranie rýchlosti Rust, Go, Java, Node, Python na úlohe ATENA |
| `docs/` | technológie, dizajn, licencie, zberač |

## Zásady
1. Každá položka má priamy odkaz na originál, dátum zachytenia a verziu.
2. Autorita zdroja (odkiaľ to je) a sila dôkazov (čo to dokazuje) sa hodnotia oddelene.
3. Automatické zaradenie a preklad sú vždy označené ako návrh, kým ich neskontroluje človek.
4. Neznámy údaj ostáva „nezistený“ – nič sa nedopĺňa odhadom.
5. Alternatívne pedagogické smery sa nepenalizujú.
6. **Zásada zákonnosti je nadradená všetkým pravidlám**: len otvorené zdroje; nič nesmie porušiť zákon,
   zneužiť údaje ani ochranu osobných údajov – na Slovensku, v EÚ ani v žiadnej inej krajine; platí
   právo prevádzkovateľa aj krajiny zdroja a vždy to prísnejšie; čo nie je isté, to sa nerobí. Zapísaná
   v jadre (`atena-core/src/legality.rs`) – [docs/sk/ZASADA-ZAKONNOSTI.md](docs/sk/ZASADA-ZAKONNOSTI.md)
   a [docs/sk/PRAVA-A-OCHRANA-UDAJOV.md](docs/sk/PRAVA-A-OCHRANA-UDAJOV.md).
7. **Bezpečnosť popri zákonnosti**: ATENA nikomu neposiela nič o svojich používateľoch a nepoužíva
   služby tretích strán. Sama od seba sa spája len so zdrojmi, ako anonymný návštevník, a s vlastným
   serverom prevádzkovateľa, ktorý IP adresy neukladá (`legality::CONNECTIONS`).

## Jazyky v projekte
* **Anglicky:** zdrojový kód, názvy v kóde, popisy zmien (commity) a technická dokumentácia – aby mohli prispievať aj zahraniční vývojári.
* **Slovensky:** prvý a referenčný jazyk rozhrania, používateľská príručka a komunikácia s autorom; slovenské verzie dokumentov sú v `docs/sk/`.
* **Texty rozhrania** nie sú v kóde, ale v súboroch prekladov (Project Fluent, ako v ALTAIS); nový jazyk sa pridá preložením súboru, bez zmeny programu.
* **Nosné jazyky** s rovnakým postavením: slovenčina (referenčná), angličtina (sprostredkujúca pre ďalších prekladateľov), čeština, ruština, čínština.
