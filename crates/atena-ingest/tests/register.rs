//! The bundled register in a real PostgreSQL (see `atena_store::testing`): every licence and
//! terms value of the rights review is accepted, and only permitted sources (never tools) are
//! enabled.

use atena_core::Taxonomy;
use atena_core::legality::{SourceFacts, permission};
use atena_core::source::{BUNDLED_SOURCES, load_sources};
use atena_ingest::register_entries;
use atena_store::testing::TestDb;

#[tokio::test]
async fn the_reviewed_register_is_stored_and_only_permitted_sources_are_enabled() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let sources = load_sources(BUNDLED_SOURCES, &Taxonomy::bundled().unwrap()).unwrap();
    let report = db
        .store
        .sync_register(&register_entries(&sources))
        .await
        .unwrap();
    assert_eq!(report.sources, sources.len() as u64);

    let rows: Vec<(String, String, bool)> =
        sqlx::query_as("SELECT id, terms, enabled FROM source ORDER BY id")
            .fetch_all(db.store.pool())
            .await
            .unwrap();
    assert_eq!(rows.len(), sources.len());
    for (id, terms, enabled) in &rows {
        let source = sources.iter().find(|s| &s.id == id).unwrap();
        assert_eq!(terms, source.terms.as_str(), "{id}");
        // a tool is never crawled, whatever the law would allow
        let crawl = permission(SourceFacts::from(source), false).crawl && !source.is_tool();
        assert_eq!(*enabled, crawl, "{id}");
    }
    // the review produced every kind of terms, and the database accepts all of them
    for terms in ["allowed", "forbidden", "reserved", "unknown"] {
        assert!(
            rows.iter().any(|(_, t, _)| t == terms),
            "no source with terms {terms}"
        );
    }
    db.drop().await;
}
