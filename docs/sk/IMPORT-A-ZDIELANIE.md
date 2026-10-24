# Import zdrojov a zdieľanie návrhov

*English: [../IMPORT-AND-SHARING.md](../IMPORT-AND-SHARING.md)*

Používatelia ktorejkoľvek inštalácie ATENY môžu navrhovať zdroje, jeden na riadok. Môže ísť o
webové stránky a weby, kanály a mapy stránok, dokumenty, rozhrania OAI-PMH a DOI. Každý návrh prejde
zásadou zákonnosti ([ZASADA-ZAKONNOSTI.md](ZASADA-ZAKONNOSTI.md)) rovnako ako samotný register.
Návrhy všetkých inštalácií sa môžu zbehnúť na jednom mieste, v spoločnom registri v repozitári
ATENA, a to bez vlastného servera.

## V jednej inštalácii
```
atena-ingest import navrhy.txt [--country SK]   # vyhodnotiť a uložiť ako návrhy
atena-ingest imports                            # návrhy s automatickým vyhodnotením
atena-ingest confirm U-1a2b3c4d5e --by "Meno" --reason "podmienky prečítané 24. 10. 2026"
atena-ingest reject  U-1a2b3c4d5e --by "Meno" --reason "nejde o vzdelávanie"
```
1. **Vyhodnotenie.** Každý riadok sa prečíta tak, ako opisuje [HODNOTENIE.md](HODNOTENIE.md):
   robots.txt, TDMRep, zákaz robotov umelej inteligencie, licencia a podmienky používania. Výsledkom
   je návrh licencie, podmienok a záznamu o overení s citátmi. Stránky o ľuďoch sa neukladajú vôbec.
2. **Návrh.** Zdroj sa uloží ako `proposed`, teda vypnutý, `copyrighted` a `unknown`. Bez ohľadu na
   to, čo návrh hovorí, sa nič nezbiera ani neuchováva.
3. **Rozhodnutie.** Človek návrh s odôvodnením potvrdí alebo opraví licenciu, podmienky či krajinu.
   Program rozhodnutie skontroluje:
   - krajina musí mať právny profil,
   - hodnoty musia byť platné,
   - `official` je dovolené len tam, kde zákon úradné texty uvoľňuje.

   Potom rozhodne `legality::permission`: či sa zdroj zbiera, či sa uchovajú texty alebo len
   metadáta a či sa smie zobraziť celý text. Zamietnutie odstráni všetko, čo sa zozbieralo.
4. **Neskorší import rozhodnutie nikdy nezmení.** Uloží sa len ako ďalšie vyhodnotenie.

Žiadosť o výmaz (`forget`) sa vzťahuje aj na návrhy: vyhodnotenia sa vymažú a adresa sa odstráni a
zablokuje.

## Medzi inštaláciami: spoločná databáza
```
inštalácia ──(len adresy)──> sprostredkovateľ: téma na ntfy.sh ──(každých 30 min)──> issues na GitHube
                               (drží 12 hodín)                 workflow Actions     (štítok navrh-zdrojov)
                                      │                                                    │
                                      └────────────────> inštalácia redaktora <────────────┘
                                          atena-ingest proposals: každú adresu sama znova vyhodnotí,
                                          uloží ako návrh, redaktor rozhodne
                                                     │
                 atena-ingest publish → data/sources.yaml → pull request → spoločný register
                                                     │
                 každá inštalácia: atena-ingest sync --shared
```
- **Čo sa posiela:** riadky, ktoré používateľ importoval, teda adresy, DOI a kódy krajín.
  - Neposielajú sa vyhodnotenia ani to, kto ich poslal.
  - Neposielajú sa stránky o ľuďoch ani adresy, ktoré register už obsahuje.
  - Zdieľanie je predvolene zapnuté a vypína ho `ATENA_SHARE=0`.
- **Sprostredkovateľ:** téma na verejnom serveri [ntfy.sh](https://docs.ntfy.sh/faq/). Je bezplatný a
  bez účtov a správy po 12 hodinách zmaže. Jedna správa má najviac 4 096 bajtov, dlhšie zoznamy sa
  rozdelia. Cez `ATENA_RELAY` sa dá nastaviť iný server, napríklad server ntfy v EÚ. Téma je
  verejná, lebo zoznamy nie sú tajné.
- **Uchovanie:** workflow `.github/workflows/proposals.yml` každých 30 minút skopíruje nové zoznamy do
  issues, najviac 50 naraz. Obsah je v ohraničenom bloku, takže z neho GitHub nič nevykreslí.
- **Redaktor:** príkaz `atena-ingest proposals`
  - prečíta sprostredkovateľa aj otvorené issues a preskočí zoznamy, ktoré už prijal,
  - **každú adresu znova sám vyhodnotí**, lebo prijaté vyhodnotenie by nič nedokazovalo,
  - uloží ich ako návrhy `via shared`.

  Voľba `--close` spracované issues zavrie (s `ATENA_GITHUB_TOKEN`). Po `confirm` príkaz
  `atena-ingest publish U-… --id SK25 --authority C1` pridá záznam aj so záznamom o overení do
  `data/sources.yaml`. Pull request a jeho CI register skontrolujú ako obvykle.
- **Všetci:** `atena-ingest sync --shared` načíta spoločný register z repozitára. Importy, ktoré
  register medzičasom obsahuje, prevezme (`merged`).

## Ochrana osobných údajov
Zoznamy obsahujú adresy vzdelávacích stránok a kódy krajín. Sprostredkovateľ (ntfy.sh, prevádzkovaný
v USA) vidí IP adresu odosielateľa, ako každá webová služba, a správy drží 12 hodín. Issues sú verejné.
Nič ďalšie o používateľovi sa neposiela. Prevádzkovateľ inštalácie zdieľanie uvedie v informáciách o
spracúvaní osobných údajov a môže ho vypnúť.
