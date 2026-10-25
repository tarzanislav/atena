# Zberač ATENA

*English: [../CRAWLER.md](../CRAWLER.md)*

ATENA zbiera úradné, výskumné a praktické informácie o vzdelávaní a pri každej uvádza odkaz na originál.
Táto stránka je pre **správcov webov** (čo zberač robí a ako ho obmedziť) a pre **prevádzkovateľov**
ATENY (čo zberač zaručuje).

## Ako sa zberač predstavuje
* Názov pre pravidlá: **`ATENA`** – používa sa v `robots.txt` a v hlavičke `X-Robots-Tag`.
* User-Agent: `Mozilla/5.0 (compatible; ATENA/<verzia>; +https://github.com/tarzanislav/atena/blob/main/docs/CRAWLER.md)`
* Otázky a problémy: [GitHub issues](https://github.com/tarzanislav/atena/issues).

## Čo zberač dodržiava
| Pravidlo | Správanie |
|---|---|
| **robots.txt** ([RFC 9309](https://www.rfc-editor.org/rfc/rfc9309)) | Riadi sa skupinou `ATENA`, inak skupinou `*`; rozhoduje najdlhšie zhodné pravidlo, pri zhode vyhráva `Allow`; vzory `*` a `$`. Číta sa pred každou inou požiadavkou na web a platí 24 hodín. |
| robots.txt neexistuje (4xx) | Stiahnuť sa smie všetko. |
| robots.txt je nedostupný (5xx, 429, chyba siete) | Nestiahne sa **nič**; ak existuje posledná platná kópia, použije sa. Nový pokus o 30 minút. |
| `Crawl-delay` | Dodržiava sa, najviac 60 sekúnd. |
| Tempo | Na jeden server vždy len jedna požiadavka a medzi požiadavkami aspoň 5 sekúnd (predvolené nastavenie). |
| `429` / `503` s `Retry-After` | Server dostane pokoj na požadovaný čas (najviac 6 hodín; bez hlavičky 60 sekúnd). |
| Podmienené požiadavky | `If-None-Match` / `If-Modified-Since` – nezmenená stránka stojí len odpoveď `304`. |
| **TDMRep** ([W3C](https://www.w3.org/community/reports/tdmrep/CG-FINAL-tdmrep-20240510/)) | `/.well-known/tdmrep.json` (platí 24 h), hlavičky `tdm-reservation` / `tdm-policy`, neskôr aj údaje v HTML a PDF. Výhrada sa zapíše ku každej stránke a ďalšie spracovanie ju rešpektuje (vyťažovanie textov a dát podľa čl. 4 smernice (EÚ) 2019/790). |
| **Roboty umelej inteligencie v robots.txt** | Ak skupina v robots.txt zakazuje stránku niektorému robotovi, ktorý zbiera pre trénovanie umelej inteligencie alebo vyťažovanie (`MINING_AGENTS`: GPTBot, CCBot, ClaudeBot, Google-Extended a ďalšie), platí to ako výhrada vyťažovania. ATENA sa riadi len svojou skupinou alebo skupinou `*`, takže stránku stiahnuť smie, ale uloží z nej len metadáta. Neskorší údaj TDMRep túto výhradu nezruší. |
| `X-Robots-Tag` / `<meta name="robots">` | `noindex`, `nofollow`, `noarchive` (aj `nocache`), `nosnippet`, `notranslate`, `none`; pokyny pre iné zberače (`otherbot: noindex`) sa ignorujú. |

Príklady:

```text
# ATENA nesmie nič
User-agent: ATENA
Disallow: /

# pomalšie a bez jedného priečinka
User-agent: ATENA
Crawl-delay: 30
Disallow: /interne/
```

## Osobné údaje a výmaz
ATENA pred uložením odstráni z textov e-mailové adresy a telefónne čísla a neukladá žiadne pôvodné
súbory. Nesleduje fotogalérie, zoznamy zamestnancov ani kontakty. Na požiadanie vymaže stránku alebo
celú sekciu a tá sa už potom nikdy nestiahne. Žiadosti: [GitHub issues](https://github.com/tarzanislav/atena/issues)
alebo kontakt prevádzkovateľa. Podrobnosti: [PRAVA-A-OCHRANA-UDAJOV.md](PRAVA-A-OCHRANA-UDAJOV.md).

## Čo zberač sťahuje
Webové stránky, dokumenty PDF a kancelárske súbory, kanály RSS/Atom a sitemapy (XML, text, gzip).
Z jedného zdroja najviac 50 MiB po rozbalení a len typy obsahu, ktoré vie spracovať. Neposiela cookies,
`Referer` ani prihlasovacie údaje a nespúšťa JavaScript.

## Kam až zberač ide
Zo vstupnej stránky zdroja ide ATENA len po odkazoch na webe vydavateľa: na prílohy (PDF, kancelárske
súbory) a na stránky pod vstupnou stránkou, o jednu úroveň. Úradné zdroje (ministerstvá, štátne
inštitúcie) sa smú zbierať v celom webe: každá stránka toho istého servera najviac tri odkazy od
vstupnej stránky. Na iné weby, do galérií, zoznamov zamestnancov ani na kontakty ATENA nikdy nejde
a žiadny zdroj nemá viac ako 100 000 adries.

## Čo zaručuje prevádzkovateľovi
* **Len verejný internet.** Každá adresa – aj každé presmerovanie a každý odkaz z kanála či sitemapy – sa
  pred spojením overí: len `http`/`https`, žiadne meno ani heslo v adrese, len povolené porty
  (80, 443, 8080, 8443), žiadne jednoslovné ani vyhradené mená (`localhost`, `*.local`, `*.internal`,
  `*.home.arpa`…). Odpovede DNS sa filtrujú a spojenie sa nadviaže len na verejnú adresu. Preto sa zberač
  nikdy nespojí s menom, ktoré vedie na `127.0.0.1`, `10.x`, `169.254.169.254` (údaje cloudového servera)
  alebo súkromné rozsahy IPv6. Platí to aj pri podvrhnutí odpovede DNS (DNS rebinding). IPv6 len
  v rozsahu `2000::/3`. Vnútornú sieť možno povoliť len výslovne (`NetPolicy::allowed_ranges`).
* **Žiadne proxy.** Systémové proxy sa nepoužívajú, lebo proxy by si adresy prekladala sama a obišla by
  kontrolu.
* **Ohraničená práca.** Časové limity (10 s na spojenie, 60 s na požiadavku), 5 presmerovaní, limity
  veľkosti kontrolované priebežne, robots.txt do 500 KiB, sitemapa do 50 000 položiek / 50 MiB, kanál do
  10 000 položiek. Entity definované v DTD sa nikdy nerozbaľujú.
* **Šifrovanie** cez rustls s úložiskom certifikátov operačného systému.
* **Nebezpečný obsah zostane vonku** (`atena_core::safety`). Pred uložením:
  * sa z každého textu odstránia riadiace znaky, znaky obracajúce smer textu a neviditeľné znaky
    (mohli by pokaziť databázu, ovládať terminál alebo zmeniť, ako sa text číta);
  * sa ponechajú alebo sledujú len obyčajné adresy `http(s)` s najviac 2 000 znakmi – nikdy
    `javascript:`, `data:` ani `file:`, nikdy adresa s menom a heslom;
  * sa zahodia dátumy pred rokom 1900 alebo v budúcnosti;
  * sa odmietnu binárne dáta vydávané za text.
* **Programy a aktívny obsah sa odmietnu** (`atena-extract/src/danger.rs`): programy a skripty, nech ich
  server nazýva akokoľvek; súbory, ktoré sa len tvária ako PDF; PDF s JavaScriptom, spúšťacími akciami,
  vloženými súbormi, multimédiami alebo formulármi XFA – aj zapísanými priamo vo vnorenom slovníku
  alebo poli; dokumenty Word s makrami, ActiveX, objektmi OLE
  alebo vzdialenými šablónami; dokumenty OpenDocument s makrami alebo skriptmi. Z takého súboru sa nič
  neuloží a jeho adresa sa už neskúša. Pôvodné súbory sa neukladajú vôbec ([DATABAZA.md](DATABAZA.md)).
* **Spracovanie v samostatnom, obmedzenom procese** (`atena_extract::Parser`). Stránky a dokumenty
  spracúva podriadený proces, ktorý smie použiť najviac 768 MB pamäte a 60 sekúnd. Zákerný súbor
  (dekompresná bomba, pasca na parser) tak môže ukončiť len tento proces. Taký dokument sa považuje za
  príliš náročný (`too_demanding`): nič z neho sa neuloží a jeho adresa sa už neskúša. Porovnanie verzií
  má limit 2 sekundy a odstránenie kontaktných údajov prečíta text len pevne daný počet ráz.
* **Každý zdroj zostane na svojom webe.** Weby sa rozlišujú podľa zoznamu verejných domén (public
  suffix list): `static.minedu.sk` patrí k `www.minedu.sk`, ale dva weby pod `gov.uk` sú dvaja rôzni
  vydavatelia. Obsah, ktorý po presmerovaní ponúkne iný web, sa pod menom zdroja neuloží
  (`moved_elsewhere`). Kanály a sitemapy pridajú len adresy na vlastnom webe. Stránky o ľuďoch sa
  vynechajú, nech sa nájdu kdekoľvek. Vymazaná adresa sa nevráti ani cez presmerovanie (`erased`).
  Presmerovanie z `https` na nešifrované `http` sa odmietne.
* **Z chyby sa nikdy nestane slučka.** Keď databáza výsledok uložiť nevie, kontrola sa počíta za
  neúspešnú a adresa počká aspoň hodinu, ako po každej inej chybe.

Rast je ohraničený: jeden zdroj môže mať najviac 100 000 adries a stav serverov v pamäti sa nad
10 000 serverov prečisťuje. Obmedzenia tejto verzie: kanály v inom kódovaní ako UTF-8 sa čítajú s náhradnými znakmi. Pravidlá z HTML `<meta>` a
TDMRep v HTML a PDF číta modul `atena-extract` a pri spracovaní sa spoja s pravidlami z hlavičiek HTTP;
EPUB sa zatiaľ nečíta.
