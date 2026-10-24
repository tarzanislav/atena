# Databáza a úložisko súborov

*English: [../DATABASE.md](../DATABASE.md)*

ATENA ukladá všetko do databázy **PostgreSQL** (verzia 15 alebo novšia) a pôvodné súbory do úložiska,
kde má každý súbor meno podľa svojho odtlačku. Databáza je zdrojom pravdy; vyhľadávacie indexy sa z nej
dajú kedykoľvek prepočítať. Všetky príkazy SQL sú v module `atena-store`.

## Tabuľky
| Tabuľka | Čo obsahuje |
|---|---|
| `source` | register zdrojov (`data/sources.yaml`); zdroj, ktorý z registra zmizne, sa vypne, nezmaže |
| `host` | jeden riadok na server: kedy je na rade, kto ho práve spracúva, prestávka po `429`/`503` |
| `endpoint` | každá sledovaná adresa: kanály, sitemapy, stránky; plán kontrol, `ETag`, `Last-Modified`, chyby |
| `fetch_log` | záznam o každej kontrole; po 90 dňoch sa maže |
| `blob` | pôvodné súbory v úložisku podľa odtlačku BLAKE3 |
| `document` | dokument zo stránky alebo súboru: aktuálny názov, jazyk, stav, pravidlá vydavateľa |
| `document_version` | nový riadok len vtedy, keď sa zmení **text**: druh zmeny, pridané a odobraté odseky, zmena čísel a dátumov |
| `document_link` | odkazy v aktuálnej verzii (prílohy, podstránky) a videá (`kind = 'video'`: videá na YouTube a Vimeo, na ktoré stránka odkazuje alebo ktoré vkladá – zapíšu sa, nikdy sa neotvárajú) |
| `document_label` | zaradenie: návrhy s dôvodom a rozhodnutia redaktora |
| `blocked_url` | adresy (alebo celé sekcie), ktoré sa už nikdy nestiahnu – žiadosti o výmaz |
| `erasure_log` | čo výmaz odstránil a prečo, bez odstráneného obsahu |

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
  odtlačky. Text, pôvodný súbor ani odkazy sa neukladajú a zaradenie sa robí len podľa názvu. Ak sa výhrada
  objaví neskôr, doteraz uložené texty, odkazy a automatické návrhy sa odstránia. Toto pravidlo schválil
  autor projektu; pred verejným spustením ho odporúčame overiť s právnikom.
* `noindex`, `noarchive`, `nosnippet`, `notranslate` sa uložia k dokumentu. Vyhľadávanie a rozhranie ich
  dodržia; kópia pri `noarchive` slúži len na zistenie zmien a nikdy sa nezobrazí.

## Úložisko súborov
Ukladajú sa len pôvodné dokumenty (PDF, DOCX, ODT), ktorých text sa smie zobraziť celý (úradné texty,
otvorené licencie – [ZASADA-ZAKONNOSTI.md](ZASADA-ZAKONNOSTI.md)); z webových stránok sa ukladá len upravený text
(pozri [PRAVA-A-OCHRANA-UDAJOV.md](PRAVA-A-OCHRANA-UDAJOV.md)). Súbory sú v `<koreň>/ab/cd/<odtlačok>`. Zapisujú sa pod dočasným menom a potom sa premenujú, takže po
páde nezostane pokazený súbor pod platným menom. HTML, XML, JSON a čistý text sa ukladajú komprimované
(gzip). Pri čítaní sa odtlačok overí. Rovnaký súbor sa uloží len raz.

## Migrácie
Súbory `crates/atena-store/migrations/NNNN_nazov.sql` sú vložené priamo do programu. Spúšťa ich príkaz
`atena-ingest migrate`. Už použitá migrácia sa nikdy neupravuje; každá zmena príde ako nový súbor.
