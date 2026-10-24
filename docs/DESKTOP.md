# The desktop app

*Slovenčina: [sk/APLIKACIA.md](sk/APLIKACIA.md)*

The desktop app is ATENA for one person on one computer: search, sources, proposals and the
crawler in a window, without a server and without installing anything else. It is built from
two parts:

| Part | What it is |
|---|---|
| `crates/atena-app` | the backend without a window: the database inside the app, every operation the interface may call, the errors it shows; tested without a window |
| `apps/desktop` | the window: Tauri 2 with a React interface; it passes calls to `atena-app` and decides nothing itself |

## The database inside the app

The app brings the PostgreSQL programs with it (`EmbeddedPostgres`). The first start creates a
database cluster in the user's own application folder; every start runs it, every end stops it.

| Protection | How |
|---|---|
| only this computer | `listen_addresses = '127.0.0.1'`, no Unix socket |
| only ATENA | one user, `atena`, with a random 256-bit password kept next to the data (on Unix with mode `0600`; on Windows the user's own application folder is closed to other users) |
| nothing else is let in | `pg_hba.conf` holds one rule: database `postgres`, user `atena`, `127.0.0.1/32`, `scram-sha-256` |
| no collected text in logs | `log_min_error_statement = panic`: an error is logged without the statement that caused it |
| a new port each start | a port is chosen among the free ones; nothing is fixed that another program could expect |
| one app per folder | `atena.lock` is locked while the app runs: a second app would stop the first one's database |
| a crash leaves no harm | a server left running is stopped and started anew; host leases of the crashed crawler are given back |
| no surprise upgrades | a database made by another major version of PostgreSQL is not opened (`database-version`) |

Developers may use a database of their own instead: `Start::database_url`.

## What happens at a start

1. The folder is locked, PostgreSQL starts (or the given database is used).
2. Migrations bring the schema up to date.
3. The register that came with this version of the app is synced when it differs from the one
   synced last time (`register.synced` holds its fingerprint); the legality principle applies
   retroactively, as with `atena-ingest sync`.
4. The search index is opened (rebuilt when it was made by another version) and kept fresh: every
   minute, after the crawler stops and after an erasure or a rejection.

## Connections

The app follows `atena_core::legality::CONNECTIONS`. It connects to nothing by itself. Only the
user's commands reach the network:

| Command | Connects to |
|---|---|
| start the crawler | the sources of the register and of confirmed imports, as an anonymous visitor |
| import addresses, read a list | those addresses (and their robots.txt and terms of use) |
| sync the shared register | `raw.githubusercontent.com` (the register file of the ATENA repository), over https only |
| open a result | the user's own browser opens it |

Sharing proposals sends nothing: the app shows the list (addresses and country codes only) for
the user to copy and hand to the editor as they choose.

## Operations

Every operation is a method of `App`; the window exposes each as a command of the same name.
Inputs are checked in `atena-app` whatever the interface sent: pasted text at most 1 MB, a reason
of a decision at most 2,000 characters and required, country codes only those with a legal
profile, addresses only `http`/`https`, settings cleaned and bounded.

| Method | What it does |
|---|---|
| `overview` | numbers: sources, documents, versions, searchable documents, checks, proposals, lists waiting |
| `sources` | every source with what the legality principle allows and why not more (codes) |
| `taxonomy(language)` | schemes and concepts with labels in the interface language |
| `search(query)` | a page of 20 hits; snippets come as pieces with marked words |
| `import(text, country)` | assesses addresses or DOIs and stores them as proposals |
| `proposals(state)` | imported sources with the newest automatic proposal |
| `confirm(id, choice, reason)` / `reject(id, reason)` | a person's decision; the name comes from the settings |
| `pending_share` / `handed_over(entries)` | the list for the editor, and marking it handed over |
| `receive_list(text)` | editor: a list from another installation, assessed here |
| `sync_shared_register` | downloads the shared register and syncs it |
| `forget(url, prefix, reason)` | erasure (GDPR Art. 17): everything about the address is removed and it is blocked |
| `crawl_start` / `crawl_stop` / `crawl_status` | the crawler, with live counts |
| `settings` / `set_settings` | language, theme, name, sharing, crawler concurrency |

## Errors

An error reaches the interface as `{ "code": "...", "detail": "..." }`. The code is translated
(`error-<code>` in the Fluent files); the detail is technical English for those who look further.
`atena_app::CODES` lists every code, and a test keeps the list sorted and unique.

## The window

The window (`apps/desktop`) is described with its build in its own README. It loads nothing from
the network: fonts and icons are part of the app, the content security policy allows only the
app's own files, collected texts are shown as text and never as markup, and links open in the
user's browser only after the address is checked to be `http` or `https`.
