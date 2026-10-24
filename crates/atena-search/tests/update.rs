//! The search queue of the database and the index updated from it, against a real PostgreSQL
//! (`ATENA_TEST_DATABASE_URL`; skipped without it).

use atena_core::Taxonomy;
use atena_search::{SearchIndex, SearchRequest, update};
use atena_store::testing::TestDb;
use atena_store::{
    Capture, EndpointKind, LabelDecision, LabelSuggestion, NewEndpoint, NewVersion, Origin,
    RegisterSource, Rights, SourceRecord, Store, hash,
};
use jiff::Timestamp;

const PAGE: &str = "https://sk01.example.sk/zapis";

fn register(licence: &str, terms: &str) -> Vec<RegisterSource> {
    vec![RegisterSource {
        source: SourceRecord {
            id: "SK01".into(),
            name: "Ministerstvo".into(),
            url: "https://sk01.example.sk/".into(),
            country: "SK".into(),
            languages: vec!["sk".into()],
            authority: "A1".into(),
            priority: "P0".into(),
            connector: "html".into(),
            interval_s: 3600,
            licence: licence.into(),
            terms: terms.into(),
            enabled: true,
        },
        endpoints: vec![NewEndpoint {
            url: PAGE.into(),
            host: "sk01.example.sk".into(),
            kind: EndpointKind::Page,
            origin: Origin::Register,
            follow_links: false,
            interval_s: 3600,
        }],
    }]
}

/// The register with one official source and one saved page; returns the document ID.
async fn page(s: &Store) -> i64 {
    s.sync_register(&register("official", "allowed"))
        .await
        .unwrap();
    let endpoint: i64 = sqlx::query_scalar("SELECT id FROM endpoint")
        .fetch_one(s.pool())
        .await
        .unwrap();
    let text = "Zápis do prvého ročníka základnej školy bude 15. apríla.";
    let capture = Capture {
        endpoint_id: endpoint,
        source_id: "SK01".into(),
        url: PAGE.into(),
        format: "html".into(),
        raw_hash: hash(text.as_bytes()),
        fingerprint: hash(text.as_bytes()),
        fetched_at: Timestamp::now(),
        rights: Rights::default(),
    };
    let version = NewVersion {
        change_kind: "initial".into(),
        title: Some("Zápis do 1. ročníka".into()),
        body: Some(text.into()),
        language: Some("sk".into()),
        ..NewVersion::default()
    };
    let suggestion = LabelSuggestion {
        scheme: "topic".into(),
        concept: "legislation".into(),
        score: 0.7,
        evidence: vec!["sk:zápis×2".into()],
    };
    s.save_version(&capture, &version, &[], &[suggestion])
        .await
        .unwrap()
        .document_id
}

async fn queued(s: &Store) -> Vec<i64> {
    s.search_queue_take(100)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.document_id)
        .collect()
}

async fn clear(s: &Store) {
    let entries = s.search_queue_take(100).await.unwrap();
    s.search_queue_done(&entries).await.unwrap();
}

#[tokio::test]
async fn changes_that_matter_for_search_are_queued() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    let id = page(s).await;
    assert_eq!(queued(s).await, [id], "a new document");
    clear(s).await;

    let endpoint: i64 = sqlx::query_scalar("SELECT endpoint_id FROM document")
        .fetch_one(s.pool())
        .await
        .unwrap();
    s.mark_seen(endpoint).await.unwrap();
    assert!(queued(s).await.is_empty(), "a check that changes nothing");

    s.decide_label(id, "topic", "legislation", LabelDecision::Reject, "editor")
        .await
        .unwrap();
    assert_eq!(queued(s).await, [id], "a label decision");
    clear(s).await;

    s.sync_register(&register("official", "allowed"))
        .await
        .unwrap();
    assert!(queued(s).await.is_empty(), "the same register");
    s.sync_register(&register("copyrighted", "allowed"))
        .await
        .unwrap();
    assert_eq!(queued(s).await, [id], "a new licence of the source");
    clear(s).await;

    // an entry queued again after it was taken stays until it is taken again
    let taken = s.search_queue_take(100).await.unwrap();
    assert!(taken.is_empty());
    s.decide_label(id, "topic", "legislation", LabelDecision::Confirm, "editor")
        .await
        .unwrap();
    let taken = s.search_queue_take(100).await.unwrap();
    s.decide_label(
        id,
        "school-type",
        "primary-school",
        LabelDecision::Confirm,
        "editor",
    )
    .await
    .unwrap();
    assert_eq!(s.search_queue_done(&taken).await.unwrap(), 0);
    assert_eq!(s.search_queue_len().await.unwrap(), 1);
    clear(s).await;

    s.forget(PAGE, false, "erasure request").await.unwrap();
    assert_eq!(queued(s).await, [id], "an erased document");
    assert!(s.search_rows(&[id]).await.unwrap().is_empty());
    db.drop().await;
}

#[tokio::test]
async fn the_index_follows_the_database() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    let taxonomy = Taxonomy::bundled().unwrap();
    let id = page(s).await;
    let index = SearchIndex::in_memory().unwrap();
    let find = |text: &str| {
        index
            .search(
                &taxonomy,
                &SearchRequest {
                    text: text.into(),
                    limit: 10,
                    ..SearchRequest::default()
                },
            )
            .unwrap()
    };

    let report = update(s, &index, 10).await.unwrap();
    assert_eq!((report.indexed, report.left_out), (1, 0));
    assert_eq!(s.search_queue_len().await.unwrap(), 0);
    let found = find("zápisu do ročníka");
    assert_eq!(found.hits.len(), 1);
    let hit = &found.hits[0];
    assert_eq!(
        (hit.document_id, hit.source_name.as_str()),
        (id, "Ministerstvo")
    );
    assert_eq!(
        hit.labels,
        ["authority/A1", "country/SK", "topic/legislation"]
    );
    assert!(hit.snippet.as_ref().unwrap().text.contains("15. apríla"));

    // the terms now forbid automated access: the document must not be found
    s.sync_register(&register("official", "forbidden"))
        .await
        .unwrap();
    let report = update(s, &index, 10).await.unwrap();
    assert_eq!((report.indexed, report.left_out), (0, 1));
    assert!(find("zápis").hits.is_empty());

    // allowed again, but copyrighted: found, no snippet
    s.sync_register(&register("copyrighted", "allowed"))
        .await
        .unwrap();
    update(s, &index, 10).await.unwrap();
    let found = find("zápis");
    assert_eq!(found.hits.len(), 1);
    assert!(found.hits[0].snippet.is_none());

    s.forget(PAGE, false, "erasure request").await.unwrap();
    let report = update(s, &index, 10).await.unwrap();
    assert_eq!((report.indexed, report.left_out), (0, 1));
    assert!(index.is_empty());
    db.drop().await;
}
