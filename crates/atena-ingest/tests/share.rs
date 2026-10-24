//! Sharing proposals without any third party: an installation keeps the addresses its user
//! imported and writes them into a list file; the editor's installation reads the file, assesses
//! every address itself and stores them as proposals; nothing is crawled before a person confirms.
//! The only server contacted is the proposed source itself. Against a local web server and a real
//! PostgreSQL (`ATENA_TEST_DATABASE_URL`).

use std::sync::Arc;

use atena_assess::{Assessor, parse_input};
use atena_crawl::Fetcher;
use atena_crawl::testing::{Server, reply, test_config};
use atena_ingest::import::{decision, to_import};
use atena_ingest::share::{HEADER, list_id, read_list, write_list};
use atena_store::testing::TestDb;
use atena_store::{ImportOutcome, ImportState, Via};

fn school() -> String {
    "<html><head><title>ZŠ Príkladná</title></head><body><main><p>Oznamy školy o zápise a krúžkoch.</p></main>\
     <footer>© 2026 ZŠ Príkladná. Všetky práva vyhradené. <a href=\"/podmienky\">Podmienky používania</a></footer>\
     </body></html>"
        .into()
}

async fn school_site() -> Server {
    Server::start(move |req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/robots.txt") => reply(404, ""),
        ("GET", "/skola/") => reply(200, school()),
        ("GET", "/podmienky") => reply(
            200,
            "<html><body><main><p>Pri citovaní uveďte zdroj.</p></main></body></html>",
        ),
        _ => reply(404, "not found"),
    })
    .await
}

#[tokio::test]
async fn lists_travel_as_files_and_the_editor_assesses_them_again() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let server = school_site().await;
    let config = test_config(&server);
    let store = &db.store;

    // an installation keeps what its user imported: addresses and countries only …
    let school_line = format!("{} SK", server.url("/skola/"));
    store
        .outbox_add(&write_list([school_line.as_str()]))
        .await
        .unwrap();
    // … and `share FILE` writes it into a list file
    let pending = store.outbox_pending().await.unwrap();
    let lines: Vec<String> = pending
        .iter()
        .filter_map(|m| read_list(&m.body))
        .flatten()
        .collect();
    let file = write_list(lines.iter().map(String::as_str));
    assert_eq!(file, format!("{HEADER}\n{school_line}\n"));
    store.outbox_sent(pending[0].id).await.unwrap();
    assert!(store.outbox_pending().await.unwrap().is_empty());

    // the editor reads the file and assesses every address itself
    let received = read_list(&file).unwrap();
    let id = list_id(&received);
    assert!(
        store
            .inbox_known(std::slice::from_ref(&id))
            .await
            .unwrap()
            .is_empty()
    );
    let parsed = parse_input(&received.join("\n"));
    let assessor = Assessor::new(Arc::new(Fetcher::new(config).unwrap()));
    let assessments = assessor.assess_all(&parsed.items, 2).await;
    let (source, auto) = to_import(&assessments[0], Via::Shared).unwrap();
    assert_eq!(auto.terms, "allowed");
    assert_eq!(
        store.import_source(&source, &auto).await.unwrap(),
        ImportOutcome::New
    );
    assert!(store.inbox_add(&id, &received.join("\n"), 1).await.unwrap());
    assert_eq!(
        store.inbox_known(&[id]).await.unwrap().len(),
        1,
        "read once"
    );

    // no server but the proposed source itself was contacted
    for path in server.paths() {
        assert!(
            [
                "/robots.txt",
                "/.well-known/tdmrep.json",
                "/skola/",
                "/podmienky"
            ]
            .contains(&path.as_str()),
            "{path}"
        );
    }

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
