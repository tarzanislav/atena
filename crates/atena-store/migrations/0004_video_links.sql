-- ATENA schema, version 4: links to videos (YouTube, Vimeo) a page links to or embeds.
-- They are recorded like other links, never fetched.
ALTER TABLE document_link
    ADD COLUMN kind text NOT NULL DEFAULT 'link' CHECK (kind IN ('link', 'video'));
