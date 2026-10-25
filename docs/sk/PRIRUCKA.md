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

### Archív ATENY: dokumenty, ktoré z webu zmiznú

Weby často mažú staré stránky a prílohy. ATENA preto pri dokumentoch, ktoré **smie každý kopírovať
a zobrazovať** (úradné texty, napríklad zákony a vyhlášky, a dokumenty s otvorenou licenciou),
uchováva okrem textu aj **pôvodný súbor** (PDF, Word, OpenDocument).

- Súbor sa uloží len **po kontrole nebezpečného obsahu** a len vtedy, keď sa v texte **nenašli osobné
  údaje** (e-maily, telefónne čísla, rodné čísla). Webové stránky si ATENA uchová len ako text.
- Keď vydavateľ dokument z webu odstráni, ATENA ho ďalej zobrazí s označením **Odstránené z webu**
  a dátumom; zdroj a licencia ostávajú uvedené. Na karte dokumentu je tlačidlo **Otvoriť pôvodný
  súbor z archívu**.
- Všetky odstránené dokumenty nájdete v Hľadaní cez filter **Archív ATENY → Odstránené z webu**
  alebo kliknutím na riadok **Archív** v Prehľade.
- Dokumenty chránené autorským právom sa do archívu neukladajú; po odstránení z webu z ATENY zmiznú
  (pre seba si ich môžete uložiť v súkromnej časti).
- Archív zaberá miesto na disku. Vypnete ho v **Nastaveniach → Zber → Uchovávať pôvodné súbory…**.

### Nástroje a slovníky

Niektoré užitočné weby ATENA nezbiera, lebo ich obsah sa hľadá cez ich vlastné vyhľadávanie alebo
aplikáciu (slovníky JÚĽŠ SAV, Slovenský národný korpus, Viki a Planéta vedomostí). Nájdete ich
v ľavom menu pod **Nástroje** s popisom a tlačidlom **Otvoriť nástroj**. Pri hľadaní sa ukážu
nástroje, ktoré sa k hľadaným slovám hodia.

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

## Súkromná časť

V ľavom menu pod Nastaveniami je orámovaná časť **Súkromné**: **Moja ATENA**, **Zmeny a
legislatíva**, **Vyučovanie v praxi**, **Kurikulum a reformy** a **Gramotnosti a myslenie**
(slovenské údaje).

1. **Konto si vytvoríte v Nastaveniach → Súkromné konto** (meno a heslo, aspoň 10 znakov). Konto
   je len na tomto počítači; ATENA nič neposiela na žiadny server. Na jednom počítači je jedno konto.
2. **Záchranný kľúč** (`ATENA-XXXX-…`) sa ukáže len raz, pri vytvorení konta. Vytlačte si ho alebo
   ho uložte mimo počítača. Keď zabudnete heslo, v Nastaveniach ním nastavíte nové
   (**Zabudli ste heslo?**). Bez hesla a bez záchranného kľúča sa súkromná časť otvoriť nedá – ani
   autori ATENY to nedokážu.
3. **Prihlásenie:** kliknite na položku v časti Súkromné a zadajte meno a heslo. ATENA sa spýta, či
   má prihlásenie pamätať **do zatvorenia ATENY**, alebo súkromnú časť **zamknúť hneď, ako ju
   opustíte**. Zamknúť ju môžete aj tlačidlom **Zamknúť súkromnú časť**.
4. **Trvalé prihlásenie** (len vo Windows): v tej istej otázke alebo v Nastaveniach → Súkromné
   konto. Zapne sa až vtedy, keď **zaškrtnete, že to robíte na vlastnú zodpovednosť**: súkromná časť
   sa potom pri každom spustení ATENY otvorí bez hesla – **otvorí ju každý, kto sa prihlási do
   vášho účtu vo Windows**. Kľúč uloží Správca poverení Windows (len na tomto počítači, nikdy
   nie v cestovnom profile); nič neodchádza z počítača. Zrušíte ho v Nastaveniach tlačidlom
   **Zrušiť trvalé prihlásenie**.
5. Heslo sa nikde neukladá. Odvodí sa z neho kľúč (Argon2id), ktorý šifruje (XChaCha20-Poly1305)
   vašu súkromnú časť v priečinku `private`.

### Ukladanie dokumentov pre seba

V kategóriách súkromnej časti (napríklad **Vyučovanie v praxi**) má každý dokument tlačidlo
**Uložiť pre seba**. Nad zoznamom je **Uložiť vybrané** (zaškrtnuté dokumenty) a **Uložiť všetky
v zozname** – to sa ešte raz spýta. Ukladá sa po jednom dokumente; priebeh vidíte hore
a môžete ho **zastaviť**. Medzitým môžete pracovať ďalej.

- **Úradné a voľne licencované dokumenty** (napríklad zákony) sa neukladajú druhýkrát: uloží sa
  **odkaz** na verejnú časť ATENY, ktorá ich aj tak uchováva – v jej archíve aj s pôvodným súborom
  a aj po tom, čo ich vydavateľ z webu odstráni.
- **Ostatné dokumenty** sa uložia ako **kópia pre vlastnú potrebu**: ATENA ich v tej chvíli
  stiahne z webu vydavateľa a uloží **zašifrované** – text a pri PDF a dokumentoch Word/OpenDocument
  aj súbor.
- **Kedy sa dokument neuloží** (dôvod uvidíte pri ňom): vydavateľ si neželá ukladanie kópií,
  robots.txt webu to nedovoľuje, podmienky webu zakazujú automatický prístup, súbor obsahuje skript
  alebo iný aktívny obsah, alebo sa dokument nedá stiahnuť či prečítať.
- Uložené dokumenty sú v **Mojej ATENE → Uložené dokumenty** a v každej kategórii na karte
  **Uložené**. **Čítať** ukáže text aj **bez internetu**. **Otvoriť súbor** ho otvorí vo vašom
  programe (napríklad v prehliadači PDF) – na ten čas sa súbor dešifruje do priečinka ATENY
  a zmaže sa pri zamknutí súkromnej časti alebo zatvorení ATENY. **Odstrániť** ho zmaže z knižnice.

**Kópia pre vlastnú potrebu:** podľa § 42 Autorského zákona (č. 185/2015 Z. z.) smie fyzická osoba
bez súhlasu autora vyhotoviť rozmnoženinu diela pre svoju súkromnú potrebu, ak nejde o priamo ani
nepriamo obchodný účel. Kópie patria ich autorom: **nezdieľajte ich, nešírte a nepoužívajte na iné
účely**. Ak si nie ste istí, či vaše použitie (napríklad na vyučovaní) je ešte súkromnou potrebou,
použite radšej odkaz na originál.

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
| `postgres` | databáza: zdroje, dokumenty, verzie, návrhy a archív pôvodných súborov |
| `index` | index vyhľadávania (dá sa kedykoľvek postaviť nanovo) |
| `settings.json` | vaše nastavenia |
| `private` | súkromné konto (`account.json` – bez hesla a bez kľúča v čitateľnej podobe), pri trvalom prihlásení `remember.json` a zašifrovaná knižnica uložených dokumentov (`library`) |
| `private-open` | súkromný súbor otvorený na prezeranie (dešifrovaný); zmaže sa pri zamknutí alebo zatvorení ATENY |
| `archive-open` | pôvodný súbor z archívu otvorený na prezeranie; zmaže sa pri zatvorení alebo ďalšom spustení ATENY |
| `logs` | záznamy o behu aplikácie – pri probléme ich priložte k hláseniu |

Licencie všetkých súčastí sú v priečinku inštalácie v podpriečinku `notices`.

## Odinštalovanie

ATENU odinštalujete v **Nastaveniach Windows → Aplikácie**. Odinštalátor ponúkne aj
**vymazanie všetkých údajov ATENY** (databázu, zozbierané dokumenty, nastavenia aj súkromnú
časť) – **zaškrtnite ho len vtedy, keď ich naozaj už nechcete**; pri preinštalovaní na novú verziu
ho nechajte prázdne. Inak vaše údaje ostanú v priečinku uvedenom vyššie. Ak máte zapnuté trvalé
prihlásenie, pred odinštalovaním ho zrušte (inak v Správcovi poverení Windows zostane položka
„ATENA private part …“, ktorá bez priečinka `private` nič neotvorí; zmazať ju môžete aj ručne).

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
