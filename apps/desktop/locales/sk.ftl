# ATENA – texty rozhrania. Slovenčina je referenčný jazyk: každá správa vzniká najprv tu.
# Ostatné jazyky majú presne tie isté identifikátory (kontroluje scripts/check-locales.mjs).

language-name = Slovenčina

## Aplikácia

app-claim = Overené informácie o vzdelávaní
app-version = Verzia { $version } · otvorený softvér
translation-draft = preklad – návrh
yes = áno
no = nie
action-close = Zavrieť
action-cancel = Zrušiť
action-open-original = Otvoriť originál

## Spustenie

startup-starting = Spúšťam ATENU…
startup-first-time = Pri prvom spustení si ATENA pripraví vlastnú databázu na tomto počítači a načíta register zdrojov. Trvá to asi pol minúty; nabudúce to pôjde rýchlejšie.
startup-failed-hint = ATENU zatvorte a otvorte znova. Ak sa chyba zopakuje, podrobnosti sú v záznamoch v priečinku aplikácie (logs/atena.log).

## Navigácia

nav-label = Hlavná ponuka
nav-overview = Prehľad
nav-search = Hľadať
nav-tools = Nástroje
nav-sources = Zdroje
nav-proposals = Návrhy
nav-crawl = Zber
nav-settings = Nastavenia

## Prehľad

overview-title = Prehľad
overview-empty-title = Zatiaľ nie je nič zozbierané
overview-empty-text = Spustite zber. ATENA navštívi zdroje registra ako slušný návštevník: dodrží ich pravidlá, nikam sa neponáhľa a vezme si len to, čo zákon a licencia dovoľujú.
overview-empty-action = Prejsť na zber
overview-documents =
    { $count ->
        [one] dokument
        [few] dokumenty
       *[other] dokumentov
    }
overview-versions =
    { $count ->
        [one] { $count } verzia
        [few] { $count } verzie
       *[other] { $count } verzií
    }
overview-searchable = vyhľadateľných dokumentov
overview-sources = zdrojov sa zbiera (zo všetkých v registri)
overview-proposals =
    { $count ->
        [one] návrh čaká na rozhodnutie
        [few] návrhy čakajú na rozhodnutie
       *[other] návrhov čaká na rozhodnutie
    }
overview-last-check = Posledná kontrola: { $time } · za posledných 24 hodín { $checks ->
        [one] { $checks } kontrola
        [few] { $checks } kontroly
       *[other] { $checks } kontrol
    }
overview-never-checked = Zdroje ešte neboli skontrolované.
overview-newest = Najnovšie
overview-search-all = Hľadať vo všetkom
overview-sections = Kategórie
overview-in-scope =
    { $total ->
        [one] { $total } dokument
        [few] { $total } dokumenty
       *[other] { $total } dokumentov
    } v tomto výbere
overview-new-in-scope =
    { $new ->
        [one] { $new } nový
        [few] { $new } nové
       *[other] { $new } nových
    } za { $days } dní
overview-counts-legend = Pri kategóriách: nové za posledných { $days } dní / všetky
overview-counts-title =
    { $new ->
        [one] { $new } nový
        [few] { $new } nové
       *[other] { $new } nových
    } za posledných { $days } dní, spolu { $total }
overview-sources-line = Zbiera sa { $crawled } z { $sources } zdrojov

## Slovenské alebo zahraničné údaje, dôveryhodnosť zdroja

scope-label = Ktoré údaje zobraziť
scope-sk = Slovensko
scope-abroad = Zahraničie
scope-languages = Jazyk zahraničných údajov
scope-all-languages = Všetky jazyky
trust-official = Úradný zdroj
trust-expert = Odborný zdroj
trust-other = Iný zdroj – overte
tools-title = Nástroje a slovníky
tools-intro = Slovníky, korpusy a aplikácie, ktoré ATENA nezbiera – obsah sa v nich hľadá priamo. Otvoria sa vo vašom prehliadači.
tools-open = Otvoriť nástroj
tools-none = Pre tieto údaje tu zatiaľ nie sú žiadne nástroje.

## Hľadanie

search-title = Hľadať
search-placeholder = Metódy, predpisy, materiály… v ľubovoľnom jazyku
search-action = Hľadať
search-hint = Napíšte, čo hľadáte, v ľubovoľnom jazyku. ATENA hľadá aj v iných jazykoch cez pojmy, ktoré v slovách rozpozná.
search-hint-syntax = Frázu dajte do úvodzoviek („čitateľská gramotnosť“), slovo vylúčite mínusom (-prázdniny).
search-filters = Zúžené na:
search-filter-remove = Zrušiť toto zúženie
search-narrow = Zúžiť výsledky
search-found =
    { $count ->
        [0] Nenašiel sa žiadny dokument.
        [one] Našiel sa { $count } dokument.
        [few] Našli sa { $count } dokumenty.
       *[other] Našlo sa { $count } dokumentov.
    }
search-some-words = Žiadny dokument nemá všetky slová – toto sú dokumenty s niektorými z nich.
search-concepts = Hľadal som aj pojmy:
search-nothing = Skúste iné slová alebo zrušte niektoré zúženie.
search-previous = Predchádzajúca
search-next = Ďalšia
search-page = Strana { $page } z { $pages }
search-only-official = Len úradné zdroje
search-tools = Nástroje k hľadaným slovám
hit-no-text = Text tohto zdroja sa v ATENE nezobrazuje – licencia to nedovoľuje. Otvorte originál.

facet-section = Sekcia
facet-school-type = Druh školy
facet-resource-type = Typ obsahu
facet-topic = Téma
facet-country = Krajina
facet-authority = Autorita zdroja
facet-language = Jazyk
facet-source = Zdroj
facet-trust = Dôveryhodnosť zdroja

## Karta dokumentu

document-title = Dokument
document-untitled = (bez názvu)
document-not-shown = Tento dokument ATENA nezobrazuje: bol odstránený, jeho vydavateľ si neželá, aby sa našiel, alebo jeho zdroj sa nesmie zbierať.
document-published = vydané { $date }
document-labels-are-suggestions = Zaradenie do sekcií a tém je návrh ATENY, kým ho neoverí človek.
document-text = Text
document-text-not-shown = Celý text sa v ATENE nezobrazuje – licencia zdroja ({ $licence }) to nedovoľuje. Prečítajte si ho v origináli.
document-versions =
    { $count ->
        [one] { $count } verzia
        [few] { $count } verzie
       *[other] { $count } verzií
    }
version-number = Verzia
version-captured = Zachytená
version-change = Zmena
version-paragraphs = (+{ $added } / −{ $removed } odsekov)
version-numbers-changed = zmenilo sa číslo alebo dátum

change-initial = prvé zachytenie
change-minor = drobná zmena
change-substantive = podstatná zmena
change-changed = text sa zmenil (predchádzajúci sa neuchováva)

## Zdroje

sources-title = Zdroje
sources-intro = Zdroje registra a potvrdené návrhy. Pri každom vidíte, čo dovoľuje zásada zákonnosti: či sa smie zbierať, či sa smie uchovať text a či sa smie celý zobraziť.
sources-sync = Synchronizovať spoločný register
sources-sync-explain = ATENA stiahne aktuálny spoločný register zdrojov z repozitára ATENA na GitHube (súbor data/sources.yaml, len cez šifrované spojenie https) a zosúladí s ním databázu. Čo už zásada zákonnosti nedovoľuje, sa odstráni.
sources-sync-privacy = Odošle sa len bežná požiadavka na súbor; o vás ani o vašich návrhoch sa nič neodosiela.
sources-sync-confirm = Stiahnuť a synchronizovať
sources-synced = Register je zosúladený: { $sources } zdrojov, { $added } nových adries. Odstránených dokumentov: { $purged }, verzií obmedzených na metaúdaje: { $restricted }.
sources-show-all = Všetky
sources-show-crawled = Zbierajú sa
sources-show-not-crawled = Nezbierajú sa
sources-show-imported = Z návrhov
sources-filter = Filtrovať podľa názvu, adresy alebo krajiny
sources-column-source = Zdroj
sources-column-country = Krajina
sources-column-rights = Licencia a podmienky
sources-column-crawl = Zber
sources-column-text = Text
sources-column-show = Zobraziť
sources-column-documents = Dokumenty
sources-checked = kontrola { $date }
sources-tool = Nástroj (nezbiera sa)
sources-site-wide = Celý web
sources-none = Žiadny zdroj nezodpovedá výberu.

allowed-crawl = smie sa zbierať
allowed-text = smie sa uchovať text
allowed-show = smie sa zobraziť celý text

licence-copyrighted = chránené autorským právom
licence-official = úradný text (bez autorskej ochrany)

terms-allowed = podmienky dovoľujú
terms-forbidden = podmienky zakazujú
terms-reserved = vyhradené (dolovanie textov)
terms-unknown = podmienky neoverené

state-proposed = návrh
state-confirmed = potvrdený
state-rejected = zamietnutý
state-merged = prevzatý do registra

restriction-no-profile = pre krajinu zdroja nemá ATENA právny profil
restriction-terms-forbid = podmienky používania zakazujú automatizovaný prístup
restriction-terms-not-verified = chránený obsah: podmienky používania ešte nikto neoveril
restriction-research-only = chránený obsah: krajina zdroja dovoľuje dolovanie len pre nekomerčný výskum
restriction-no-mining-exception = chránený obsah: krajina zdroja nemá výnimku pre dolovanie textov a dát
restriction-mining-reserved = vydavateľ si vyhradil dolovanie textov a dát
restriction-privacy-law = zákon krajiny zdroja o ochrane údajov nedovoľuje uchovávať texty v zahraničí

## Návrhy

proposals-title = Návrhy zdrojov
proposals-intro = Navrhnite stránky, ktoré má ATENA sledovať. Každú adresu najprv posúdi; zbierať sa začne, až keď návrh potvrdí človek.
proposals-tab-add = Pridať adresy
proposals-tab-waiting = Čakajú na rozhodnutie
proposals-tab-decided = Rozhodnuté
proposals-tab-share = Odovzdať editorovi
proposals-tab-receive = Prijať zoznam

import-addresses = Adresy alebo DOI, každá na samostatnom riadku
import-placeholder = https://www.skola.sk/oznamy/
import-format = Za adresu môžete pridať kód krajiny, ktorej právo platí (napríklad „https://www.skola.cz/ CZ“). Najviac 500 adries naraz.
import-country = Krajina pre adresy bez kódu
import-country-auto = podľa adresy
import-how = ATENA otvorí každú adresu ako slušný návštevník: prečíta robots.txt, vyhlásenie licencie a podmienky používania a navrhne, čo zákon dovoľuje. Nič sa nezbiera, kým návrh nepotvrdíte. Stránky o ľuďoch sa neukladajú ani ako návrh.
import-action =
    { $count ->
        [0] Posúdiť
        [one] Posúdiť { $count } adresu
        [few] Posúdiť { $count } adresy
       *[other] Posúdiť { $count } adries
    }
import-busy = Posudzujem… pri mnohých adresách to môže trvať niekoľko minút.
import-done =
    { $count ->
        [one] Posúdená { $count } adresa.
        [few] Posúdené { $count } adresy.
       *[other] Posúdených { $count } adries.
    }
import-kept-for-sharing =
    { $count ->
        [one] { $count } adresa čaká na odovzdanie editorovi.
        [few] { $count } adresy čakajú na odovzdanie editorovi.
       *[other] { $count } adries čaká na odovzdanie editorovi.
    }
import-column-line = Riadok
import-column-address = Adresa
import-column-result = Výsledok
import-column-proposal = Návrh

outcome-new = nový návrh
outcome-reassessed = návrh nanovo posúdený
outcome-already-proposed = už je navrhnutý
outcome-already-confirmed = už je potvrdený
outcome-already-rejected = už bol zamietnutý
outcome-already-merged = už je v registri
outcome-in-register = už je v registri ({ $register })
outcome-blocked = zablokované na žiadosť o vymazanie
outcome-personal-page = stránka o ľuďoch – neukladá sa

problem-robots = robots.txt nedovoľuje ATENE stránku čítať
problem-robots-unreachable = robots.txt sa nepodarilo načítať, preto sa zatiaľ nič nečíta
problem-blocked = táto adresa nie je povolená (súkromná sieť, nezvyčajný port…)
problem-status = server odpovedal chybou
problem-unreachable = server sa nepodarilo zastihnúť
problem-retry-later = server požiadal, aby sme prišli neskôr
problem-unsupported-type = takýto druh obsahu ATENA nečíta
problem-too-large = obsah je väčší, než ATENA číta
problem-dangerous = ponúka program alebo dokument s aktívnym obsahom – z takých zdrojov ATENA nečerpá
problem-too-demanding = obsah by potreboval viac pamäte alebo času než akýkoľvek skutočný dokument – ATENA z neho nečerpá

reason-cannot-read = adresu sa nepodarilo prečítať
reason-personal-page = stránka o ľuďoch
reason-automation-ban = podmienky zakazujú automatizovaný prístup
reason-personal-use-only = podmienky dovoľujú len osobné použitie
reason-mining-reserved = podmienky vyhradzujú dolovanie textov alebo AI
reason-tdm-reserved = TDMRep vyhradzuje dolovanie textov a dát
reason-ai-crawlers-refused = robots.txt odmieta zberače pre AI
reason-open-licence = otvorená licencia
reason-licence-not-open = licencia nie je otvorená alebo si odporuje
reason-non-commercial = len nekomerčné použitie
reason-rights-reserved = všetky práva vyhradené
reason-terms-read = podmienky prečítané
reason-no-terms-found = podmienky sa nenašli
reason-terms-unreadable = podmienky sa nepodarilo prečítať
reason-unknown-country = krajinu treba určiť
reason-dangerous-content = nebezpečný obsah

rejection-not-an-address = nie je to adresa ani DOI
rejection-unsupported-scheme = len adresy http a https
rejection-credentials = adresa obsahuje meno alebo heslo
rejection-duplicate = opakuje sa
rejection-too-many = priveľa adries naraz
rejection-too-long = riadok je pridlhý

proposals-none-waiting = Žiadny návrh nečaká na rozhodnutie.
proposals-none-decided = Zatiaľ ste o žiadnom návrhu nerozhodli.
proposal-shared = z inej inštalácie
proposal-confirm = Potvrdiť
proposal-reject = Zamietnuť
proposal-automatic = Návrh ATENY ({ $date }): { $licence } · { $terms } · krajina { $country }
proposal-decided = Rozhodnutie: { $licence } · { $terms } · krajina { $country }
proposal-would-allow = Potvrdenie by dovolilo:
proposal-finding = Záznam o overení (v angličtine, ako sa uchováva)
proposal-rights-page = Otvoriť stránku s licenciou alebo podmienkami
proposal-confirm-title = Potvrdiť „{ $name }“
proposal-reject-title = Zamietnuť „{ $name }“
proposal-confirmed-crawled = Potvrdené. Zdroj sa bude zbierať pri najbližšom zbere.
proposal-confirmed-not-crawled = Potvrdené. Zásada zákonnosti však nedovoľuje tento zdroj zbierať.
proposal-rejected = Zamietnuté. Všetko, čo sa z tohto zdroja zozbieralo, sa odstránilo.

decision-licence = Licencia
decision-terms = Podmienky používania
decision-country = Krajina, ktorej právo platí
decision-reason-confirm = Prečo – čo ste si prečítali a kedy (povinné)
decision-reason-reject = Prečo zamietate (povinné)
decision-by = Rozhoduje: { $name } (meno sa uchováva len na tomto počítači)

share-explain = Zoznam obsahuje len adresy a kódy krajín – nič o vás. ATENA ho nikam neposiela: skopírujte ho a odovzdajte editorovi ATENY, ako sami chcete (napríklad e-mailom). Potom ho označte ako odovzdaný.
share-nothing = Nič nečaká na odovzdanie.
share-list =
    { $count ->
        [one] Zoznam s { $count } adresou
       *[other] Zoznam s { $count } adresami
    }
share-copy = Kopírovať
share-copied = Skopírované
share-mark-handed-over = Označiť ako odovzdané
share-handed-over = Zoznam je označený ako odovzdaný.

receive-explain = Pre editora: vložte zoznam, ktorý vám poslal používateľ inej inštalácie. ATENA každú adresu posúdi sama – prijatému posúdeniu by nebolo čo veriť – a uloží ich ako návrhy.
receive-list = Zoznam (začína riadkom ATENA-NAVRHY 1)
receive-action = Prečítať zoznam
receive-not-a-list = Toto nie je zoznam ATENY: prvý riadok musí byť ATENA-NAVRHY 1.
receive-read-before = Tento zoznam už bol prečítaný ({ $list }).

## Zber

crawl-title = Zber
crawl-intro = ATENA navštevuje zdroje registra a potvrdené návrhy, ukladá nové a zmenené dokumenty a sprístupňuje ich vyhľadávaniu. Zber beží len vtedy, keď ho spustíte.
crawl-state-running = Zber beží
crawl-state-stopping = Zber sa zastavuje – dokončujú sa rozbehnuté kontroly
crawl-state-stopped = Zber je zastavený
crawl-running = zber beží
crawl-start = Spustiť zber
crawl-stop = Zastaviť
crawl-started = spustený { $time }
crawl-checks = kontrol
crawl-hosts = navštívených serverov
crawl-due = adries čaká na kontrolu
crawl-store-errors = chýb databázy
crawl-outcomes = Výsledky kontrol
crawl-rules-title = Ako ATENA zbiera
crawl-rule-robots = Dodržiava robots.txt a pravidlá vydavateľa (TDMRep, noindex, nosnippet).
crawl-rule-pace = Na každý server chodí pomaly, jedna požiadavka za druhou, a keď server požiada o pokoj, počká.
crawl-rule-open = Čerpá len z otvorených zdrojov a uchová len to, čo dovoľuje zákon a licencia.
crawl-rule-privacy = Ide ako anonymný návštevník: bez súborov cookie, bez skriptov a bez obrázkov. O vás nič neposiela.
crawl-rule-safety = Programy a dokumenty s aktívnym obsahom odmieta, dokumenty spracúva v obmedzenom procese a pôvodné súbory neuchováva.
crawl-concurrency =
    { $count ->
        [one] Naraz sa pracuje s { $count } serverom (dá sa zmeniť v nastaveniach).
       *[other] Naraz sa pracuje s { $count } servermi (dá sa zmeniť v nastaveniach).
    }

check-new = nový dokument
check-changed = zmenený dokument
check-unchanged = bez zmeny
check-not-modified = bez zmeny (podľa servera)
check-feed = kanál RSS/Atom prečítaný
check-sitemap = mapa stránok prečítaná
check-robots = robots.txt nedovoľuje
check-robots-unreachable = robots.txt nedostupný
check-blocked = adresa nie je povolená
check-http-error = chyba servera
check-gone = dokument už neexistuje
check-network = sieť nedostupná
check-timeout = vypršal čas
check-redirects = priveľa presmerovaní
check-retry-later = server žiada počkať
check-too-large = príliš veľké
check-unsupported = nepodporovaný druh obsahu
check-dangerous = nebezpečný obsah odmietnutý
check-too-demanding = príliš náročný obsah odmietnutý
check-extract-error = obsah sa nepodarilo spracovať
check-feed-error = chybný kanál RSS/Atom
check-sitemap-error = chybná mapa stránok
check-moved-elsewhere = presunuté na iný web
check-erased = vymazané na žiadosť
check-not-permitted = zásada zákonnosti nedovoľuje
check-invalid-url = neplatná adresa
check-store-error = chyba databázy

## Nastavenia

settings-title = Nastavenia
settings-saved = Uložené.
settings-appearance = Jazyk a vzhľad
settings-language = Jazyk rozhrania
settings-theme = Vzhľad
settings-theme-system = podľa systému
settings-theme-light = svetlý
settings-theme-dark = tmavý
settings-decisions = Rozhodnutia o návrhoch
settings-editor = Vaše meno alebo prezývka
settings-editor-privacy = Zapisuje sa k vašim rozhodnutiam a ostáva len na tomto počítači. Spoločný register žiadnu osobu neuvádza.
settings-share = Odkladať navrhnuté adresy pre editora
settings-share-explain = ATENA nič neodosiela sama; odložené adresy nájdete v Návrhoch → Odovzdať editorovi.
settings-crawler = Zber
settings-concurrency = Počet serverov naraz (1 – 16)
settings-concurrency-explain = Na každý server ide ATENA vždy len jednou požiadavkou naraz; toto číslo hovorí, koľko rôznych serverov navštevuje súčasne.

erasure-title = Žiadosť o vymazanie
erasure-explain = Ak niekto požiada o vymazanie (čl. 17 GDPR), ATENA odstráni všetko o danej adrese a už ju nikdy nenavštívi.
erasure-address = Adresa
erasure-prefix = aj všetky adresy, ktoré ňou začínajú
erasure-reason = Dôvod (napríklad: žiadosť zo dňa…)
erasure-action = Vymazať a zablokovať
erasure-done = Vymazané: { $documents } dokumentov a { $versions } verzií. Adresa je zablokovaná.

about-title = O ATENE
about-version = ATENA { $version } – otvorený softvér pod licenciou Apache 2.0, bezplatný pre súkromné aj komerčné použitie.
about-connections = ATENA nikomu neposiela nič o svojich používateľoch. Sama sa nikam nepripája; do siete ide len vtedy, keď spustíte zber, posúdenie adries alebo synchronizáciu registra, a vtedy ako anonymný návštevník. Nepoužíva služby tretích strán.
about-pillars = Štyri piliere ATENY
pillar-1 = Serióznosť, otvorenosť a snaha posúvať poznanie každého človeka dopredu.
pillar-2 = Anonymita a rešpekt k súkromiu – nášmu aj iných.
pillar-3 = Bezpečnosť a ochrana pred nebezpečnými ľuďmi, robotmi, kódmi a procesmi.
pillar-4 = Dodržiavanie zákona: žiaden zákon nesmie byť porušený, nikde na svete.
about-licence = Licencie všetkých súčastí sú v priečinku aplikácie „notices“.
about-repository = Zdrojový kód na GitHube

## Chyby

error-detail = Podrobnosti
error-already-running = ATENA už beží. Pracujte v otvorenom okne.
error-crawl = Zber sa nepodarilo pripraviť.
error-database = Chyba databázy.
error-database-start = Databázu sa nepodarilo spustiť.
error-database-version = Databázu vytvorila iná verzia PostgreSQL, než akú prináša táto ATENA.
error-decision-country = Pre túto krajinu nemá ATENA právny profil.
error-decision-incomplete = Rozhodnutie potrebuje meno a dôvod.
error-decision-licence = Takúto licenciu register nepozná.
error-decision-official = Úradné texty tejto krajiny nie sú voľné; vyberte jej otvorenú licenciu.
error-decision-terms = Takéto podmienky register nepozná.
error-editor-missing = Rozhodnutie potrebuje vaše meno – zadajte ho v nastaveniach.
error-index = Chyba indexu vyhľadávania.
error-index-foreign = Priečinok indexu obsahuje niečo iné než index ATENY.
error-input-too-long = Text je pridlhý.
error-internal = Nastala neočakávaná chyba.
error-io = Súbor alebo priečinok sa nepodarilo prečítať alebo zapísať.
error-not-a-list = Toto nie je zoznam ATENY.
error-not-found = Nenašlo sa.
error-not-ready = ATENA sa ešte spúšťa.
error-reason-missing = Napíšte dôvod.
error-search = Vyhľadávanie zlyhalo.
error-search-empty = Napíšte, čo hľadáte, alebo vyberte zúženie.
error-search-filter = Takéto zúženie ATENA nepozná.
error-settings = Nastavenia sa nepodarilo uložiť.
error-shared-register-invalid = Spoločný register nie je platný; nič sa nezmenilo.
error-shared-register-unreachable = Spoločný register sa nepodarilo stiahnuť. Skontrolujte pripojenie k internetu.
error-url = Toto nie je platná webová adresa.
error-private-damaged = Súbor súkromného konta je poškodený.
error-private-exists = Súkromné konto na tomto počítači už existuje.
error-private-locked = Súkromná časť je zamknutá – najprv sa prihláste.
error-private-login = Nesprávne meno alebo heslo.
error-private-name = Meno musí mať 1 až 40 znakov.
error-private-none = Súkromné konto ešte nie je vytvorené.
error-private-recovery = Toto nie je záchranný kľúč tohto konta.
error-private-weak = Heslo musí mať aspoň 10 znakov a nesmie sa zhodovať s menom.
error-private-keychain = Správca poverení Windows teraz nie je dostupný. Skúste to znova.
error-private-remember-none = Trvalé prihlásenie je dostupné len vo Windows (Správca poverení).
error-private-library-full = Súkromná knižnica je plná – odstráňte niektoré uložené dokumenty.

## Súkromné / Private

private-group = Súkromné
private-overview-title = Moja ATENA
private-overview-of = Súkromná časť používateľa { $name } – len na tomto počítači.
private-note-title = Súkromné – len pre vás
private-note-text = Táto časť je zašifrovaná vaším heslom a zostáva len na tomto počítači. Úradné a voľne licencované dokumenty sa neukladajú druhýkrát – uloží sa odkaz na verejnú časť ATENY. Ostatné dokumenty sa uložia ako kópia pre vlastnú potrebu.
private-all = Všetko
private-empty = V tejto kategórii zatiaľ nie sú slovenské dokumenty. Spustite zber.
private-search-all = Hľadať v kategórii
private-create-first = Najprv si v Nastaveniach vytvorte súkromné konto.
private-lock = Zamknúť súkromnú časť
private-unlock-title = Prihlásenie do súkromnej časti
private-unlock = Prihlásiť
private-name = Meno
private-password = Heslo
private-remember-title = Zapamätať prihlásenie?
private-remember-text = Ak zvolíte áno, súkromná časť zostane otvorená, kým ATENU nezatvoríte. Ak nie, zamkne sa hneď, ako ju opustíte.
private-remember-yes = Áno, do zatvorenia ATENY
private-remember-no = Nie, zamknúť po odchode
private-remember-forever-title = Trvalé prihlásenie na tomto počítači
private-remember-forever-text = Súkromná časť sa potom otvorí pri každom spustení ATENY bez hesla. Kľúč uloží Správca poverení Windows, chránený vaším prihlásením do Windows; nič neodchádza z tohto počítača. Zrušiť ho môžete kedykoľvek v Nastaveniach.
private-remember-risk = Rozumiem: ktokoľvek sa prihlási do môjho účtu vo Windows, otvorí moju súkromnú časť bez hesla. Trvalé prihlásenie zapínam na vlastnú zodpovednosť.
private-remember-forever = Prihlasovať trvalo
private-remember-on = Trvalé prihlásenie je zapnuté – súkromná časť sa na tomto počítači otvára bez hesla.
private-remember-off = Zrušiť trvalé prihlásenie
private-remember-enable = Zapnúť trvalé prihlásenie
private-account-title = Súkromné konto
private-account-intro = Konto je len na tomto počítači – ATENA nič neposiela na žiadny server. Heslo sa nikde neukladá; odvodí sa z neho kľúč, ktorý šifruje vašu súkromnú časť. Na jednom počítači je jedno konto.
private-password-again = Heslo znova
private-password-rules = Heslo musí mať aspoň 10 znakov a nesmie sa zhodovať s menom. Pri vytvorení dostanete záchranný kľúč – ním si nastavíte nové heslo, ak ho zabudnete.
private-password-mismatch = Heslá sa nezhodujú.
private-create = Vytvoriť súkromné konto
private-account-of = Konto { $name }:
private-state-open = otvorené
private-state-locked = zamknuté
private-change-password = Zmeniť heslo
private-forgot = Zabudli ste heslo?
private-forgot-text = Zadajte záchranný kľúč, ktorý ste dostali pri vytvorení konta, a nové heslo. Vaše údaje zostanú.
private-password-old = Súčasné heslo
private-password-new = Nové heslo
private-set-password = Nastaviť nové heslo
private-password-changed = Heslo je zmenené.
private-recovery-title = Záchranný kľúč
private-recovery-text = Toto je váš záchranný kľúč. Vytlačte si ho alebo ho uložte na bezpečné miesto mimo tohto počítača. Ukáže sa len teraz – bez neho a bez hesla sa súkromná časť otvoriť nedá.
private-recovery-copy = Kopírovať
private-recovery-copied = Skopírované
private-recovery-kept = Záchranný kľúč mám uložený.
private-recovery-done = Pokračovať
private-select = Vybrať: { $title }
private-save-all = Uložiť všetky v zozname ({ $count })
private-save-selected = Uložiť vybrané ({ $count })
private-select-all = Vybrať všetky neuložené
private-view-saved = Uložené
private-view-documents = Dokumenty
private-saved-counts = Spolu { $all } · kópie pre seba { $copies } · odkazy { $references }
private-saved-title = Uložené dokumenty
private-text-chars = { $chars } znakov textu
private-saved-empty = Zatiaľ nemáte uložené žiadne dokumenty. V kategóriách pri dokumente kliknite na „Uložiť pre seba“.
private-text-gone = Text tohto dokumentu už verejná časť nemá. Otvorte originál.
private-remove-confirm = Naozaj odstrániť
private-remove = Odstrániť
private-open-file-note = Súbor sa na čas prezerania dešifruje do priečinka ATENY a zmaže sa pri zamknutí súkromnej časti alebo zatvorení ATENY.
private-open-file = Otvoriť súbor ({ $size })
private-licence = Licencia zdroja: { $licence }
private-saved-on = uložené { $date }
private-save-all-confirm = Uložiť ({ $count })
private-save-all-pace = Kópie sa sťahujú po jednej a v tempe, ktoré servery dovoľujú – môže to trvať niekoľko minút. Medzitým môžete pracovať ďalej.
private-save-all-saved = (Uložených už je { $saved }.)
private-save-all-text = Počet dokumentov na uloženie: { $count }.
private-save-all-title = Uložiť všetky v zozname?
private-saving-counts = Uložené: { $saved } · už boli uložené: { $already } · neuložené: { $refused } (dôvod je pri dokumente)
private-saving-stop = Zastaviť
private-saving-done = Hotovo: spracované { $done } z { $total }
private-saving = Ukladá sa { $done } z { $total }…
private-reason-unreadable = Neuložené: dokument sa nedá prečítať.
private-reason-dangerous = Neuložené: súbor obsahuje aktívny obsah (napríklad skript), z bezpečnostných dôvodov sa neukladá.
private-reason-unavailable = Neuložené: dokument sa nepodarilo stiahnuť.
private-reason-busy = Neuložené: server je teraz zaneprázdnený – skúste to neskôr.
private-reason-robots = Neuložené: súbor robots.txt webu to nedovoľuje.
private-reason-noarchive = Neuložené: vydavateľ si neželá ukladanie kópií.
private-reason-not-allowed = Neuložené: podmienky tohto webu nedovoľujú automatický prístup.
private-reason-gone = Neuložené: dokument už nie je vo verejnej časti ani na webe.
private-save = Uložiť pre seba
private-read = Čítať
private-kind-copy-title = Kópia pre vlastnú potrebu – zašifrovaná len pre vás na tomto počítači.
private-kind-reference-title = Úradný alebo voľne licencovaný dokument – uchováva ho verejná časť ATENY, súkromná časť naň len odkazuje.
private-kind-copy = Kópia pre seba
private-kind-reference = Odkaz
private-copy-right = Kópia pre vlastnú potrebu: podľa § 42 Autorského zákona (č. 185/2015 Z. z.) smie fyzická osoba bez súhlasu autora vyhotoviť rozmnoženinu diela pre svoju súkromnú potrebu, ak nejde o priamo ani nepriamo obchodný účel. ATENA kópiu stiahne z webu vydavateľa až na váš pokyn, rešpektuje jeho pravidlá (robots.txt, zákaz kópií) a uloží ju zašifrovanú len pre vás. Kópie nezdieľajte, nešírte a nepoužívajte na iné účely – autorské práva zostávajú autorom.

# ATENA's archive of original files
facet-archive = Archív ATENY
archive-facet-removed = Odstránené z webu
archive-gone = Vydavateľ dokument odstránil z webu ({ $date }).
archive-gone-text = ATENA ho uchováva vo svojom archíve, lebo ide o dokument, ktorý smie každý kopírovať a zobrazovať ({ $licence }). Zdroj a licencia ostávajú uvedené.
archive-text = Pôvodný súbor uchováva archív ATENY – zostane dostupný, aj keď ho vydavateľ z webu odstráni ({ $licence }).
archive-file = Pôvodný súbor v archíve od { $date } · { $size }
archive-open = Otvoriť pôvodný súbor z archívu
archive-open-size = Otvoriť pôvodný súbor z archívu ({ $size })
archive-gone-hit = Odstránené z webu
archive-gone-hit-title = Vydavateľ dokument odstránil z webu; ATENA ho uchováva vo svojom archíve.
overview-archive = Archív: { $files } pôvodných súborov ({ $size }) · odstránené z webu: { $gone }
settings-keep-originals = Uchovávať pôvodné súbory úradných a voľne licencovaných dokumentov
settings-keep-originals-explain = PDF a dokumenty Word/OpenDocument, ktoré smie každý kopírovať, ostanú dostupné, aj keď ich vydavateľ z webu odstráni. Ukladajú sa len po kontrole nebezpečného obsahu a len ak ich text neobsahuje osobné údaje. Zaberajú miesto na disku.
