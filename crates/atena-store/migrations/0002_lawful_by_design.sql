-- ATENA schema, version 2: licences and terms of use of sources, erasure on request.

-- How content of a source may be shown (official | copyrighted | an open licence) and what its
-- terms of use say about automated access. Sources whose terms forbid it are disabled.
ALTER TABLE source
    ADD COLUMN licence text NOT NULL DEFAULT 'copyrighted',
    ADD COLUMN terms text NOT NULL DEFAULT 'unknown' CHECK (terms IN ('allowed', 'forbidden', 'unknown'));

-- URLs that must never be fetched again, e.g. after an erasure request (GDPR Art. 17);
-- `prefix` covers every URL that starts with this one.
CREATE TABLE blocked_url (
    url      text PRIMARY KEY,
    prefix   boolean NOT NULL DEFAULT false,
    reason   text NOT NULL,
    added_at timestamptz NOT NULL DEFAULT now()
);

-- Accountability for erasures: what was removed, when and why – never the removed content.
CREATE TABLE erasure_log (
    id        bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    at        timestamptz NOT NULL DEFAULT now(),
    url       text NOT NULL,
    prefix    boolean NOT NULL,
    reason    text NOT NULL,
    documents integer NOT NULL,
    versions  integer NOT NULL,
    files     integer NOT NULL
);
