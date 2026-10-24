//! Test support: a local HTTP server and fetcher settings that allow it. Only built with the
//! `testing` feature, which test code of this and other crates enables.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

use crate::{Config, IpRange, NetPolicy};

/// A received request: method, path with query, lower-cased header lines, body, arrival time.
#[derive(Debug, Clone)]
pub struct Seen {
    /// Method (`GET`, `POST` …).
    pub method: String,
    /// Path with query.
    pub path: String,
    /// Header lines, lower-cased.
    pub headers: Vec<String>,
    /// Body (read when `content-length` announces one).
    pub body: Vec<u8>,
    /// Arrival time.
    pub at: Instant,
}

/// A response: status, extra headers, body; `close_delimited` omits `Content-Length`.
pub struct Reply {
    /// Status code.
    pub status: u16,
    /// Headers.
    pub headers: Vec<(&'static str, String)>,
    /// Body.
    pub body: Vec<u8>,
    /// No `Content-Length`: the body ends when the connection closes.
    pub close_delimited: bool,
}

/// An HTML response with the given status and body.
pub fn reply(status: u16, body: impl Into<Vec<u8>>) -> Reply {
    Reply {
        status,
        headers: vec![("content-type", "text/html; charset=utf-8".into())],
        body: body.into(),
        close_delimited: false,
    }
}

impl Reply {
    /// Adds a header; a content type replaces the default one, other headers may repeat.
    pub fn header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        if name == "content-type" {
            self.headers.retain(|(n, _)| *n != name);
        }
        self.headers.push((name, value.into()));
        self
    }
}

type Handler = dyn Fn(&Seen) -> Reply + Send + Sync;

/// A minimal HTTP/1.1 server on 127.0.0.1 that answers every request with a handler and logs it.
pub struct Server {
    addr: SocketAddr,
    log: Arc<Mutex<Vec<Seen>>>,
}

impl Server {
    /// Starts the server on a free port.
    pub async fn start(handler: impl Fn(&Seen) -> Reply + Send + Sync + 'static) -> Self {
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
                    let end = raw
                        .windows(4)
                        .position(|w| w == b"\r\n\r\n")
                        .unwrap_or(raw.len())
                        + 4;
                    let text = String::from_utf8_lossy(&raw[..end.min(raw.len())]).to_string();
                    let mut lines = text.split("\r\n");
                    let request_line = lines.next().unwrap_or_default().to_string();
                    let mut parts = request_line.split(' ');
                    let method = parts.next().unwrap_or("GET").to_string();
                    let path = parts.next().unwrap_or("/").to_string();
                    let headers: Vec<String> = lines
                        .take_while(|l| !l.is_empty())
                        .map(str::to_ascii_lowercase)
                        .collect();
                    let length = headers
                        .iter()
                        .find_map(|h| h.strip_prefix("content-length:"))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0)
                        .min(1 << 20);
                    let mut body = raw.get(end..).unwrap_or_default().to_vec();
                    while body.len() < length {
                        match stream.read(&mut buf).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => body.extend_from_slice(&buf[..n]),
                        }
                    }
                    let seen = Seen {
                        method,
                        path,
                        headers,
                        body,
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

    /// The port the server listens on.
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// A URL on this server.
    pub fn url(&self, path: &str) -> Url {
        Url::parse(&format!("http://{}{path}", self.addr)).unwrap()
    }

    /// Paths of all requests so far, in order.
    pub fn paths(&self) -> Vec<String> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .map(|s| s.path.clone())
            .collect()
    }

    /// Requests to one path (with query).
    pub fn requests_to(&self, path: &str) -> Vec<Seen> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.path == path)
            .cloned()
            .collect()
    }
}

/// Fetcher settings for tests: loopback and the server's port allowed, no delay between requests.
pub fn test_config(server: &Server) -> Config {
    Config {
        net: NetPolicy {
            ports: vec![server.port(), 80, 443],
            allowed_ranges: vec![IpRange::new("127.0.0.0".parse().unwrap(), 8).unwrap()],
        },
        min_delay: Duration::ZERO,
        timeout: Duration::from_secs(10),
        ..Config::default()
    }
}
