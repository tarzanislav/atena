# PostgreSQL inside the app

The release build puts the PostgreSQL programs here (`bin/`, `lib/`, `share/`); see
`.github/workflows/desktop.yml`. They are not part of the repository. For development, set
`ATENA_PG_BIN` to the `bin` folder of a PostgreSQL installation or `ATENA_DATABASE_URL` to a
database of your own.
