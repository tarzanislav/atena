//! Hostile content against the whole chain (pillar 3): hidden characters, dangerous addresses and
//! impossible dates never reach the database, and a failure of the store is a failed check with a
//! pause – never a loop that hammers the publisher. Against a local web server and a real
//! PostgreSQL (`ATENA_TEST_DATABASE_URL`).

use std::sync::Arc;
use std::time::Duration;

use atena_core::Taxonomy;
use atena_crawl::Fetcher;
use atena_crawl::testing::{Server, reply, test_config};
use atena_ingest::{Pipeline, PipelineConfig, WorkerConfig, run};
use atena_store::testing::TestDb;
use atena_store::{EndpointKind, NewEndpoint, Origin, RegisterSource, SourceRecord, Store};
use tokio::sync::watch;

fn hostile_page() -> String {
    let long = format!("/skola/{}", "a".repeat(3_000));
    format!(
        "<!doctype html><html lang=\"sk\"><head>\
         <title>Oznam</title>\
         <link rel=\"canonical\" href=\"javascript:alert(document.domain)\">\
         <meta name=\"tdm-policy\" content=\"<script>alert(1)</script>\">\
         <meta property=\"article:published_time\" content=\"2999-01-01T00:00:00Z\">\
         </head><body><main><h1>Oznam\u{1b}[2J \u{202e}fdp.exe\u{202c} riaditeľa</h1>\
         <p>Riaditeľ\u{0} školy oznamuje rodičom, že v piatok bude riaditeľské voľno a vyučovanie sa nekoná.</p>\
         <p><a href=\"javascript:alert(1)\">klikni</a> <a href=\"data:text/html,&lt;script&gt;\">dáta</a>\
         <a href=\"{long}\">dlhý odkaz</a> <a href=\"/skola/dokument.txt\">dokument</a> <a href=\"/skola/program.pdf\">tlačivo</a></p>\
         </main></body></html>"
    )
}

async fn serve() -> Server {
    let feed = format!(
        "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>Oznamy</title>\
         <item><title>Dlhý</title><link>/skola/{}</link></item>\
         <item><title>Skript</title><link>javascript:alert(1)</link></item>\
         <item><title>Cudzí web</title><link>https://www.example.com/clanok</link></item>\
         <item><title>Zamestnanci</title><link>/skola/zamestnanci/</link></item></channel></rss>",
        "b".repeat(3_000)
    );
    Server::start(move |req| match req.path.as_str() {
        "/skola/" => reply(200, hostile_page()),
        // a program posing as a PDF
        "/skola/program.pdf" => reply(200, &b"MZ\x90\x00\x03\x00\x00\x00%PDF-1.7"[..]),
        "/skola/rss.xml" => reply(200, feed.clone()).header("content-type", "application/rss+xml"),
        // an address that leads to an erased page
        "/presun/" => reply(301, "").header("location", "/skola/vymazane.html"),
        "/skola/vymazane.html" => reply(200, "<html><body><p>Ján Novák, učiteľ</p></body></html>"),
        "/zlyha/" => reply(
            200,
            "<html><body><main><p>Stránka, ktorú databáza odmietne uložiť, lebo je to skúška.</p></main></body></html>",
        ),
        _ => reply(404, "not found"),
    })
    .await
}

async fn register(store: &Store, server: &Server) {
    let endpoint = |path: &str, kind| NewEndpoint {
        url: server.url(path).to_string(),
        host: "127.0.0.1".into(),
        kind,
        origin: Origin::Register,
        follow_links: true,
        interval_s: 86_400,
    };
    store
        .sync_register(&[RegisterSource {
            source: SourceRecord {
                id: "H01".into(),
                name: "Škola".into(),
                url: server.url("/skola/").to_string(),
                country: "SK".into(),
                languages: vec!["sk".into()],
                authority: "C1".into(),
                priority: "P0".into(),
                connector: "html".into(),
                interval_s: 86_400,
                licence: "copyrighted".into(),
                terms: "allowed".into(),
                enabled: true,
            },
            endpoints: vec![
                endpoint("/skola/", EndpointKind::Page),
                endpoint("/skola/rss.xml", EndpointKind::Feed),
                endpoint("/zlyha/", EndpointKind::Page),
                endpoint("/presun/", EndpointKind::Page),
            ],
        }])
        .await
        .unwrap();
}

async fn run_once(store: &Store, server: &Server) {
    let pipeline = Arc::new(Pipeline::new(
        store.clone(),
        Arc::new(Fetcher::new(test_config(server)).unwrap()),
        Arc::new(Taxonomy::bundled().unwrap()),
        PipelineConfig::default(),
    ));
    let config = WorkerConfig {
        worker_id: "test-worker".into(),
        concurrency: 2,
        idle_wait: Duration::from_millis(50),
        ..WorkerConfig::default()
    };
    let (_stop, stop_rx) = watch::channel(false);
    // an endpoint left due after a failure would keep this run going until the timeout
    tokio::time::timeout(
        Duration::from_secs(60),
        run(pipeline, config, stop_rx, true),
    )
    .await
    .expect("the run finishes: nothing is fetched again and again");
}

async fn text(store: &Store, sql: &str) -> Vec<String> {
    sqlx::query_scalar::<_, String>(sql)
        .fetch_all(store.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn hostile_content_is_cleaned_and_failures_pause() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = &db.store;
    let server = serve().await;
    register(store, &server).await;
    // a page erased on request must not come back through a redirect
    store
        .forget(
            server.url("/skola/vymazane.html").as_str(),
            false,
            "erasure request",
        )
        .await
        .unwrap();
    // the store refuses this one page, as it would refuse a value it cannot hold
    sqlx::query("ALTER TABLE document ADD CONSTRAINT test_refuse CHECK (url NOT LIKE '%/zlyha/%')")
        .execute(store.pool())
        .await
        .unwrap();
    run_once(store, &server).await;

    // hidden characters are gone from titles and texts
    let titles = text(store, "SELECT title FROM document WHERE title IS NOT NULL").await;
    assert_eq!(titles, ["Oznam[2J fdp.exe riaditeľa"]);
    let bodies = text(
        store,
        "SELECT body FROM document_version WHERE body IS NOT NULL",
    )
    .await;
    assert!(bodies[0].contains("Riaditeľ školy oznamuje"));
    assert!(!bodies[0].contains('\u{0}'));

    // only plain web addresses are stored, and none longer than the limit
    let links = text(store, "SELECT url FROM document_link ORDER BY url").await;
    assert_eq!(links.len(), 2, "{links:?}");
    assert!(links[0].ends_with("/skola/dokument.txt"));
    let endpoints = text(store, "SELECT url FROM endpoint").await;
    assert!(
        endpoints.iter().all(|u| u.starts_with("http://127.0.0.1:")
            && u.len() <= 2_000
            && !u.contains("zamestnanci")),
        "no other site, no page about people: {endpoints:?}"
    );
    let unsafe_values: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM document_version WHERE canonical_url IS NOT NULL OR published_at IS NOT NULL",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(
        unsafe_values, 0,
        "javascript: canonical and a date in 2999 are dropped"
    );
    let policies = text(
        store,
        "SELECT tdm_policy FROM document WHERE tdm_policy IS NOT NULL",
    )
    .await;
    assert!(policies.is_empty());

    // the page the store refused: one failed check, then a pause of at least an hour
    let (outcome, checks, paused): (String, i64, bool) = sqlx::query_as(
        "SELECT e.last_outcome,
                (SELECT count(*) FROM fetch_log f WHERE f.endpoint_id = e.id),
                e.next_fetch_at > now() + interval '50 minutes'
         FROM endpoint e WHERE e.url LIKE '%/zlyha/'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!((outcome.as_str(), checks, paused), ("store_error", 1, true));

    // the program posing as a PDF: refused, nothing kept, never fetched again
    let (outcome, enabled, documents): (String, bool, i64) = sqlx::query_as(
        "SELECT e.last_outcome, e.enabled,
                (SELECT count(*) FROM document d WHERE d.endpoint_id = e.id)
         FROM endpoint e WHERE e.url LIKE '%/program.pdf'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(
        (outcome.as_str(), enabled, documents),
        ("dangerous", false, 0)
    );

    // the erased page reached through a redirect: nothing stored
    let erased: Vec<String> = text(
        store,
        "SELECT last_outcome FROM endpoint WHERE url LIKE '%/presun/'",
    )
    .await;
    assert_eq!(erased, ["erased"]);
    let kept = text(
        store,
        "SELECT url FROM document WHERE url LIKE '%vymazane%'",
    )
    .await;
    assert!(kept.is_empty());
    db.drop().await;
}

#[test]
fn the_crawler_fetches_only_what_may_be_stored() {
    assert_eq!(atena_crawl::MAX_URL_LEN, atena_core::safety::MAX_URL_LEN);
}
