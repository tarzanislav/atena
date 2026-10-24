//! The fetcher against a real HTTP server on 127.0.0.1 (allowed for these tests only).

use std::io::Write;
use std::time::{Duration, Instant};

use atena_crawl::testing::{Reply, Server, reply, test_config as config};
use atena_crawl::{Blocked, Config, FetchError, Fetched, Fetcher, Page, Validators};
use url::Url;

async fn page(fetcher: &Fetcher, url: Url) -> Page {
    match fetcher.fetch(&url, &Validators::default()).await {
        Ok(Fetched::Page(page)) => page,
        other => panic!("{url}: {other:?}"),
    }
}

async fn error(fetcher: &Fetcher, url: Url) -> FetchError {
    match fetcher.fetch(&url, &Validators::default()).await {
        Err(e) => e,
        Ok(other) => panic!("{url}: expected an error, got {other:?}"),
    }
}

#[tokio::test]
async fn robots_txt_is_obeyed_before_anything_else() {
    let server = Server::start(|req| match req.path.as_str() {
        "/robots.txt" => reply(
            200,
            "User-agent: *\nDisallow: /private\n\nUser-agent: ATENA\nDisallow: /no-atena\n\
             Sitemap: /sitemap.xml\n",
        )
        .header("content-type", "text/plain"),
        _ => reply(200, "<p>ok</p>"),
    })
    .await;
    let f = Fetcher::new(config(&server)).unwrap();

    assert!(matches!(
        error(&f, server.url("/no-atena/a")).await,
        FetchError::Robots { .. }
    ));
    // the ATENA group replaces the * group, so /private is allowed for ATENA
    let p = page(&f, server.url("/private/x?y=1")).await;
    assert_eq!(&p.body[..], b"<p>ok</p>");
    assert_eq!(
        f.sitemaps(&server.url("/")).await,
        [server.url("/sitemap.xml")]
    );
    assert_eq!(
        server.requests_to("/robots.txt").len(),
        1,
        "robots.txt is cached"
    );
    assert!(server.requests_to("/no-atena/a").is_empty());
    let ua = &server.requests_to("/private/x?y=1")[0].headers;
    assert!(
        ua.iter()
            .any(|h| h.starts_with("user-agent: mozilla/5.0 (compatible; atena/"))
    );
}

#[tokio::test]
async fn unreachable_robots_txt_means_complete_disallow_and_missing_means_allow() {
    let busy = Server::start(|req| match req.path.as_str() {
        "/robots.txt" => reply(503, "").header("retry-after", "300"),
        _ => reply(200, "page"),
    })
    .await;
    let f = Fetcher::new(config(&busy)).unwrap();
    assert!(matches!(
        error(&f, busy.url("/a")).await,
        FetchError::RobotsUnreachable { .. }
    ));
    assert_eq!(busy.paths(), ["/robots.txt"]);
    // the host asked for five minutes of rest; the cached "disallow" answers meanwhile
    assert!(matches!(
        error(&f, busy.url("/b")).await,
        FetchError::RobotsUnreachable { .. }
    ));
    assert_eq!(busy.paths(), ["/robots.txt"]);

    let missing = Server::start(|req| match req.path.as_str() {
        "/robots.txt" => reply(404, "not found"),
        _ => reply(200, "page"),
    })
    .await;
    let f = Fetcher::new(config(&missing)).unwrap();
    assert_eq!(&page(&f, missing.url("/a")).await.body[..], b"page");
}

#[tokio::test]
async fn redirects_are_followed_and_every_hop_is_checked() {
    let server = Server::start(|req| {
        let to = |location: &str| reply(301, "").header("location", location.to_string());
        match req.path.as_str() {
            "/robots.txt" => reply(200, "User-agent: *\nDisallow: /secret\n"),
            "/old" => to("/new"),
            "/new" => reply(200, "new page"),
            "/to-intranet" => to("http://10.0.0.1/admin"),
            "/to-metadata" => to("http://169.254.169.254/latest/meta-data/"),
            "/to-localhost" => to("http://localhost/"),
            "/to-file" => to("file:///etc/passwd"),
            "/to-secret" => to("/secret/x"),
            "/loop" => to("/loop"),
            _ => reply(404, ""),
        }
    })
    .await;
    let f = Fetcher::new(config(&server)).unwrap();

    let p = page(&f, server.url("/old")).await;
    assert_eq!(p.url, server.url("/new"));
    for (path, expected) in [
        ("/to-intranet", "10.0.0.1"),
        ("/to-metadata", "169.254.169.254"),
    ] {
        match error(&f, server.url(path)).await {
            FetchError::Blocked {
                reason: Blocked::Address(a),
                ..
            } => assert_eq!(a, expected),
            other => panic!("{path}: {other:?}"),
        }
    }
    assert!(matches!(
        error(&f, server.url("/to-localhost")).await,
        FetchError::Blocked { .. }
    ));
    assert!(matches!(
        error(&f, server.url("/to-file")).await,
        FetchError::Blocked {
            reason: Blocked::Scheme(_),
            ..
        }
    ));
    assert!(matches!(
        error(&f, server.url("/to-secret")).await,
        FetchError::Robots { .. }
    ));
    assert!(matches!(
        error(&f, server.url("/loop")).await,
        FetchError::TooManyRedirects { .. }
    ));
    assert_eq!(
        server.requests_to("/loop").len(),
        6,
        "first request and 5 redirects"
    );
    assert!(server.requests_to("/secret/x").is_empty());
}

#[tokio::test]
async fn the_default_policy_never_contacts_local_addresses() {
    let server = Server::start(|_| reply(200, "should never be served")).await;
    let f = Fetcher::new(Config {
        min_delay: Duration::ZERO,
        ..Config::default()
    })
    .unwrap();
    for url in [
        "http://127.0.0.1/",
        "http://[::1]/",
        "http://localhost/",
        "http://0x7f000001/",
    ] {
        assert!(
            matches!(
                error(&f, Url::parse(url).unwrap()).await,
                FetchError::Blocked { .. }
            ),
            "{url}"
        );
    }
    assert!(matches!(
        error(&f, server.url("/")).await,
        FetchError::Blocked { .. }
    ));
    assert!(server.paths().is_empty());
}

#[tokio::test]
async fn conditional_requests_and_metadata() {
    let server = Server::start(|req| match req.path.as_str() {
        "/robots.txt" => reply(404, ""),
        "/doc" if req.headers.iter().any(|h| h == "if-none-match: \"v1\"") => reply(304, ""),
        "/doc" => reply(200, "version 1")
            .header("etag", "\"v1\"")
            .header("last-modified", "Sat, 24 Oct 2026 08:00:00 GMT")
            .header("content-language", "sk")
            .header("x-robots-tag", "noarchive")
            .header("x-robots-tag", "otherbot: noindex"),
        _ => reply(404, ""),
    })
    .await;
    let f = Fetcher::new(config(&server)).unwrap();
    let first = page(&f, server.url("/doc")).await;
    assert_eq!(first.validators.etag.as_deref(), Some("\"v1\""));
    assert_eq!(first.content_language.as_deref(), Some("sk"));
    assert!(first.index.noarchive && !first.index.noindex);
    match f.fetch(&server.url("/doc"), &first.validators).await {
        Ok(Fetched::NotModified { url }) => assert_eq!(url, server.url("/doc")),
        other => panic!("{other:?}"),
    }
    let second = &server.requests_to("/doc")[1].headers;
    assert!(
        second
            .iter()
            .any(|h| h == "if-modified-since: sat, 24 oct 2026 08:00:00 gmt")
    );
}

#[tokio::test]
async fn tdm_reservation_from_the_well_known_file_and_headers() {
    let server = Server::start(|req| match req.path.as_str() {
        "/robots.txt" => reply(404, ""),
        "/.well-known/tdmrep.json" => reply(
            200,
            r#"[{"location": "/reserved/*", "tdm-reservation": 1, "tdm-policy": "https://pub.example/policy"},
                {"location": "/open", "tdm-reservation": 0}]"#,
        )
        .header("content-type", "application/json"),
        "/open/override" => reply(200, "x").header("tdm-reservation", "1"),
        _ => reply(200, "x"),
    })
    .await;
    let f = Fetcher::new(config(&server)).unwrap();
    let reserved = page(&f, server.url("/reserved/a.pdf")).await.tdm;
    assert_eq!(reserved.reserved, Some(true));
    assert_eq!(
        reserved.policy.as_deref(),
        Some("https://pub.example/policy")
    );
    assert_eq!(
        page(&f, server.url("/open/a")).await.tdm.reserved,
        Some(false)
    );
    assert_eq!(
        page(&f, server.url("/open/override")).await.tdm.reserved,
        Some(true)
    );
    assert_eq!(page(&f, server.url("/elsewhere")).await.tdm.reserved, None);
    assert_eq!(server.requests_to("/.well-known/tdmrep.json").len(), 1);
}

#[tokio::test]
async fn refusing_ai_crawlers_in_robots_txt_reserves_mining() {
    let server = Server::start(|req| match req.path.as_str() {
        "/robots.txt" => reply(
            200,
            "User-agent: *\nAllow: /\n\nUser-agent: GPTBot\nUser-agent: CCBot\nDisallow: /news/\n",
        ),
        "/.well-known/tdmrep.json" => reply(200, r#"[{"location": "/*", "tdm-reservation": 0}]"#)
            .header("content-type", "application/json"),
        _ => reply(200, "x"),
    })
    .await;
    let f = Fetcher::new(config(&server)).unwrap();
    // ATENA may fetch the page, but keeps it as metadata only – TDMRep "0" cannot lift that
    let news = page(&f, server.url("/news/a")).await.tdm;
    assert!(news.refused_to_ai_crawlers && news.is_reserved());
    assert_eq!(news.reserved, Some(false));
    let other = page(&f, server.url("/about")).await.tdm;
    assert!(!other.refused_to_ai_crawlers && !other.is_reserved());
}

#[tokio::test]
async fn size_and_type_limits_also_after_decompression() {
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    gz.write_all(&vec![b'a'; 1_000_000]).unwrap();
    let bomb = gz.finish().unwrap();
    let server = Server::start(move |req| match req.path.as_str() {
        "/robots.txt" => reply(404, ""),
        "/big" => reply(200, vec![b'x'; 5_000]),
        "/stream" => Reply {
            close_delimited: true,
            ..reply(200, vec![b'y'; 5_000])
        },
        "/bomb" => reply(200, bomb.clone()).header("content-encoding", "gzip"),
        "/small-gzip" => {
            let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            gz.write_all(b"hello").unwrap();
            reply(200, gz.finish().unwrap()).header("content-encoding", "gzip")
        }
        "/video" => reply(200, "binary").header("content-type", "video/mp4"),
        _ => reply(404, ""),
    })
    .await;
    let f = Fetcher::new(Config {
        max_body_bytes: 1_000,
        ..config(&server)
    })
    .unwrap();
    for path in ["/big", "/stream", "/bomb"] {
        assert!(
            matches!(
                error(&f, server.url(path)).await,
                FetchError::TooLarge { limit: 1_000, .. }
            ),
            "{path}"
        );
    }
    assert_eq!(
        &page(&f, server.url("/small-gzip")).await.body[..],
        b"hello"
    );
    assert!(matches!(
        error(&f, server.url("/video")).await,
        FetchError::UnsupportedType { .. }
    ));
    assert!(matches!(
        error(&f, server.url("/missing")).await,
        FetchError::Status { status: 404, .. }
    ));
}

#[tokio::test]
async fn hosts_are_paced_and_retry_after_is_respected() {
    let server = Server::start(|req| match req.path.as_str() {
        "/robots.txt" => reply(200, "User-agent: *\nCrawl-delay: 0.3\n"),
        "/busy" => reply(429, "").header("retry-after", "120"),
        _ => reply(200, "ok"),
    })
    .await;
    let f = Fetcher::new(config(&server)).unwrap();
    for i in 0..3 {
        page(&f, server.url(&format!("/p{i}"))).await;
    }
    let times: Vec<Instant> = (0..3)
        .map(|i| server.requests_to(&format!("/p{i}"))[0].at)
        .collect();
    for pair in times.windows(2) {
        assert!(
            pair[1] - pair[0] >= Duration::from_millis(280),
            "{:?}",
            pair[1] - pair[0]
        );
    }

    match error(&f, server.url("/busy")).await {
        FetchError::RetryLater { after, .. } => assert_eq!(after, Duration::from_secs(120)),
        other => panic!("{other:?}"),
    }
    let before = server.paths().len();
    match error(&f, server.url("/p0")).await {
        FetchError::RetryLater { after, .. } => assert!(after > Duration::from_secs(110)),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        server.paths().len(),
        before,
        "nothing is sent while backing off"
    );
}
