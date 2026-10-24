# Aplikácia pre počítač

*English: [../DESKTOP.md](../DESKTOP.md)*

Aplikácia pre počítač je ATENA pre jedného človeka na jednom počítači. Vyhľadávanie, zdroje,
návrhy aj zberač sú v jednom okne, bez servera a bez inštalovania čohokoľvek ďalšieho. Skladá sa
z dvoch častí:

| Časť | Čo to je |
|---|---|
| `crates/atena-app` | jadro aplikácie bez okna: databáza vo vnútri aplikácie, všetky operácie, ktoré rozhranie smie volať, a chyby, ktoré ukazuje; testuje sa bez okna |
| `apps/desktop` | okno: Tauri 2 s rozhraním v Reacte; volania len odovzdáva do `atena-app` a samo nič nerozhoduje |

## Databáza vo vnútri aplikácie

Aplikácia si nesie programy PostgreSQL so sebou (`EmbeddedPostgres`). Pri prvom spustení si
vytvorí databázu v priečinku aplikácie používateľa. Pri každom spustení ju zapne a pri každom
ukončení vypne.

| Ochrana | Ako |
|---|---|
| len tento počítač | `listen_addresses = '127.0.0.1'`, žiadny unixový soket |
| len ATENA | jeden používateľ `atena` s náhodným 256-bitovým heslom uloženým pri dátach (na Unixe s právami `0600`; vo Windows je priečinok aplikácie používateľa ostatným používateľom zatvorený) |
| nikto iný sa nedostane dnu | `pg_hba.conf` má jediné pravidlo: databáza `postgres`, používateľ `atena`, `127.0.0.1/32`, `scram-sha-256` |
| žiadny zozbieraný text v záznamoch | `log_min_error_statement = panic`: chyba sa zapíše bez príkazu, ktorý ju spôsobil |
| pri každom spustení nový port | port sa vyberie spomedzi voľných; nič nie je pevné, s čím by mohol iný program počítať |
| jedna aplikácia na priečinok | `atena.lock` je zamknutý, kým aplikácia beží; druhá aplikácia by zastavila databázu prvej |
| pád nič nepoškodí | server, ktorý ostal bežať, sa zastaví a spustí nanovo; zberač po páde vráti pridelené hostiteľské servery |
| žiadne prekvapivé povýšenie verzie | databáza z inej hlavnej verzie PostgreSQL sa neotvorí (`database-version`) |

Vývojári môžu namiesto nej použiť vlastnú databázu: `Start::database_url`.

## Čo sa deje pri spustení

1. Priečinok sa zamkne a spustí sa PostgreSQL (alebo sa použije zadaná databáza).
2. Migrácie zosúladia schému databázy s aktuálnou verziou.
3. Ak sa register dodaný s touto verziou aplikácie líši od naposledy synchronizovaného,
   synchronizuje sa (`register.synced` uchováva jeho odtlačok). Zásada zákonnosti platí spätne,
   rovnako ako pri `atena-ingest sync`.
4. Otvorí sa index vyhľadávania; ak ho vytvorila iná verzia, postaví sa nanovo. Udržiava sa
   aktuálny každú minútu, po zastavení zberača a po vymazaní alebo zamietnutí.

## Spojenia

Aplikácia dodržiava `atena_core::legality::CONNECTIONS` a sama sa nikam nepripája. Do siete
idú len príkazy používateľa:

| Príkaz | Pripojí sa k |
|---|---|
| spustiť zberač | zdrojom registra a potvrdených importov, ako anonymný návštevník |
| importovať adresy, prečítať zoznam | týmto adresám (a ich robots.txt a podmienkam používania) |
| synchronizovať spoločný register | `raw.githubusercontent.com` (súbor registra v repozitári ATENA), len cez https |
| otvoriť výsledok | otvorí ho vlastný prehliadač používateľa |

Zdieľanie návrhov nič neposiela. Aplikácia ukáže zoznam, ktorý obsahuje len adresy a kódy krajín;
používateľ si ho skopíruje a odovzdá editorovi, ako sám chce.

## Operácie

Každá operácia je metóda `App`; okno ju sprístupní ako príkaz s rovnakým menom. `atena-app`
kontroluje každý vstup bez ohľadu na to, čo rozhranie poslalo:

- vložený text má najviac 1 MB;
- dôvod rozhodnutia je povinný a má najviac 2 000 znakov;
- kód krajiny sa prijme, len ak má krajina právny profil;
- adresy musia byť `http` alebo `https`;
- nastavenia sa vyčistia a obmedzia.

| Metóda | Čo robí |
|---|---|
| `overview` | čísla: zdroje, dokumenty, verzie, vyhľadateľné dokumenty, kontroly, návrhy, čakajúce zoznamy |
| `sources` | všetky zdroje s tým, čo dovoľuje zásada zákonnosti, a prečo nie viac (kódy) |
| `taxonomy(language)` | schémy a pojmy s názvami v jazyku rozhrania |
| `search(query)` | strana s 20 výsledkami; ukážky textu prichádzajú po kúskoch s označenými slovami |
| `newest(filters, limit)` | najnovšie dokumenty – všetky alebo zo sekcie, krajiny, zdroja … |
| `document(id)` | karta dokumentu: čo smie vyhľadávanie ukázať, jeho verzie a text tam, kde sa smie ukázať celý |
| `import(text, country)` | posúdi adresy alebo DOI a uloží ich ako návrhy |
| `proposals(state)` | importované zdroje s najnovším automatickým návrhom |
| `confirm(id, choice, reason)` / `reject(id, reason)` | rozhodnutie človeka; meno sa berie z nastavení |
| `pending_share` / `handed_over(entries)` | zoznam pre editora a označenie, že bol odovzdaný |
| `receive_list(text)` | editor: zoznam z inej inštalácie, posúdený tu |
| `sync_shared_register` | stiahne spoločný register a synchronizuje ho |
| `forget(url, prefix, reason)` | vymazanie (čl. 17 GDPR): všetko o adrese sa odstráni a adresa sa zablokuje |
| `crawl_start` / `crawl_stop` / `crawl_status` | zberač s priebežnými počtami |
| `settings` / `set_settings` | jazyk, vzhľad, meno, zdieľanie, súbežnosť zberača |

## Chyby

Chyba príde do rozhrania ako `{ "code": "...", "detail": "..." }`. Kód sa prekladá
(`error-<code>` v súboroch Fluent). Podrobnosť je technická angličtina pre tých, ktorí chcú
vedieť viac. `atena_app::CODES` vymenúva všetky kódy; test stráži, aby bol zoznam zoradený a bez
opakovaní.

## Okno

Okno (`apps/desktop`) a jeho zostavenie opisuje jeho vlastné README. Okno nič nenačítava zo siete:

- písma a ikony sú súčasťou aplikácie;
- pravidlá zabezpečenia obsahu povoľujú len vlastné súbory aplikácie;
- zozbierané texty sa ukazujú ako text, nikdy ako značkovanie;
- odkazy sa otvoria vo vlastnom prehliadači používateľa, až keď sa overí, že adresa je `http` alebo
  `https`.
