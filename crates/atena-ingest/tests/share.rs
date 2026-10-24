//! Sharing proposals: an installation posts addresses to a relay, the editor's installation reads
//! them (from the relay and from issues), assesses them itself and stores them as proposals;
//! nothing is crawled before a person confirms. Against a local relay, a local "GitHub API" and a
//! real PostgreSQL (`ATENA_TEST_DATABASE_URL`).

use std::sync::{Arc, Mutex};

use atena_assess::{Assessor, parse_input};
use atena_crawl::testing::{Server, reply, test_config};
use atena_crawl::{Fetcher, ServiceClient};
use atena_ingest::import::{decision, to_import};
use atena_ingest::share::{HEADER, pack, receive, receive_issues, send};
use atena_store::testing::TestDb;
use atena_store::{ImportOutcome, ImportState, Via};
use url::Url;

fn school() -> String {
    "<html><head><title>ZŠ Príkladná</title></head><body><main><p>Oznamy školy o zápise a krúžkoch.</p></main>\
     <footer>© 2026 ZŠ Príkladná. Všetky práva vyhradené. <a href=\"/podmienky\">Podmienky používania</a></footer>\
     </body></html>"
        .into()
}

async fn world(relay: Arc<Mutex<Vec<String>>>) -> Server {
    Server::start(move |req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/robots.txt") => reply(404, ""),
        ("GET", "/skola/") => reply(200, school()),
        ("GET", "/podmienky") => reply(200, "<html><body><main><p>Pri citovaní uveďte zdroj.</p></main></body></html>"),
        // the relay: messages are kept and returned as ntfy does
        ("POST", "/atena-navrhy-test") => {
            relay.lock().unwrap().push(String::from_utf8_lossy(&req.body).into_owned());
            reply(200, "{}").header("content-type", "application/json")
        }
        ("GET", "/atena-navrhy-test/json?poll=1&since=all") => {
            let lines: String = relay
                .lock()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    serde_json::json!({"id": format!("msg{i}"), "event": "message", "message": m}).to_string() + "\n"
                })
                .collect();
            reply(200, lines).header("content-type", "application/x-ndjson")
        }
        // the GitHub API: one issue copied by the workflow, one unrelated
        ("GET", "/repos/o/r/issues?labels=navrh-zdrojov&state=open&per_page=100") => reply(
            200,
            serde_json::json!([
                {"number": 7, "title": "Návrh zdrojov msg0",
                 "body": format!("Kópia správy.\n\n```text\n{HEADER}\nhttps://x.example.sk/ SK\n```\n")},
                {"number": 8, "title": "Something else", "body": "hello"}
            ])
            .to_string(),
        )
        .header("content-type", "application/json"),
        _ => reply(404, "not found"),
    })
    .await
}

#[tokio::test]
async fn lists_travel_and_the_editor_assesses_them_again() {
    let relay_store = Arc::new(Mutex::new(Vec::new()));
    let server = world(Arc::clone(&relay_store)).await;
    let config = test_config(&server);
    let client = ServiceClient::new(&config).unwrap();
    let relay = server.url("/atena-navrhy-test");

    // an installation shares what its user imported: addresses and countries only
    let school_line = format!("{} SK", server.url("/skola/"));
    for message in pack(std::slice::from_ref(&school_line)) {
        send(&client, &relay, &message).await.unwrap();
    }
    let posted = relay_store.lock().unwrap().clone();
    assert_eq!(posted, [format!("{HEADER}\n{school_line}")]);

    // the editor reads the relay and the issues
    let from_relay = receive(&client, &relay).await.unwrap();
    assert_eq!(from_relay.len(), 1);
    assert_eq!(from_relay[0].lines, school_line);
    let api = Url::parse(&format!(
        "{}/",
        server.url("").as_str().trim_end_matches('/')
    ))
    .unwrap();
    let from_issues = receive_issues(&client, &api, "o/r").await.unwrap();
    assert_eq!(from_issues.len(), 1);
    assert_eq!(
        (from_issues[0].id.as_str(), from_issues[0].issue),
        ("msg0", Some(7))
    );

    // … and assesses every address itself
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = &db.store;
    let parsed = parse_input(&from_relay[0].lines);
    let assessor = Assessor::new(Arc::new(Fetcher::new(config).unwrap()));
    let assessments = assessor.assess_all(&parsed.items, 2).await;
    let (source, auto) = to_import(&assessments[0], Via::Shared).unwrap();
    assert_eq!(auto.terms, "allowed");
    assert_eq!(
        store.import_source(&source, &auto).await.unwrap(),
        ImportOutcome::New
    );
    assert!(
        store
            .inbox_add(&from_relay[0].id, &from_relay[0].lines, 1)
            .await
            .unwrap()
    );

    let proposed = store.imported(Some(ImportState::Proposed)).await.unwrap();
    assert_eq!(proposed.len(), 1);
    assert_eq!(proposed[0].via, Via::Shared);
    assert!(
        !proposed[0].enabled,
        "nothing is crawled before a person decides"
    );

    // the editor confirms; the legality principle decides whether it is crawled
    let (record, crawl) = decision(
        "editor",
        "SK",
        &auto.licence,
        &auto.terms,
        &auto.rights_page,
        "terms read",
    )
    .unwrap();
    assert!(crawl);
    assert!(
        store
            .confirm_import(&source.id, &record, crawl)
            .await
            .unwrap()
    );
    let confirmed = store.imported(Some(ImportState::Confirmed)).await.unwrap();
    assert!(confirmed[0].enabled);
    db.drop().await;
}
