# Spustenie zberača

*English: [../RUNNING.md](../RUNNING.md)*

## Čo treba mať
* Rust (aktuálna stabilná verzia).
* PostgreSQL 15 alebo novší a v ňom databázu s používateľom, ktorý ju vlastní:
  ```sh
  createuser atena && createdb -O atena atena
  ```

## Príkazy
```sh
export ATENA_DATABASE_URL=postgres://atena@localhost/atena
export ATENA_FILES=/var/lib/atena/files          # úložisko pôvodných súborov

cargo run --release -p atena-ingest -- migrate   # vytvorí alebo aktualizuje tabuľky
cargo run --release -p atena-ingest -- sync      # načíta register zdrojov data/sources.yaml
cargo run --release -p atena-ingest -- run       # zbiera, kým ho nezastavíte (Ctrl+C)
cargo run --release -p atena-ingest -- run --once --concurrency 8   # zbiera, kým je čo
cargo run --release -p atena-ingest -- fetch https://www.minedu.sk/  # jedna adresa, bez databázy
cargo run --release -p atena-ingest -- legality  # čo zásada zákonnosti dovoľuje pri každom zdroji
cargo run --release -p atena-ingest -- assess navrhy.txt [--json]
                                                 # návrh licencie a podmienok pre adresy, jedna na riadok
cargo run --release -p atena-ingest -- import navrhy.txt   # uložiť ako návrhy (a zdieľať ich)
cargo run --release -p atena-ingest -- confirm ID --by MENO --reason TEXT
cargo run --release -p atena-ingest -- sync --shared       # načítať spoločný register
                                                 # viac: docs/sk/IMPORT-A-ZDIELANIE.md
cargo run --release -p atena-ingest -- forget ADRESA --reason "žiadosť z 24. 10. 2026" [--prefix]
                                                 # žiadosť o výmaz: vymaže a navždy zablokuje
cargo run --release -p atena-ingest -- index     # aktualizovať index vyhľadávania (--rebuild: celý znova)
cargo run --release -p atena-ingest -- search citatelska gramotnost --country SK
                                                 # vyhľadávanie; viac: docs/sk/VYHLADAVANIE.md
```
Príkaz `migrate` sa spustí aj sám pred `sync` a `run`. Na jednej databáze môže bežať viac zberačov,
aj na rôznych počítačoch; ten istý server pritom nikdy nespracúvajú dva naraz.
`ATENA_LOG=debug` vypisuje podrobnosti. Príkaz `run` každú minútu a pri skončení aktualizuje aj index
vyhľadávania v priečinku `ATENA_INDEX` (predvolene `./atena-index`).

## Čo zberač robí
1. Vezme servery, ktoré sú na rade, a skontroluje ich adresy.
2. Z **RSS kanálov** a **sitemáp** pridá nové články a stránky. Tie sa potom kontrolujú raz za 30 dní.
3. **Stránky a súbory** spracuje. Ak sa text zmenil, uloží novú verziu spolu s odkazmi a návrhmi
   zaradenia.
4. Pri stránkach z registra pridá aj ich prílohy (PDF, Word, Excel…), podstránky vlastnej sekcie a RSS
   kanály, ktoré stránka ponúka.
5. Výsledok každej kontroly zapíše do tabuľky `fetch_log`.

## Výsledky kontrol
| Výsledok | Význam |
|---|---|
| `new`, `changed` | prvá verzia, nová verzia |
| `unchanged`, `not_modified` | text sa nezmenil / server odpovedal „nezmenené“ (304) |
| `feed`, `sitemap` | položky pridané na kontrolu |
| `not_permitted` | zásada zákonnosti zber zo zdroja nedovoľuje |
| `robots` | robots.txt ATENE zakazuje prístup |
| `robots_unreachable` | robots.txt sa nedal stiahnuť, preto sa nesmie sťahovať nič; skúsi sa znova o 1 h, 4 h, 16 h… |
| `retry_later` | server žiada prestávku (429/503); celý server počká |
| `gone` | stránka zmizla (404/410); dokument sa označí, jeho história ostáva |
| `http_error`, `network`, `timeout`, `redirects`, `blocked` | chyby; ďalší pokus s odstupom |
| `too_large`, `unsupported` | súbor nad 50 MiB alebo formát, ktorý ATENA nečíta |
| `extract_error`, `feed_error`, `sitemap_error` | poškodený obsah |

Adresa, ktorú ATENA našla sama (článok z kanála, príloha), sa po 6 neúspešných pokusoch za sebou
prestane kontrolovať. Adresy z registra sa nikdy nevypnú, len sa kontrolujú s väčším odstupom, kým
zdroj znova nefunguje.
