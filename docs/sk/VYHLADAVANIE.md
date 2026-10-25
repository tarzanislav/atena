# Vyhľadávanie

*English: [../SEARCH.md](../SEARCH.md)*

Modul `crates/atena-search` vedie fulltextový index dokumentov z databázy
([Tantivy](https://github.com/quickwit-oss/tantivy), licencia MIT) a vyhľadáva v ňom. Zdrojom pravdy
zostáva databáza: index sa z nej odvodzuje a dá sa kedykoľvek zostaviť znova.

## Čo sa smie nájsť – zásada zákonnosti
Index prevezme dokument len do tej miery, do akej to dovoľuje
[ZASADA-ZAKONNOSTI.md](ZASADA-ZAKONNOSTI.md). Rozhoduje funkcia `atena_search::searchable`, ktorá sa
pýta `legality::permission`.

| Dokument | Nájde sa | Hľadá sa v texte | Text sa zobrazí |
|---|---|---|---|
| zaniknutý, zo zdroja vypnutého alebo zamietnutého, označený `noindex` alebo zo zdroja, ktorý ATENA nesmie zbierať (podmienky zakazujú automatický prístup, krajina nemá právny profil) | **nie** | – | – |
| len metadáta: podmienky neoverené alebo si vyhradzujú vyťažovanie, výhrada TDM (TDMRep, zákaz robotov umelej inteligencie), krajina, ktorej zákon o ochrane údajov drží texty doma | podľa názvu, zaradenia, zdroja, krajiny a jazyka | nie | nie |
| chránený autorským právom, podmienky overené, vyťažovanie dovolené v oboch krajinách | áno | áno | **nie** – len názov, zdroj, zaradenie a odkaz na originál |
| úradné texty a otvorené licencie | áno | áno | ukážka (nie pri `nosnippet`); celý text v ATENE (nie pri `noarchive`) |

Text, ktorý sa nesmie zobraziť, sa indexuje, ale **do indexu sa vôbec neuloží**. Uložia sa tam len texty,
ktoré sa zobraziť smú, a to kvôli ukážkam.

## Jazyky
Každá skupina jazykov má vlastné polia pre názov a text a vlastný analyzátor:
1. rozdelenie na slová,
2. vynechanie neprimerane dlhých slov,
3. malé písmená,
4. odstránenie koncoviek (stemmer),
5. odstránenie diakritiky – až po stemmeri, lebo ten potrebuje `ä` či `ách`.

| Skupina | Stemmer |
|---|---|
| slovenčina, čeština, poľština | vlastné ľahké stemmery ATENY (`stem.rs`): odstráni sa najdlhšia ohýbacia koncovka, po ktorej zostanú aspoň tri písmená – `školách`, `školy`, `škole` → `škol` |
| angličtina, nemčina, francúzština, španielčina, taliančina, portugalčina, holandčina, švédčina, dánčina, nórčina, fínčina, maďarčina, rumunčina, ruština, gréčtina, turečtina, arabčina | Snowball (BSD-3-Clause) |
| čínština, japončina, kórejčina | žiadny: súvislý rad znakov sa rozdelí na prekrývajúce sa dvojice (`阅读素养` → `阅读 读素 素养`), ktoré nájdu každé slovo aj bez slovníka |
| ostatné jazyky a neznámy jazyk | žiadny |

Preto `citatelskej gramotnosti` nájde `Čitateľská gramotnosť` a `skoly` nájde `školách`.
- Slová sa hľadajú vo všetkých skupinách; názov váži trikrát viac ako text.
- Musia sa zhodovať všetky slová. Ak žiadny dokument nemá všetky, ukážu sa dokumenty s niektorými
  z nich a výsledok to povie.
- `"…"` hľadá presné spojenie, `-slovo` slovo vylúči.

## Naprieč jazykmi: pojmy taxonómie
Slová sa porovnajú aj s kľúčovými slovami taxonómie (`atena_core::classify`). Dokumenty zaradené
k rozpoznanému pojmu alebo k užšiemu pojmu sa nájdu v ktoromkoľvek jazyku: `čitateľská gramotnosť`
nájde anglické aj čínske dokumenty o čitateľskej gramotnosti. Výsledok vymenuje pojmy, ktoré rozpoznal.
Zaradenie, ktoré redaktor zamietol, sa nepočíta.

## Filtre
`os/pojem`, `country/SK`, `authority/A1`, `language/sk`, `source/SK01`, `region/sk|abroad`,
`trust/official|expert|other`, `archive/removed`.
- **Odstránené z webu** (`archive/removed`, formát indexu 3): dokument, ktorý vydavateľ odstránil
  (HTTP 404/410), sa dá ďalej nájsť v archíve ATENY len tam, kde sa smie zobraziť celý text (úradné
  texty, otvorené licencie, bez `noarchive`); každý iný odstránený dokument z indexu zmizne.
- Hodnoty tej istej osi sú alternatívy.
- Rôzne osi musia platiť všetky.
- Výsledok spočíta hodnoty každej osi medzi nájdenými dokumentmi (fazety).
- Pojem zahŕňa seba, užšie pojmy a to, čo má v taxonómii uvedené v `includes` (pojmy iných osí
  s ich užšími pojmami alebo celú os). Kategória je tak pohľad na fazety, z ktorých sa skladá:
  `section/practice` nájde dokumenty zaradené k `method/jigsaw` aj `resource-type/worksheet`.
- **Slovenské údaje** (`region/sk`) sú dokumenty slovenských zdrojov a dokumenty v slovenčine,
  nech sú odkiaľkoľvek; ostatné sú **zahraničné** (`region/abroad`).
- **Dôveryhodnosť zdroja** (`trust`): úradné (A1, A2), odborné (B1, B2, C1), ostatné (C2, D, E).

## Počty v Prehľade
Pri každej kategórii sa ukazuje **nové / všetky**: nové sú dokumenty, ktoré ATENA prvýkrát videla za
posledných 7 dní. Počty platia pre zvolené údaje (Slovensko, alebo Zahraničie a jazyk).

## Nové zaradenie po zmene taxonómie
Keď nová verzia ATENY prinesie zmenenú taxonómiu (nové kategórie, kľúčové slová), aplikácia na pozadí
znova zaradí už uložené dokumenty. Vidí pritom presne to, čo videl zberač a smel uchovať (názov
a uchovaný text). Mení len automatické návrhy, rozhodnutia človeka zostávajú.

## Poradie výsledkov
Zhoda × autorita zdroja × aktuálnosť.

| Autorita | A1 | A2 | B1 | B2 | C1 | C2 | D | E |
|---|---|---|---|---|---|---|---|---|
| váha | 1,5 | 1,4 | 1,3 | 1,25 | 1,1 | 1,05 | 1,0 | 0,8 |

Aktuálnosť: najnovšie dokumenty sú až o štvrtinu vyššie a výhoda sa každý rok zmenší na polovicu.
Počíta sa od dátumu úpravy alebo zverejnenia dokumentu, inak od poslednej zmeny, ktorú ATENA videla.
Pedagogické smery sa nikdy neradia vyššie ani nižšie – váhu majú len zdroje.

## Ako sa index udržiava aktuálny
Spúšťače v databáze (migrácia `0006_search_queue.sql`) zaradia dokument do fronty `search_queue` vždy,
keď sa mohlo zmeniť niečo, čo vyhľadávanie ukazuje:
- nový dokument alebo nová verzia,
- text zredukovaný na metadáta,
- pravidlá vydavateľa,
- rozhodnutie o zaradení,
- výmaz,
- zmena názvu, krajiny, autority, licencie, podmienok alebo stavu zdroja.

Kontrola, ktorá stránku len potvrdí, nezaradí nič. Indexer vezme dokumenty z fronty, zapíše ich alebo
odstráni, potvrdí zápis (commit) a až potom ich z fronty vyberie, pokiaľ sa tam medzitým nedostali znova.
Index tak nemôže zaostať za databázou.

```
atena-ingest index              # aktualizovať index (atena-ingest run to robí každú minútu)
atena-ingest index --rebuild    # zostaviť ho celý znova (po aktualizácii ATENY, ktorá mení formát indexu)
atena-ingest search citatelska gramotnost --lang sk
atena-ingest search zápis --country SK --label language/sk --limit 20 --page 2
atena-ingest search "školský poriadok" --facets
atena-ingest search inklúzia --json
```

`ATENA_INDEX` je priečinok indexu (predvolene `./atena-index`). ATENA doň zapíše značkový súbor a otvorí
alebo zmaže len priečinok s touto značkou (alebo prázdny). Priečinok uvedený omylom nikdy nezmaže.
Na vyhľadávanie netreba databázu.

## Testy
`cargo test -p atena-search` overí analyzátory a stemmery, právne obmedzenia, pojmy naprieč jazykmi,
filtre, poradie a index na disku. S `ATENA_TEST_DATABASE_URL` overí aj spúšťače a aktualizácie zo
skutočnej databázy.
