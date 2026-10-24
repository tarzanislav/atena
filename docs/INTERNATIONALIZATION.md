# Internationalisation and country packs

ATENA is built to be used in any country and language. Nothing country-specific is hard-coded in the core.

## Core languages from the start
| Language | Code | Role |
|---|---|---|
| Slovak | sk-SK | reference locale, first country pack |
| English | en | technical language, international default |
| Czech | cs-CZ | second country pack |
| Russian | ru | UI and translation target; Russian sources as foreign context |
| Chinese (Simplified) | zh-Hans | UI and translation target; Chinese sources as foreign context; Traditional (zh-Hant) later |

All five are complete in the UI, the taxonomy labels, the glossary and search before a release; a release
is blocked if any core locale has missing strings (checked in CI). Further languages (de, pl, hu…) follow
the same gate. UI language and country pack are independent: a Russian-speaking teacher in Slovakia uses the
Russian UI with the Slovak pack.

### Reference, pivot and core languages
* **Reference locale – Slovak (one only).** Every UI message is created and approved in Slovak first; it is the
  single source of truth for message IDs and meaning, and the CI completeness check compares all locales to it.
  Two or more references would conflict whenever their texts differ.
* **Pivot locale – English.** Kept in lockstep with Slovak so that translators of any further language work
  from English.
* **Core locales – Czech, Russian, Chinese (Simplified).** Equal status with Slovak and English: always complete,
  reviewed and release-blocking. A UI string translated by AI stays marked as a draft until a native speaker
  approves it.

### What the five languages require
* **Search:** diacritics folded for sk/cs, `ё → е` for Russian, Chinese segmented into words (jieba-rs, MIT)
  with a per-character fallback (ALTAIS indexes each character as a word; ATENA adds word segmentation).
* **Fonts:** Latin Extended, Cyrillic and Simplified Chinese (Noto Sans SC, OFL) in the app bundle.
* **Translation models:** EuroLLM covers sk, cs, en, ru, zh; Qwen3 is the stronger choice for Chinese.
* **Plural rules:** Slovak, Czech and Russian have several plural forms – handled by Fluent, never by string concatenation.
* **Legal sources:** Russian and Chinese official documents are always labelled "foreign context".

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
