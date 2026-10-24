# Zásada zákonnosti

*English: [../LEGALITY.md](../LEGALITY.md)*

> **ATENA čerpá iba z otvorených zdrojov. Nič z toho, čo robí, nesmie porušiť zákon, zneužiť údaje
> ani porušiť ochranu osobných údajov – na Slovensku, v Európskej únii ani v žiadnej inej krajine:
> v Amerike, Číne, Rusku ani nikde inde. Platí právo miesta, kde ATENA beží, aj právo krajiny, z ktorej
> zdroj pochádza, a vždy sa uplatní to prísnejšie. Kde nie je isté, že niečo je dovolené, ATENA to
> nerobí.**

Túto zásadu určil autor projektu. Je nadradená všetkým ostatným pravidlám programu a je zapísaná
priamo v jadre: v module `crates/atena-core/src/legality.rs`. Každé rozhodnutie o tom, čo sa smie
stiahnuť, uložiť a zobraziť, ide cez funkciu `permission`. Iná cesta v programe neexistuje.

## Ako zásada funguje
1. **Každá krajina má právny profil.** Zdroj z krajiny bez profilu nemožno zapísať do registra.
   Profil určuje, či tamojšie právo dovoľuje vyťažovať texty a dáta (TDM), či sú úradné texty voľné
   a aké prísne je tam právo na ochranu osobných údajov.
2. **Platia dve práva naraz.** Právo Slovenska a EÚ, kde ATENA beží, aj právo krajiny zdroja.
   Povolené je len to, čo dovoľujú obe.
3. **Neisté je zakázané.** Chránený obsah smie ATENA uchovávať len vtedy, keď:
   - podmienky webu boli overené (`terms: allowed`),
   - **obe** práva majú výnimku na vyťažovanie textov a dát, ktorá pokrýva účel ATENY,
   - vydavateľ si vyťažovanie nevyhradil.
4. **Zobraziť celý text** sa smie len pri úradných textoch, ktoré sú voľné podľa oboch práv, a pri
   otvorených licenciách. Rovnako to platí pre preklad celého textu a pre ukladanie pôvodných súborov.
5. **Prísne právo o osobných údajoch** (Čína – PIPL, Rusko – 152-FZ, Južná Kórea – PIPA): z týchto
   krajín sa ukladajú len metadáta, a to ani pri úradných a otvorených textoch nie.
6. **Spätná platnosť:** pri každom načítaní registra (`atena-ingest sync`) sa odstráni všetko, čo
   zdroj už mať nesmie. Texty zdroja sa zredukujú na metadáta; ak sa zdroj nesmie zbierať vôbec,
   odstráni sa celý.
7. **Testy:** pre každú krajinu a každú kombináciu licencie, podmienok a výhrady testy overujú, že sa
   pravidlá nikdy neporušia. Každá krajina v taxonómii musí mať profil.

## Právne profily krajín
| Krajina | Vyťažovanie chráneného obsahu | Úradné texty voľné | Osobné údaje | Výsledok pre chránený obsah |
|---|---|---|---|---|
| SK, CZ, PL, DE, AT, HU, FI, EE, NL, FR, SE, IT, EÚ | áno (čl. 4 smernice 2019/790, s výhradou vydavateľa) | áno | GDPR | text len pri overených podmienkach |
| IE | áno | **nie** (štátna autorská ochrana) | GDPR | text len pri overených podmienkach; úradné texty len s otvorenou licenciou |
| WORLD (medzinárodné organizácie) | podľa práva EÚ | **nie** | GDPR | text len pri overených podmienkach; dokumenty len s otvorenou licenciou |
| GB (Anglicko, Škótsko) | **len nekomerčný výskum** (CDPA s. 29A) | **nie** (Crown copyright, treba OGL) | UK GDPR | len metadáta, okrem otvorenej licencie |
| US | **nie** (fair use sa posudzuje prípad od prípadu) | len federálne diela (17 U.S.C. § 105) | štátne zákony | len metadáta, okrem úradných federálnych a otvorených |
| CA, AU | **nie** | **nie** (Crown copyright) | PIPEDA / Privacy Act | len metadáta, okrem otvorenej licencie |
| NZ | **nie** | áno (s. 27) | Privacy Act 2020 | len metadáta, okrem úradných a otvorených |
| JP | áno (čl. 30-4) | áno (čl. 13) | APPI | text len pri overených podmienkach |
| SG | áno (s. 244) | **nie** | PDPA | text len pri overených podmienkach |
| KR | **nie** | áno (čl. 7) | PIPA – považované za prísne | **vždy len metadáta** |
| CN | **nie** | áno (čl. 5) | PIPL – prísne | **vždy len metadáta** |
| RU | **nie** | áno (čl. 1259) | 152-FZ – spracúvanie len v Rusku | **vždy len metadáta** |

Profily sú opatrným zhrnutím práva, nie právnou radou. Profil smie ktokoľvek **sprísniť**. Zmierniť
ho možno len po posúdení právnikom, ktoré sa zapíše do pull requestu.

## Čo z toho dnes vyplýva
Program to kedykoľvek ukáže príkazom `atena-ingest legality`.
- Zo 186 zdrojov sa smie zbierať 185; YouTube nie, lebo jeho podmienky to zakazujú.
- Texty si smie ponechať a zobraziť zatiaľ len 5 zbierok predpisov.
- Pri ostatných zdrojoch ATENA zatiaľ ukladá len metadáta: názov, adresu, dátumy a zaradenie podľa
  názvu. Zmenu stránky zistí aj tak, cez odtlačok textu, ale neuchová jej obsah.
- Overenie licencií a podmienok jednotlivých zdrojov (`terms: allowed`, otvorené licencie, federálne
  úradné diela) otvorí uchovávanie textov tam, kde to obe práva dovoľujú. Americký, britský, ruský a
  čínsky chránený obsah zostane len v metadátach aj potom.
