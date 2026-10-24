//! The whole chain against a local web server and a real PostgreSQL (see `atena_store::testing`):
//! register → worker → fetch → extraction → versions → classification → discovery.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use atena_core::Taxonomy;
use atena_crawl::Fetcher;
use atena_crawl::testing::{Server, reply, test_config};
use atena_ingest::{Pipeline, PipelineConfig, RunStats, WorkerConfig, run};
use atena_store::testing::TestDb;
use atena_store::{
    EndpointKind, LabelDecision, NewEndpoint, Origin, RegisterSource, SourceRecord, Store,
};
use tokio::sync::watch;

fn school_page(date: &str) -> String {
    format!(
        r#"<!doctype html><html lang="sk"><head><title>ZŠ Príkladná – Zápis</title>
        <link rel="alternate" type="application/rss+xml" href="/skola/rss.xml"></head>
        <body><nav><a href="/kontakt/">Kontakt</a></nav>
        <main><h1>Zápis do prvého ročníka</h1>
        <p>Zápis detí do prvého ročníka základnej školy sa uskutoční {date}. Zákonný zástupca prinesie žiadosť o prijatie a rodný list dieťaťa.</p>
        <p>Ďalšie informácie nájdete v <a href="/skola/oznamy/zapis.html">ozname riaditeľa</a>, v <a href="/skola/dokumenty/kriteria.txt">kritériách prijatia</a>
        a v <a href="/skola/vyhrada.html">podmienkach používania</a>. Pozri aj <a href="https://www.example.com/x.pdf">externý dokument</a>
        a <a href="/skola/zmazane.html">starý oznam</a>.</p>
        <p>Kontakt: riaditel@zs-prikladna.sk, tel. 0905 123 456. <a href="/skola/zamestnanci/">Zamestnanci</a>,
        <a href="/skola/fotogaleria/vylet.html">fotogaléria</a>.</p></main></body></html>"#
    )
}

const FEED: &str = r#"<?xml version="1.0"?><rss version="2.0"><channel><title>Oznamy</title>
    <item><title>Riaditeľské voľno</title><link>/skola/clanok-1.html?utm_source=rss</link></item>
    <item><title>Plavecký výcvik</title><link>/skola/clanok-2.html</link></item></channel></rss>"#;

fn article(title: &str) -> String {
    format!(
        "<html><body><article><h1>{title}</h1><p>{title}: oznamujeme rodičom žiakov, že v piatok bude riaditeľské voľno a školský klub bude zatvorený.</p></article></body></html>"
    )
}

async fn serve(round: Arc<AtomicUsize>) -> Server {
    Server::start(move |req| match req.path.as_str() {
        "/skola/" => reply(
            200,
            school_page(if round.load(Ordering::SeqCst) == 1 { "15. apríla 2026" } else { "22. apríla 2026" }),
        ),
        "/skola/rss.xml" => reply(200, FEED).header("content-type", "application/rss+xml"),
        "/skola/oznamy/zapis.html" => reply(200, article("Oznam o zápise")),
        "/skola/clanok-1.html" => reply(200, article("Riaditeľské voľno")),
        "/skola/clanok-2.html" => reply(200, article("Plavecký výcvik")),
        "/skola/dokumenty/kriteria.txt" => reply(
            200,
            "Kritériá prijatia\n\nPrednostne sa prijímajú deti s trvalým pobytom v školskom obvode.",
        )
        .header("content-type", "text/plain; charset=utf-8"),
        "/skola/vyhrada.html" => reply(200, article("Podmienky používania webu")).header("tdm-reservation", "1"),
        _ => reply(404, "not found"),
    })
    .await
}

async fn run_once(store: &Store, server: &Server) -> RunStats {
    let fetcher = Fetcher::new(test_config(server)).unwrap();
    let pipeline = Arc::new(Pipeline::new(
        store.clone(),
        Arc::new(fetcher),
        Arc::new(Taxonomy::bundled().unwrap()),
        PipelineConfig::default(),
    ));
    let config = WorkerConfig {
        worker_id: "test-worker".into(),
        concurrency: 4,
        idle_wait: Duration::from_millis(50),
        ..WorkerConfig::default()
    };
    let (_stop_tx, stop_rx) = watch::channel(false);
    tokio::time::timeout(
        Duration::from_secs(120),
        run(pipeline, config, stop_rx, true),
    )
    .await
    .expect("the run finishes")
}

async fn count(store: &Store, sql: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(store.pool())
        .await
        .unwrap()
}

async fn document_id(store: &Store, path: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM document WHERE url LIKE '%' || $1")
        .bind(path)
        .fetch_one(store.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn register_to_versions_and_classification() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = &db.store;
    let round = Arc::new(AtomicUsize::new(1));
    let server = serve(Arc::clone(&round)).await;
    let endpoint = |path: &str, kind, follow_links| NewEndpoint {
        url: server.url(path).to_string(),
        host: "127.0.0.1".into(),
        kind,
        origin: Origin::Register,
        follow_links,
        interval_s: 86_400,
    };
    store
        .sync_register(&[RegisterSource {
            source: SourceRecord {
                id: "T01".into(),
                name: "ZŠ Príkladná".into(),
                url: server.url("/skola/").to_string(),
                country: "SK".into(),
                languages: vec!["sk".into()],
                authority: "C1".into(),
                priority: "P0".into(),
                connector: "html".into(),
                interval_s: 86_400,
                licence: "copyrighted".into(),
                terms: "unknown".into(),
                enabled: true,
            },
            endpoints: vec![
                endpoint("/skola/", EndpointKind::Page, true),
                endpoint("/skola/rss.xml", EndpointKind::Feed, false),
            ],
        }])
        .await
        .unwrap();

    // ---- round 1: everything is new
    let stats = run_once(store, &server).await;
    assert_eq!(stats.store_errors, 0, "{stats:?}");
    assert_eq!(stats.outcomes.get("new"), Some(&6), "{stats:?}");
    assert_eq!(stats.outcomes.get("feed"), Some(&1));
    assert_eq!(stats.outcomes.get("gone"), Some(&1));

    // links: attachments and subpages of the section; not the external file, not the menu
    let urls: Vec<String> = sqlx::query_scalar("SELECT url FROM endpoint ORDER BY url")
        .fetch_all(store.pool())
        .await
        .unwrap();
    let paths: Vec<&str> = urls
        .iter()
        .map(|u| u.split_once(&server.port().to_string()).unwrap().1)
        .collect();
    assert_eq!(
        paths,
        [
            "/skola/",
            "/skola/clanok-1.html",
            "/skola/clanok-2.html",
            "/skola/dokumenty/kriteria.txt",
            "/skola/oznamy/zapis.html",
            "/skola/rss.xml",
            "/skola/vyhrada.html",
            "/skola/zmazane.html",
        ],
        "tracking parameters removed; external, menu and personal pages not followed"
    );
    assert_eq!(server.requests_to("/robots.txt").len(), 1);

    let skola = document_id(store, "/skola/").await;
    let history = store.versions(skola).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].change_kind, "initial");
    assert_eq!(history[0].title.as_deref(), Some("Zápis do prvého ročníka"));
    let language: String = sqlx::query_scalar("SELECT language FROM document WHERE id = $1")
        .bind(skola)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(language, "sk");
    assert_eq!(
        count(
            store,
            &format!("SELECT count(*) FROM document_link WHERE document_id = {skola}")
        )
        .await,
        7
    );
    let body = store.version_body(history[0].id).await.unwrap().unwrap();
    assert!(body.contains("Kontakt: [email], tel. [phone]."), "{body}");
    assert!(!body.contains("0905") && !body.contains("riaditel@"));
    let labels = store.labels(skola).await.unwrap();
    assert!(
        labels.iter().any(|l| l.scheme == "school-type"),
        "{labels:?}"
    );
    assert_eq!(
        count(store, "SELECT count(*) FROM blob").await,
        0,
        "of web pages only the cleaned text is kept"
    );

    // the publisher reserved text and data mining: metadata only
    let reserved = document_id(store, "/skola/vyhrada.html").await;
    let (tdm, body): (bool, Option<String>) = sqlx::query_as(
        "SELECT d.tdm_reserved, v.body FROM document d JOIN document_version v ON v.id = d.current_version_id WHERE d.id = $1",
    )
    .bind(reserved)
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(tdm && body.is_none());

    // an editor rejects one suggestion
    let rejected = labels[0].clone();
    store
        .decide_label(
            skola,
            &rejected.scheme,
            &rejected.concept,
            LabelDecision::Reject,
            "editor",
        )
        .await
        .unwrap();

    // ---- round 2: the date on the page changes, everything else stays
    round.store(2, Ordering::SeqCst);
    sqlx::query("UPDATE endpoint SET next_fetch_at = now() - interval '1 s'")
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE host SET next_due_at = now() - interval '1 s'")
        .execute(store.pool())
        .await
        .unwrap();
    let stats = run_once(store, &server).await;
    assert_eq!(stats.store_errors, 0, "{stats:?}");
    assert_eq!(stats.outcomes.get("changed"), Some(&1), "{stats:?}");
    assert_eq!(stats.outcomes.get("unchanged"), Some(&5), "{stats:?}");

    let history = store.versions(skola).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(
        history[0].change_kind, "substantive",
        "a changed date is substantive"
    );
    let after = store.labels(skola).await.unwrap();
    let decided = after
        .iter()
        .find(|l| l.scheme == rejected.scheme && l.concept == rejected.concept)
        .unwrap();
    assert_eq!(
        decided.status, "rejected",
        "the editor's decision survives a new version"
    );

    // the deleted page was checked again and failed again
    let fails = count(
        store,
        "SELECT fail_count::bigint FROM endpoint WHERE url LIKE '%zmazane.html'",
    )
    .await;
    assert_eq!(fails, 2);
    db.drop().await;
}
