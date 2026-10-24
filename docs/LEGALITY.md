# The legality principle

*Slovenčina: [sk/ZASADA-ZAKONNOSTI.md](sk/ZASADA-ZAKONNOSTI.md)*

> **ATENA draws only on open sources. Nothing it does may infringe the law, misuse data or breach
> data protection – in Slovakia, in the European Union or in any other country: America, China,
> Russia or anywhere else. Both the law of the place where ATENA runs and the law of the country each
> source comes from apply, and the stricter of the two always wins. Where it is not certain that
> something is allowed, ATENA does not do it.**

This principle, set by the project author, stands above every other rule of the program and is
written into its core: `crates/atena-core/src/legality.rs`. Every decision about what may be fetched,
kept and shown goes through `legality::permission`; there is no other path.

## Security beside legality: whom ATENA connects to
> **ATENA sends nothing about its users to anyone. By itself it connects only to the sources it draws
> from – as an anonymous visitor that runs no scripts, loads no images and keeps no cookies, so no
> tracking or advertising service learns of it – and to the operator's own server, which stores no IP
> addresses. No third-party services stand in between: no relays, no analytics, no fonts or
> libraries from other servers. Anything else happens only on a user's explicit command.**

The project author set security as a priority equal to legality (24 Oct 2026). Every server sees the
address of whoever connects to it – an answer needs somewhere to go – so the rule decides whom ATENA
connects to and what it sends. It is written into the core as `legality::CONNECTIONS`. Proposals of
sources therefore travel as files people hand over themselves
([IMPORT-AND-SHARING.md](IMPORT-AND-SHARING.md)); the user interface will carry its fonts and icons
itself.

## How it works
1. **Every country has a legal profile** (text and data mining, official texts, data protection).
   The register refuses a source from a country without one.
2. **Two laws at once**: the operator's (Slovakia/EU) and the source country's; only what both allow.
3. **Uncertain means forbidden**: copyrighted content is kept as text only with **verified** terms of
   use (`terms: allowed`), where **both** laws have a mining exception covering ATENA's purpose, and
   without a publisher's reservation – in TDMRep, in the terms of use (`terms: reserved`), or by
   refusing AI crawlers in robots.txt. (German courts have not settled whether a reservation written
   only in the terms of use counts – OLG Hamburg, Dec 2025, left it open for the future – so the terms
   must be read, and every such reservation is respected.)
4. **Full text and full translations** only for official texts free under both laws and for openly
   licensed content. Original files are never kept, whatever the licence ([DATABASE.md](DATABASE.md)).
5. **Strict data protection** (China – PIPL, Russia – 152-FZ, South Korea – PIPA treated as strict):
   metadata only, even for official and open texts.
6. **Retroactive**: every `atena-ingest sync` reduces sources to metadata, or removes them completely,
   when the principle no longer allows what was kept.
7. **Tests** check the invariants for every country and every combination of licence, terms and
   reservation, and that every country of the taxonomy has a profile.
8. **Sources proposed by users** are assessed automatically, but nothing is crawled until a person
   confirms the proposal; the decision is checked and then `permission` applies to it like to any
   register entry ([IMPORT-AND-SHARING.md](IMPORT-AND-SHARING.md)).
9. **Search** finds nothing ATENA may not collect or the publisher marked `noindex`; it searches text
   only where ATENA may keep it and shows snippets only where the text may be shown – that text alone is
   stored in the index ([SEARCH.md](SEARCH.md)).

## Country profiles
| Country | Mining of copyrighted content | Official texts free | Personal data | Copyrighted content |
|---|---|---|---|---|
| SK, CZ, PL, DE, AT, HU, FI, EE, NL, FR, SE, IT, EU | yes (Directive 2019/790 Art. 4, with opt-out) | yes | GDPR | text with verified terms |
| IE | yes | **no** (state copyright) | GDPR | text with verified terms; official texts need an open licence |
| WORLD (international organisations) | operator law | **no** | GDPR | text with verified terms; documents need an open licence |
| GB-ENG, GB-SCT | **non-commercial research only** (CDPA s. 29A) | **no** (Crown copyright; OGL) | UK GDPR | metadata only unless openly licensed |
| US | **no** (fair use is case by case) | federal works only (17 U.S.C. § 105) | state laws | metadata only unless federal official or open |
| CA, AU | **no** | **no** (Crown copyright) | PIPEDA / Privacy Act | metadata only unless openly licensed |
| NZ | **no** | yes (s. 27) | Privacy Act 2020 | metadata only unless official or open |
| JP | yes (Art. 30-4) | yes (Art. 13) | APPI | text with verified terms |
| SG | yes (s. 244) | **no** | PDPA | text with verified terms |
| KR | **no** | yes (Art. 7) | PIPA (strict) | **always metadata only** |
| CN | **no** | yes (Art. 5) | PIPL (strict) | **always metadata only** |
| RU | **no** | yes (Art. 1259) | 152-FZ (processing in Russia only) | **always metadata only** |

The profiles are conservative summaries, not legal advice. Anyone may make a profile **stricter**;
making one more permissive requires a legal review recorded in the pull request.

## Today
Every source went through a rights review on 24 Oct 2026; each entry of `data/sources.yaml` records
the page that was read and what it says ([SOURCE-RIGHTS.md](SOURCE-RIGHTS.md)).
`atena-ingest legality` prints the decision for every source. Of 186 sources:
* **158 may be crawled**; 28 are not, because their terms forbid automated access or allow access only
  for personal use (UNESCO, YouTube, Edutopia, PBS LearningMedia, CommonLit, OER Commons, CORE …);
* **114 may keep texts** for change detection and classification – never shown in full;
* **22 may be shown in full**: legal databases, US federal works, and openly licensed content
  (CC BY, CC BY-SA, CC0, OGL, Licence Ouverte).
