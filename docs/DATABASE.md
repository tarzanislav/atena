# Database

*Slovenčina: [sk/DATABAZA.md](sk/DATABAZA.md)*

ATENA keeps everything in **PostgreSQL** (15 or newer) and **no original files** (see below). The
database is the source of truth; search indexes are derived from it. All SQL lives in
`crates/atena-store`; other crates call its `Store` API.

## Tables
| Table | Holds |
|---|---|
| `source` | the source register (`data/sources.yaml`), kept in sync; sources that leave it are disabled, never deleted. Imported sources (`origin = 'import'`) are `proposed` and disabled until a person confirms them |
| `assessment` | rights assessments of imported sources: the automatic one (with its evidence as JSON) and the decisions of people |
| `inbox` / `outbox` | lists of proposed addresses received from other installations / waiting to be shared |
| `host` | one row per host name: when it is next due, who holds its lease, back-off after `429`/`503` |
| `endpoint` | every URL that is checked: feeds, sitemaps and pages; schedule, validators (`ETag`, `Last-Modified`), failures; how many more levels of links are followed (`depth`, migration 0008) |
| `fetch_log` | one row per check (outcome, status, size, time); pruned after 90 days |
| `document` | one per page endpoint: current title, language, status, the publisher's rules, current version |
| `document_version` | a new row only when the **text** changes: change kind, paragraphs added/removed, number/date changes, text, metadata |
| `document_link` | links in the current version (attachments, subpages) and videos (`kind = 'video'`: YouTube and Vimeo videos the page links to or embeds – recorded, never fetched) |
| `document_label` | classification: automatic suggestions with evidence, and editors' decisions |
| `blocked_url` | URLs (or prefixes) that are never fetched again – erasure requests |
| `erasure_log` | what an erasure removed and why, without the removed content |
| `search_queue` | documents the search index must update, filled by triggers – see [SEARCH.md](SEARCH.md) |

## Scheduling: host leases
Politeness is per host, so the host is the unit of work. A worker **leases** due hosts
(`FOR UPDATE SKIP LOCKED`), checks the due endpoints of each host one after another and releases it; the
host's `next_due_at` is then recomputed from its earliest endpoint. Consequences:
* any number of workers, also in several processes or machines, never send one host two requests at once;
* a host with 50,000 sitemap entries cannot starve the other hosts – every host gets its turn;
* a crashed worker loses nothing: its lease expires and another worker takes the host over;
* scheduling uses the database clock (`now()`), so the workers' clocks do not matter.

## Versions
A capture whose bytes equal the previous one, or whose normalised text has the same fingerprint, only
updates `last_seen_at`. A new text becomes the next version in one transaction with its links and the new
classification suggestions. Kinds: `initial`, `minor`, `substantive` (new or removed paragraphs, at least
40 changed characters, or any changed number or date) and `changed` (text changed, but the previous text
was not kept).

## The publisher's rules
* **TDM reservation** (TDMRep): only metadata is kept – title, URL, dates, fingerprints. No text, no
  links; classification uses the title only. When a reservation appears later, texts,
  links and automatic suggestions kept until then are removed.
* `noindex`, `noarchive`, `nosnippet`, `notranslate` are stored with the document; search and the user
  interface respect them (a `noarchive` copy is kept for change detection but never shown; a `noindex`
  document is never found, a `nosnippet` one never shows a snippet – see [SEARCH.md](SEARCH.md)).

## No original files
The author decided on 24 Oct 2026 that ATENA keeps **no original files** – no web pages, PDFs or
office documents: a file may carry harmful content or personal data that cannot be removed from it
reliably. Every version keeps the cleaned text (where the legality principle allows) and the BLAKE3
hash of the original bytes (`raw_hash`), which proves what the publisher served; the original itself
stays with its publisher, one click away. Migration 0007 removed the former file store; files left
by earlier versions in `./atena-files` may be deleted.

## Size
Version texts are compressed by PostgreSQL (LZ4 where the server supports it). For 2 million documents
the estimate in [TECHNOLOGY.md](TECHNOLOGY.md) applies: about 85 GB in the database.

## Migrations
`crates/atena-store/migrations/NNNN_name.sql`, applied in order by `atena-ingest migrate` (embedded in
the program). An applied migration is never edited – changes come as a new file.

## Tests
`ATENA_TEST_DATABASE_URL` points to a server where tests may create databases, e.g.
`postgres://postgres@localhost/postgres`. Each test creates its own database and drops it afterwards.
Without the variable the database tests are skipped; CI runs them against PostgreSQL 17.
