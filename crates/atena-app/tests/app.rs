//! The operations of the desktop app on a test database (`ATENA_TEST_DATABASE_URL`, see
//! `atena_store::testing`); skipped without one. Nothing here goes to the network.

use std::path::PathBuf;

use atena_app::views::SearchQuery;
use std::time::Duration;

use atena_app::{App, Choice, Keychain, PrivateFetch, Settings};
use atena_crawl::testing::{Server, reply, test_config};
use atena_extract::Parser;
use atena_store::testing::TestDb;
use atena_store::{Capture, EndpointKind, NewEndpoint, NewVersion, Origin, Rights, Store, hash};

fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("atena-app-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    folder
}

#[tokio::test]
async fn the_app_opens_with_the_register_of_its_version() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let folder = folder("open");
    let app = App::open(db.store.clone(), &folder).await.unwrap();
    let overview = app.overview().await.unwrap();
    assert!(overview.sources > 50, "{overview:?}");
    assert!(overview.sources_crawled > 0 && overview.sources_crawled < overview.sources);
    assert_eq!(overview.documents, 0);
    assert!(!overview.crawling);
    assert!(folder.join("register.synced").exists());
    assert!(folder.join("index").join("meta.json").exists());

    let sources = app.sources().await.unwrap();
    assert_eq!(
        sources.iter().filter(|s| !s.tool).count() as i64,
        overview.sources
    );
    let youtube = sources.iter().find(|s| s.id == "N39").unwrap();
    assert!(!youtube.enabled && !youtube.allowed.crawl);
    let json = serde_json::to_value(youtube).unwrap();
    assert_eq!(json["allowed"]["restrictions"][0], "terms-forbid");
    assert!(
        json.get("lastCheck").is_some(),
        "camelCase for the interface"
    );

    // tools are listed and described, never crawled; official sources may be crawled site-wide
    let tools = app.tools("en");
    assert_eq!(tools.len(), 9);
    let juls = tools.iter().find(|t| t.id == "NA01").unwrap();
    assert!(juls.description.starts_with("Dictionaries of Slovak"));
    assert_eq!(
        (juls.region.as_str(), juls.trust.as_str()),
        ("sk", "expert")
    );
    let listed = sources.iter().find(|s| s.id == "NA01").unwrap();
    assert!(listed.tool && !listed.enabled);
    assert!(sources.iter().find(|s| s.id == "SK07").unwrap().site_wide);
    assert!(
        overview.sources < sources.len() as i64,
        "tools are not counted as sources"
    );

    let taxonomy = app.taxonomy("sk");
    assert!(taxonomy.schemes.iter().any(|s| s.id == "topic"));

    // nothing collected yet: nothing found, and an empty search is an error the interface knows
    let found = app
        .search(SearchQuery {
            text: "zápis do prvého ročníka".into(),
            ..SearchQuery::default()
        })
        .await
        .unwrap();
    assert_eq!((found.total, found.page, found.pages), (0, 1, 0));
    let empty = app.search(SearchQuery::default()).await.unwrap_err();
    assert_eq!(empty.code, "search-empty");

    assert!(app.newest(Vec::new(), 10).await.unwrap().is_empty());
    assert!(
        app.newest(vec!["section/changes".into()], 10)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        app.newest(vec!["nonsense".into()], 10)
            .await
            .unwrap_err()
            .code,
        "search-filter"
    );
    assert_eq!(app.document(999).await.unwrap(), None);

    // the overview counts every section, also when it is empty
    let tally = app.tally(vec!["region/sk".into()]).await.unwrap();
    assert_eq!((tally.total, tally.new, tally.new_days), (0, 0, 7));
    assert!(
        tally
            .sections
            .iter()
            .any(|c| c.key == "section/e-testing" && c.total == 0)
    );
    assert_eq!(
        tally.sections.len(),
        app.taxonomy("sk")
            .schemes
            .iter()
            .find(|s| s.id == "section")
            .unwrap()
            .concepts
            .len()
    );
    assert_eq!(
        app.tally(vec!["region/moon".into()])
            .await
            .unwrap_err()
            .code,
        "search-filter"
    );
    // the stored documents are classified with this taxonomy in the background
    for _ in 0..100 {
        if folder.join("taxonomy.classified").exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(folder.join("taxonomy.classified").exists());

    assert!(!app.crawl_status().await.running);
    app.close().await;
    std::fs::remove_dir_all(&folder).unwrap();
    db.drop().await;
}

#[tokio::test]
async fn decisions_need_a_person_and_a_reason() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let folder = folder("decide");
    let app = App::open(db.store.clone(), &folder).await.unwrap();

    let no_editor = app
        .confirm("U-0123456789", Choice::default(), "read the terms")
        .await
        .unwrap_err();
    assert_eq!(no_editor.code, "editor-missing");

    let settings = app
        .set_settings(Settings {
            editor: "  Ján  Hraško ".into(),
            ..Settings::default()
        })
        .unwrap();
    assert_eq!(settings.editor, "Ján Hraško");
    assert_eq!(Settings::load(&folder.join("settings.json")), settings);

    let no_reason = app
        .confirm("U-0123456789", Choice::default(), "  ")
        .await
        .unwrap_err();
    assert_eq!(no_reason.code, "reason-missing");
    let unknown = app
        .confirm("U-0123456789", Choice::default(), "read the terms")
        .await
        .unwrap_err();
    assert_eq!(unknown.code, "not-found");
    let bad_country = app
        .confirm(
            "U-0123456789",
            Choice {
                country: Some("XX".into()),
                ..Choice::default()
            },
            "read the terms",
        )
        .await
        .unwrap_err();
    assert_eq!(bad_country.code, "decision-country");
    assert_eq!(
        app.reject("U-0123456789", "not about education")
            .await
            .unwrap_err()
            .code,
        "not-found"
    );
    assert!(app.proposals(Some("proposed")).await.unwrap().is_empty());
    assert_eq!(
        app.proposals(Some("maybe")).await.unwrap_err().code,
        "not-found"
    );

    let bad_url = app
        .forget("javascript:alert(1)", false, "request of 24 Oct")
        .await
        .unwrap_err();
    assert_eq!(bad_url.code, "url");
    let erased = app
        .forget("https://www.example.sk/ludia/", true, "request of 24 Oct")
        .await
        .unwrap();
    assert_eq!((erased.documents, erased.versions), (0, 0));
    assert!(
        db.store
            .is_blocked("https://www.example.sk/ludia/jan")
            .await
            .unwrap()
    );
    app.close().await;
    std::fs::remove_dir_all(&folder).unwrap();
    db.drop().await;
}

#[tokio::test]
async fn proposals_wait_to_be_handed_over_by_the_user() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let folder = folder("share");
    let app = App::open(db.store.clone(), &folder).await.unwrap();
    assert_eq!(app.pending_share().await.unwrap(), None);
    db.store
        .outbox_add("ATENA-NAVRHY 1\nhttps://a.example.sk/ SK\n")
        .await
        .unwrap();
    db.store
        .outbox_add("ATENA-NAVRHY 1\nhttps://b.example.cz/ CZ\nhttps://a.example.sk/ SK\n")
        .await
        .unwrap();
    let pending = app.pending_share().await.unwrap().unwrap();
    assert_eq!(pending.addresses, 2);
    assert_eq!(
        pending.list,
        "ATENA-NAVRHY 1\nhttps://a.example.sk/ SK\nhttps://b.example.cz/ CZ\n"
    );
    // a list added after the user looked stays waiting
    db.store
        .outbox_add("ATENA-NAVRHY 1\nhttps://c.example.pl/ PL\n")
        .await
        .unwrap();
    app.handed_over(&pending.entries).await.unwrap();
    let left = app.pending_share().await.unwrap().unwrap();
    assert_eq!(left.list, "ATENA-NAVRHY 1\nhttps://c.example.pl/ PL\n");

    let not_a_list = app.receive_list("hello").await.unwrap();
    assert_eq!(not_a_list.status, "not-a-list");
    let too_long = app.import(&"x".repeat(2 * 1024 * 1024), None).await;
    assert_eq!(too_long.unwrap_err().code, "input-too-long");
    let bad_country = app.import("https://a.example.sk/", Some("XX")).await;
    assert_eq!(bad_country.unwrap_err().code, "decision-country");
    app.close().await;
    std::fs::remove_dir_all(&folder).unwrap();
    db.drop().await;
}

#[tokio::test]
async fn the_private_account_stays_on_this_computer() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let folder = folder("private");
    // without a credential store: no persistent sign-in
    let app = App::open_with_keychain(db.store.clone(), &folder, None)
        .await
        .unwrap();
    let status = app.private_status().await.unwrap();
    assert!(!status.exists && !status.unlocked);
    assert_eq!(
        app.private_change_password("a".into(), "b".into())
            .await
            .unwrap_err()
            .code,
        "private-locked"
    );
    assert_eq!(
        app.private_create("Ján".into(), "krátke".into())
            .await
            .unwrap_err()
            .code,
        "private-weak"
    );
    let created = app
        .private_create("Ján Hraško".into(), "moje dlhé heslo".into())
        .await
        .unwrap();
    assert!(created.recovery_key.starts_with("ATENA-"));
    assert!(created.status.exists && created.status.unlocked);
    assert_eq!(created.status.name.as_deref(), Some("Ján Hraško"));
    // the account file holds neither the password nor the recovery key
    let file = std::fs::read_to_string(folder.join("private").join("account.json")).unwrap();
    assert!(!file.contains("moje dlhé heslo") && !file.contains(&created.recovery_key[6..]));

    assert!(!app.private_lock().await.unwrap().unlocked);
    assert_eq!(
        app.private_unlock("Ján Hraško".into(), "zlé heslo 123".into())
            .await
            .unwrap_err()
            .code,
        "private-login"
    );
    assert!(
        app.private_unlock("ján hraško".into(), "moje dlhé heslo".into())
            .await
            .unwrap()
            .unlocked
    );
    app.private_change_password("moje dlhé heslo".into(), "druhé dlhé heslo".into())
        .await
        .unwrap();
    app.private_lock().await.unwrap();
    assert!(
        app.private_recover(created.recovery_key.clone(), "tretie dlhé heslo".into())
            .await
            .unwrap()
            .unlocked
    );
    assert_eq!(
        app.private_create("Iný".into(), "iné dlhé heslo".into())
            .await
            .unwrap_err()
            .code,
        "private-exists"
    );
    let status = app.private_status().await.unwrap();
    assert!(!status.remember_available && !status.remembered);
    assert_eq!(
        app.private_remember().await.unwrap_err().code,
        "private-remember-none"
    );
    app.close().await;
    assert!(!app.private_status().await.unwrap().unlocked);
    std::fs::remove_dir_all(&folder).unwrap();
    db.drop().await;
}

#[tokio::test]
async fn the_persistent_sign_in_opens_the_private_part_at_start() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let folder = folder("remember");
    let keychain = Keychain::memory().unwrap();
    let app = App::open_with_keychain(db.store.clone(), &folder, Some(keychain.clone()))
        .await
        .unwrap();
    assert!(app.private_status().await.unwrap().remember_available);
    app.private_create("Eva".into(), "moje dlhé heslo".into())
        .await
        .unwrap();
    app.private_lock().await.unwrap();
    // only the open account can be remembered
    assert_eq!(
        app.private_remember().await.unwrap_err().code,
        "private-locked"
    );
    app.private_unlock("Eva".into(), "moje dlhé heslo".into())
        .await
        .unwrap();
    assert!(app.private_remember().await.unwrap().remembered);
    app.close().await;
    db.drop().await;

    // the next start opens it without the password (the private part lives in the folder; a
    // closed app has closed its database, so each start gets one of its own)
    let db = TestDb::new().await.unwrap();
    let app = App::open_with_keychain(db.store.clone(), &folder, Some(keychain.clone()))
        .await
        .unwrap();
    let status = app.private_status().await.unwrap();
    assert!(status.unlocked && status.remembered);
    assert_eq!(status.name.as_deref(), Some("Eva"));
    // turned off, it stays open now and is locked at the next start
    let status = app.private_forget().await.unwrap();
    assert!(status.unlocked && !status.remembered);
    app.close().await;
    db.drop().await;
    let db = TestDb::new().await.unwrap();
    let app = App::open_with_keychain(db.store.clone(), &folder, Some(keychain))
        .await
        .unwrap();
    assert!(!app.private_status().await.unwrap().unlocked);
    app.close().await;
    std::fs::remove_dir_all(&folder).unwrap();
    db.drop().await;
}

/// A one-page PDF with `text` in Helvetica; with `javascript` it runs a script when opened.
fn pdf(text: &str, javascript: bool) -> Vec<u8> {
    let content = format!("BT /F1 12 Tf 72 720 Td ({text}) Tj ET");
    let catalog = if javascript {
        "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (app.alert(1)) >> >>"
    } else {
        "<< /Type /Catalog /Pages 2 0 R >>"
    };
    let objects = [
        catalog.to_string(),
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

/// Stores a document of `source` at `url` as the crawler would; returns its ID.
async fn document(
    store: &Store,
    source: &str,
    url: &str,
    body: Option<&str>,
    noarchive: bool,
) -> i64 {
    let host = url::Url::parse(url)
        .unwrap()
        .host_str()
        .unwrap()
        .to_string();
    store
        .add_endpoints(
            source,
            None,
            &[NewEndpoint {
                url: url.into(),
                host,
                kind: EndpointKind::Page,
                origin: Origin::Link,
                follow_links: false,
                depth: 0,
                interval_s: 3600,
            }],
        )
        .await
        .unwrap();
    let endpoint: i64 = sqlx::query_scalar("SELECT id FROM endpoint WHERE url = $1")
        .bind(url)
        .fetch_one(store.pool())
        .await
        .unwrap();
    let text = body.unwrap_or("metadata only");
    store
        .save_version(
            &Capture {
                endpoint_id: endpoint,
                source_id: source.into(),
                url: url.into(),
                format: "html".into(),
                raw_hash: hash(text.as_bytes()),
                fingerprint: hash(text.as_bytes()),
                fetched_at: jiff::Timestamp::now(),
                rights: Rights {
                    noarchive,
                    ..Rights::default()
                },
            },
            &NewVersion {
                change_kind: "initial".into(),
                title: Some(format!("Dokument {source}")),
                body: body.map(str::to_string),
                language: Some("sk".into()),
                ..NewVersion::default()
            },
            &[],
            &[],
        )
        .await
        .unwrap()
        .document_id
}

#[tokio::test]
async fn documents_are_saved_for_the_user_alone() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let server = Server::start(|seen| match seen.path.as_str() {
        "/robots.txt" => reply(200, "User-agent: *\nDisallow: /tajne/\n"),
        "/metodika.pdf" => reply(200, pdf("Metodika hodnotenia ziakov v skole", false))
            .header("content-type", "application/pdf"),
        "/skript.pdf" => {
            reply(200, pdf("Oznam riaditela skoly", true)).header("content-type", "application/pdf")
        }
        "/stranka.html" => reply(
            200,
            "<html lang=\"sk\"><head><title>Úlohy</title></head><body><main>\
             <p>Úlohy z finančnej gramotnosti pre žiakov druhého stupňa základnej školy.</p>\
             </main></body></html>",
        )
        .header("content-type", "text/html; charset=utf-8"),
        "/bez-kopie.html" => reply(200, "<html><body><p>Text bez kópie.</p></body></html>")
            .header("content-type", "text/html")
            .header("x-robots-tag", "noarchive"),
        _ => reply(404, ""),
    })
    .await;
    let folder = folder("library");
    let app = App::open_with_keychain(db.store.clone(), &folder, None)
        .await
        .unwrap();
    app.set_private_fetch(
        PrivateFetch::new(
            test_config(&server),
            Parser::InProcess {
                time_limit: Duration::from_secs(30),
            },
        )
        .unwrap(),
    );
    let s = &db.store;
    // an official text the public part shows in full; copyrighted ones it does not
    let law = document(
        s,
        "SK03",
        "https://www.example.sk/zakon",
        Some("Zákon o výchove a vzdelávaní."),
        false,
    )
    .await;
    let method = document(s, "SK36", server.url("/metodika.pdf").as_str(), None, false).await;
    let page = document(s, "SK36", server.url("/stranka.html").as_str(), None, false).await;
    let script = document(s, "SK36", server.url("/skript.pdf").as_str(), None, false).await;
    let secret = document(s, "SK36", server.url("/tajne/x.html").as_str(), None, false).await;
    let no_copy = document(
        s,
        "SK36",
        server.url("/bez-kopie.html").as_str(),
        None,
        false,
    )
    .await;
    let marked = document(s, "SK36", server.url("/oznacene.html").as_str(), None, true).await;
    let forbidden = document(s, "SK25", "https://www.example.sk/direktor", None, false).await;

    // nothing without the open account
    assert_eq!(
        app.private_save(law).await.unwrap_err().code,
        "private-locked"
    );
    assert_eq!(
        app.private_library().await.unwrap_err().code,
        "private-locked"
    );
    app.private_create("Eva".into(), "moje dlhé heslo".into())
        .await
        .unwrap();

    let saved = app.private_save(law).await.unwrap();
    assert_eq!(saved.status, "saved");
    let reference = saved.item.unwrap();
    assert_eq!(
        (reference.kind.as_str(), reference.file_bytes),
        ("reference", None)
    );
    let saved = app.private_save(method).await.unwrap();
    assert_eq!(saved.status, "saved", "{saved:?}");
    let copy = saved.item.unwrap();
    assert_eq!(copy.kind, "copy");
    assert_eq!(copy.media_type.as_deref(), Some("application/pdf"));
    assert!(copy.file_bytes.unwrap() > 100);
    let html = app.private_save(page).await.unwrap().item.unwrap();
    assert_eq!((html.kind.as_str(), html.file_bytes), ("copy", None));
    assert_eq!(app.private_save(method).await.unwrap().status, "already");

    let reason = |id| {
        let app = &app;
        async move { app.private_save(id).await.unwrap().reason }
    };
    assert_eq!(reason(script).await, Some("dangerous"));
    assert_eq!(reason(secret).await, Some("robots"));
    assert_eq!(reason(no_copy).await, Some("noarchive"));
    assert_eq!(reason(marked).await, Some("noarchive"));
    assert_eq!(reason(forbidden).await, Some("not-allowed"));
    assert_eq!(reason(999_999).await, Some("gone"));
    // the refused copies were not even fetched where the rules said so before
    assert!(server.requests_to("/oznacene.html").is_empty());

    // read offline: the copy from the library, the reference from the public part
    let library = app.private_library().await.unwrap();
    assert_eq!(library.len(), 3);
    let read = app.private_read(copy.id.clone()).await.unwrap();
    assert!(read.text.unwrap().contains("Metodika hodnotenia"));
    let read = app.private_read(reference.id.clone()).await.unwrap();
    assert_eq!(read.text.as_deref(), Some("Zákon o výchove a vzdelávaní."));
    assert!(
        app.private_read(html.id.clone())
            .await
            .unwrap()
            .text
            .unwrap()
            .contains("finančnej gramotnosti")
    );

    // nothing readable on disk; the opened file goes away with the lock
    let mut found = false;
    for entry in std::fs::read_dir(folder.join("private").join("library")).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        found |= String::from_utf8_lossy(&bytes).contains("Metodika");
    }
    assert!(!found);
    let opened = app.private_open_file(copy.id.clone()).await.unwrap();
    assert!(std::fs::read(&opened).unwrap().starts_with(b"%PDF"));
    assert_eq!(
        app.private_open_file(html.id.clone())
            .await
            .unwrap_err()
            .code,
        "not-found"
    );
    app.private_lock().await.unwrap();
    assert!(!opened.exists());
    assert_eq!(
        app.private_read(copy.id.clone()).await.unwrap_err().code,
        "private-locked"
    );

    app.private_unlock("Eva".into(), "moje dlhé heslo".into())
        .await
        .unwrap();
    assert_eq!(app.private_remove(vec![copy.id, html.id]).await.unwrap(), 2);
    assert_eq!(app.private_library().await.unwrap().len(), 1);
    app.close().await;
    std::fs::remove_dir_all(&folder).unwrap();
    db.drop().await;
}
