# ATENA – príručka pre používateľa

Táto príručka opisuje aplikáciu ATENA pre počítač so systémom Windows, verziu 0.2 – prvé
vydanie na vyskúšanie.

## Čo ATENA robí

ATENA zbiera informácie o vzdelávaní z otvorených zdrojov a sprístupňuje ich na vyhľadávanie v
piatich jazykoch. Zdrojmi sú najmä ministerstvá, štátne inštitúcie, zbierka zákonov, odborné
organizácie a výskum. Pri každom dokumente vidíte, odkiaľ pochádza a aký autoritatívny je jeho
zdroj, a jedným kliknutím otvoríte originál.

ATENA stojí na štyroch pilieroch:

1. serióznosť, otvorenosť a snaha posúvať poznanie každého človeka dopredu;
2. anonymita a rešpekt k súkromiu – nášmu aj iných;
3. bezpečnosť a ochrana pred nebezpečnými ľuďmi, robotmi, kódmi a procesmi;
4. dodržiavanie zákona – žiaden zákon nesmie byť porušený, nikde na svete.

## Inštalácia

1. Stiahnite si inštalátor `ATENA_…_x64-setup.exe` z najnovšieho vydania na
   https://github.com/tarzanislav/atena/releases (bodky nahrádzajú číslo verzie).
2. Spustite ho. Inštalátor zatiaľ nie je digitálne podpísaný, preto sa Windows môže opýtať, či
   mu dôverujete („Systém Windows ochránil váš počítač“). Kliknite na **Ďalšie informácie** a
   potom na **Spustiť aj tak**.
3. Vyberte jazyk inštalátora a potvrďte inštaláciu. ATENA sa inštaluje len pre vás a nepotrebuje
   práva správcu.

Ak by vo Windows chýbal súčasný komponent Microsoft Edge WebView2 (týka sa len starších
inštalácií Windows 10), inštalátor si ho stiahne od Microsoftu.

## Prvé spustenie

Pri prvom spustení si ATENA pripraví vlastnú databázu na vašom počítači a načíta register
zdrojov. Trvá to asi pol minúty; ďalšie spustenia sú rýchlejšie.

Databáza je súčasťou aplikácie a nič ďalšie netreba inštalovať. Je dostupná len na vašom
počítači, chráni ju náhodné heslo a zapína sa a vypína spolu s ATENOU.

## Obrazovky

| Obrazovka | Čo na nej nájdete |
|---|---|
| **Prehľad** | prepínač **Slovensko / Zahraničie**, kategórie s počtom nových a všetkých dokumentov, najnovšie dokumenty – kliknutím na kategóriu sa otvorí vyhľadávanie v nej |
| **Hľadať** | vyhľadávanie v ľubovoľnom jazyku; výsledky zúžite vľavo podľa kategórie, dôveryhodnosti zdroja, druhu školy, krajiny, autority, jazyka alebo zdroja |
| **Zdroje** | všetky zdroje a pri každom, čo smie ATENA robiť: zbierať, uchovať text, zobraziť celý text |
| **Návrhy** | navrhnutie nových stránok, rozhodovanie o návrhoch a odovzdanie návrhov editorovi |
| **Zber** | spustenie a zastavenie zberu a jeho priebeh |
| **Nastavenia** | jazyk, svetlý alebo tmavý vzhľad, vaše meno pre rozhodnutia, žiadosti o vymazanie |

### Slovensko alebo Zahraničie

Prepínač navrchu Prehľadu aj Hľadania určuje, ktoré údaje vidíte:

- **Slovensko** – dokumenty slovenských zdrojov a dokumenty v slovenčine;
- **Zahraničie** – ostatné; pod prepínačom si vyberiete jazyk (alebo všetky jazyky).

Voľba sa pamätá. Čísla pri kategóriách znamenajú **nové / všetky** – nové sú dokumenty, ktoré ATENA
našla za posledných 7 dní.

### Dôveryhodnosť zdroja

Každý výsledok má štítok a farebný pruh vľavo:

| Štítok | Zdroje |
|---|---|
| **Úradný zdroj** (modrý) | zákony, ministerstvo a štátne inštitúcie (A1, A2) |
| **Odborný zdroj** (zelený) | medzinárodné organizácie, univerzity a výskum, odborné organizácie (B1, B2, C1) |
| **Iný zdroj – overte** (oranžový) | asociácie, učitelia a autori, komunity, médiá (C2, D, E) – overte v pôvodnom zdroji |

Pri rovnakej zhode sú úradné zdroje vo výsledkoch vyššie. Tlačidlom **Len úradné zdroje** pri
vyhľadávaní ukážete len ich.

### Hľadanie

- Píšte bežné slová v ľubovoľnom jazyku. Na diakritike nezáleží: „zapis“ nájde aj „zápis“.
- Frázu dajte do úvodzoviek („čitateľská gramotnosť“), slovo vylúčite mínusom (-prázdniny).
- Pri výsledku vidíte zdroj, jeho autoritu (A1 – záväzný úradný zdroj … E – médiá), dátum a
  jazyk. Tlačidlom **Otvoriť originál** sa dokument otvorí vo vašom prehliadači.
- Text sa ukazuje len tam, kde to dovoľuje licencia. Pri ostatných dokumentoch ATENA ukáže len
  názov a odkaz na originál.
- Zaradenie do kategórií a tém je návrh ATENY, kým ho neoverí človek. Po aktualizácii ATENY sa
  už uložené dokumenty na pozadí znova zaradia podľa novej taxonómie.

### Zber

Zber sa spúšťa len tlačidlom **Spustiť zber**. ATENA potom navštevuje zdroje ako slušný
návštevník:

- dodržiava pravidlá webov (robots.txt, vyhradenia vydavateľov);
- na každý server chodí pomaly, jednu požiadavku za druhou;
- odmieta programy a dokumenty s aktívnym obsahom;
- uchová len to, čo dovoľuje zákon a licencia.

Zber môžete kedykoľvek zastaviť; rozbehnuté kontroly sa dokončia. Nové dokumenty sa vo
vyhľadávaní objavia do minúty.

### Návrhy zdrojov

1. V záložke **Pridať adresy** vložte adresy stránok, každú na samostatný riadok, a kliknite na
   **Posúdiť**.
2. ATENA každú adresu otvorí, prečíta jej pravidlá, licenciu a podmienky používania a navrhne, čo
   zákon dovoľuje. Stránky o ľuďoch sa neukladajú ani ako návrh.
3. V záložke **Čakajú na rozhodnutie** si návrh prečítajte. Potvrdiť alebo zamietnuť ho môžete,
   až keď v Nastaveniach zadáte svoje meno alebo prezývku a napíšete dôvod rozhodnutia. Meno
   ostáva len na vašom počítači.
4. Potvrdený zdroj sa začne zbierať pri najbližšom zbere – ak to zásada zákonnosti dovoľuje.

Ak chcete, aby sa vaše návrhy dostali do spoločného registra pre všetkých, otvorte záložku
**Odovzdať editorovi**, zoznam skopírujte a pošlite editorovi ATENY, napríklad e-mailom.
Zoznam obsahuje len adresy a kódy krajín. ATENA ho sama nikam neposiela.

## Súkromie a spojenia

- ATENA nikomu neposiela nič o vás a nepoužíva služby tretích strán.
- Do internetu ide len na váš príkaz: keď spustíte zber, keď posudzujete adresy a keď
  synchronizujete spoločný register.
- Pri zbere a posudzovaní navštevuje weby ako anonymný návštevník: bez súborov cookie, bez
  skriptov a bez obrázkov. Predstaví sa ako „ATENA“ s odkazom na opis zberača, aby vydavatelia
  vedeli, kto ich navštevuje.
- Navštívené servery – rovnako ako pri bežnom prezeraní webu – vidia IP adresu vášho pripojenia.
- Synchronizácia spoločného registra stiahne len súbor registra z GitHubu cez šifrované spojenie.

## Kde sú vaše údaje

Všetky údaje ATENY sú v priečinku

```
%LOCALAPPDATA%\io.github.tarzanislav.atena
```

(do panela s adresou v Prieskumníkovi napíšte presne tento text a stlačte Enter):

| Priečinok alebo súbor | Obsah |
|---|---|
| `postgres` | databáza: zdroje, dokumenty, verzie, návrhy |
| `index` | index vyhľadávania (dá sa kedykoľvek postaviť nanovo) |
| `settings.json` | vaše nastavenia |
| `logs` | záznamy o behu aplikácie – pri probléme ich priložte k hláseniu |

Licencie všetkých súčastí sú v priečinku inštalácie v podpriečinku `notices`.

## Odinštalovanie

ATENU odinštalujete v **Nastaveniach Windows → Aplikácie**. Ak odinštalátor ponúkne aj
vymazanie údajov aplikácie, môžete ho zaškrtnúť. Inak vaše údaje ostanú v priečinku uvedenom
vyššie; ak ich už nepotrebujete, priečinok zmažte.

## Keď niečo nejde

- **ATENA sa nespustí a ukáže chybu:** zatvorte ju a otvorte znova. Ak sa chyba zopakuje,
  pošlite nám súbor `logs\atena.log`.
- **„ATENA už beží“:** ATENA je už otvorená v inom okne. Dve ATENY nad tými istými údajmi naraz
  bežať nemôžu.
- **Vo vyhľadávaní nič nie je:** najprv spustite zber a nechajte ho chvíľu bežať.

## Čo prvé vydanie ešte nemá

- Inštalátor je len pre Windows a nie je digitálne podpísaný.
- Texty v angličtine, češtine, ruštine a čínštine sú strojový preklad. Pri jazyku je v
  nastaveniach označenie „preklad – návrh“.
- Automatické vyhľadávanie nových zdrojov podľa tém (katalógy ERIC, OpenAlex, Crossref, DOAJ) a
  správa o „bielych miestach“ prídu v ďalšej verzii.
