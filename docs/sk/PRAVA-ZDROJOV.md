# Overenie práv zdrojov

*English: [../SOURCE-RIGHTS.md](../SOURCE-RIGHTS.md)*

Každý zdroj v `data/sources.yaml` má tri údaje:
- licenciu (`licence`),
- podmienky (`terms`),
- záznam o overení (`rights`): dátum overenia, stránku, ktorá sa čítala, a čo na nej je. Stránkou sú
  podmienky používania, právne upozornenie alebo stránka s licenciou. Pri úradných textoch je to zákon,
  podľa ktorého nie sú chránené.

Ak má zdroj inú licenciu alebo iné podmienky než predvolené, musí mať záznam o overení, inak ho
register odmietne. Prvé overenie zahŕňalo všetkých 186 zdrojov a urobilo sa **24. 10. 2026**. Nie je to
právna rada. Overenie treba opakovať aspoň raz ročne a vždy, keď vydavateľ zmení podmienky.

## Ako sa zo zistenia stane licencia a podmienky
| Čo vydavateľ uvádza | `licence` | `terms` | Čo robí ATENA |
|---|---|---|---|
| Otvorenú licenciu na vlastný obsah webu (CC BY, CC BY-SA, CC0, OGL, Licence Ouverte) | daná licencia | `allowed` | text uloží a zobrazí celý s uvedením autora |
| Zákony, úradné dokumenty, federálne diela USA | `official` | `allowed` | text uloží a zobrazí celý |
| Žiadne podmienky alebo len „všetky práva vyhradené“ | `copyrighted` | `allowed` | text uloží na zistenie zmien a zaradenie, ak vyťažovanie dovoľujú obe práva; celý ho nikdy nezobrazí |
| Povoľuje opakované použitie, aj keď len nekomerčne | `copyrighted` | `allowed` | ako vyššie |
| Zakazuje roboty, zber, prehľadávanie či indexovanie alebo povoľuje prístup len na osobné použitie | `copyrighted` | `forbidden` | vôbec ho nezbiera |
| Vyhradzuje si vyťažovanie textov a dát alebo použitie pre umelú inteligenciu | `copyrighted` | `reserved` | uloží len metadáta |
| Podmienky sa nedali prečítať (robots.txt zakázal náš nástroj) alebo si odporujú | `copyrighted` | `unknown` | uloží len metadáta |

Pravidlá za tabuľkou:
- **Mlčanie a bežné „všetky práva vyhradené“ nie sú výhradou.** Podľa čl. 4 ods. 3 smernice (EÚ)
  2019/790 môže nositeľ práv vyťažovanie vylúčiť len výslovnou výhradou. Všeobecné „všetky práva
  vyhradené“ takou výhradou nie je. Autor projektu 24. 10. 2026 rozhodol, že oba prípady sa považujú za
  `allowed`. Strojovo čitateľné výhrady však majú vždy prednosť: TDMRep aj zákaz robotov umelej
  inteligencie v robots.txt.
- **Nekomerčné povolenie nerobí obsah otvoreným.** ATENA musí byť použiteľná aj komerčne. Za otvorené
  sa preto považujú len licencie zo zoznamu `OPEN_LICENCES`. Licencie CC BY-NC a podobné zostávajú
  `copyrighted`.
- **Repozitáre a zmiešané weby zostávajú `copyrighted`**, aj keď sú ich vlastné stránky otvorené.
  Uchovávajú totiž diela iných autorov pod inými licenciami. Ide o Zenodo, Scientix, Európsku platformu
  pre školské vzdelávanie, Better Internet for Kids a Oak National Academy.
- **Podmienky, ktoré obmedzujú samotný prístup, sú zákazom.** Napríklad „len na vaše osobné
  použitie“, „obsah nesmiete zaradiť do žiadnej databázy“ alebo „webové služby len na nekomerčné
  použitie“. Takéto zdroje dostanú `forbidden`, lebo čo nie je isté, to sa nerobí.

## Výsledok (24. 10. 2026)
| | Zdroje |
|---|---|
| `allowed` | 141 |
| `forbidden` – nezbiera sa | 28 |
| `reserved` – len metadáta | 1 (NRICH) |
| `unknown` – len metadáta | 16 |
| zobrazí sa celý text (úradný alebo otvorený) | 22 |

**Nezbiera sa**, lebo podmienky zakazujú automatický prístup:
- UNESCO (GL01 – GL04, GL13), YouTube (N39), British Council TeachingEnglish (UK05);
- Reading Rockets (US06, US07), Edutopia (US08), PBS LearningMedia (US09), IRIS Center (US10), CAST UDL
  Guidelines (US11), CASEL (US13), Common Sense Education (US14), ISTE+ASCD (US19);
- GeoGebra (TO02), Scratch Foundation (TO03), iCivics (TO06), OER Commons (TO07);
- American Montessori Society (AL02), HighScope (AL10), The Philosophy Foundation (AL17), Cult of
  Pedagogy (AL24);
- klicksafe (N11), Evidence for ESSA (N22), CommonLit (N24), CORE (N33).

**Zostáva overiť**, lebo robots.txt odmietol náš nástroj na čítanie alebo si podmienky odporujú:
- Deutscher Bildungsserver a Elixier (DE01, DE02), Fachportal Pädagogik (DE04);
- Oktatási Hivatal a Nemzeti Köznevelési Portál (HU01 – HU03), edu.gov.ru (RU01, RU02);
- waldorf.sk (AL04), Reggio Children (AL09);
- CKE (N07), Eduthek (N13), NCCA (N20), HundrED (N21), e-Koolikott (N30), DOAJ (N32).

Ich stránky sa smú sťahovať, ak to robots.txt ATENE dovolí, ale ukladajú sa z nich len metadáta.

Odkazy na videá ATENA berie z vlastných stránok inštitúcií a na YouTube nechodí. Rozhodol o tom autor
projektu 24. 10. 2026.
- Odkazy aj vložené prehrávače YouTube a Vimeo sa zapíšu ako jedna stála adresa na každé video
  (`document_link.kind = 'video'`).
- Zapisujú sa len pri zdrojoch, ktorých text si ATENA smie ponechať, lebo aj zisťovať, čo stránka
  obsahuje, je vyťažovanie.
- Zberač tieto adresy nikdy neotvára ani po nich ďalej nesleduje odkazy.
