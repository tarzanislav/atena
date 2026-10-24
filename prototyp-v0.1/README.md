# ATENA

**Otvorený agregátor overených informácií o vzdelávaní** – zbiera úradné, výskumné a praktické zdroje
zo Slovenska a zo sveta, sleduje ich zmeny, triedi ich podľa viacjazyčnej taxonómie a každú informáciu
vedie späť na pôvodný zdroj.

* Licencia kódu: **Apache-2.0** (voľne súkromne aj komerčne) · dáta: CC BY 4.0 / CC0 · bez platených závislostí ([docs/LICENCIE.md](docs/LICENCIE.md))
* Stav: **v0.1 – jadro** (zber, verzie, zaradenie, vyhľadávanie, web, API). Plán etáp je v špecifikácii.

## Čo vie verzia 0.1

| Oblasť | Funkcia |
|---|---|
| Register | 186 zdrojov z 20 krajín, EÚ a globálnych organizácií (`atena/data/zdroje.yaml`), autorita A1–E, priority, frekvencie, overené RSS kanály |
| Slušný zber | robots.txt podľa RFC 9309, Crawl-delay, 1 požiadavka na server naraz, ETag/Last-Modified, Retry-After, limity veľkosti |
| Bezpečnosť | ochrana pred SSRF (kontrola DNS a každého presmerovania), bezpečné XML, izolácia chýb, žiadne tajomstvá v kóde |
| Práva | rozpoznanie výhrady vyťažovania (TDMRep, noindex) → len metadáta a odkaz |
| Dokumenty | HTML (hlavný text bez menu), PDF s číslami strán, DOCX; zistenie jazyka (aj SK vs. CS) |
| Zmeny | nemenné verzie, rozlíšenie drobnej a podstatnej zmeny (čísla a dátumy sú vždy podstatné), porovnanie verzií |
| Taxonómia | 296 pojmov v 15 osiach, 8 jazykov; priehľadný návrh zaradenia s dôvodom, potvrdzuje redaktor |
| Vyhľadávanie | bez diakritiky, so skloňovaním, s preklepmi, fazety, prepínače druhov škôl v profile |
| Web | prehľad dňa, 13 sekcií, karta zdroja, história verzií, register, pojmy, profil; pastelové farby, väčšie písmo, mobil |
| API | `/api/v1/search`, `/documents/{id}`, `/sources`, `/taxonomy`, `/novinky?autorita=A1,A2`, Atom `/feed/zmeny.atom` |
| Preklad | adaptér na lokálny model s otvorenou licenciou, kontrola čísel a terminológie, poctivé stavy prekladu |

## Spustenie

### Docker (Windows s Docker Desktop, Linux, Mac)
```bash
docker compose up -d db
docker compose run --rm web atena init
docker compose up -d          # web na http://127.0.0.1:8000, worker zbiera podľa plánu
```

### Bez Dockera
Potrebné: Python 3.11+, PostgreSQL 15+.
```bash
pip install -e ".[dev]"
set ATENA_DATABASE_URL=postgresql://postgres:heslo@localhost:5432/atena     # Windows (Linux: export)
atena init            # schéma, taxonómia, register zdrojov
atena crawl --zdroj SK07 --vsetko    # jeden zdroj hneď
atena serve           # http://127.0.0.1:8000
atena worker          # nepretržitý zber podľa plánu
```

### Testy
```bash
set ATENA_TEST_DATABASE_URL=postgresql://postgres:heslo@localhost:5432/atena_test
pytest -q
```

## Štruktúra
```
atena/crawl/       zber: netguard (SSRF), robots (RFC 9309), fetcher, feeds (RSS/Atom, sitemap, objavovanie)
atena/extract.py   HTML / PDF / DOCX → text + jazyk
atena/versioning.py  verzie a podstatné zmeny
atena/taxonomy.py  taxonómia a návrh zaradenia
atena/store.py     ukladanie (idempotentné)
atena/pipeline.py  zberový proces
atena/search.py    vyhľadávanie a fazety
atena/translate.py preklad s kontrolami
atena/jobs.py      fronta úloh v PostgreSQL
atena/web/         web a API
atena/data/        taxonomia.yaml, zdroje.yaml (upraviteľné bez zmeny kódu)
```

## Zásady
1. Každá položka má priamy odkaz na originál, dátum zachytenia a verziu.
2. Autorita zdroja (odkiaľ to je) ≠ sila dôkazov (čo to dokazuje) – hodnotia sa oddelene.
3. Automatické zaradenie a preklad sú vždy označené ako návrh, kým ich neskontroluje človek.
4. Neznámy údaj ostáva „nezistený“ – nič sa nedopĺňa odhadom.
5. Alternatívne pedagogické smery sa nepenalizujú.
