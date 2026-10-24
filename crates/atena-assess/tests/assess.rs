//! The assessment against a local web server: what is read, what is proposed and why.

use std::sync::Arc;

use atena_assess::{Assessment, Assessor, FindingKind, Kind, Problem, Reason, parse_input};
use atena_crawl::Fetcher;
use atena_crawl::testing::{Server, reply, test_config};

const FEED: &str = r#"<?xml version="1.0"?><rss version="2.0"><channel><title>Oznamy školy</title>
    <item><title>Zápis</title><link>/a.html</link></item></channel></rss>"#;

fn page(body: &str, footer: &str) -> String {
    format!(
        "<html><head><title>Škola</title></head><body><main><h1>Škola</h1><p>{body}</p></main>\
         <footer>{footer}</footer></body></html>"
    )
}

async fn site() -> Server {
    Server::start(|req| match req.path.as_str() {
        "/robots.txt" => reply(
            200,
            "User-agent: *\nDisallow: /tajne/\n\nUser-agent: GPTBot\nDisallow: /ai/\n",
        ),
        // a school: copyright notice and terms that say nothing against ATENA
        "/skola/" => reply(
            200,
            page(
                "Oznamy o zápise do prvého ročníka a o školskom klube detí.",
                "<p>© 2026 ZŠ Príkladná. Všetky práva vyhradené.</p>\
                 <a href=\"/podmienky-pouzivania\">Podmienky používania</a>\
                 <a href=\"/ochrana-osobnych-udajov\">Ochrana osobných údajov</a>",
            ),
        ),
        "/podmienky-pouzivania" => reply(
            200,
            page(
                "Obsah webu je chránený autorským zákonom. Pri citovaní uveďte zdroj.",
                "",
            ),
        ),
        // a portal whose terms ban robots
        "/portal/" => reply(
            200,
            page("Metodické materiály pre učiteľov.", "<a href=\"/portal/terms\">Terms of use</a>"),
        ),
        "/portal/terms" => reply(
            200,
            page(
                "You may not use any robot, spider, scraper or other automated means to access \
                 the site for any purpose without our express written permission.",
                "",
            ),
        ),
        // a "document" that is a program
        "/program.pdf" => reply(200, &b"MZ\x90\x00\x03\x00\x00\x00%PDF-1.7"[..])
            .header("content-type", "application/pdf"),
        // openly licensed, no terms page
        "/oer/" => reply(
            200,
            "<html><head><link rel=\"license\" href=\"https://creativecommons.org/licenses/by/4.0/\">\
             </head><body><main><p>Otvorené vzdelávacie zdroje pre základné školy.</p></main>\
             <footer>Obsah je dostupný pod licenciou CC BY 4.0.</footer></body></html>",
        ),
        // pages refused to AI crawlers
        "/ai/clanok" => reply(200, page("Článok o umelej inteligencii vo vyučovaní.", "")),
        // a feed: the site's home page speaks for it
        "/rss.xml" => reply(200, FEED).header("content-type", "application/rss+xml"),
        "/" => reply(
            200,
            page("Vitajte.", "<a href=\"/podmienky-pouzivania\">Podmienky</a>"),
        ),
        // terms that cannot be read
        "/zlyhanie/" => reply(200, page("Text.", "<a href=\"/zlyhanie/podmienky\">Podmienky</a>")),
        "/zlyhanie/podmienky" => reply(500, "error"),
        "/zamestnanci/" => reply(200, page("Zoznam zamestnancov.", "")),
        _ => reply(404, "not found"),
    })
    .await
}

async fn assess(server: &Server, path: &str, country: &str) -> Assessment {
    let fetcher = Arc::new(Fetcher::new(test_config(server)).unwrap());
    let line = format!("{} {country}", server.url(path));
    let parsed = parse_input(&line);
    assert!(parsed.rejected.is_empty(), "{:?}", parsed.rejected);
    Assessor::new(fetcher).assess(&parsed.items[0]).await
}

#[tokio::test]
async fn a_school_with_a_copyright_notice_may_be_crawled_and_kept() {
    let server = site().await;
    let a = assess(&server, "/skola/", "SK").await;
    assert_eq!(a.kind, Some(Kind::Page));
    assert_eq!(a.country.as_deref(), Some("SK"));
    assert_eq!(a.proposal.terms, "allowed");
    assert_eq!(a.proposal.licence, "copyrighted");
    assert!(a.proposal.reasons.contains(&Reason::TermsRead));
    assert!(a.proposal.reasons.contains(&Reason::RightsReserved));
    // the privacy policy is not read as terms
    assert_eq!(a.legal_pages.len(), 1, "{:?}", a.legal_pages);
    assert!(a.legal_pages[0].path().ends_with("/podmienky-pouzivania"));
    assert!(a.proposal.rights.page.ends_with("/podmienky-pouzivania"));
    assert!(
        a.proposal.rights.finding.contains("Všetky práva vyhradené"),
        "{}",
        a.proposal.rights.finding
    );
    assert_eq!(a.checked.len(), 10);
}

#[tokio::test]
async fn terms_that_ban_robots_forbid_crawling() {
    let server = site().await;
    let a = assess(&server, "/portal/", "SK").await;
    assert_eq!(a.proposal.terms, "forbidden");
    assert_eq!(a.proposal.reasons, [Reason::AutomationBan]);
    let ban = a
        .findings
        .iter()
        .find(|f| f.kind == FindingKind::AutomationBan)
        .unwrap();
    assert!(ban.quote.starts_with("You may not use any robot"));
    assert!(
        a.proposal
            .rights
            .finding
            .contains("You may not use any robot")
    );
}

#[tokio::test]
async fn an_open_licence_is_proposed_and_noted() {
    let server = site().await;
    let a = assess(&server, "/oer/", "SK").await;
    assert_eq!(a.proposal.licence, "CC-BY-4.0");
    assert_eq!(a.proposal.terms, "allowed");
    assert!(a.proposal.reasons.contains(&Reason::OpenLicence));
}

#[tokio::test]
async fn refusing_ai_crawlers_reserves_mining() {
    let server = site().await;
    let a = assess(&server, "/ai/clanok", "SK").await;
    assert!(a.refused_to_ai_crawlers);
    assert_eq!(a.proposal.terms, "reserved");
    assert!(a.proposal.reasons.contains(&Reason::AiCrawlersRefused));
}

#[tokio::test]
async fn a_feed_is_judged_by_its_site() {
    let server = site().await;
    let a = assess(&server, "/rss.xml", "SK").await;
    assert_eq!(a.kind, Some(Kind::Feed));
    assert_eq!(a.title.as_deref(), Some("Oznamy školy"));
    assert_eq!(a.proposal.terms, "allowed");
    assert!(a.legal_pages[0].path().ends_with("/podmienky-pouzivania"));
}

#[tokio::test]
async fn an_address_serving_a_program_is_forbidden() {
    let server = site().await;
    let a = assess(&server, "/program.pdf", "SK").await;
    assert_eq!(a.problem, Some(Problem::Dangerous));
    assert_eq!(a.proposal.terms, "forbidden");
    assert!(a.proposal.reasons.contains(&Reason::DangerousContent));
}

#[tokio::test]
async fn what_cannot_be_read_stays_unknown() {
    let server = site().await;
    let robots = assess(&server, "/tajne/x", "SK").await;
    assert_eq!(robots.problem, Some(Problem::Robots));
    assert_eq!(robots.proposal.terms, "unknown");
    let failing = assess(&server, "/zlyhanie/", "SK").await;
    assert_eq!(failing.proposal.terms, "unknown");
    assert_eq!(failing.proposal.reasons, [Reason::TermsUnreadable]);
}

#[tokio::test]
async fn pages_about_people_and_unknown_countries() {
    let server = site().await;
    let people = assess(&server, "/zamestnanci/", "SK").await;
    assert_eq!(people.proposal.terms, "forbidden");
    assert_eq!(people.proposal.reasons, [Reason::PersonalPage]);
    // 127.0.0.1 tells no country: a person must name it
    let nowhere = assess(&server, "/skola/", "").await;
    assert_eq!(nowhere.country, None);
    assert!(nowhere.proposal.reasons.contains(&Reason::UnknownCountry));
}
