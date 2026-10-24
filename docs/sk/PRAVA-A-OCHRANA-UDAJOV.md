# Zákonnosť od návrhu: práva k obsahu a osobné údaje

*English: [../PRIVACY-AND-RIGHTS.md](../PRIVACY-AND-RIGHTS.md)*

**Pravidlo projektu (určil autor projektu):** ATENA čerpá iba z otvorených zdrojov. Nič z toho, čo
robí, nesmie porušovať zákon, zneužívať údaje ani byť v rozpore s GDPR. Táto stránka opisuje, ako to
zabezpečuje program. Nie je to právna rada. Pred verejným spustením treba riešenie dať posúdiť
právnikovi a urobiť posúdenie vplyvu na ochranu údajov (DPIA, čl. 35 GDPR).

## 1. Z ktorých zdrojov
* Len obsah **verejne dostupný bez prihlásenia**. ATENA sa nikdy neprihlasuje, neobchádza platený
  prístup, overenie CAPTCHA ani iné zábrany a nespúšťa JavaScript.
* Dodržiava **robots.txt** (RFC 9309). Keď sa nedá stiahnuť, nestiahne sa nič.
* Dodržiava **výhradu vyťažovania textov a dát** (TDMRep, čl. 4 smernice (EÚ) 2019/790). Pri výhrade
  sa uložia len metadáta: názov, adresa, dátumy a odtlačky.
* Web, ktorý v **robots.txt zakazuje roboty zbierajúce pre umelú inteligenciu** (GPTBot, CCBot,
  ClaudeBot, Google-Extended a ďalšie), si tým vyhradzuje aj vyťažovanie. ATENA jeho stránky smie
  stiahnuť, ale uloží z nich len metadáta.
* Každý zdroj má v registri **podmienky používania** (`terms`):
  - `allowed`: podmienky sme prečítali a automatický prístup nezakazujú ani si nevyhradzujú vyťažovanie;
  - `forbidden`: automatický prístup zakazujú, takže sa zdroj vôbec nezbiera (napríklad YouTube
    alebo UNESCO);
  - `reserved`: vyťažovanie si vyhradzujú, takže sa ukladajú len metadáta;
  - `unknown`: podmienky nie sú overené, takže sa ukladajú len metadáta.

  Každá hodnota okrem `unknown` má v registri zapísané overenie ([PRAVA-ZDROJOV.md](PRAVA-ZDROJOV.md)).

## 2. Čo sa smie zobraziť
Každý zdroj má v registri **licenciu** (`licence`):

| Licencia | Príklady | Čo uvidí používateľ |
|---|---|---|
| `official` | zákony, vyhlášky a úradné dokumenty v zbierkach predpisov (Slov-Lex, e-Sbírka, RIS, Nemzeti Jogszabálytár, EUR-Lex). Nie sú chránené autorským právom (napr. § 5 autorského zákona) | celý text |
| otvorená licencia (`CC0-1.0`, `CC-BY-4.0`, `CC-BY-SA-4.0`, OGL, `etalab-2.0` a ďalšie) | otvorené vzdelávacie zdroje, otvorené dáta | celý text s uvedením autora podľa licencie |
| `copyrighted` (**predvolené**) | všetko, čo nie je overené | názov, údaje o dokumente, zaradenie podľa ATENY a odkaz na originál, **bez celého textu** |

O tom, či si ATENA smie ponechať text chráneného obsahu (na zistenie zmien a zaradenie), rozhoduje
zásada zákonnosti ([ZASADA-ZAKONNOSTI.md](ZASADA-ZAKONNOSTI.md)). Smie len vtedy, keď sú podmienky
webu overené, keď výnimku na vyťažovanie textov a dát pre účel ATENY má právo prevádzkovateľa aj právo
krajiny zdroja a keď si vydavateľ vyťažovanie nevyhradil. Inak sa ukladajú len metadáta. Kým licencia
a podmienky zdroja nie sú overené, zdroj sa považuje za chránený (`copyrighted`) s neoverenými
podmienkami (`unknown`).

## 3. Osobné údaje (GDPR)
Stránky škôl a úradov uvádzajú mená učiteľov, riaditeľov a úradníkov aj ich kontakty. ATENA zbiera
informácie o vzdelávaní, nie o ľuďoch, preto si osobných údajov necháva čo najmenej:

| Opatrenie | Kde v programe |
|---|---|
| **E-mailové adresy a telefónne čísla sa odstránia** z textov, názvov, popisov a textov odkazov ešte pred uložením, aj v zakrytom tvare ako `meno(at)skola.sk` | `atena-ingest/src/privacy.rs` |
| **Pôvodné webové stránky sa neukladajú**, len upravený text. Pôvodné súbory sa ukladajú len pri PDF a kancelárskych dokumentoch, ktorých text sa smie zobraziť celý (úradné texty, otvorené licencie) | `legality::permission` |
| **Stránky o ľuďoch sa nesledujú**: fotogalérie, zoznamy zamestnancov, kontakty, profily, prihlasovanie | `urls::is_personal_page` |
| **Autor webovej stránky sa neukladá** (býva to správca webu); autor dokumentu áno | spracovanie |
| **Výmaz na žiadosť** (čl. 17). Príkaz `atena-ingest forget ADRESA --reason …` odstráni dokumenty, verzie, odkazy, záznamy o kontrolách a súbory, ktoré nepoužíva nič iné. Adresu (alebo celú sekciu) navždy zablokuje | `Store::forget`, `blocked_url`, `erasure_log` |
| **Obmedzenie uchovávania**: záznamy o kontrolách sa po 90 dňoch zmažú | zberač |
| Žiadne obrázky, žiadne cookies, žiadne sledovanie návštevníkov webov | zberač |

Mená v samotnom texte (napr. „riaditeľka školy Mgr. J. Nováková“) sa automaticky neodstraňujú, lebo
sú súčasťou zverejnenej úradnej informácie. Kto nesúhlasí, môže dať dokument alebo celú sekciu
vymazať príkazom `forget`.

**Právny základ** musí potvrdiť prevádzkovateľ. Ide o oprávnený záujem (čl. 6 ods. 1 písm. f) na
poskytovaní overených informácií o vzdelávaní. Test proporcionality treba zapísať v DPIA.
Prevádzkovateľ zverejní informácie o spracúvaní (čl. 14) s kontaktom na námietky a žiadosti o výmaz.

## 4. Kontrolný zoznam pred verejným spustením
1. Udržiavať overenie práv zdrojov v `data/sources.yaml` aktuálne. Pri všetkých zdrojoch sa urobilo
   24. 10. 2026; opakovať ho aspoň raz za rok a pred spustením v novej krajine.
2. Urobiť DPIA, dať riešenie posúdiť právnikovi a viesť záznamy o spracovateľských činnostiach (čl. 30).
3. Zverejniť informácie o spracúvaní a kontakt na žiadosti o výmaz; žiadosti vybavovať príkazom `forget`.
4. Rozhranie zobrazí celý text len pri úradných textoch a otvorených licenciách a dodrží `noindex`,
   `noarchive`, `nosnippet` a `notranslate`.
