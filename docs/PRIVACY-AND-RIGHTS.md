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
| **E-mail addresses and telephone numbers are removed** from texts, titles, descriptions and link texts before anything is fingerprinted or stored (also obfuscated forms such as `name(at)school.sk`) | `atena-ingest/src/privacy.rs` |
| **No originals of web pages** – only the cleaned text. Originals are kept only for PDF/office documents whose text may be shown in full (official texts, open licences) | `legality::permission` |
| **Pages about people are not followed**: photo galleries, staff lists, contacts, profiles, accounts | `urls::is_personal_page` |
| **Authors of web pages are not stored** (usually the webmaster); authors of documents are | pipeline |
| **Erasure on request** (Art. 17): `atena-ingest forget URL --reason …` removes documents, versions, links, check records and unshared originals and blocks the URL (or everything below a prefix) for good | `Store::forget`, `blocked_url`, `erasure_log` |
| **Storage limitation**: the check log is deleted after 90 days | worker |
| No images, no cookies, no tracking of visitors of the sites | crawler |
| **Sharing of proposals** sends only the imported addresses and country codes – no assessments, no user data; pages about people are never sent or stored; `ATENA_SHARE=0` turns it off ([IMPORT-AND-SHARING.md](IMPORT-AND-SHARING.md)) | `atena-ingest/src/share.rs` |

Names inside texts (e.g. "the head teacher Mgr. J. Novák") are not removed automatically; they are
part of the published official information. A person who objects can have a document or a whole
section erased with `forget`.

**Legal basis** to be confirmed by the operator: legitimate interest (Art. 6(1)(f)) in providing
verified information about education, with the balancing test documented in the DPIA. The operator
publishes a privacy notice (Art. 14) naming a contact for objections and erasure requests.

## 4. Operator checklist before a public launch
1. Keep the rights review of `data/sources.yaml` current – done for all sources on 24 Oct 2026; repeat it
   at least yearly and before launch in a new country.
2. DPIA and legal review; record of processing activities (Art. 30).
3. Privacy notice and a contact address for erasure requests; process for handling them with `forget`.
4. The user interface shows full text only for `official` and openly licensed sources and respects
   `noindex`, `noarchive`, `nosnippet` and `notranslate`.
