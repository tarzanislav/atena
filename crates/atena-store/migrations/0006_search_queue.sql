-- ATENA schema, version 6: the queue of documents the search index must update.
-- Triggers fill it on every change that can matter for search – a new version, a text reduced to
-- metadata, a label decision, an erasure, a change of the source's licence or terms – so that no
-- code path can leave the index behind. The indexer removes an entry only after its commit, and
-- only if nothing was queued again in the meantime.
CREATE TABLE search_queue (
    document_id bigint PRIMARY KEY,
    queued_at   timestamptz NOT NULL DEFAULT clock_timestamp()
);

CREATE FUNCTION queue_document(id bigint) RETURNS void LANGUAGE sql AS $$
    INSERT INTO search_queue (document_id) VALUES (id)
    ON CONFLICT (document_id) DO UPDATE SET queued_at = clock_timestamp();
$$;

CREATE FUNCTION queue_document_row() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM queue_document(CASE WHEN TG_OP = 'DELETE' THEN OLD.id ELSE NEW.id END);
    RETURN NULL;
END $$;

CREATE FUNCTION queue_related_row() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM queue_document(CASE WHEN TG_OP = 'DELETE' THEN OLD.document_id ELSE NEW.document_id END);
    RETURN NULL;
END $$;

CREATE FUNCTION queue_source_documents() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO search_queue (document_id)
    SELECT id FROM document WHERE source_id = NEW.id
    ON CONFLICT (document_id) DO UPDATE SET queued_at = clock_timestamp();
    RETURN NULL;
END $$;

CREATE TRIGGER document_search AFTER INSERT OR DELETE ON document
    FOR EACH ROW EXECUTE FUNCTION queue_document_row();
-- not on every check: a check that only confirms the page changes nothing that search shows
CREATE TRIGGER document_search_update AFTER UPDATE ON document
    FOR EACH ROW
    WHEN ((OLD.current_version_id, OLD.source_id, OLD.url, OLD.title, OLD.language, OLD.published_at,
           OLD.modified_at, OLD.status, OLD.tdm_reserved, OLD.noindex, OLD.noarchive, OLD.nosnippet,
           OLD.last_changed_at)
          IS DISTINCT FROM
          (NEW.current_version_id, NEW.source_id, NEW.url, NEW.title, NEW.language, NEW.published_at,
           NEW.modified_at, NEW.status, NEW.tdm_reserved, NEW.noindex, NEW.noarchive, NEW.nosnippet,
           NEW.last_changed_at))
    EXECUTE FUNCTION queue_document_row();
CREATE TRIGGER document_version_search AFTER UPDATE OF body, title, description ON document_version
    FOR EACH ROW EXECUTE FUNCTION queue_related_row();
CREATE TRIGGER document_label_search AFTER INSERT OR UPDATE OR DELETE ON document_label
    FOR EACH ROW EXECUTE FUNCTION queue_related_row();
CREATE TRIGGER source_search AFTER UPDATE OF name, country, authority, licence, terms, enabled ON source
    FOR EACH ROW
    WHEN ((OLD.name, OLD.country, OLD.authority, OLD.licence, OLD.terms, OLD.enabled)
          IS DISTINCT FROM (NEW.name, NEW.country, NEW.authority, NEW.licence, NEW.terms, NEW.enabled))
    EXECUTE FUNCTION queue_source_documents();

-- documents that existed before the queue
INSERT INTO search_queue (document_id) SELECT id FROM document;
