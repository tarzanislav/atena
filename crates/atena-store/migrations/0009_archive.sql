-- ATENA schema, version 9: the archive of original files, and when a document left the web.
-- The author decided on 25 Oct 2026 to keep, in place of the rule of version 7, the original file
-- of documents that anyone may copy and show – official texts free of copyright under both laws
-- and openly licensed works – so they stay available after the publisher removes them. A file is
-- kept only for PDF and office documents, only after the reader's danger check let it through, and
-- only when no personal data was found in its text; programs and active content never reach this
-- table. Deleting a version or a document (an erasure) deletes its file.
CREATE TABLE version_file (
    version_id bigint PRIMARY KEY REFERENCES document_version (id) ON DELETE CASCADE,
    media_type text NOT NULL,
    byte_size  integer NOT NULL CHECK (byte_size > 0),
    raw_hash   bytea NOT NULL CHECK (length(raw_hash) = 32),
    content    bytea NOT NULL,
    stored_at  timestamptz NOT NULL DEFAULT now()
);
-- files are never compressed again by the database (PDF and office files are compressed already)
ALTER TABLE version_file ALTER COLUMN content SET STORAGE EXTERNAL;

-- when the publisher stopped serving the document (HTTP 404/410); NULL while it is published
ALTER TABLE document ADD COLUMN gone_at timestamptz;
UPDATE document SET gone_at = last_seen_at WHERE status = 'gone';

COMMENT ON COLUMN document_version.raw_hash IS
    'BLAKE3 hash of the original bytes; the original itself is kept only in version_file';
