//! The store against a real PostgreSQL. Set `ATENA_TEST_DATABASE_URL` to a connection that may
//! create databases (e.g. `postgres://postgres@localhost/postgres`); without it these tests are skipped.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use atena_store::{
    BlobStore, Capture, Check, EndpointKind, LabelDecision, LabelSuggestion, NewEndpoint,
    NewVersion, Origin, RegisterSource, Rights, SourceRecord, Store, hash,
};
use jiff::Timestamp;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, Executor, PgConnection};

static COUNTER: AtomicU32 = AtomicU32::new(0);

struct TestDb {
    admin: String,
    name: String,
    root: PathBuf,
    store: Store,
}

impl TestDb {
    async fn new() -> Option<Self> {
        let Ok(admin) = std::env::var("ATENA_TEST_DATABASE_URL") else {
            eprintln!("ATENA_TEST_DATABASE_URL is not set – database tests skipped");
            return None;
        };
        let name = format!(
            "atena_t_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let mut conn = PgConnection::connect(&admin)
            .await
            .expect("admin connection");
        conn.execute(format!("CREATE DATABASE {name}").as_str())
            .await
            .expect("create database");
        conn.close().await.ok();
        let options: PgConnectOptions = admin.parse().expect("database URL");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect_with(options.database(&name))
            .await
            .expect("test database");
        let root = std::env::temp_dir().join(format!("atena-store-{name}"));
        let store = Store::from_pool(pool, BlobStore::new(&root));
        store.migrate().await.expect("migrations");
        Some(Self {
            admin,
            name,
            root,
            store,
        })
    }

    async fn drop(self) {
        self.store.pool().close().await;
        let mut conn = PgConnection::connect(&self.admin)
            .await
            .expect("admin connection");
        conn.execute(format!("DROP DATABASE {} WITH (FORCE)", self.name).as_str())
            .await
            .ok();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn source(id: &str, enabled: bool) -> SourceRecord {
    SourceRecord {
        id: id.into(),
        name: format!("Zdroj {id}"),
        url: format!("https://{}.example.sk/", id.to_lowercase()),
        country: "SK".into(),
        languages: vec!["sk".into()],
        authority: "A2".into(),
        priority: "P0".into(),
        connector: "html".into(),
        interval_s: 3600,
        enabled,
    }
}

fn endpoint(url: &str, kind: EndpointKind, origin: Origin) -> NewEndpoint {
    let host = url::Url::parse(url)
        .unwrap()
        .host_str()
        .unwrap()
        .to_string();
    NewEndpoint {
        url: url.into(),
        host,
        kind,
        origin,
        follow_links: kind == EndpointKind::Page && origin == Origin::Register,
        interval_s: 3600,
    }
}

fn capture(endpoint_id: i64, text: &str, rights: Rights) -> Capture {
    Capture {
        endpoint_id,
        source_id: "SK01".into(),
        url: "https://sk01.example.sk/".into(),
        format: "html".into(),
        raw_hash: hash(format!("<p>{text}</p>").as_bytes()),
        fingerprint: hash(text.as_bytes()),
        fetched_at: Timestamp::now(),
        rights,
    }
}

fn version(kind: &str, body: &str) -> NewVersion {
    NewVersion {
        change_kind: kind.into(),
        title: Some("Zápis do 1. ročníka".into()),
        body: Some(body.into()),
        language: Some("sk".into()),
        language_source: Some("detected".into()),
        ..NewVersion::default()
    }
}

fn suggestion(scheme: &str, concept: &str) -> LabelSuggestion {
    LabelSuggestion {
        scheme: scheme.into(),
        concept: concept.into(),
        score: 0.7,
        evidence: vec!["sk:zápis×2".into(), "sk:ročník×1".into()],
    }
}

async fn scalar_i64(store: &Store, sql: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(store.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn register_sync_and_host_leases() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    db.store.migrate().await.expect("migrations are idempotent");

    let register = vec![
        RegisterSource {
            source: source("SK01", true),
            endpoints: vec![
                endpoint(
                    "https://sk01.example.sk/",
                    EndpointKind::Page,
                    Origin::Register,
                ),
                endpoint(
                    "https://sk01.example.sk/rss/",
                    EndpointKind::Feed,
                    Origin::Register,
                ),
            ],
        },
        RegisterSource {
            source: source("SK02", true),
            endpoints: vec![endpoint(
                "https://sk02.example.sk/",
                EndpointKind::Page,
                Origin::Register,
            )],
        },
    ];
    let report = s.sync_register(&register).await.unwrap();
    assert_eq!(
        (report.sources, report.endpoints_added, report.removed),
        (2, 3, 0)
    );

    // two workers: each host goes to exactly one of them
    let a = s
        .lease_hosts("worker-a", 10, Duration::from_secs(600))
        .await
        .unwrap();
    let b = s
        .lease_hosts("worker-b", 10, Duration::from_secs(600))
        .await
        .unwrap();
    assert_eq!(a.len(), 2);
    assert!(b.is_empty());

    let sk01 = a.iter().find(|h| h.name == "sk01.example.sk").unwrap();
    let due = s.due_endpoints(sk01.id, 10).await.unwrap();
    assert_eq!(due.len(), 2);
    assert_eq!(due[0].source_interval_s, 3600);
    for ep in &due {
        let check = Check {
            outcome: "unchanged".into(),
            status: Some(200),
            next_in: Duration::from_secs(3600),
            validators: Some((Some("\"v1\"".into()), None)),
            ..Check::default()
        };
        s.record_check(ep.id, &check).await.unwrap();
    }
    // a release by a worker that does not hold the lease changes nothing
    s.release_host(sk01.id, "worker-b", None).await.unwrap();
    s.release_host(sk01.id, "worker-a", None).await.unwrap();
    assert!(
        s.lease_hosts("worker-b", 10, Duration::from_secs(600))
            .await
            .unwrap()
            .is_empty()
    );
    let etag: Option<String> =
        sqlx::query_scalar("SELECT etag FROM endpoint WHERE url = 'https://sk01.example.sk/'")
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(etag.as_deref(), Some("\"v1\""));
    assert_eq!(scalar_i64(s, "SELECT count(*) FROM fetch_log").await, 2);

    // an expired lease can be taken over; a back-off keeps the host away
    let sk02 = a.iter().find(|h| h.name == "sk02.example.sk").unwrap();
    s.release_host(sk02.id, "worker-a", Some(Duration::from_secs(900)))
        .await
        .unwrap();
    assert!(
        s.lease_hosts("worker-b", 10, Duration::from_secs(600))
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query("UPDATE host SET backoff_until = now() - interval '1 s', next_due_at = now() - interval '1 s'")
        .execute(s.pool())
        .await
        .unwrap();
    assert_eq!(
        s.lease_hosts("worker-b", 10, Duration::ZERO)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        s.lease_hosts("worker-c", 10, Duration::from_secs(60))
            .await
            .unwrap()
            .len(),
        2,
        "an expired lease is taken over"
    );

    // the register drops SK02 and the feed of SK01
    let report = s
        .sync_register(&[RegisterSource {
            source: source("SK01", true),
            endpoints: vec![endpoint(
                "https://sk01.example.sk/",
                EndpointKind::Page,
                Origin::Register,
            )],
        }])
        .await
        .unwrap();
    assert_eq!(
        (
            report.removed,
            report.endpoints_disabled,
            report.endpoints_added
        ),
        (1, 1, 0)
    );
    sqlx::query("UPDATE endpoint SET next_fetch_at = now() - interval '1 s'")
        .execute(s.pool())
        .await
        .unwrap();
    assert!(
        s.due_endpoints(sk02.id, 10).await.unwrap().is_empty(),
        "disabled source"
    );
    assert_eq!(
        s.due_endpoints(sk01.id, 10).await.unwrap().len(),
        1,
        "disabled register endpoint"
    );
    db.drop().await;
}

#[tokio::test]
async fn discovered_endpoints_are_added_once() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    s.sync_register(&[RegisterSource {
        source: source("SK01", true),
        endpoints: vec![],
    }])
    .await
    .unwrap();
    let found = vec![
        endpoint(
            "https://sk01.example.sk/a.pdf",
            EndpointKind::Page,
            Origin::Link,
        ),
        endpoint(
            "https://cdn.example.sk/b.pdf",
            EndpointKind::Page,
            Origin::Link,
        ),
        endpoint(
            "https://sk01.example.sk/a.pdf",
            EndpointKind::Page,
            Origin::Link,
        ),
    ];
    assert_eq!(s.add_endpoints("SK01", None, &found).await.unwrap(), 2);
    assert_eq!(s.add_endpoints("SK01", None, &found).await.unwrap(), 0);
    assert_eq!(
        scalar_i64(s, "SELECT count(*) FROM host WHERE next_due_at IS NOT NULL").await,
        2
    );
    db.drop().await;
}

#[tokio::test]
async fn versions_labels_and_editor_decisions() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    s.sync_register(&[RegisterSource {
        source: source("SK01", true),
        endpoints: vec![endpoint(
            "https://sk01.example.sk/",
            EndpointKind::Page,
            Origin::Register,
        )],
    }])
    .await
    .unwrap();
    let ep: i64 = scalar_i64(s, "SELECT id FROM endpoint").await;
    assert_eq!(s.current_document(ep).await.unwrap(), None);
    assert!(
        !s.touch_document(&capture(ep, "x", Rights::default()))
            .await
            .unwrap()
    );

    let links = vec![(
        "https://sk01.example.sk/ziadost.pdf".to_string(),
        "žiadosť".to_string(),
    )];
    let first = s
        .save_version(
            &capture(ep, "Zápis bude 15. apríla.", Rights::default()),
            &version("initial", "Zápis bude 15. apríla."),
            &links,
            &[
                suggestion("topic", "legislation"),
                suggestion("school-type", "primary-school"),
            ],
        )
        .await
        .unwrap();
    assert_eq!(first.version_no, 1);
    let current = s.current_document(ep).await.unwrap().unwrap();
    assert_eq!(current.body.as_deref(), Some("Zápis bude 15. apríla."));
    assert_eq!(
        current.last_fingerprint,
        Some(hash("Zápis bude 15. apríla.".as_bytes()))
    );

    // an editor rejects one suggestion and confirms one that was never suggested
    s.decide_label(
        first.document_id,
        "topic",
        "legislation",
        LabelDecision::Reject,
        "editor@atena",
    )
    .await
    .unwrap();
    s.decide_label(
        first.document_id,
        "topic",
        "student-assessment",
        LabelDecision::Confirm,
        "editor@atena",
    )
    .await
    .unwrap();

    let second = s
        .save_version(
            &capture(ep, "Zápis bude 22. apríla.", Rights::default()),
            &version("substantive", "Zápis bude 22. apríla."),
            &links,
            &[
                suggestion("topic", "legislation"),
                suggestion("topic", "trends"),
            ],
        )
        .await
        .unwrap();
    assert_eq!(
        (second.document_id, second.version_no),
        (first.document_id, 2)
    );
    let labels = s.labels(first.document_id).await.unwrap();
    let status = |c: &str| {
        labels
            .iter()
            .find(|l| l.concept == c)
            .map(|l| l.status.as_str())
    };
    assert_eq!(
        status("legislation"),
        Some("rejected"),
        "a rejected label is not suggested again"
    );
    assert_eq!(status("student-assessment"), Some("confirmed"));
    assert_eq!(status("trends"), Some("suggested"));
    assert_eq!(
        status("primary-school"),
        None,
        "stale suggestions are removed"
    );
    let evidence: Vec<String> =
        sqlx::query_scalar("SELECT evidence FROM document_label WHERE concept = 'trends'")
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(evidence, ["sk:zápis×2", "sk:ročník×1"]);

    let history = s.versions(first.document_id).await.unwrap();
    assert_eq!(
        history.iter().map(|v| v.version_no).collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(history[0].change_kind, "substantive");
    assert_eq!(
        s.version_body(first.version_id).await.unwrap().as_deref(),
        Some("Zápis bude 15. apríla.")
    );

    // the same text again only touches the document
    assert!(
        s.touch_document(&capture(ep, "Zápis bude 22. apríla.", Rights::default()))
            .await
            .unwrap()
    );
    assert_eq!(s.versions(first.document_id).await.unwrap().len(), 2);

    s.mark_gone(ep).await.unwrap();
    assert_eq!(
        s.current_document(ep).await.unwrap().unwrap().status,
        "gone"
    );
    db.drop().await;
}

#[tokio::test]
async fn a_tdm_reservation_removes_kept_text() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    s.sync_register(&[RegisterSource {
        source: source("SK01", true),
        endpoints: vec![endpoint(
            "https://sk01.example.sk/",
            EndpointKind::Page,
            Origin::Register,
        )],
    }])
    .await
    .unwrap();
    let ep: i64 = scalar_i64(s, "SELECT id FROM endpoint").await;
    let links = vec![("https://sk01.example.sk/a.pdf".to_string(), "a".to_string())];
    let first = s
        .save_version(
            &capture(ep, "Text 1", Rights::default()),
            &version("initial", "Text 1"),
            &links,
            &[suggestion("topic", "ai")],
        )
        .await
        .unwrap();

    // the publisher reserves text and data mining without changing the page
    let reserved = Rights {
        tdm_reserved: true,
        tdm_policy: Some("https://pub.example/policy.json".into()),
        ..Rights::default()
    };
    assert!(
        s.touch_document(&capture(ep, "Text 1", reserved.clone()))
            .await
            .unwrap()
    );
    assert_eq!(s.version_body(first.version_id).await.unwrap(), None);
    assert_eq!(scalar_i64(s, "SELECT count(*) FROM document_link").await, 0);
    assert!(s.labels(first.document_id).await.unwrap().is_empty());

    // a later version under the reservation keeps metadata only
    let second = s
        .save_version(
            &capture(ep, "Text 2", reserved),
            &version("changed", "Text 2"),
            &links,
            &[],
        )
        .await
        .unwrap();
    let history = s.versions(second.document_id).await.unwrap();
    assert!(history.iter().all(|v| !v.has_body));
    assert_eq!(history[0].title.as_deref(), Some("Zápis do 1. ročníka"));
    db.drop().await;
}

#[tokio::test]
async fn blobs_and_log_pruning() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let s = &db.store;
    let html = "<html><body><p>Školský vzdelávací program</p></body></html>".repeat(50);
    let stored = s
        .put_blob(html.clone().into_bytes(), Some("text/html; charset=utf-8"))
        .await
        .unwrap();
    assert!(stored.stored_size < stored.size);
    let again = s
        .put_blob(html.clone().into_bytes(), Some("text/html"))
        .await
        .unwrap();
    assert!(!again.written);
    assert_eq!(
        s.get_blob(stored.hash).await.unwrap().unwrap(),
        html.as_bytes()
    );
    assert_eq!(s.get_blob(hash(b"never stored")).await.unwrap(), None);
    assert_eq!(scalar_i64(s, "SELECT count(*) FROM blob").await, 1);

    s.sync_register(&[RegisterSource {
        source: source("SK01", true),
        endpoints: vec![endpoint(
            "https://sk01.example.sk/",
            EndpointKind::Page,
            Origin::Register,
        )],
    }])
    .await
    .unwrap();
    let ep: i64 = scalar_i64(s, "SELECT id FROM endpoint").await;
    let check = Check {
        outcome: "network".into(),
        failed: true,
        detail: Some("x".repeat(10_000)),
        next_in: Duration::from_secs(60),
        ..Check::default()
    };
    s.record_check(ep, &check).await.unwrap();
    s.record_check(ep, &check).await.unwrap();
    assert_eq!(
        scalar_i64(s, "SELECT fail_count::bigint FROM endpoint").await,
        2
    );
    assert_eq!(
        scalar_i64(s, "SELECT max(length(detail))::bigint FROM fetch_log").await,
        2_000
    );
    sqlx::query("UPDATE fetch_log SET at = now() - interval '100 days' WHERE id = (SELECT min(id) FROM fetch_log)")
        .execute(s.pool())
        .await
        .unwrap();
    assert_eq!(
        s.prune_fetch_log(Duration::from_secs(90 * 86_400))
            .await
            .unwrap(),
        1
    );
    assert!(s.record_check(999_999, &check).await.is_err());
    db.drop().await;
}
