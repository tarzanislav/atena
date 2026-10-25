//! Imported sources, their assessments and the exchanged lists against a real PostgreSQL
//! (`ATENA_TEST_DATABASE_URL`; skipped without it).

use atena_store::testing::TestDb;
use atena_store::{
    AssessmentRecord, EndpointKind, ImportOutcome, ImportState, NewEndpoint, NewImport, Origin,
    RegisterSource, SourceRecord, Via,
};

fn import(id: &str, url: &str) -> NewImport {
    NewImport {
        id: id.into(),
        name: "Škola".into(),
        url: url.into(),
        country: Some("SK".into()),
        connector: "html".into(),
        interval_s: 86_400,
        via: Via::Local,
        endpoints: vec![NewEndpoint {
            url: url.into(),
            host: url::Url::parse(url).unwrap().host_str().unwrap().into(),
            kind: EndpointKind::Page,
            origin: Origin::Register,
            follow_links: true,
            depth: 0,
            interval_s: 86_400,
        }],
    }
}

fn auto(terms: &str) -> AssessmentRecord {
    AssessmentRecord {
        by_person: None,
        licence: "copyrighted".into(),
        terms: terms.into(),
        country: Some("SK".into()),
        rights_page: "https://skola.example.sk/podmienky".into(),
        finding: "Automatic assessment: terms read.".into(),
        evidence: Some("{}".into()),
    }
}

fn person(terms: &str) -> AssessmentRecord {
    AssessmentRecord {
        by_person: Some("editor".into()),
        finding: "Terms read by the editor on 24 Oct 2026.".into(),
        evidence: None,
        ..auto(terms)
    }
}

async fn enabled(store: &atena_store::Store, id: &str) -> bool {
    sqlx::query_scalar("SELECT enabled FROM source WHERE id = $1")
        .bind(id)
        .fetch_one(store.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn an_import_waits_for_a_person() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    let item = import("U-0000000001", "https://skola.example.sk/");
    assert_eq!(
        s.import_source(&item, &auto("allowed")).await.unwrap(),
        ImportOutcome::New
    );
    // proposed: disabled, strictest values, the proposal beside it
    let listed = s.imported(Some(ImportState::Proposed)).await.unwrap();
    assert_eq!(listed.len(), 1);
    let src = &listed[0];
    assert_eq!(
        (src.licence.as_str(), src.terms.as_str(), src.enabled),
        ("copyrighted", "unknown", false)
    );
    assert_eq!(src.proposal.as_ref().unwrap().terms, "allowed");
    // an automatic record may not pose as a decision, and a decision needs a person and a reason
    assert!(s.import_source(&item, &person("allowed")).await.is_err());
    let mut nameless = person("allowed");
    nameless.by_person = None;
    assert!(s.confirm_import(&item.id, &nameless, true).await.is_err());

    assert_eq!(
        s.import_source(&item, &auto("reserved")).await.unwrap(),
        ImportOutcome::Reassessed
    );
    assert!(
        s.confirm_import(&item.id, &person("allowed"), true)
            .await
            .unwrap()
    );
    assert!(enabled(s, &item.id).await);
    // a later import never changes a decision
    assert_eq!(
        s.import_source(&item, &auto("forbidden")).await.unwrap(),
        ImportOutcome::AlreadyDecided(ImportState::Confirmed)
    );
    let confirmed = &s.imported(Some(ImportState::Confirmed)).await.unwrap()[0];
    assert_eq!(confirmed.terms, "allowed");
    let history = s.assessments(&item.id).await.unwrap();
    assert_eq!(history.len(), 4);
    assert_eq!(history[1].by_person.as_deref(), Some("editor"));

    // rejecting removes it from the crawl
    assert!(
        s.reject_import(&item.id, "editor", "not about education")
            .await
            .unwrap()
    );
    assert!(!enabled(s, &item.id).await);
    assert!(!s.reject_import("U-missing", "editor", "x").await.unwrap());
    db.drop().await;
}

#[tokio::test]
async fn the_register_comes_first_and_takes_imports_over() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    let register_source = |url: &str| RegisterSource {
        source: SourceRecord {
            id: "SK01".into(),
            name: "Ministerstvo".into(),
            url: url.into(),
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
            url: url.into(),
            host: "www.minedu.example.sk".into(),
            kind: EndpointKind::Page,
            origin: Origin::Register,
            follow_links: true,
            depth: 0,
            interval_s: 3600,
        }],
    };
    s.sync_register(&[register_source("https://www.minedu.example.sk/")])
        .await
        .unwrap();
    // an address the register lists is not imported again
    assert_eq!(
        s.import_source(
            &import("U-1", "https://www.minedu.example.sk/"),
            &auto("allowed")
        )
        .await
        .unwrap(),
        ImportOutcome::InRegister("SK01".into())
    );
    // a confirmed import survives a sync that does not list it
    let item = import("U-2", "https://skola.example.sk/");
    s.import_source(&item, &auto("allowed")).await.unwrap();
    s.confirm_import(&item.id, &person("allowed"), true)
        .await
        .unwrap();
    let report = s
        .sync_register(&[register_source("https://www.minedu.example.sk/")])
        .await
        .unwrap();
    assert_eq!((report.removed, report.merged), (0, 0));
    assert!(enabled(s, &item.id).await);
    // when the shared register lists it, the register takes it over
    let mut both = vec![register_source("https://www.minedu.example.sk/")];
    let mut shared = register_source("https://skola.example.sk/");
    shared.source.id = "SK99".into();
    shared.endpoints[0].host = "skola.example.sk".into();
    both.push(shared);
    let report = s.sync_register(&both).await.unwrap();
    assert_eq!(report.merged, 1);
    assert!(!enabled(s, &item.id).await);
    let owner: String = sqlx::query_scalar(
        "SELECT source_id FROM endpoint WHERE url = 'https://skola.example.sk/'",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(owner, "SK99");
    db.drop().await;
}

#[tokio::test]
async fn erasure_removes_proposals_and_lists() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    let item = import("U-3", "https://osoba.example.sk/blog/");
    s.import_source(&item, &auto("allowed")).await.unwrap();
    assert!(
        s.inbox_add("m1", "ATENA-NAVRHY 1\nhttps://osoba.example.sk/blog/\n", 1)
            .await
            .unwrap()
    );
    assert!(
        !s.inbox_add("m1", "again", 1).await.unwrap(),
        "a message is received once"
    );
    s.forget("https://osoba.example.sk/", true, "request of the author")
        .await
        .unwrap();
    assert!(s.assessments(&item.id).await.unwrap().is_empty());
    let (url, state): (String, String) =
        sqlx::query_as("SELECT url, state FROM source WHERE id = 'U-3'")
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!((url.as_str(), state.as_str()), ("", "rejected"));
    assert!(s.inbox_known(&["m1".into()]).await.unwrap().is_empty());
    // and the address cannot be imported again
    assert_eq!(
        s.import_source(&item, &auto("allowed")).await.unwrap(),
        ImportOutcome::Blocked
    );
    db.drop().await;
}

#[tokio::test]
async fn shared_lists_stay_queued_until_sent() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    let a = s
        .outbox_add("ATENA-NAVRHY 1\nhttps://a.example.sk/\n")
        .await
        .unwrap();
    let b = s
        .outbox_add("ATENA-NAVRHY 1\nhttps://b.example.sk/\n")
        .await
        .unwrap();
    s.outbox_failed(a, "network").await.unwrap();
    let pending = s.outbox_pending().await.unwrap();
    assert_eq!(
        pending
            .iter()
            .map(|m| (m.id, m.attempts))
            .collect::<Vec<_>>(),
        [(a, 1), (b, 0)]
    );
    s.outbox_sent(a).await.unwrap();
    assert_eq!(s.outbox_pending().await.unwrap().len(), 1);
    db.drop().await;
}
