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
export ATENA_FILES=/var/lib/atena/files          # file store for originals

cargo run --release -p atena-ingest -- migrate   # create or update the tables
cargo run --release -p atena-ingest -- sync      # load data/sources.yaml (or: sync path/to/register.yaml)
cargo run --release -p atena-ingest -- run       # crawl until Ctrl+C
cargo run --release -p atena-ingest -- run --once --concurrency 8   # until nothing is due
cargo run --release -p atena-ingest -- fetch https://www.minedu.sk/  # one URL, no database
cargo run --release -p atena-ingest -- forget URL --reason "request of 24 Oct 2026" [--prefix]
                                                 # erasure request: remove and block for good
```
`migrate` also runs automatically before `sync` and `run`. Several `run` processes – also on several
machines – may share one database: host leases keep them from contacting a host at the same time.
`ATENA_LOG=debug` shows more detail; `ATENA_WORKER` names the process in host leases.

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
| `robots` | robots.txt disallows ATENA | normal interval |
| `robots_unreachable` | robots.txt could not be fetched – nothing may be fetched | 1 h, 4 h, 16 h …, at most the interval |
| `retry_later` | the host asked for rest (`429`/`503`) | as the host asked; the whole host waits |
| `gone` | HTTP 404/410; the document is marked `gone`, its history stays | back-off |
| `http_error`, `network`, `timeout`, `redirects`, `blocked` | failures | back-off |
| `too_large`, `unsupported` | over 50 MiB, or a format ATENA does not read | discovered endpoints are switched off |
| `extract_error`, `feed_error`, `sitemap_error` | damaged content | back-off |

After 6 failed checks in a row a **discovered** endpoint is switched off. Register endpoints are
never switched off; they keep backing off until the source works again.
