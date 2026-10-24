-- ATENA schema, version 1. PostgreSQL 15 or newer.
--
-- The database is the source of truth; search indexes are derived from it and can be rebuilt.
-- Times are timestamptz (UTC). Hashes are 32-byte BLAKE3 digests stored as bytea.
-- Enumerations are text with CHECK constraints, which are simpler to extend than enum types.

-- ------------------------------------------------------------------ register
-- One row per source of data/sources.yaml; kept in sync by `atena-ingest sync`.
CREATE TABLE source (
    id          text PRIMARY KEY,                       -- register ID, e.g. SK01
    name        text NOT NULL,
    url         text NOT NULL,
    country     text NOT NULL,                          -- concept of the `country` scheme
    languages   text[] NOT NULL DEFAULT '{}',
    authority   text NOT NULL,                          -- concept of the `authority` scheme (A1…E)
    priority    text NOT NULL,                          -- P0…P3
    connector   text NOT NULL,
    interval_s  integer NOT NULL CHECK (interval_s >= 60),
    enabled     boolean NOT NULL DEFAULT true,          -- false: archived or removed from the register
    synced_at   timestamptz NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------- scheduling
-- A host is the unit of politeness: one worker at a time holds its lease, so all processes
-- together send it one request at a time. `next_due_at` is the earliest due endpoint.
CREATE TABLE host (
    id            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name          text NOT NULL UNIQUE,                 -- lower-case host name
    next_due_at   timestamptz,                          -- NULL: nothing scheduled
    lease_until   timestamptz,
    leased_by     text,
    backoff_until timestamptz                           -- the host asked for rest (429/503)
);
CREATE INDEX host_due ON host (next_due_at) WHERE next_due_at IS NOT NULL;

-- Every URL ATENA checks: feeds, sitemaps and pages (a page yields a document).
CREATE TABLE endpoint (
    id              bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    source_id       text NOT NULL REFERENCES source (id) ON DELETE CASCADE,
    host_id         bigint NOT NULL REFERENCES host (id),
    url             text NOT NULL UNIQUE,               -- normalised (no fragment, no tracking parameters)
    kind            text NOT NULL CHECK (kind IN ('feed', 'sitemap', 'page')),
    origin          text NOT NULL CHECK (origin IN ('register', 'feed', 'sitemap', 'link')),
    follow_links    boolean NOT NULL DEFAULT false,     -- enqueue attachments and subpages of this page
    interval_s      integer NOT NULL CHECK (interval_s >= 60),
    enabled         boolean NOT NULL DEFAULT true,
    next_fetch_at   timestamptz NOT NULL DEFAULT now(),
    last_fetch_at   timestamptz,
    last_outcome    text,
    last_status     smallint,
    fail_count      integer NOT NULL DEFAULT 0,
    etag            text,                               -- validators of the last full response
    last_modified   text,
    discovered_from bigint REFERENCES endpoint (id) ON DELETE SET NULL,
    created_at      timestamptz NOT NULL DEFAULT now()
) WITH (fillfactor = 85);                               -- room for frequent updates
CREATE INDEX endpoint_host_due ON endpoint (host_id, next_fetch_at) WHERE enabled;
CREATE INDEX endpoint_source ON endpoint (source_id);

-- One row per check, for monitoring; old rows are deleted by `Store::prune_fetch_log`.
CREATE TABLE fetch_log (
    id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    endpoint_id bigint NOT NULL REFERENCES endpoint (id) ON DELETE CASCADE,
    at          timestamptz NOT NULL DEFAULT now(),
    outcome     text NOT NULL,
    status      smallint,
    bytes       bigint,
    elapsed_ms  integer,
    detail      text
);
CREATE INDEX fetch_log_endpoint ON fetch_log (endpoint_id, at DESC);
CREATE INDEX fetch_log_at ON fetch_log (at);

-- ------------------------------------------------------------------- content
-- Original files, stored once per content in the file store (see `BlobStore`).
CREATE TABLE blob (
    hash         bytea PRIMARY KEY CHECK (length(hash) = 32),   -- BLAKE3 of the original bytes
    size         bigint NOT NULL,
    stored_size  bigint NOT NULL,
    compression  text NOT NULL CHECK (compression IN ('none', 'gzip')),
    content_type text,
    created_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE document (
    id                 bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    endpoint_id        bigint NOT NULL UNIQUE REFERENCES endpoint (id) ON DELETE CASCADE,
    source_id          text NOT NULL REFERENCES source (id) ON DELETE CASCADE,
    url                text NOT NULL,                   -- final URL of the latest capture
    format             text NOT NULL CHECK (format IN ('html', 'pdf', 'docx', 'odt', 'text')),
    title              text,
    language           text,
    published_at       timestamptz,
    modified_at        timestamptz,
    status             text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'gone')),
    -- the publisher's rules from the latest capture (HTTP headers, tdmrep.json, document metadata)
    tdm_reserved       boolean NOT NULL DEFAULT false,  -- true: only metadata is kept
    tdm_policy         text,
    noindex            boolean NOT NULL DEFAULT false,
    noarchive          boolean NOT NULL DEFAULT false,
    nosnippet          boolean NOT NULL DEFAULT false,
    notranslate        boolean NOT NULL DEFAULT false,
    -- the latest capture, also when it did not become a version (same text, new bytes)
    last_raw_hash      bytea CHECK (length(last_raw_hash) = 32),
    last_fingerprint   bytea CHECK (length(last_fingerprint) = 32),
    first_seen_at      timestamptz NOT NULL DEFAULT now(),
    last_seen_at       timestamptz NOT NULL DEFAULT now(),
    last_changed_at    timestamptz NOT NULL DEFAULT now(),
    version_count      integer NOT NULL DEFAULT 0,
    current_version_id bigint                           -- foreign key added below
);
CREATE INDEX document_source_changed ON document (source_id, last_changed_at DESC);
CREATE INDEX document_changed ON document (last_changed_at DESC);

-- A version is a capture whose text differs from the previous one.
CREATE TABLE document_version (
    id                  bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    document_id         bigint NOT NULL REFERENCES document (id) ON DELETE CASCADE,
    version_no          integer NOT NULL CHECK (version_no >= 1),
    fetched_at          timestamptz NOT NULL,
    -- `changed`: the text changed but the previous text was not kept (TDM reservation)
    change_kind         text NOT NULL CHECK (change_kind IN ('initial', 'minor', 'substantive', 'changed')),
    paragraphs_added    integer NOT NULL DEFAULT 0,
    paragraphs_removed  integer NOT NULL DEFAULT 0,
    numbers_changed     boolean NOT NULL DEFAULT false,
    raw_hash            bytea NOT NULL CHECK (length(raw_hash) = 32),   -- in `blob` only if kept
    text_fingerprint    bytea NOT NULL CHECK (length(text_fingerprint) = 32),
    content_type        text,
    format              text NOT NULL,
    encoding            text,
    title               text,
    description         text,
    body                text,                           -- NULL when only metadata may be kept
    body_truncated      boolean NOT NULL DEFAULT false,
    language            text,
    language_source     text CHECK (language_source IN ('detected', 'declared')),
    language_confidence real,
    authors             text[] NOT NULL DEFAULT '{}',
    published_at        timestamptz,
    modified_at         timestamptz,
    canonical_url       text,
    UNIQUE (document_id, version_no)
);

ALTER TABLE document ADD CONSTRAINT document_current_version_fk
    FOREIGN KEY (current_version_id) REFERENCES document_version (id) DEFERRABLE INITIALLY DEFERRED;

-- LZ4 compresses and decompresses texts several times faster than the default pglz;
-- servers built without LZ4 keep the default.
DO $$
BEGIN
    ALTER TABLE document_version ALTER COLUMN body SET COMPRESSION lz4;
EXCEPTION WHEN feature_not_supported OR invalid_parameter_value THEN
    RAISE NOTICE 'LZ4 is not available; document_version.body uses the default compression';
END $$;

-- Links in the content of the current version (attachments, subpages).
CREATE TABLE document_link (
    document_id bigint NOT NULL REFERENCES document (id) ON DELETE CASCADE,
    url         text NOT NULL,
    text        text,
    PRIMARY KEY (document_id, url)
);

-- ------------------------------------------------------------ classification
-- Suggestions are replaced with every new version; an editor's decision is never overwritten.
CREATE TABLE document_label (
    document_id bigint NOT NULL REFERENCES document (id) ON DELETE CASCADE,
    scheme      text NOT NULL,
    concept     text NOT NULL,
    score       real NOT NULL CHECK (score >= 0 AND score <= 1),
    evidence    text[] NOT NULL DEFAULT '{}',
    origin      text NOT NULL CHECK (origin IN ('auto', 'editor')),
    status      text NOT NULL CHECK (status IN ('suggested', 'confirmed', 'rejected')),
    version_id  bigint REFERENCES document_version (id) ON DELETE SET NULL,
    decided_by  text,
    decided_at  timestamptz,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (document_id, scheme, concept)
);
CREATE INDEX document_label_concept ON document_label (scheme, concept) WHERE status <> 'rejected';
