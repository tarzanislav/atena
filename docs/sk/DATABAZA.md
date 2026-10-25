# Databáza

*English: [../DATABASE.md](../DATABASE.md)*

ATENA ukladá všetko do databázy **PostgreSQL** (verzia 15 alebo novšia) a **neukladá žiadne pôvodné
súbory** (pozri nižšie). Databáza je zdrojom pravdy; vyhľadávacie indexy sa z nej dajú kedykoľvek
prepočítať. Všetky príkazy SQL sú v module `atena-store`.

## Tabuľky
| Tabuľka | Čo obsahuje |
|---|---|
| `source` | register zdrojov (`data/sources.yaml`); zdroj, ktorý z registra zmizne, sa vypne, nezmaže |
| `assessment` | právne vyhodnotenia importovaných zdrojov: automatické (s dokladmi ako JSON) a rozhodnutia ľudí. Importovaný zdroj (`origin = 'import'`) je `proposed` a vypnutý, kým ho človek nepotvrdí |
| `inbox` / `outbox` | zoznamy navrhnutých adries prijaté z iných inštalácií / čakajúce na odoslanie |
| `host` | jeden riadok na server: kedy je na rade, kto ho práve spracúva, prestávka po `429`/`503` |
| `endpoint` | každá sledovaná adresa: kanály, sitemapy, stránky; plán kontrol, `ETag`, `Last-Modified`, chyby; koľko úrovní odkazov sa ešte sleduje (`depth`, migrácia 0008) |
| `fetch_log` | záznam o každej kontrole; po 90 dňoch sa maže |
| `document` | dokument zo stránky alebo súboru: aktuálny názov, jazyk, stav, pravidlá vydavateľa |
| `document_version` | nový riadok len vtedy, keď sa zmení **text**: druh zmeny, pridané a odobraté odseky, zmena čísel a dátumov |
| `document_link` | odkazy v aktuálnej verzii (prílohy, podstránky) a videá (`kind = 'video'`: videá na YouTube a Vimeo, na ktoré stránka odkazuje alebo ktoré vkladá – zapíšu sa, nikdy sa neotvárajú) |
| `document_label` | zaradenie: návrhy s dôvodom a rozhodnutia redaktora |
| `blocked_url` | adresy (alebo celé sekcie), ktoré sa už nikdy nestiahnu – žiadosti o výmaz |
| `erasure_log` | čo výmaz odstránil a prečo, bez odstráneného obsahu |
| `search_queue` | dokumenty, ktoré musí index vyhľadávania aktualizovať; plnia ju spúšťače – [VYHLADAVANIE.md](VYHLADAVANIE.md) |

## Plánovanie: prenájom servera
Slušnosť voči webom sa meria po serveroch, preto je jednotkou práce server. Pracovník si server
**prenajme**, postupne skontroluje jeho adresy, ktoré sú na rade, a server uvoľní. Výsledok:
* ľubovoľný počet pracovníkov, aj na viacerých počítačoch, nikdy nepošle jednému serveru dve požiadavky naraz;
* server s 50 000 adresami zo sitemapy nezablokuje ostatné – každý server príde na rad;
* keď pracovník spadne, nič sa nestratí: prenájom vyprší a server prevezme iný pracovník;
* čas sa berie z databázy, takže nezáleží na hodinách jednotlivých počítačov.

## Verzie
Ak sú bajty alebo upravený text rovnaké ako minule, zmení sa len čas poslednej kontroly. Nový text sa
uloží ako ďalšia verzia, a to v jednej transakcii spolu s odkazmi a novými návrhmi zaradenia. Druhy zmien:
* `initial` – prvé zachytenie,
* `minor` – drobná zmena,
* `substantive` – podstatná zmena: pribudol alebo ubudol odsek, zmenilo sa aspoň 40 znakov alebo hocijaké
  číslo či dátum,
* `changed` – text sa zmenil, ale predchádzajúci text nebol uložený.

## Pravidlá vydavateľa
* **Výhrada vyťažovania textu a dát** (TDMRep): ukladajú sa len metadáta, teda názov, adresa, dátumy a
  odtlačky. Text ani odkazy sa neukladajú a zaradenie sa robí len podľa názvu. Ak sa výhrada
  objaví neskôr, doteraz uložené texty, odkazy a automatické návrhy sa odstránia. Toto pravidlo schválil
  autor projektu; pred verejným spustením ho odporúčame overiť s právnikom.
* `noindex`, `noarchive`, `nosnippet`, `notranslate` sa uložia k dokumentu. Vyhľadávanie a rozhranie ich
  dodržia: kópia pri `noarchive` slúži len na zistenie zmien a nikdy sa nezobrazí, dokument s `noindex`
  sa nikdy nenájde a pri `nosnippet` sa neukáže ukážka textu ([VYHLADAVANIE.md](VYHLADAVANIE.md)).

## Pôvodné súbory: len archív
Každá verzia uchová upravený text (tam, kde to zásada zákonnosti dovoľuje) a odtlačok BLAKE3
pôvodných bajtov (`raw_hash`), ktorý dokazuje, čo vydavateľ zverejnil.

Autor projektu 24. 10. 2026 rozhodol, že ATENA neukladá pôvodné súbory (migrácia 0007): súbor môže
niesť škodlivý obsah alebo osobné údaje, ktoré sa z neho nedajú spoľahlivo odstrániť. 25. 10. 2026
otvoril jednu výnimku, **archív** (migrácia 0009, tabuľka `version_file`), aby dokumenty ostali
dostupné aj po tom, čo ich vydavateľ odstráni. Súbor sa uloží s verziou, len keď platí všetko
(`atena_ingest::pipeline::archivable`):
- dokument smie každý kopírovať a zobrazovať – úradný text voľný podľa oboch práv alebo otvorená
  licencia (`show_full_text` zásady zákonnosti) – a vydavateľ nepožiadal o neukladanie kópií
  (`noarchive`) ani si nevyhradil vyťažovanie;
- ide o PDF, DOCX alebo ODT – webové stránky si uchovajú len text;
- prešiel kontrolou nebezpečných súborov (programy a aktívny obsah sa sem nikdy nedostanú);
- v jeho texte sa nenašli osobné údaje (e-maily, telefónne čísla, rodné čísla) – súbor sa od nich
  nedá vyčistiť tak ako text.

`document.gone_at` zaznamená, kedy vydavateľ dokument prestal zverejňovať (HTTP 404/410). Odstránený
dokument sa dá ďalej nájsť a zobraziť z archívu len tam, kde sa smie zobraziť celý text; má fazetu
vyhľadávania `archive/removed`. Súbory sa zmažú spolu s verziou alebo dokumentom (výmaz), keď
neskoršie stiahnutie text vyhradí a pri každom spustení aplikácie, keď register zmenil licenciu alebo
podmienky zdroja tak, že sa už uchovávať nesmú. Archiváciu vypína nastavenie „Uchovávať pôvodné
súbory…“. Súbory v priečinku `./atena-files` zo starších verzií sa môžu zmazať.

## Migrácie
Súbory `crates/atena-store/migrations/NNNN_nazov.sql` sú vložené priamo do programu. Spúšťa ich príkaz
`atena-ingest migrate`. Už použitá migrácia sa nikdy neupravuje; každá zmena príde ako nový súbor.
