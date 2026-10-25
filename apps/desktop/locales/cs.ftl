# ATENA – texty rozhraní v češtině.
# Referenční jazyk je slovenština (sk.ftl); každá zpráva zde má stejný identifikátor.
# Přeloženo strojově: návrh, dokud jej nezkontroluje rodilý mluvčí.

language-name = Čeština

## Aplikace

app-claim = Ověřené informace o vzdělávání
app-version = Verze { $version } · otevřený software
translation-draft = překlad – návrh
yes = ano
no = ne
action-close = Zavřít
action-cancel = Zrušit
action-open-original = Otevřít originál

## Spuštění

startup-starting = Spouštím ATENU…
startup-first-time = Při prvním spuštění si ATENA připraví vlastní databázi na tomto počítači a načte registr zdrojů. Trvá to asi půl minuty; příště to půjde rychleji.
startup-failed-hint = ATENU zavřete a otevřete znovu. Pokud se chyba zopakuje, podrobnosti jsou v záznamu ve složce aplikace (logs/atena.log).

## Navigace

nav-label = Hlavní nabídka
nav-overview = Přehled
nav-search = Hledat
nav-sources = Zdroje
nav-proposals = Návrhy
nav-crawl = Sběr
nav-settings = Nastavení

## Přehled

overview-title = Přehled
overview-empty-title = Zatím není nic shromážděno
overview-empty-text = Spusťte sběr. ATENA navštíví zdroje registru jako slušný návštěvník: dodrží jejich pravidla, nikam nespěchá a vezme si jen to, co dovoluje zákon a licence.
overview-empty-action = Přejít ke sběru
overview-documents =
    { $count ->
        [one] dokument
        [few] dokumenty
       *[other] dokumentů
    }
overview-versions =
    { $count ->
        [one] { $count } verze
        [few] { $count } verze
       *[other] { $count } verzí
    }
overview-searchable = vyhledatelných dokumentů
overview-sources = zdrojů se sbírá (ze všech v registru)
overview-proposals =
    { $count ->
        [one] návrh čeká na rozhodnutí
        [few] návrhy čekají na rozhodnutí
       *[other] návrhů čeká na rozhodnutí
    }
overview-last-check = Poslední kontrola: { $time } · za posledních 24 hodin { $checks ->
        [one] { $checks } kontrola
        [few] { $checks } kontroly
       *[other] { $checks } kontrol
    }
overview-never-checked = Zdroje ještě nebyly zkontrolovány.
overview-newest = Nejnovější
overview-search-all = Hledat ve všem
overview-sections = Kategorie
overview-in-scope =
    { $total ->
        [one] { $total } dokument
        [few] { $total } dokumenty
       *[other] { $total } dokumentů
    } v tomto výběru
overview-new-in-scope =
    { $new ->
        [one] { $new } nový
        [few] { $new } nové
       *[other] { $new } nových
    } za { $days } dní
overview-counts-legend = U kategorií: nové za posledních { $days } dní / všechny
overview-counts-title =
    { $new ->
        [one] { $new } nový
        [few] { $new } nové
       *[other] { $new } nových
    } za posledních { $days } dní, celkem { $total }
overview-sources-line = Sbírá se { $crawled } z { $sources } zdrojů

## Slovenská nebo zahraniční data, důvěryhodnost zdroje

scope-label = Která data zobrazit
scope-sk = Slovensko
scope-abroad = Zahraničí
scope-languages = Jazyk zahraničních dat
scope-all-languages = Všechny jazyky
trust-official = Úřední zdroj
trust-expert = Odborný zdroj
trust-other = Jiný zdroj – ověřte

## Hledání

search-title = Hledat
search-placeholder = Metody, předpisy, materiály… v libovolném jazyce
search-action = Hledat
search-hint = Napište, co hledáte, v libovolném jazyce. ATENA hledá i v jiných jazycích přes pojmy, které ve slovech rozpozná.
search-hint-syntax = Frázi dejte do uvozovek („čtenářská gramotnost“), slovo vyloučíte mínusem (-prázdniny).
search-filters = Zúženo na:
search-filter-remove = Zrušit toto zúžení
search-narrow = Zúžit výsledky
search-found =
    { $count ->
        [0] Nenašel se žádný dokument.
        [one] Nalezen { $count } dokument.
        [few] Nalezeny { $count } dokumenty.
       *[other] Nalezeno { $count } dokumentů.
    }
search-some-words = Žádný dokument nemá všechna slova – toto jsou dokumenty s některými z nich.
search-concepts = Hledal jsem i pojmy:
search-nothing = Zkuste jiná slova nebo zrušte některé zúžení.
search-previous = Předchozí
search-next = Další
search-page = Strana { $page } z { $pages }
search-only-official = Jen úřední zdroje
hit-no-text = Text tohoto zdroje se v ATENĚ nezobrazuje – licence to nedovoluje. Otevřete originál.

facet-section = Sekce
facet-school-type = Druh školy
facet-resource-type = Typ obsahu
facet-topic = Téma
facet-country = Země
facet-authority = Autorita zdroje
facet-language = Jazyk
facet-source = Zdroj
facet-trust = Důvěryhodnost zdroje

## Karta dokumentu

document-title = Dokument
document-untitled = (bez názvu)
document-not-shown = Tento dokument ATENA nezobrazuje: byl odstraněn, jeho vydavatel si nepřeje, aby se našel, nebo jeho zdroj se nesmí sbírat.
document-published = vydáno { $date }
document-labels-are-suggestions = Zařazení do sekcí a témat je návrh ATENY, dokud jej neověří člověk.
document-text = Text
document-text-not-shown = Celý text se v ATENĚ nezobrazuje – licence zdroje ({ $licence }) to nedovoluje. Přečtěte si jej v originálu.
document-versions =
    { $count ->
        [one] { $count } verze
        [few] { $count } verze
       *[other] { $count } verzí
    }
version-number = Verze
version-captured = Zachycena
version-change = Změna
version-paragraphs = (+{ $added } / −{ $removed } odstavců)
version-numbers-changed = změnilo se číslo nebo datum

change-initial = první zachycení
change-minor = drobná změna
change-substantive = podstatná změna
change-changed = text se změnil (předchozí se neuchovává)

## Zdroje

sources-title = Zdroje
sources-intro = Zdroje registru a potvrzené návrhy. U každého vidíte, co dovoluje zásada zákonnosti: zda se smí sbírat, zda se smí uchovat text a zda se smí celý zobrazit.
sources-sync = Synchronizovat společný registr
sources-sync-explain = ATENA stáhne aktuální společný registr zdrojů z repozitáře ATENA na GitHubu (soubor data/sources.yaml, jen přes šifrované spojení https) a sladí s ním databázi. Co už zásada zákonnosti nedovoluje, se odstraní.
sources-sync-privacy = Odešle se jen běžný požadavek na soubor; o vás ani o vašich návrzích se nic neodesílá.
sources-sync-confirm = Stáhnout a synchronizovat
sources-synced = Registr je sladěn: { $sources } zdrojů, { $added } nových adres. Odstraněných dokumentů: { $purged }, verzí omezených na metadata: { $restricted }.
sources-show-all = Všechny
sources-show-crawled = Sbírají se
sources-show-not-crawled = Nesbírají se
sources-show-imported = Z návrhů
sources-filter = Filtrovat podle názvu, adresy nebo země
sources-column-source = Zdroj
sources-column-country = Země
sources-column-rights = Licence a podmínky
sources-column-crawl = Sběr
sources-column-text = Text
sources-column-show = Zobrazit
sources-column-documents = Dokumenty
sources-checked = kontrola { $date }
sources-none = Výběru neodpovídá žádný zdroj.

allowed-crawl = smí se sbírat
allowed-text = smí se uchovat text
allowed-show = smí se zobrazit celý text

licence-copyrighted = chráněno autorským právem
licence-official = úřední text (bez autorskoprávní ochrany)

terms-allowed = podmínky dovolují
terms-forbidden = podmínky zakazují
terms-reserved = vyhrazeno (vytěžování textů)
terms-unknown = podmínky neověřeny

state-proposed = návrh
state-confirmed = potvrzen
state-rejected = zamítnut
state-merged = převzat do registru

restriction-no-profile = pro zemi zdroje nemá ATENA právní profil
restriction-terms-forbid = podmínky užívání zakazují automatizovaný přístup
restriction-terms-not-verified = chráněný obsah: podmínky užívání ještě nikdo neověřil
restriction-research-only = chráněný obsah: země zdroje dovoluje vytěžování jen pro nekomerční výzkum
restriction-no-mining-exception = chráněný obsah: země zdroje nemá výjimku pro vytěžování textů a dat
restriction-mining-reserved = vydavatel si vyhradil vytěžování textů a dat
restriction-privacy-law = zákon země zdroje o ochraně údajů nedovoluje uchovávat texty v zahraničí

## Návrhy

proposals-title = Návrhy zdrojů
proposals-intro = Navrhněte stránky, které má ATENA sledovat. Každou adresu nejprve posoudí; sbírat se začne, až když návrh potvrdí člověk.
proposals-tab-add = Přidat adresy
proposals-tab-waiting = Čekají na rozhodnutí
proposals-tab-decided = Rozhodnuté
proposals-tab-share = Předat editorovi
proposals-tab-receive = Přijmout seznam

import-addresses = Adresy nebo DOI, každá na samostatném řádku
import-placeholder = https://www.skola.cz/aktuality/
import-format = Za adresu můžete přidat kód země, jejíž právo platí (například „https://www.skola.sk/ SK“). Nejvýše 500 adres najednou.
import-country = Země pro adresy bez kódu
import-country-auto = podle adresy
import-how = ATENA otevře každou adresu jako slušný návštěvník: přečte robots.txt, prohlášení o licenci a podmínky užívání a navrhne, co zákon dovoluje. Nic se nesbírá, dokud návrh nepotvrdíte. Stránky o lidech se neukládají ani jako návrh.
import-action =
    { $count ->
        [0] Posoudit
        [one] Posoudit { $count } adresu
        [few] Posoudit { $count } adresy
       *[other] Posoudit { $count } adres
    }
import-busy = Posuzuji… u mnoha adres to může trvat několik minut.
import-done =
    { $count ->
        [one] Posouzena { $count } adresa.
        [few] Posouzeny { $count } adresy.
       *[other] Posouzeno { $count } adres.
    }
import-kept-for-sharing =
    { $count ->
        [one] { $count } adresa čeká na předání editorovi.
        [few] { $count } adresy čekají na předání editorovi.
       *[other] { $count } adres čeká na předání editorovi.
    }
import-column-line = Řádek
import-column-address = Adresa
import-column-result = Výsledek
import-column-proposal = Návrh

outcome-new = nový návrh
outcome-reassessed = návrh znovu posouzen
outcome-already-proposed = už je navržen
outcome-already-confirmed = už je potvrzen
outcome-already-rejected = už byl zamítnut
outcome-already-merged = už je v registru
outcome-in-register = už je v registru ({ $register })
outcome-blocked = zablokováno na žádost o výmaz
outcome-personal-page = stránka o lidech – neukládá se

problem-robots = robots.txt nedovoluje ATENĚ stránku číst
problem-robots-unreachable = robots.txt se nepodařilo načíst, proto se zatím nic nečte
problem-blocked = tato adresa není povolena (soukromá síť, neobvyklý port…)
problem-status = server odpověděl chybou
problem-unreachable = server se nepodařilo zastihnout
problem-retry-later = server požádal, abychom přišli později
problem-unsupported-type = takový druh obsahu ATENA nečte
problem-too-large = obsah je větší, než ATENA čte
problem-dangerous = nabízí program nebo dokument s aktivním obsahem – z takových zdrojů ATENA nečerpá
problem-too-demanding = obsah by potřeboval víc paměti nebo času než jakýkoli skutečný dokument – ATENA z něj nečerpá

reason-cannot-read = adresu se nepodařilo přečíst
reason-personal-page = stránka o lidech
reason-automation-ban = podmínky zakazují automatizovaný přístup
reason-personal-use-only = podmínky dovolují jen osobní užití
reason-mining-reserved = podmínky vyhrazují vytěžování textů nebo AI
reason-tdm-reserved = TDMRep vyhrazuje vytěžování textů a dat
reason-ai-crawlers-refused = robots.txt odmítá sběrače pro AI
reason-open-licence = otevřená licence
reason-licence-not-open = licence není otevřená nebo si odporuje
reason-non-commercial = jen nekomerční užití
reason-rights-reserved = všechna práva vyhrazena
reason-terms-read = podmínky přečteny
reason-no-terms-found = podmínky nenalezeny
reason-terms-unreadable = podmínky se nepodařilo přečíst
reason-unknown-country = zemi je třeba určit
reason-dangerous-content = nebezpečný obsah

rejection-not-an-address = není to adresa ani DOI
rejection-unsupported-scheme = jen adresy http a https
rejection-credentials = adresa obsahuje jméno nebo heslo
rejection-duplicate = opakuje se
rejection-too-many = příliš mnoho adres najednou
rejection-too-long = řádek je příliš dlouhý

proposals-none-waiting = Žádný návrh nečeká na rozhodnutí.
proposals-none-decided = Zatím jste o žádném návrhu nerozhodli.
proposal-shared = z jiné instalace
proposal-confirm = Potvrdit
proposal-reject = Zamítnout
proposal-automatic = Návrh ATENY ({ $date }): { $licence } · { $terms } · země { $country }
proposal-decided = Rozhodnutí: { $licence } · { $terms } · země { $country }
proposal-would-allow = Potvrzení by dovolilo:
proposal-finding = Záznam o ověření (v angličtině, jak se uchovává)
proposal-rights-page = Otevřít stránku s licencí nebo podmínkami
proposal-confirm-title = Potvrdit „{ $name }“
proposal-reject-title = Zamítnout „{ $name }“
proposal-confirmed-crawled = Potvrzeno. Zdroj se bude sbírat při nejbližším sběru.
proposal-confirmed-not-crawled = Potvrzeno. Zásada zákonnosti však nedovoluje tento zdroj sbírat.
proposal-rejected = Zamítnuto. Vše, co se z tohoto zdroje shromáždilo, bylo odstraněno.

decision-licence = Licence
decision-terms = Podmínky užívání
decision-country = Země, jejíž právo platí
decision-reason-confirm = Proč – co jste si přečetli a kdy (povinné)
decision-reason-reject = Proč zamítáte (povinné)
decision-by = Rozhoduje: { $name } (jméno se uchovává jen na tomto počítači)

share-explain = Seznam obsahuje jen adresy a kódy zemí – nic o vás. ATENA jej nikam neposílá: zkopírujte jej a předejte editorovi ATENY, jak sami chcete (například e-mailem). Potom jej označte jako předaný.
share-nothing = Nic nečeká na předání.
share-list =
    { $count ->
        [one] Seznam s { $count } adresou
       *[other] Seznam s { $count } adresami
    }
share-copy = Kopírovat
share-copied = Zkopírováno
share-mark-handed-over = Označit jako předané
share-handed-over = Seznam je označen jako předaný.

receive-explain = Pro editora: vložte seznam, který vám poslal uživatel jiné instalace. ATENA každou adresu posoudí sama – přijatému posouzení by nebylo co věřit – a uloží je jako návrhy.
receive-list = Seznam (začíná řádkem ATENA-NAVRHY 1)
receive-action = Přečíst seznam
receive-not-a-list = Toto není seznam ATENY: první řádek musí být ATENA-NAVRHY 1.
receive-read-before = Tento seznam už byl přečten ({ $list }).

## Sběr

crawl-title = Sběr
crawl-intro = ATENA navštěvuje zdroje registru a potvrzené návrhy, ukládá nové a změněné dokumenty a zpřístupňuje je vyhledávání. Sběr běží jen tehdy, když jej spustíte.
crawl-state-running = Sběr běží
crawl-state-stopping = Sběr se zastavuje – dokončují se rozběhnuté kontroly
crawl-state-stopped = Sběr je zastaven
crawl-running = sběr běží
crawl-start = Spustit sběr
crawl-stop = Zastavit
crawl-started = spuštěn { $time }
crawl-checks = kontrol
crawl-hosts = navštívených serverů
crawl-due = adres čeká na kontrolu
crawl-store-errors = chyb databáze
crawl-outcomes = Výsledky kontrol
crawl-rules-title = Jak ATENA sbírá
crawl-rule-robots = Dodržuje robots.txt a pravidla vydavatele (TDMRep, noindex, nosnippet).
crawl-rule-pace = Na každý server chodí pomalu, jeden požadavek za druhým, a když server požádá o klid, počká.
crawl-rule-open = Čerpá jen z otevřených zdrojů a uchová jen to, co dovoluje zákon a licence.
crawl-rule-privacy = Jde jako anonymní návštěvník: bez souborů cookie, bez skriptů a bez obrázků. O vás nic neposílá.
crawl-rule-safety = Programy a dokumenty s aktivním obsahem odmítá, dokumenty zpracovává v omezeném procesu a původní soubory neuchovává.
crawl-concurrency =
    { $count ->
        [one] Najednou se pracuje s { $count } serverem (lze změnit v nastavení).
       *[other] Najednou se pracuje s { $count } servery (lze změnit v nastavení).
    }

check-new = nový dokument
check-changed = změněný dokument
check-unchanged = beze změny
check-not-modified = beze změny (podle serveru)
check-feed = kanál RSS/Atom přečten
check-sitemap = mapa stránek přečtena
check-robots = robots.txt nedovoluje
check-robots-unreachable = robots.txt nedostupný
check-blocked = adresa není povolena
check-http-error = chyba serveru
check-gone = dokument už neexistuje
check-network = síť nedostupná
check-timeout = vypršel čas
check-redirects = příliš mnoho přesměrování
check-retry-later = server žádá počkat
check-too-large = příliš velké
check-unsupported = nepodporovaný druh obsahu
check-dangerous = nebezpečný obsah odmítnut
check-too-demanding = příliš náročný obsah odmítnut
check-extract-error = obsah se nepodařilo zpracovat
check-feed-error = chybný kanál RSS/Atom
check-sitemap-error = chybná mapa stránek
check-moved-elsewhere = přesunuto na jiný web
check-erased = vymazáno na žádost
check-not-permitted = zásada zákonnosti nedovoluje
check-invalid-url = neplatná adresa
check-store-error = chyba databáze

## Nastavení

settings-title = Nastavení
settings-saved = Uloženo.
settings-appearance = Jazyk a vzhled
settings-language = Jazyk rozhraní
settings-theme = Vzhled
settings-theme-system = podle systému
settings-theme-light = světlý
settings-theme-dark = tmavý
settings-decisions = Rozhodnutí o návrzích
settings-editor = Vaše jméno nebo přezdívka
settings-editor-privacy = Zapisuje se k vašim rozhodnutím a zůstává jen na tomto počítači. Společný registr žádnou osobu neuvádí.
settings-share = Odkládat navržené adresy pro editora
settings-share-explain = ATENA nic neodesílá sama; odložené adresy najdete v Návrzích → Předat editorovi.
settings-crawler = Sběr
settings-concurrency = Počet serverů najednou (1–16)
settings-concurrency-explain = Na každý server jde ATENA vždy jen jedním požadavkem najednou; toto číslo říká, kolik různých serverů navštěvuje současně.

erasure-title = Žádost o výmaz
erasure-explain = Pokud někdo požádá o výmaz (čl. 17 GDPR), ATENA odstraní vše o dané adrese a už ji nikdy nenavštíví.
erasure-address = Adresa
erasure-prefix = i všechny adresy, které jí začínají
erasure-reason = Důvod (například: žádost ze dne…)
erasure-action = Vymazat a zablokovat
erasure-done = Vymazáno: { $documents } dokumentů a { $versions } verzí. Adresa je zablokována.

about-title = O ATENĚ
about-version = ATENA { $version } – otevřený software pod licencí Apache 2.0, bezplatný pro soukromé i komerční užití.
about-connections = ATENA nikomu neposílá nic o svých uživatelích. Sama se nikam nepřipojuje; do sítě jde jen tehdy, když spustíte sběr, posouzení adres nebo synchronizaci registru, a tehdy jako anonymní návštěvník. Nepoužívá služby třetích stran.
about-pillars = Čtyři pilíře ATENY
pillar-1 = Serióznost, otevřenost a snaha posouvat poznání každého člověka dopředu.
pillar-2 = Anonymita a respekt k soukromí – našemu i jiných.
pillar-3 = Bezpečnost a ochrana před nebezpečnými lidmi, roboty, kódy a procesy.
pillar-4 = Dodržování zákona: žádný zákon nesmí být porušen, nikde na světě.
about-licence = Licence všech součástí jsou ve složce aplikace „notices“.
about-repository = Zdrojový kód na GitHubu

## Chyby

error-detail = Podrobnosti
error-already-running = ATENA už běží. Pracujte v otevřeném okně.
error-crawl = Sběr se nepodařilo připravit.
error-database = Chyba databáze.
error-database-start = Databázi se nepodařilo spustit.
error-database-version = Databázi vytvořila jiná verze PostgreSQL, než jakou přináší tato ATENA.
error-decision-country = Pro tuto zemi nemá ATENA právní profil.
error-decision-incomplete = Rozhodnutí potřebuje jméno a důvod.
error-decision-licence = Takovou licenci registr nezná.
error-decision-official = Úřední texty této země nejsou volné; vyberte její otevřenou licenci.
error-decision-terms = Takové podmínky registr nezná.
error-editor-missing = Rozhodnutí potřebuje vaše jméno – zadejte jej v nastavení.
error-index = Chyba indexu vyhledávání.
error-index-foreign = Složka indexu obsahuje něco jiného než index ATENY.
error-input-too-long = Text je příliš dlouhý.
error-internal = Došlo k neočekávané chybě.
error-io = Soubor nebo složku se nepodařilo přečíst nebo zapsat.
error-not-a-list = Toto není seznam ATENY.
error-not-found = Nenalezeno.
error-not-ready = ATENA se ještě spouští.
error-reason-missing = Napište důvod.
error-search = Vyhledávání selhalo.
error-search-empty = Napište, co hledáte, nebo vyberte zúžení.
error-search-filter = Takové zúžení ATENA nezná.
error-settings = Nastavení se nepodařilo uložit.
error-shared-register-invalid = Společný registr není platný; nic se nezměnilo.
error-shared-register-unreachable = Společný registr se nepodařilo stáhnout. Zkontrolujte připojení k internetu.
error-url = Toto není platná webová adresa.
