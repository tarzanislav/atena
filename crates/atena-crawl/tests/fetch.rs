//! The fetcher against a real HTTP server on 127.0.0.1 (allowed for these tests only).

use std::io::Write;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use atena_crawl::{
    Blocked, Config, FetchError, Fetched, Fetcher, IpRange, NetPolicy, Page, Validators,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

/// A received request: path with query, lower-cased header lines, arrival time.
#[derive(Debug, Clone)]
struct Seen {
    path: String,
    headers: Vec<String>,
    at: Instant,
}

/// A response: status, extra headers, body; `close_delimited` omits `Content-Length`.
struct Reply {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: Vec<u8>,
    close_delimited: bool,
}

fn reply(status: u16, body: impl Into<Vec<u8>>) -> Reply {
    Reply {
        status,
        headers: vec![("content-type", "text/html; charset=utf-8".into())],
        body: body.into(),
        close_delimited: false,
    }
}

impl Reply {
    /// Adds a header; a content type replaces the default one, other headers may repeat.
    fn header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        if name == "content-type" {
            self.headers.retain(|(n, _)| *n != name);
        }
        self.headers.push((name, value.into()));
        self
    }
}

type Handler = dyn Fn(&Seen) -> Reply + Send + Sync;

struct Server {
    addr: SocketAddr,
    log: Arc<Mutex<Vec<Seen>>>,
}

impl Server {
    async fn start(handler: impl Fn(&Seen) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let log = Arc::new(Mutex::new(Vec::new()));
        let handler: Arc<Handler> = Arc::new(handler);
        let server_log = Arc::clone(&log);
        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                let (handler, log) = (Arc::clone(&handler), Arc::clone(&server_log));
                tokio::spawn(async move {
                    let mut raw = Vec::new();
                    let mut buf = [0u8; 4096];
                    while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
                        match stream.read(&mut buf).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => raw.extend_from_slice(&buf[..n]),
                        }
                    }
                    let text = String::from_utf8_lossy(&raw).to_string();
                    let mut lines = text.split("\r\n");
                    let path = lines
                        .next()
                        .and_then(|l| l.split(' ').nth(1))
                        .unwrap_or("/")
                        .to_string();
                    let headers = lines
                        .take_while(|l| !l.is_empty())
                        .map(str::to_ascii_lowercase)
                        .collect();
                    let seen = Seen {
                        path,
                        headers,
                        at: Instant::now(),
                    };
                    log.lock().unwrap().push(seen.clone());
                    let r = handler(&seen);
                    let mut out = format!("HTTP/1.1 {} X\r\nconnection: close\r\n", r.status);
                    if !r.close_delimited {
                        out.push_str(&format!("content-length: {}\r\n", r.body.len()));
                    }
                    for (name, value) in &r.headers {
                        out.push_str(&format!("{name}: {value}\r\n"));
                    }
                    out.push_str("\r\n");
                    let _ = stream.write_all(out.as_bytes()).await;
                    let _ = stream.write_all(&r.body).await;
                    let _ = stream.shutdown().await;
                });
            }
        });
        Self { addr, log }
    }

    fn url(&self, path: &str) -> Url {
        Url::parse(&format!("http://{}{path}", self.addr)).unwrap()
    }

    fn paths(&self) -> Vec<String> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .map(|s| s.path.clone())
            .collect()
    }

    fn requests_to(&self, path: &str) -> Vec<Seen> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.path == path)
            .cloned()
            .collect()
    }
}

fn config(server: &Server) -> Config {
    Config {
        net: NetPolicy {
            ports: vec![server.addr.port(), 80, 443],
            allowed_ranges: vec![IpRange::new("127.0.0.0".parse().unwrap(), 8).unwrap()],
        },
        min_delay: Duration::ZERO,
        timeout: Duration::from_secs(10),
        ..Config::default()
    }
}

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
        FetchError::Robots { .. }
    ));
    assert_eq!(busy.paths(), ["/robots.txt"]);
    // the host asked for five minutes of rest; the cached "disallow" answers meanwhile
    assert!(matches!(
        error(&f, busy.url("/b")).await,
        FetchError::Robots { .. }
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
