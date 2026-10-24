# Import zdrojov a zdieľanie návrhov

*English: [../IMPORT-AND-SHARING.md](../IMPORT-AND-SHARING.md)*

Používatelia ktorejkoľvek inštalácie ATENY môžu navrhovať zdroje, jeden na riadok. Môže ísť o
webové stránky a weby, kanály a mapy stránok, dokumenty, rozhrania OAI-PMH a DOI. Každý návrh prejde
zásadou zákonnosti ([ZASADA-ZAKONNOSTI.md](ZASADA-ZAKONNOSTI.md)) rovnako ako samotný register.
Návrhy všetkých inštalácií sa môžu zbehnúť na jednom mieste, v spoločnom registri v repozitári
ATENA, a to bez akejkoľvek tretej strany.

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
ATENA nikomu neposiela nič o svojich používateľoch a nepoužíva služby tretích strán
(`atena_core::legality::CONNECTIONS`, [ZASADA-ZAKONNOSTI.md](ZASADA-ZAKONNOSTI.md)). Návrhy preto
putujú ako súbory, ktoré odovzdávajú sami ľudia:
```
inštalácia: atena-ingest import …          adresy sa odložia na odoslanie
            atena-ingest share navrhy.txt  … a zapíšu sa do súboru so zoznamom
                     │
                     │  používateľ odovzdá súbor redaktorovi, ako sám uzná za vhodné
                     ▼
redaktor:   atena-ingest proposals navrhy.txt   každú adresu sám znova vyhodnotí, uloží ako návrh
            atena-ingest confirm … / reject …   rozhodne
            atena-ingest publish U-… --id SK25 --authority C1
                                                 → data/sources.yaml → pull request → spoločný register
                     │
každá inštalácia:    atena-ingest sync --shared  (len na tento výslovný príkaz)
```
- **Čo obsahuje zoznam:** riadky, ktoré používateľ importoval, teda adresy, DOI a kódy krajín.
  - Neobsahuje vyhodnotenia ani nič o tom, kto adresy importoval.
  - Stránky o ľuďoch a adresy, ktoré register už obsahuje, sa vynechajú.
  - `ATENA_SHARE=0` alebo `import --no-share` nechá importy len v danej inštalácii.
- **Súbor so zoznamom** začína riadkom `ATENA-NAVRHY 1`, za ním je na každom riadku jedna adresa.
  Je to obyčajný text, ktorý si každý môže pred odovzdaním prečítať.
- **Redaktor:** príkaz `atena-ingest proposals SÚBOR…`
  - prečíta súbory so zoznamami a preskočí tie, ktoré už prečítal,
  - **každú adresu znova sám vyhodnotí**, lebo prijaté vyhodnotenie by nič nedokazovalo,
  - uloží ich ako návrhy `via shared`.

  Po `confirm` príkaz `atena-ingest publish` pridá záznam aj so záznamom o overení do
  `data/sources.yaml`. Pull request a jeho CI register skontrolujú ako obvykle.
- **Všetci:** `atena-ingest sync --shared` stiahne spoločný register z repozitára, a to len vtedy, keď
  to používateľ výslovne zadá, a len cez šifrované `https`. Importy, ktoré register medzičasom obsahuje, prevezme (`merged`).
- **Neskôr:** keď bude mať prevádzkovateľ vlastný server v EÚ, ktorý IP adresy neukladá, inštalácie
  budú zoznamy posielať priamo tam.

## Ochrana osobných údajov
Zoznam obsahuje adresy vzdelávacích stránok a kódy krajín, nič o používateľovi. ATENA ho sama nikam
neposiela; ako sa dostane k redaktorovi, rozhodne používateľ. Stiahnutie spoločného registra na
príkaz sa spojí s GitHubom rovnako ako stiahnutie samotnej ATENY.
