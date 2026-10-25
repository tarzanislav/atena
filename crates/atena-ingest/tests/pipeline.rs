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
        <iframe title="Prehliadka školy – riaditel@zs-prikladna.sk" src="https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ"></iframe>
        <p>Video si pozrite aj <a href="https://youtu.be/dQw4w9WgXcQ">na YouTube</a>.</p>
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
        "/cudzi/" => reply(200, article("Metodika čitateľskej gramotnosti")),
        "/skola/vyhrada.html" => reply(200, article("Podmienky používania webu")).header("tdm-reservation", "1"),
        _ => reply(404, "not found"),
    })
    .await
}

async fn run_once(store: &Store, server: &Server) -> RunStats {
    run_with(store, server, PipelineConfig::default()).await
}

async fn run_with(store: &Store, server: &Server, pipeline: PipelineConfig) -> RunStats {
    let fetcher = Fetcher::new(test_config(server)).unwrap();
    let pipeline = Arc::new(Pipeline::new(
        store.clone(),
        Arc::new(fetcher),
        Arc::new(Taxonomy::bundled().unwrap()),
        pipeline,
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
        depth: 0,
        interval_s: 86_400,
    };
    store
        .sync_register(&[
            RegisterSource {
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
                    // verified terms in an EU country: texts may be kept for change detection
                    terms: "allowed".into(),
                    enabled: true,
                    site_wide: false,
                },
                endpoints: vec![
                    endpoint("/skola/", EndpointKind::Page, true),
                    endpoint("/skola/rss.xml", EndpointKind::Feed, false),
                ],
            },
            RegisterSource {
                source: SourceRecord {
                    id: "T02".into(),
                    name: "Neoverený zdroj".into(),
                    url: server.url("/cudzi/").to_string(),
                    country: "SK".into(),
                    languages: vec!["sk".into()],
                    authority: "C1".into(),
                    priority: "P1".into(),
                    connector: "html".into(),
                    interval_s: 86_400,
                    licence: "copyrighted".into(),
                    // terms not verified yet: the legality principle allows metadata only
                    terms: "unknown".into(),
                    enabled: true,
                    site_wide: false,
                },
                endpoints: vec![endpoint("/cudzi/", EndpointKind::Page, false)],
            },
        ])
        .await
        .unwrap();

    // ---- round 1: everything is new
    let stats = run_once(store, &server).await;
    assert_eq!(stats.store_errors, 0, "{stats:?}");
    assert_eq!(stats.outcomes.get("new"), Some(&7), "{stats:?}");
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
            "/cudzi/",
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
        8
    );
    // the embedded and linked video is recorded once, cleaned of personal data, and never fetched
    let videos: Vec<(String, String)> = sqlx::query_as(
        "SELECT url, text FROM document_link WHERE document_id = $1 AND kind = 'video'",
    )
    .bind(skola)
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert_eq!(
        videos,
        [(
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
            "Prehliadka školy – [email]".to_string()
        )]
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
        count(
            store,
            "SELECT count(*) FROM document_version WHERE content_type <> 'text/html' AND format = 'html'"
        )
        .await,
        0,
        "the type ATENA recognised is recorded, not the server's claim"
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

    // terms not verified: metadata only, classification from the title alone
    let unverified = document_id(store, "/cudzi/").await;
    let versions = store.versions(unverified).await.unwrap();
    assert!(!versions[0].has_body);
    assert_eq!(
        versions[0].title.as_deref(),
        Some("Metodika čitateľskej gramotnosti")
    );
    let reasons: String = sqlx::query_scalar("SELECT detail FROM fetch_log l JOIN endpoint e ON e.id = l.endpoint_id WHERE e.url LIKE '%/cudzi/'")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert!(
        reasons.contains("metadata only: copyrighted content: the terms of use are not verified"),
        "{reasons}"
    );

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
    assert_eq!(stats.outcomes.get("unchanged"), Some(&6), "{stats:?}");

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

#[tokio::test]
async fn official_sources_crawled_site_wide_follow_links_level_by_level() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = &db.store;
    // a ministry-like site: flat addresses, so nothing lies "below" the entry page
    let page = |title: &str, next: &str| {
        format!(
            "<html><body><article><h1>{title}</h1><p>{title}: usmernenie ministerstva pre školy \
             o organizácii vyučovania a hodnotení žiakov v školskom roku.</p>\
             <a href=\"{next}\">ďalej</a> <a href=\"/kontakty/zamestnanci/\">zamestnanci</a></article></body></html>"
        )
    };
    let server = Server::start(move |req| match req.path.as_str() {
        "/1-sk/vstup/" => reply(200, page("Vstup", "/2-sk/prva/")),
        "/2-sk/prva/" => reply(200, page("Prvá", "/3-sk/druha/")),
        "/3-sk/druha/" => reply(200, page("Druhá", "/4-sk/tretia/")),
        "/4-sk/tretia/" => reply(200, page("Tretia", "/5-sk/stvrta/")),
        "/5-sk/stvrta/" => reply(200, page("Štvrtá", "/1-sk/vstup/")),
        _ => reply(404, "not found"),
    })
    .await;
    let depth = atena_core::source::SITE_DEPTH - 1;
    store
        .sync_register(&[RegisterSource {
            source: SourceRecord {
                id: "T03".into(),
                name: "Ministerstvo".into(),
                url: server.url("/1-sk/vstup/").to_string(),
                country: "SK".into(),
                languages: vec!["sk".into()],
                authority: "A2".into(),
                priority: "P0".into(),
                connector: "html".into(),
                interval_s: 86_400,
                licence: "copyrighted".into(),
                terms: "allowed".into(),
                enabled: true,
                site_wide: true,
            },
            endpoints: vec![NewEndpoint {
                url: server.url("/1-sk/vstup/").to_string(),
                host: "127.0.0.1".into(),
                kind: EndpointKind::Page,
                origin: Origin::Register,
                follow_links: true,
                depth,
                interval_s: 86_400,
            }],
        }])
        .await
        .unwrap();
    let stats = run_once(store, &server).await;
    assert_eq!(stats.store_errors, 0, "{stats:?}");
    // the entry page and three levels of links below it – never the fourth, never people's pages
    let mut found: Vec<String> = sqlx::query_scalar("SELECT url FROM document ORDER BY url")
        .fetch_all(store.pool())
        .await
        .unwrap();
    found.iter_mut().for_each(|u| {
        *u = url::Url::parse(u).unwrap().path().to_string();
    });
    assert_eq!(
        found,
        [
            "/1-sk/vstup/",
            "/2-sk/prva/",
            "/3-sk/druha/",
            "/4-sk/tretia/"
        ]
    );
    let depths: Vec<(String, bool, i16)> =
        sqlx::query_as("SELECT url, follow_links, depth FROM endpoint ORDER BY url")
            .fetch_all(store.pool())
            .await
            .unwrap();
    let of = |path: &str| {
        depths
            .iter()
            .find(|(u, _, _)| u.ends_with(path))
            .map(|(_, f, d)| (*f, *d))
    };
    assert_eq!(of("/2-sk/prva/"), Some((true, 1)));
    assert_eq!(of("/3-sk/druha/"), Some((true, 0)));
    assert_eq!(of("/4-sk/tretia/"), Some((false, 0)));
    assert_eq!(of("/5-sk/stvrta/"), None);
    assert_eq!(of("/kontakty/zamestnanci/"), None);
    db.drop().await;
}

/// A one-page PDF with `text` in Helvetica.
fn pdf(text: &str) -> Vec<u8> {
    let content = format!("BT /F1 12 Tf 72 720 Td ({text}) Tj ET");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> >> >>"
            .into(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        out.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// Every address is checked again now.
async fn due_again(store: &Store) {
    for sql in [
        "UPDATE endpoint SET next_fetch_at = now() - interval '1 s'",
        "UPDATE host SET next_due_at = now() - interval '1 s'",
    ] {
        sqlx::query(sql).execute(store.pool()).await.unwrap();
    }
}

fn register_source(server: &Server, id: &str, path: &str, licence: &str) -> RegisterSource {
    RegisterSource {
        source: SourceRecord {
            id: id.into(),
            name: format!("Zdroj {id}"),
            url: server.url(path).to_string(),
            country: "SK".into(),
            languages: vec!["sk".into()],
            authority: "A1".into(),
            priority: "P0".into(),
            connector: "html".into(),
            interval_s: 86_400,
            licence: licence.into(),
            terms: "allowed".into(),
            enabled: true,
            site_wide: false,
        },
        endpoints: vec![NewEndpoint {
            url: server.url(path).to_string(),
            host: "127.0.0.1".into(),
            kind: EndpointKind::Page,
            origin: Origin::Register,
            follow_links: true,
            depth: 0,
            interval_s: 86_400,
        }],
    }
}

#[tokio::test]
async fn original_files_of_official_documents_are_archived_and_outlive_the_web() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = &db.store;
    let removed = Arc::new(AtomicUsize::new(0));
    let gone = Arc::clone(&removed);
    let index = "<html lang=\"sk\"><body><main><h1>Predpisy</h1><p>Zbierka predpisov pre školy \
                 a školské zariadenia v Slovenskej republike.</p>\
                 <a href=\"zakon.pdf\">zákon</a> <a href=\"oznam.pdf\">oznam</a> \
                 <a href=\"vyklad.html\">výklad</a></main></body></html>";
    let server = Server::start(move |req| match req.path.as_str() {
        "/predpisy/" | "/metodiky/" => reply(200, index),
        "/predpisy/zakon.pdf" if gone.load(Ordering::SeqCst) == 0 => reply(
            200,
            pdf("Zakon o vychove a vzdelavani a o zmene a doplneni niektorych zakonov"),
        )
        .header("content-type", "application/pdf"),
        "/metodiky/zakon.pdf" => reply(200, pdf("Metodika hodnotenia ziakov v zakladnej skole"))
            .header("content-type", "application/pdf"),
        "/predpisy/oznam.pdf" | "/metodiky/oznam.pdf" => reply(
            200,
            pdf("Kontakt na referenta: referent@ministerstvo.example.sk"),
        )
        .header("content-type", "application/pdf"),
        "/predpisy/vyklad.html" | "/metodiky/vyklad.html" => reply(
            200,
            "<html><body><main><p>Výklad zákona pre riaditeľov škôl a školských zariadení \
             o povinnej školskej dochádzke.</p></main></body></html>",
        ),
        _ => reply(404, "not found"),
    })
    .await;
    store
        .sync_register(&[
            register_source(&server, "T10", "/predpisy/", "official"),
            register_source(&server, "T11", "/metodiky/", "copyrighted"),
        ])
        .await
        .unwrap();
    let files = |sql: &'static str| count(store, sql);

    // first without the archive: no files at all
    let stats = run_with(
        store,
        &server,
        PipelineConfig {
            archive_files: false,
            ..PipelineConfig::default()
        },
    )
    .await;
    assert_eq!(stats.store_errors, 0, "{stats:?}");
    assert_eq!(files("SELECT count(*) FROM version_file").await, 0);
    let law = document_id(store, "/predpisy/zakon.pdf").await;
    assert!(store.archived_file(law).await.unwrap().is_none());
    // the PDF with an e-mail address was read (and the address removed from its text)
    let notice = document_id(store, "/predpisy/oznam.pdf").await;

    // with it: the unchanged official PDF gets its file, nothing else does – not a copyrighted
    // PDF, not a PDF with personal data, not a web page
    due_again(store).await;
    let stats = run_once(store, &server).await;
    assert_eq!(stats.store_errors, 0, "{stats:?}");
    assert_eq!(files("SELECT count(*) FROM version_file").await, 1);
    let archived = store.archived_file(law).await.unwrap().unwrap();
    assert_eq!(archived.media_type, "application/pdf");
    let (_, content) = store.archived_content(law).await.unwrap().unwrap();
    assert!(content.starts_with(b"%PDF"));
    assert_eq!(archived.byte_size, content.len() as i64);
    assert_eq!(archived.raw_hash, atena_store::hash(&content));
    assert_eq!(store.archive_rights().await.unwrap().len(), 1);
    assert!(store.archived_file(notice).await.unwrap().is_none());

    // the publisher removes it: the document is gone, the archive keeps the file
    removed.store(1, Ordering::SeqCst);
    due_again(store).await;
    run_once(store, &server).await;
    let row = store.search_rows(&[law]).await.unwrap().remove(0);
    assert_eq!(row.status, "gone");
    assert!(row.gone_at.is_some());
    assert!(store.archived_file(law).await.unwrap().is_some());
    let overview = store.overview().await.unwrap();
    assert_eq!((overview.archived_files, overview.gone), (1, 1));
    assert_eq!(overview.archived_bytes, content.len() as i64);

    // files whose rights changed are deleted
    assert_eq!(store.delete_files(&[law]).await.unwrap(), 1);
    assert!(store.archived_file(law).await.unwrap().is_none());
    db.drop().await;
}
