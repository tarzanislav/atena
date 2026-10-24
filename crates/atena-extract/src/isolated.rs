//! Parsing untrusted content in a separate, limited process (pillar 3).
//!
//! A small hostile file can make a parser use gigabytes of memory or run for minutes – a PDF with
//! a decompression bomb, HTML whose tree explodes. Inside the crawler that would end every check
//! in flight, and the same address would end it again after each restart. Documents are therefore
//! parsed in a child process: the same program started with [`WORKER_ARG`] and a memory limit,
//! which its allocator enforces (the program installs a capped allocator and calls [`serve`]). The
//! child is killed when the time limit passes. A child that dies or is killed leaves nothing
//! behind: the document counts as too demanding and nothing of it is kept.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use url::Url;

use crate::{Document, ExtractError, LegalSignals, extract, legal_signals};

/// First argument that starts a program as a parsing worker; the second is the memory limit.
pub const WORKER_ARG: &str = "--atena-parse-worker";
/// Memory a worker may use, in bytes – far more than any real document needs.
pub const MEMORY_LIMIT: usize = 768 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum Task {
    Extract,
    LegalSignals,
}

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    task: Task,
    content_type: Option<String>,
    url: Url,
}

#[derive(Debug, Serialize, Deserialize)]
enum Response {
    Document(Box<Result<Document, ExtractError>>),
    Signals(LegalSignals),
}

/// How documents are parsed.
#[derive(Debug, Clone)]
pub enum Parser {
    /// In this process on a blocking thread – for tests and tools. The time limit only stops
    /// waiting; neither a runaway parser nor its memory can be stopped.
    InProcess {
        /// Longest wait.
        time_limit: Duration,
    },
    /// In a child process: `program WORKER_ARG memory_limit`, killed after `time_limit`.
    Isolated {
        /// The program to start (usually the running one).
        program: PathBuf,
        /// Memory the child may use, in bytes.
        memory_limit: usize,
        /// Longest time the child may take.
        time_limit: Duration,
    },
}

fn too_demanding(why: &str) -> ExtractError {
    ExtractError::TooDemanding(why.to_string())
}

impl Parser {
    /// Parsing in a child process of `program` with the default memory limit.
    pub fn isolated(program: impl Into<PathBuf>, time_limit: Duration) -> Self {
        Self::Isolated {
            program: program.into(),
            memory_limit: MEMORY_LIMIT,
            time_limit,
        }
    }

    /// [`crate::extract`], within the limits.
    pub async fn extract(
        &self,
        body: impl AsRef<[u8]> + Send + 'static,
        content_type: Option<&str>,
        url: &Url,
    ) -> Result<Document, ExtractError> {
        match self.run(Task::Extract, body, content_type, url).await? {
            Response::Document(result) => *result,
            Response::Signals(_) => Err(too_demanding("the parser gave a wrong answer")),
        }
    }

    /// [`crate::legal_signals`], within the limits.
    pub async fn legal_signals(
        &self,
        body: impl AsRef<[u8]> + Send + 'static,
        content_type: Option<&str>,
        url: &Url,
    ) -> Result<LegalSignals, ExtractError> {
        match self
            .run(Task::LegalSignals, body, content_type, url)
            .await?
        {
            Response::Signals(signals) => Ok(signals),
            Response::Document(_) => Err(too_demanding("the parser gave a wrong answer")),
        }
    }

    async fn run(
        &self,
        task: Task,
        body: impl AsRef<[u8]> + Send + 'static,
        content_type: Option<&str>,
        url: &Url,
    ) -> Result<Response, ExtractError> {
        let request = Request {
            task,
            content_type: content_type.map(str::to_string),
            url: url.clone(),
        };
        match self {
            Self::InProcess { time_limit } => {
                let work = tokio::task::spawn_blocking(move || answer(&request, body.as_ref()));
                match tokio::time::timeout(*time_limit, work).await {
                    Ok(Ok(response)) => Ok(response),
                    Ok(Err(_)) => Err(too_demanding("the parser failed on this document")),
                    Err(_) => Err(too_demanding("parsing took longer than allowed")),
                }
            }
            Self::Isolated {
                program,
                memory_limit,
                time_limit,
            } => {
                let child = async {
                    let mut child = Command::new(program)
                        .arg(WORKER_ARG)
                        .arg(memory_limit.to_string())
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .kill_on_drop(true)
                        .spawn()
                        .map_err(|e| too_demanding(&format!("the parser could not start: {e}")))?;
                    let mut input = child
                        .stdin
                        .take()
                        .ok_or_else(|| too_demanding("the parser has no input"))?;
                    let mut header =
                        serde_json::to_vec(&request).map_err(|e| too_demanding(&e.to_string()))?;
                    header.push(b'\n');
                    // a child that dies while reading closes the pipe; its exit status tells why
                    let _ = input.write_all(&header).await;
                    let _ = input.write_all(body.as_ref()).await;
                    drop(input);
                    child
                        .wait_with_output()
                        .await
                        .map_err(|e| too_demanding(&e.to_string()))
                };
                // when the time is up, the child is dropped and with it killed
                let output = tokio::time::timeout(*time_limit, child)
                    .await
                    .map_err(|_| too_demanding("parsing took longer than allowed"))??;
                if !output.status.success() {
                    return Err(too_demanding(
                        "the parser stopped: the document needs more memory than allowed or broke it",
                    ));
                }
                serde_json::from_slice(&output.stdout)
                    .map_err(|_| too_demanding("the parser gave an unreadable answer"))
            }
        }
    }
}

fn answer(request: &Request, body: &[u8]) -> Response {
    let content_type = request.content_type.as_deref();
    match request.task {
        Task::Extract => Response::Document(Box::new(extract(body, content_type, &request.url))),
        Task::LegalSignals => Response::Signals(legal_signals(body, content_type, &request.url)),
    }
}

/// The whole worker, for a program started with [`WORKER_ARG`]: `limit` is the argument after it,
/// `set_limit` applies the limit to the program's capped allocator and says whether it could.
/// Without a limit in force nothing is parsed.
pub fn worker_main(
    limit: Option<&str>,
    set_limit: impl FnOnce(usize) -> bool,
) -> std::process::ExitCode {
    let limit = limit.and_then(|l| l.parse().ok()).unwrap_or(MEMORY_LIMIT);
    if set_limit(limit) && serve().is_ok() {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}

/// The worker: reads one request (a JSON line, then the document) from standard input and writes
/// the answer to standard output. The program sets its memory limit before calling this.
pub fn serve() -> std::io::Result<()> {
    let mut input = Vec::new();
    std::io::stdin().lock().read_to_end(&mut input)?;
    let split = input
        .iter()
        .position(|b| *b == b'\n')
        .ok_or_else(|| std::io::Error::other("no request"))?;
    let request: Request =
        serde_json::from_slice(&input[..split]).map_err(std::io::Error::other)?;
    let response = answer(&request, &input[split + 1..]);
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, &response).map_err(std::io::Error::other)?;
    out.flush()
}
