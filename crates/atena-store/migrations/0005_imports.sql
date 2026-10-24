-- ATENA schema, version 5: sources proposed by users, their rights assessments, and the lists of
-- proposals exchanged between installations.

-- `register`: from data/sources.yaml; `import`: proposed by a user of this installation or, on the
-- editor's installation, by users of other installations (`via = 'shared'`).
-- An imported source is `proposed` until a person confirms (or rejects) it; until then it is
-- disabled and keeps the strictest licence and terms. `merged`: the shared register took it over.
ALTER TABLE source
    ADD COLUMN origin text NOT NULL DEFAULT 'register' CHECK (origin IN ('register', 'import')),
    ADD COLUMN state text NOT NULL DEFAULT 'confirmed'
        CHECK (state IN ('proposed', 'confirmed', 'rejected', 'merged')),
    ADD COLUMN via text NOT NULL DEFAULT 'local' CHECK (via IN ('local', 'shared'));

-- Every rights assessment of an imported source: automatic ones (`by_person` NULL) and the
-- decisions of a person. The newest row is the current one.
CREATE TABLE assessment (
    id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    source_id   text NOT NULL REFERENCES source (id) ON DELETE CASCADE,
    at          timestamptz NOT NULL DEFAULT now(),
    by_person   text,
    licence     text NOT NULL,
    terms       text NOT NULL CHECK (terms IN ('allowed', 'forbidden', 'reserved', 'unknown')),
    country     text,
    rights_page text NOT NULL,
    finding     text NOT NULL,
    evidence    text                -- the automatic assessment as JSON
);
CREATE INDEX assessment_source ON assessment (source_id, at DESC, id DESC);

-- Lists of addresses received from other installations (on the editor's installation).
-- They hold addresses and country codes only.
CREATE TABLE inbox (
    message_id  text PRIMARY KEY,
    received_at timestamptz NOT NULL DEFAULT now(),
    body        text NOT NULL,
    items       integer NOT NULL DEFAULT 0
);

-- Lists this installation shares; kept until they are sent.
CREATE TABLE outbox (
    id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    created_at  timestamptz NOT NULL DEFAULT now(),
    body        text NOT NULL,
    attempts    integer NOT NULL DEFAULT 0,
    last_error  text
);
