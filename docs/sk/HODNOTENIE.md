# Automatické právne vyhodnotenie

*English: [../ASSESSMENT.md](../ASSESSMENT.md)*

Modul `crates/atena-assess` urobí pri akejkoľvek adrese to, čo sa pri registri robilo ručne
([PRAVA-ZDROJOV.md](PRAVA-ZDROJOV.md)). Prečíta stránku, jej licenciu a podmienky používania a navrhne
licenciu (`licence`), podmienky (`terms`) a záznam o overení (`rights`) s citátmi. Používa ho príkaz
`atena-ingest assess` a import návrhov od používateľov.

## Vstup
Na každom riadku je jedna položka. Za ňou môže nasledovať kód krajiny (`SK`, `GB-ENG`, `WORLD` a pod.).
Prázdne riadky a riadky začínajúce znakom `#` sa preskočia. Naraz sa spracuje najviac 500 položiek.

| Položka | Príklady |
|---|---|
| webová stránka alebo web | `https://www.minedu.sk/`, `www.npi.cz/aktuality` |
| kanál RSS alebo mapa stránok | `https://eduscol.education.gouv.fr/feed`, `https://example.org/sitemap.xml` |
| dokument | `https://example.org/osnovy.pdf` |
| rozhranie OAI-PMH | `https://repository.example.org/oai?verb=Identify` |
| DOI | `10.1002/cl2.1180`, `doi:10.1002/cl2.1180`, `https://doi.org/10.1002/cl2.1180` |

Adresy s menom alebo heslom a iné schémy než `http(s)` program odmietne. Každé stiahnutie ide cez
zberač, takže aj pri vyhodnotení platí robots.txt, šetrné tempo, limity veľkosti a zákaz súkromných
adries.

## Čo sa číta
1. **Samotná položka:** jej druh (stránka, kanál, mapa stránok, dokument, OAI-PMH, DOI), názov,
   výhrada TDMRep a to, či robots.txt zakazuje roboty umelej inteligencie. Pri DOI záznam v Crossrefe
   (licencie vydanej verzie) a potom stránka vydavateľa.
2. **Stránka, ktorá hovorí za web.** Pri webových stránkach je to položka sama, pri kanáloch, mapách
   stránok, dokumentoch a rozhraniach domovská stránka webu. Číta sa:
   - odkazy `rel="license"`,
   - odznaky licencií,
   - päta stránky („©“, „všetky práva vyhradené“, poznámky o licencii).

   Zvyšok bežnej stránky sa ako právny text nečíta, lebo články pokojne môžu písať o robotoch či
   umelej inteligencii.
3. **Najviac tri stránky s podmienkami**, na ktoré stránka odkazuje. Hľadajú sa v 16 jazykoch
   (podmienky, právne upozornenie, autorské práva, Impressum, mentions légales a pod.). Stránky o
   súkromí, cookies a prístupnosti sa preskočia. Ak vnútorná stránka žiadne neodkazuje, skúsia sa
   odkazy domovskej stránky.

Frázy nie sú v kóde, ale v súbore `data/legal-phrases.yaml`. Zistenie si vyžaduje predmet aj znak v
tej istej vete, napríklad „robot“ + „nesmie“ alebo „vyťažovanie textov a dát“ + „vyhradzuje“.

## Návrh – vždy najprísnejší, aký zistenia vyžadujú
| Zistenie | `terms` | `licence` |
|---|---|---|
| položka sa nedala prečítať | `unknown` | `copyrighted` |
| stránka o ľuďoch (galérie, zamestnanci, kontakty) | `forbidden` | `copyrighted` |
| podmienky zakazujú roboty, zber, prehľadávanie či indexovanie alebo dovoľujú prístup len na osobné použitie | `forbidden` | `copyrighted` |
| podmienky, TDMRep alebo robots.txt si vyhradzujú vyťažovanie alebo použitie pre umelú inteligenciu | `reserved` | `copyrighted` |
| podmienky sa našli, ale žiadne sa nedali prečítať | `unknown` | `copyrighted` |
| podmienky sa prečítali alebo sa žiadne nenašli a nič z uvedeného neplatí | `allowed` | otvorená licencia, ak sa našla len jedna prijatá, inak `copyrighted` |

`official` program nikdy nenavrhne. Či ide o úradné dielo, posúdi človek.

Krajina sa berie od používateľa alebo z názvu webu (`.sk`, `europa.eu`, `.gov` a pod.). Všeobecné
domény (`.org`, `.com`) o krajine nič nehovoria. Bez krajiny zásada zákonnosti nedovolí nič.

**Návrh sa nikdy neuplatní sám.** Stroj sa môže pri čítaní podmienok pomýliť. Importovaný zdroj sa
preto nezačne zbierať, kým jeho návrh nepotvrdí (alebo neopraví) človek.
