# ATENA desktop app – the window

*Slovenčina: [../../docs/sk/APLIKACIA.md](../../docs/sk/APLIKACIA.md)*

The window around [`crates/atena-app`](../../crates/atena-app): Tauri 2 with a React interface. It
decides nothing itself – every command passes the call to `atena_app::App`, which checks the input.
The backend, the database inside the app and its protections are described in
[docs/DESKTOP.md](../../docs/DESKTOP.md).

| Folder | Contents |
|---|---|
| `src/` | the interface (React, TypeScript): `App.tsx` (start and layout), `screens/` (Overview, Search, Sources, Proposals, Crawling, Settings), `components/`, `api.ts` (the commands and their data) |
| `locales/` | the texts of the interface (Project Fluent): `sk.ftl` is the reference, `en`, `cs`, `ru`, `zh` follow it; `index.json` lists the languages and marks machine drafts |
| `src-tauri/` | the window program (Rust): `commands.rs`, `desktop.rs` (start and clean end), `logging.rs`; `capabilities/main.json` and `build.rs` grant the interface exactly its commands |
| `scripts/` | `check-locales.mjs` (every language complete, every backend error code has a text), `notices.mjs` and `postgres-notices.mjs` (licence texts carried by the installer) |
| `brand/` | the logo (SVG); the icons in `src-tauri/icons` are made from it with `npx tauri icon` |
| `notices/` | the PostgreSQL licence, completed by the build with the libraries that come with it |

## Security of the window

* **Commands only.** The capability grants the window the commands of ATENA and nothing else: no
  file system, no shell, no network, no plugins.
* **Content security policy:** scripts, styles, fonts and images only from the app itself;
  connections only to the app's own IPC; no frames, no forms, no objects. Assets are never inlined
  as `data:` addresses.
* **Text stays text.** Collected titles, snippets and texts are rendered as React text nodes,
  never as markup; snippets arrive from the backend as pieces with marked words.
* **Links** open in the user's own browser only after the backend has checked the address
  (`atena_app::openable`: `http`/`https`, a host, no credentials, no hidden characters).
* **No network from the interface.** Fonts (Open Sans, Noto Sans SC) and icons (Fluent UI
  System Icons) are files of the app.
* **The log** (`logs/atena.log` in the app's folder) never holds the database's statements.

## Languages

Every visible text comes from `locales/*.ftl`. A new language is a new file with the same message
identifiers and a line in `locales/index.json` – no change of the program. `npm run check:locales`
(part of `npm run build`) fails when a language lacks a message, has one the reference does not
know, or a backend error code has no text. Languages other than Slovak are marked as machine
drafts until a native speaker has reviewed them.

## Development

```sh
cd apps/desktop
npm ci
npm run tauri dev        # the window with the interface's dev server
```

The window needs PostgreSQL. During development either point it to the `bin` folder of a local
PostgreSQL (`ATENA_PG_BIN=/usr/lib/postgresql/17/bin`; PostgreSQL does not run as root) or to a
database of your own (`ATENA_DATABASE_URL=postgres://…`). `ATENA_DATA_DIR` chooses another folder
for the data. On Linux the window needs WebKitGTK (`libwebkit2gtk-4.1-dev`).

The window crate has its own `Cargo.lock` and is built apart from the core workspace, which builds
and tests without a web view.

## Release build

`.github/workflows/desktop.yml` builds the Windows installer:

1. downloads the official PostgreSQL binaries for Windows (EnterpriseDB, linked from
   postgresql.org), checks their SHA-256 and takes `bin` (the three programs and their libraries),
   `lib` and `share`;
2. runs the embedded-database tests of `atena-app` against exactly these programs;
3. writes the licence texts of every part (`scripts/notices.mjs`, `scripts/postgres-notices.mjs`;
   an unknown library stops the build);
4. builds the NSIS installer (installation for the current user, no administrator rights).

A release is made on GitHub: **Releases → Draft a new release**, a new tag `vX.Y.Z` (the version of
`tauri.conf.json`), marked as a pre-release while ATENA is tried out. The new tag starts the same
build, and its last job attaches the installer and `SHA256SUMS.txt` to the release (and the notes
of `release-notes.md` when the release has none). Pushing a tag works the same way.

The installer is not signed yet: Windows SmartScreen asks once whether to run it.
