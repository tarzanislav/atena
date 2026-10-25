//! Classifying stored documents again with a changed taxonomy (`ATENA_TEST_DATABASE_URL`; skipped
//! without one).

use atena_core::Taxonomy;
use atena_ingest::PipelineConfig;
use atena_ingest::classifying::reclassify;
use atena_store::testing::TestDb;
use atena_store::{
    Capture, EndpointKind, LabelDecision, LabelSuggestion, NewEndpoint, NewVersion, Origin,
    RegisterSource, Rights, SourceRecord, hash,
};
use jiff::Timestamp;

fn register() -> RegisterSource {
    RegisterSource {
        source: SourceRecord {
            id: "SK01".into(),
            name: "Zdroj".into(),
            url: "https://sk01.example.sk/".into(),
            country: "SK".into(),
            languages: vec!["sk".into()],
            authority: "A2".into(),
            priority: "P0".into(),
            connector: "html".into(),
            interval_s: 3600,
            licence: "copyrighted".into(),
            terms: "allowed".into(),
            enabled: true,
            site_wide: false,
        },
        endpoints: vec![NewEndpoint {
            url: "https://sk01.example.sk/".into(),
            host: "sk01.example.sk".into(),
            kind: EndpointKind::Page,
            origin: Origin::Register,
            follow_links: false,
            depth: 0,
            interval_s: 3600,
        }],
    }
}

#[tokio::test]
async fn stored_documents_get_the_concepts_of_a_new_taxonomy() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    s.sync_register(&[register()]).await.unwrap();
    let ep: i64 = sqlx::query_scalar("SELECT id FROM endpoint")
        .fetch_one(s.pool())
        .await
        .unwrap();
    let text = "Elektronické testovanie žiakov: typy úloh a úlohy s výberom odpovede.";
    let saved = s
        .save_version(
            &Capture {
                endpoint_id: ep,
                source_id: "SK01".into(),
                url: "https://sk01.example.sk/".into(),
                format: "html".into(),
                raw_hash: hash(text.as_bytes()),
                fingerprint: hash(text.as_bytes()),
                fetched_at: Timestamp::now(),
                rights: Rights::default(),
            },
            &NewVersion {
                change_kind: "initial".into(),
                title: Some("Elektronické testovanie".into()),
                body: Some(text.into()),
                language: Some("sk".into()),
                ..NewVersion::default()
            },
            &[],
            // classified earlier with an older taxonomy
            &[LabelSuggestion {
                scheme: "topic".into(),
                concept: "legislation".into(),
                score: 0.5,
                evidence: vec!["sk:zákon×1".into()],
            }],
        )
        .await
        .unwrap();
    let id = saved.document_id;
    // an editor's decision is never touched
    s.decide_label(id, "topic", "ai", LabelDecision::Reject, "Ján")
        .await
        .unwrap();

    let taxonomy = Taxonomy::bundled().unwrap();
    let threshold = PipelineConfig::default().classify_threshold;
    let report = reclassify(s, &taxonomy, threshold).await.unwrap();
    assert_eq!((report.documents, report.changed), (1, 1));
    let labels = s.labels(id).await.unwrap();
    let has = |scheme: &str, concept: &str, status: &str| {
        labels
            .iter()
            .any(|l| l.scheme == scheme && l.concept == concept && l.status == status)
    };
    assert!(has("section", "e-testing", "suggested"), "{labels:?}");
    assert!(has("section", "question-types", "suggested"), "{labels:?}");
    assert!(
        !has("topic", "legislation", "suggested"),
        "stale suggestion"
    );
    assert!(has("topic", "ai", "rejected"));

    // nothing changes the second time
    let again = reclassify(s, &taxonomy, threshold).await.unwrap();
    assert_eq!((again.documents, again.changed), (1, 0));
    db.drop().await;
}
