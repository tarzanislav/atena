# Lawful by design: content rights and personal data

*Slovenčina: [sk/PRAVA-A-OCHRANA-UDAJOV.md](sk/PRAVA-A-OCHRANA-UDAJOV.md)*

**Project rule (set by the project author):** ATENA draws only on open sources. Nothing it does may
infringe the law, misuse data or breach the GDPR. This page describes how the code enforces that.
It is not legal advice: before a public launch, have the setup reviewed by a lawyer and carry out a
data protection impact assessment (DPIA, GDPR Art. 35).

## 1. Which sources
* Only content that is **publicly accessible without signing in**. ATENA never logs in, never
  bypasses paywalls, CAPTCHAs or other access controls, and does not run JavaScript.
* **robots.txt** (RFC 9309) is obeyed; if it cannot be fetched, nothing is fetched.
* A **reservation of text and data mining** (TDMRep, Art. 4 of Directive (EU) 2019/790) is obeyed:
  only metadata is kept (title, address, dates, fingerprints).
* A site that **refuses AI crawlers in robots.txt** (GPTBot, CCBot, ClaudeBot, Google-Extended …)
  reserves mining too: its pages are kept as metadata only, although ATENA itself may fetch them.
* **Terms of use**: each source has `terms` in the register – `allowed` (read; neither forbidden nor
  reserved), `forbidden` (no automated access: not crawled at all, e.g. YouTube or UNESCO), `reserved`
  (mining reserved: metadata only) or `unknown` (not verified: metadata only). Every value but
  `unknown` is backed by a recorded review ([SOURCE-RIGHTS.md](SOURCE-RIGHTS.md)).

## 2. What may be shown
Each source has a `licence` in the register:

| Licence | Examples | Shown to users |
|---|---|---|
| `official` | laws, decrees, official documents in legal databases (Slov-Lex, e-Sbírka, RIS, Nemzeti Jogszabálytár, EUR-Lex) – not protected by copyright (e.g. § 5 of the Slovak Copyright Act) | full text |
| open licence (`CC0-1.0`, `CC-BY-4.0`, `CC-BY-SA-4.0`, IGO variants, ODC, OGL, `etalab-2.0`) | open educational resources, open data | full text, with attribution as the licence requires |
| `copyrighted` (**default**) | everything not verified | title, metadata, ATENA's classification and a link to the original – no full text |

Keeping the text of copyrighted content (for change detection and classification) is decided by the
legality principle ([LEGALITY.md](LEGALITY.md)): only with verified terms of use, where both the
operator's and the source country's law have a mining exception covering ATENA, and without a
publisher's reservation – otherwise only metadata is kept. Until a source's licence and terms are
verified, it is treated as `copyrighted` with `unknown` terms.

## 3. Personal data (GDPR)
School and authority pages name people – teachers, head teachers, officials – and give their contact
details. ATENA's purpose is information about education, not about people, so it keeps as little
personal data as possible:

| Measure | Where |
|---|---|
| **E-mail addresses, telephone numbers and birth numbers are removed** from texts, titles, descriptions and link texts before anything is fingerprinted or stored – also international domains, written-out forms (`name (at) school (dot) sk`), Russian and Chinese numbers and any number after a label such as `tel.`, `mobil`, `телефон` or `电话` | `atena-ingest/src/privacy.rs` |
| **The public register names no person**: a decision published to `data/sources.yaml` says "decided by the ATENA editor"; the editor's name stays in their own installation | `atena-ingest publish` |
| **No original files at all** – no web pages, PDFs or office documents; only the cleaned text and a fingerprint of the original | pipeline, migration 0007 |
| **No programs and no active content**: programs, disguised files, PDFs with JavaScript, launch actions or embedded files and office documents with macros, ActiveX, OLE objects or remote templates are refused as a whole – nothing of them is kept | `atena-extract/src/danger.rs` |
| **Pages about people are not followed**: photo galleries, staff lists, contacts, profiles, accounts | `urls::is_personal_page` |
| **Authors of web pages are not stored** (usually the webmaster); authors of documents are | pipeline |
| **Erasure on request** (Art. 17): `atena-ingest forget URL --reason …` removes documents, versions, links and check records and blocks the URL (or everything below a prefix) for good | `Store::forget`, `blocked_url`, `erasure_log` |
| **Storage limitation**: the check log is deleted after 90 days | worker |
| No images, no cookies, no tracking of visitors of the sites | crawler |
| **Search** never finds a document marked `noindex` or of a source ATENA may not collect; snippets only where the text may be shown, and only such texts are stored in the index; an erased document leaves the index with the next update ([SEARCH.md](SEARCH.md)) | `atena_search::searchable`, `search_queue` |
| **No third parties**: ATENA sends nothing about its users to anyone; by itself it connects only to its sources (as an anonymous visitor) and to the operator's own server, which stores no IP addresses – no relays, analytics, or fonts and libraries from other servers | `atena_core::legality::CONNECTIONS` |
| **Sharing of proposals** travels as list files the users hand over themselves – only the imported addresses and country codes, no assessments, no user data; pages about people are never written into a list; `ATENA_SHARE=0` keeps imports to the installation ([IMPORT-AND-SHARING.md](IMPORT-AND-SHARING.md)) | `atena-ingest/src/share.rs` |

Names inside texts (e.g. "the head teacher Mgr. J. Novák") are not removed automatically; they are
part of the published official information. A person who objects can have a document or a whole
section erased with `forget`.

**Legal basis** to be confirmed by the operator: legitimate interest (Art. 6(1)(f)) in providing
verified information about education, with the balancing test documented in the DPIA. The operator
publishes a privacy notice (Art. 14) naming a contact for objections and erasure requests.

## 3a. The private part of the desktop app
* One local account per installation (`atena-vault`): no server, no e-mail, no recovery through a
  third party. The password is never stored; Argon2id (64 MiB, 3 passes) derives the key that
  unwraps a random 256-bit data key, which encrypts the private data with XChaCha20-Poly1305. The
  data key is also wrapped by a random 160-bit recovery key shown once (`ATENA-XXXX-…`).
* A wrong name and a wrong password give the same answer; the key is wiped from memory when the
  part is locked or ATENA closes.
* The private part exists for **copies for the user's own use** – private use by a natural person
  under copyright law (in Slovakia, the private copy exception of Act 185/2015). The copies are made
  on the user's click, stay encrypted on the user's computer and are never shared or sent.
  Sources whose terms forbid automated access are excluded. This assessment is not legal advice and
  is to be confirmed by a lawyer before a public launch (checklist below).

## 4. Operator checklist before a public launch
* Have a lawyer confirm the private copy feature of the desktop app (section 3a).
1. Keep the rights review of `data/sources.yaml` current – done for all sources on 24 Oct 2026; repeat it
   at least yearly and before launch in a new country.
2. DPIA and legal review; record of processing activities (Art. 30).
3. Privacy notice and a contact address for erasure requests; process for handling them with `forget`.
4. The user interface shows full text only for `official` and openly licensed sources and respects
   `noindex`, `noarchive`, `nosnippet` and `notranslate`.
