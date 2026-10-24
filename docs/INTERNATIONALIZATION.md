# Internationalisation and country packs

ATENA is built to be used in any country and language. Slovak (sk-SK) is the first locale and Slovakia
the first country pack; Czech (cs-CZ) is next. Nothing country-specific is hard-coded in the core.

## Three independent language settings
| Setting | Meaning | Example |
|---|---|---|
| UI language | language of menus and labels (Project Fluent files) | sk, cs, en, de, pl, hu… |
| Target language | language into which foreign content is translated | sk for a Slovak teacher, cs for a Czech one |
| Untranslated languages | languages the user reads without translation | a Slovak user: sk, cs |

Dates, numbers, plural forms and sorting follow the locale (ICU4X), e.g. „24. októbra 2026“ vs „24. října 2026“.

## Country packs
Everything that differs between education systems lives in a **country pack** (data, not code):

| Part | Slovakia (first pack) | Czechia (next) |
|---|---|---|
| School types → ISCED 2011 | MŠ, ZŠ 1./2. stupeň, gymnázium, SOŠ, konzervatórium, ŠVVP, ZUŠ, spojená škola… | MŠ, ZŠ, gymnázium, SOŠ, SOU, konzervatoř, VOŠ, ZUŠ… |
| Curriculum framework | ŠVP, cycles, learning areas, performance standards | RVP ZV / RVP G, expected outcomes |
| Legislation connector | Slov-Lex (time-versioned acts) | e-Sbírka |
| Official sources | ministry, NIVaM, ŠŠI, ŠIOV, Učíme… | MŠMT, Edu.cz, NPI, ČŠI… |
| School-year calendar | holidays by region, POP deadlines | holidays by district, deadlines |
| Day panel extras | name days, public holidays | name days, public holidays |

A user picks a country pack (or several); toggles of school types, the dashboard and alerts follow it.
Foreign packs are always shown as "foreign context", never as an obligation for the user's school.

## Language-neutral data
* Concepts, school types, sources and sections have **stable English IDs** (`reading-literacy`,
  `lower-secondary`, `sk.nivam`); every language is just a label. The Python prototype still uses Slovak IDs;
  they are migrated when the Rust core reads `data/`.
* Search folds diacritics and matches across languages through the taxonomy labels, so a Czech query finds
  Slovak, Polish or German material about the same concept.

## Documentation languages
* English: code, identifiers, commit messages, technical documentation, API.
* Slovak: reference UI locale, user guide, project planning with the author (`docs/sk/`).
* Further UI locales and user guides are added per country pack.
