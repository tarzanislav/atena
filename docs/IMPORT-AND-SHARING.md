# Importing sources and sharing proposals

*Slovenčina: [sk/IMPORT-A-ZDIELANIE.md](sk/IMPORT-A-ZDIELANIE.md)*

Users of any ATENA installation may propose sources: web pages and sites, feeds and sitemaps,
documents, OAI-PMH interfaces and DOIs, one per line. Every proposal goes through the legality
principle ([LEGALITY.md](LEGALITY.md)) like the register itself, and every installation's proposals
can reach one place – the shared register of the ATENA repository – without a server of ATENA's own.

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
```
installation ──(addresses only)──> relay: ntfy.sh topic ──(every 30 min)──> GitHub issues
                                     (kept 12 hours)      Actions workflow    (label navrh-zdrojov)
                                            │                                      │
                                            └──────────────> editor's installation <┘
                                                  atena-ingest proposals: assess every address itself,
                                                  store as proposals, the editor decides
                                                             │
                     atena-ingest publish → data/sources.yaml → pull request → the shared register
                                                             │
                     every installation: atena-ingest sync --shared
```
* **What is sent**: the lines the user imported (addresses, DOIs, country codes) – not the
  assessments, not who sent them. Pages about people and addresses the register already lists
  are not sent. Sharing is on by default and off with `ATENA_SHARE=0`.
* **Relay**: a topic on the public [ntfy.sh](https://docs.ntfy.sh/faq/) server – free, no accounts,
  messages deleted after 12 hours, up to 4,096 bytes each (longer lists are split). `ATENA_RELAY`
  points elsewhere, e.g. to an ntfy server in the EU. The topic is public: the lists are not secret.
* **Keeping**: the workflow `.github/workflows/proposals.yml` copies new lists into issues every
  30 minutes, fenced so that GitHub renders nothing of them; at most 50 per run.
* **Editor**: `atena-ingest proposals` reads the relay and the open issues, skips lists already
  received, **assesses every address again** (a received assessment would prove nothing) and stores
  them as proposals `via shared`. `--close` closes the processed issues (with `ATENA_GITHUB_TOKEN`).
  After `confirm`, `atena-ingest publish U-… --id SK25 --authority C1` appends the entry with its
  rights record to `data/sources.yaml`; the pull request and its CI check the register as usual.
* **Everyone**: `atena-ingest sync --shared` loads the shared register from the repository; imports
  the register now lists are taken over by it (`merged`).

## Data protection
The lists contain addresses of educational pages and country codes. The relay (ntfy.sh, run in the
USA) sees the sender's IP address, as every web service does; it keeps messages for 12 hours. The
issues are public. Nothing else about the user is sent. Operators mention the sharing in their
privacy notice and may turn it off.
