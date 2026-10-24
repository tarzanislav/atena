# Running the crawler

*Slovenčina: [sk/SPUSTENIE.md](sk/SPUSTENIE.md)*

## Requirements
* Rust (stable; at least the `rust-version` in `Cargo.toml`)
* PostgreSQL 15 or newer, with a database and a user that owns it:
  ```sh
  createuser atena && createdb -O atena atena
  ```

## Commands
```sh
export ATENA_DATABASE_URL=postgres://atena@localhost/atena

cargo run --release -p atena-ingest -- migrate   # create or update the tables
cargo run --release -p atena-ingest -- sync      # load data/sources.yaml (or: sync path/to/register.yaml)
cargo run --release -p atena-ingest -- run       # crawl until Ctrl+C
cargo run --release -p atena-ingest -- run --once --concurrency 8   # until nothing is due
cargo run --release -p atena-ingest -- fetch https://www.minedu.sk/  # one URL, no database
cargo run --release -p atena-ingest -- legality  # what the legality principle allows per source
cargo run --release -p atena-ingest -- assess navrhy.txt [--json]
                                                 # propose licence and terms for addresses, one per line
cargo run --release -p atena-ingest -- import navrhy.txt   # store as proposals (and keep them for sharing)
cargo run --release -p atena-ingest -- confirm ID --by NAME --reason TEXT
cargo run --release -p atena-ingest -- sync --shared       # load the shared register
                                                 # more: docs/IMPORT-AND-SHARING.md
cargo run --release -p atena-ingest -- forget URL --reason "request of 24 Oct 2026" [--prefix]
                                                 # erasure request: remove and block for good
cargo run --release -p atena-ingest -- index     # bring the search index up to date (--rebuild: anew)
cargo run --release -p atena-ingest -- search citatelska gramotnost --country SK
                                                 # search; more: docs/SEARCH.md
```
`migrate` also runs automatically before `sync` and `run`. Several `run` processes – also on several
machines – may share one database: host leases keep them from contacting a host at the same time.
`ATENA_LOG=debug` shows more detail; `ATENA_WORKER` names the process in host leases. `run` also brings
the search index in `ATENA_INDEX` (default `./atena-index`) up to date every minute and when it stops.

## What a run does
1. leases due hosts (one worker per host at a time) and checks their due endpoints;
2. **feeds** and **sitemaps** add their items as page endpoints (checked every 30 days);
3. **pages** are extracted; text that differs from the current version becomes a new version with
   links and classification suggestions; register pages also enqueue their attachments (PDF, office
   files) and subpages of their own section, and the feeds they announce;
4. the result of every check is written to `fetch_log`.

## Check outcomes (`endpoint.last_outcome`, `fetch_log.outcome`)
| Outcome | Meaning | Next check |
|---|---|---|
| `new`, `changed` | first version, new version | normal interval |
| `unchanged`, `not_modified` | same text / HTTP 304 | normal interval |
| `feed`, `sitemap` | items added as endpoints | normal interval |
| `not_permitted` | the legality principle does not allow crawling the source | normal interval |
| `robots` | robots.txt disallows ATENA | normal interval |
| `robots_unreachable` | robots.txt could not be fetched – nothing may be fetched | 1 h, 4 h, 16 h …, at most the interval |
| `retry_later` | the host asked for rest (`429`/`503`) | as the host asked; the whole host waits |
| `gone` | HTTP 404/410; the document is marked `gone`, its history stays | back-off |
| `http_error`, `network`, `timeout`, `redirects`, `blocked` | failures | back-off |
| `too_large`, `unsupported` | over 50 MiB, or a format ATENA does not read | discovered endpoints are switched off |
| `extract_error`, `feed_error`, `sitemap_error` | damaged content | back-off |

After 6 failed checks in a row a **discovered** endpoint is switched off. Register endpoints are
never switched off; they keep backing off until the source works again.
