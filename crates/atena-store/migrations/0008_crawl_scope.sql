-- How far the crawler goes (atena_core::source::CrawlScope).
-- * `source.site_wide`: the crawler may follow links to any page of the publisher's site, not only
--   to the pages below the one it is on – allowed in the register only for official sources.
-- * `endpoint.depth`: how many more levels of links are followed from the pages found through this
--   endpoint; 0 follows nothing further (the behaviour of every endpoint before this migration).
ALTER TABLE source ADD COLUMN site_wide boolean NOT NULL DEFAULT false;
ALTER TABLE endpoint ADD COLUMN depth smallint NOT NULL DEFAULT 0 CHECK (depth BETWEEN 0 AND 5);
