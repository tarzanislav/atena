//! The operations of the desktop app on a test database (`ATENA_TEST_DATABASE_URL`, see
//! `atena_store::testing`); skipped without one. Nothing here goes to the network.

use std::path::PathBuf;

use atena_app::views::SearchQuery;
use atena_app::{App, Choice, Settings};
use atena_store::testing::TestDb;

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
