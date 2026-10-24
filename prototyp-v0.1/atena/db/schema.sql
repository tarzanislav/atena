-- ATENA – databázová schéma (PostgreSQL 15+)
-- Databáza je zdrojom pravdy; vyhľadávací index je odvodený a dá sa kedykoľvek prepočítať.

CREATE EXTENSION IF NOT EXISTS unaccent;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- unaccent() nie je IMMUTABLE, preto obal (bežná a bezpečná prax pre indexy)
CREATE OR REPLACE FUNCTION atena_fold(t text) RETURNS text
  LANGUAGE sql IMMUTABLE PARALLEL SAFE STRICT
  AS $$ SELECT lower(public.unaccent('public.unaccent'::regdictionary, t)) $$;

CREATE TABLE IF NOT EXISTS schema_version (version int PRIMARY KEY, applied_at timestamptz NOT NULL DEFAULT now());

-- ---------------------------------------------------------------- register
CREATE TABLE IF NOT EXISTS source (
  id          text PRIMARY KEY,
  name        text NOT NULL,
  url         text NOT NULL,
  country     text NOT NULL DEFAULT 'WORLD',
  languages   text[] NOT NULL DEFAULT '{}',
  authority   text NOT NULL,                 -- A1..E (schéma "autorita")
  priority    text NOT NULL DEFAULT 'P1',
  connector   text NOT NULL DEFAULT 'html',
  frequency   text,
  status      text,
  note        text,
  enabled     boolean NOT NULL DEFAULT true,
  -- licenčná politika: čo smieme s obsahom robiť (predvolene len metadáta + odkaz + krátka vlastná anotácia)
  store_fulltext   boolean NOT NULL DEFAULT true,   -- interné indexovanie (TDM výnimka, ak nie je výhrada)
  show_fulltext    boolean NOT NULL DEFAULT false,  -- zobraziť celý text verejne (len pri otvorenej licencii)
  translate_allowed boolean NOT NULL DEFAULT false, -- zverejniť celý preklad (len pri otvorenej licencii)
  license     text,
  created_at  timestamptz NOT NULL DEFAULT now(),
  updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS endpoint (
  id            bigserial PRIMARY KEY,
  source_id     text NOT NULL REFERENCES source(id) ON DELETE CASCADE,
  kind          text NOT NULL,              -- rss | html | sitemap | api
  url           text NOT NULL,
  interval_s    int  NOT NULL DEFAULT 86400,
  etag          text,
  last_modified text,
  last_checked  timestamptz,
  next_check    timestamptz NOT NULL DEFAULT now(),
  last_status   int,
  last_error    text,
  fail_count    int NOT NULL DEFAULT 0,
  new_items_30d int NOT NULL DEFAULT 0,
  UNIQUE (source_id, url)
);

CREATE TABLE IF NOT EXISTS fetch_log (
  id          bigserial PRIMARY KEY,
  endpoint_id bigint REFERENCES endpoint(id) ON DELETE SET NULL,
  url         text NOT NULL,
  final_url   text,
  status      int,
  outcome     text NOT NULL,                -- ok | not_modified | robots | ssrf | too_large | error
  bytes       int,
  content_hash text,
  elapsed_ms  int,
  error       text,
  at          timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS fetch_log_at ON fetch_log (at DESC);

-- --------------------------------------------------------------- dokumenty
CREATE TABLE IF NOT EXISTS document (
  id            bigserial PRIMARY KEY,
  source_id     text NOT NULL REFERENCES source(id) ON DELETE CASCADE,
  url           text NOT NULL UNIQUE,       -- kanonická URL
  title         text NOT NULL DEFAULT '',
  language      text,
  published_at  timestamptz,
  first_seen    timestamptz NOT NULL DEFAULT now(),
  last_seen     timestamptz NOT NULL DEFAULT now(),
  last_changed  timestamptz NOT NULL DEFAULT now(),
  status        text NOT NULL DEFAULT 'active',  -- active | gone | withdrawn | archived
  rights_mode   text NOT NULL DEFAULT 'fulltext_internal', -- fulltext_internal | metadata_only (výhrada TDM)
  etag          text,
  last_modified text,
  current_version_id bigint
);
CREATE INDEX IF NOT EXISTS document_source ON document (source_id);
CREATE INDEX IF NOT EXISTS document_changed ON document (last_changed DESC);

CREATE TABLE IF NOT EXISTS document_version (
  id          bigserial PRIMARY KEY,
  document_id bigint NOT NULL REFERENCES document(id) ON DELETE CASCADE,
  version_no  int NOT NULL,
  fetched_at  timestamptz NOT NULL DEFAULT now(),
  content_hash text NOT NULL,   -- odtlačok bajtov
  text_hash   text NOT NULL,    -- odtlačok normalizovaného textu
  title       text NOT NULL DEFAULT '',
  summary     text NOT NULL DEFAULT '',
  body        text NOT NULL DEFAULT '',
  mime        text,
  byte_size   int,
  change_kind text NOT NULL,    -- initial | minor | substantive
  change      jsonb,
  tsv tsvector GENERATED ALWAYS AS (
        setweight(to_tsvector('simple'::regconfig, atena_fold(coalesce(title,''))), 'A') ||
        setweight(to_tsvector('simple'::regconfig, atena_fold(coalesce(summary,''))), 'B') ||
        setweight(to_tsvector('simple'::regconfig, atena_fold(left(coalesce(body,''), 200000))), 'C')) STORED,
  UNIQUE (document_id, version_no)
);
CREATE INDEX IF NOT EXISTS version_tsv ON document_version USING gin (tsv);
CREATE INDEX IF NOT EXISTS version_title_trgm ON document_version USING gin (atena_fold(title) gin_trgm_ops);

DO $$ BEGIN
  ALTER TABLE document ADD CONSTRAINT document_current_fk FOREIGN KEY (current_version_id)
    REFERENCES document_version(id) DEFERRABLE INITIALLY DEFERRED;
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- -------------------------------------------------------------- taxonómia
CREATE TABLE IF NOT EXISTS concept (
  scheme    text NOT NULL,
  id        text NOT NULL,
  broader   text,
  labels    jsonb NOT NULL,
  note      text,
  PRIMARY KEY (scheme, id)
);

CREATE TABLE IF NOT EXISTS label (
  document_id bigint NOT NULL REFERENCES document(id) ON DELETE CASCADE,
  scheme      text NOT NULL,
  concept_id  text NOT NULL,
  score       real NOT NULL DEFAULT 1,
  origin      text NOT NULL DEFAULT 'auto',       -- auto | source | editor
  status      text NOT NULL DEFAULT 'suggested',  -- suggested | confirmed | rejected
  evidence    text[],
  decided_by  text,
  decided_at  timestamptz,
  PRIMARY KEY (document_id, scheme, concept_id)
);
CREATE INDEX IF NOT EXISTS label_concept ON label (scheme, concept_id) WHERE status <> 'rejected';

-- ----------------------------------------------------------------- preklad
CREATE TABLE IF NOT EXISTS translation (
  id          bigserial PRIMARY KEY,
  version_id  bigint NOT NULL REFERENCES document_version(id) ON DELETE CASCADE,
  target_lang text NOT NULL,
  engine      text NOT NULL,
  title       text,
  summary     text,
  status      text NOT NULL,     -- navrh | automaticka_kontrola | chyba_kontroly | odborne_skontrolovany
  checks      jsonb,
  created_at  timestamptz NOT NULL DEFAULT now(),
  UNIQUE (version_id, target_lang, engine)
);

-- ------------------------------------------------------------- úlohy (fronta v PostgreSQL)
CREATE TABLE IF NOT EXISTS job (
  id          bigserial PRIMARY KEY,
  kind        text NOT NULL,
  payload     jsonb NOT NULL DEFAULT '{}',
  dedup_key   text,
  run_at      timestamptz NOT NULL DEFAULT now(),
  attempts    int NOT NULL DEFAULT 0,
  max_attempts int NOT NULL DEFAULT 5,
  locked_at   timestamptz,
  locked_by   text,
  last_error  text,
  done_at     timestamptz,
  created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS job_dedup ON job (dedup_key) WHERE done_at IS NULL AND dedup_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS job_ready ON job (run_at) WHERE done_at IS NULL;

-- ------------------------------------------------------------------ audit
CREATE TABLE IF NOT EXISTS audit_log (
  id      bigserial PRIMARY KEY,
  at      timestamptz NOT NULL DEFAULT now(),
  actor   text NOT NULL,
  action  text NOT NULL,
  entity  text NOT NULL,
  entity_id text NOT NULL,
  detail  jsonb
);

INSERT INTO schema_version(version) VALUES (1) ON CONFLICT DO NOTHING;
