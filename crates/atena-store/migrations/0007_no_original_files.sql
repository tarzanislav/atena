-- ATENA schema, version 7: no original files.
-- The author decided on 24 Oct 2026 that ATENA keeps no original files: a file may carry harmful
-- content or personal data that cannot be removed from it reliably. Every version keeps the cleaned
-- text (where the legality principle allows) and the BLAKE3 hash of the original bytes, which
-- proves what the publisher served; the original stays with its publisher.
DROP TABLE blob;
ALTER TABLE erasure_log DROP COLUMN files;
COMMENT ON COLUMN document_version.raw_hash IS
    'BLAKE3 hash of the original bytes; the original itself is never kept';
