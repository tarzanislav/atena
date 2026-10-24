-- ATENA schema, version 3: terms of use may reserve text and data mining (metadata only).
ALTER TABLE source DROP CONSTRAINT source_terms_check;
ALTER TABLE source ADD CONSTRAINT source_terms_check
    CHECK (terms IN ('allowed', 'forbidden', 'reserved', 'unknown'));
