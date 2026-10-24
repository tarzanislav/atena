# Importing sources and sharing proposals

*Slovenčina: [sk/IMPORT-A-ZDIELANIE.md](sk/IMPORT-A-ZDIELANIE.md)*

Users of any ATENA installation may propose sources: web pages and sites, feeds and sitemaps,
documents, OAI-PMH interfaces and DOIs, one per line. Every proposal goes through the legality
principle ([LEGALITY.md](LEGALITY.md)) like the register itself, and every installation's proposals
can reach one place – the shared register of the ATENA repository – without any third party.

## In one installation
```
atena-ingest import navrhy.txt [--country SK]   # assess and store as proposals
atena-ingest imports                            # proposals with their automatic assessment
atena-ingest confirm U-1a2b3c4d5e --by "Meno" --reason "podmienky prečítané 24. 10. 2026"
atena-ingest reject  U-1a2b3c4d5e --by "Meno" --reason "nejde o vzdelávanie"
```
1. **Assessment** – each line is read as described in [ASSESSMENT.md](ASSESSMENT.md): robots.txt,
   TDMRep, AI-crawler refusals, licence, terms of use. The result is a proposal: licence, terms and
   a rights record quoting what was found. Pages about people are not stored at all.
2. **Proposal** – the source is stored as `proposed`: disabled, `copyrighted`, `unknown`. Nothing is
   crawled and nothing is kept, whatever the proposal says.
3. **Decision** – a person confirms the proposal (or corrects licence, terms or country) with a
   reason. The decision is checked: a country with a legal profile, valid values, `official` only
   where the law frees official texts. Then `legality::permission` decides: crawled or not, texts
   kept or metadata only, shown in full or not. A rejection removes everything collected.
4. **Later imports never change a decision**; they are kept as further assessments.

Erasure requests (`forget`) cover proposals too: assessments are deleted, the address removed and
blocked.

## Between installations: the shared database
ATENA sends nothing about its users to anyone and uses no third-party services
(`atena_core::legality::CONNECTIONS`, [LEGALITY.md](LEGALITY.md)). Proposals therefore travel as files
that people hand over themselves:
```
installation: atena-ingest import …        the addresses are kept in the outbox
              atena-ingest share navrhy.txt  … and written into a list file
                       │
                       │  the user hands the file to the editor as they choose
                       ▼
editor:       atena-ingest proposals navrhy.txt   assess every address itself, store as proposals
              atena-ingest confirm … / reject …   decide
              atena-ingest publish U-… --id SK25 --authority C1
                                                   → data/sources.yaml → pull request → shared register
                       │
every installation:    atena-ingest sync --shared  (only on this explicit command)
```
* **What a list contains**: the lines the user imported (addresses, DOIs, country codes) – not the
  assessments, nothing about who imported them. Pages about people and addresses the register
  already lists are left out. `ATENA_SHARE=0` or `import --no-share` keeps imports to the installation.
* **A list file** starts with the line `ATENA-NAVRHY 1`, then one address per line – plain text anyone
  can read before handing it over. The desktop app shows the same list for copying and the editor
  pastes received lists into it ([DESKTOP.md](DESKTOP.md)).
* **The editor** reads list files with `atena-ingest proposals FILE…`, skips lists read before,
  **assesses every address again** (a received assessment would prove nothing) and stores them as
  proposals `via shared`. After `confirm`, `atena-ingest publish` appends the entry with its rights
  record to `data/sources.yaml`; the pull request and its CI check the register as usual.
* **Everyone**: `atena-ingest sync --shared` downloads the shared register from the repository – only
  when a user gives this command, and only over `https`. Imports the register now lists are taken over by it (`merged`).
* **Later**: when the operator runs a server of its own in the EU, which stores no IP addresses,
  installations will send their lists there directly.

## Data protection
A list contains addresses of educational pages and country codes, nothing about the user. ATENA
sends it nowhere by itself; the user decides how it reaches the editor. Downloading the shared
register on command contacts GitHub, just as downloading ATENA itself does.
