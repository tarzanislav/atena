//! `atena-ingest`: database migrations, register synchronisation, the crawler worker and a
//! one-off fetch for checking a single URL.

use std::process::ExitCode;
use std::sync::Arc;

use atena_core::Taxonomy;
use atena_core::source::{BUNDLED_SOURCES, load_sources};
use atena_crawl::{Config, Fetched, Fetcher, Validators};
use atena_ingest::{Pipeline, PipelineConfig, WorkerConfig, register_entries, run};
use atena_store::Store;
use tokio::sync::watch;
use tracing::Level;
use url::Url;

const USAGE: &str = "\
atena-ingest – ATENA crawler and ingestion

USAGE:
    atena-ingest migrate              apply database migrations
    atena-ingest sync [REGISTER]      load the source register (default: the bundled data/sources.yaml)
    atena-ingest run [--once] [--concurrency N]
                                      check due endpoints; --once stops when nothing is due
    atena-ingest fetch URL            fetch and extract one URL and print a summary (no database)

ENVIRONMENT:
    ATENA_DATABASE_URL   PostgreSQL connection, e.g. postgres://atena@localhost/atena (required except for fetch)
    ATENA_FILES          directory of the file store (default: ./atena-files)
    ATENA_WORKER         name of this worker in host leases (default: host name and process ID)
    ATENA_LOG            error, warn, info (default) or debug
";

async fn connect(max_connections: u32) -> Result<Store, String> {
    let url = std::env::var("ATENA_DATABASE_URL")
        .map_err(|_| "ATENA_DATABASE_URL is not set".to_string())?;
    let files = std::env::var("ATENA_FILES").unwrap_or_else(|_| "atena-files".into());
    let store = Store::connect(&url, files, max_connections)
        .await
        .map_err(|e| e.to_string())?;
    store.migrate().await.map_err(|e| e.to_string())?;
    Ok(store)
}

async fn sync(path: Option<&str>) -> Result<(), String> {
    let yaml = match path {
        Some(p) => std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?,
        None => BUNDLED_SOURCES.to_string(),
    };
    let taxonomy = Taxonomy::bundled().map_err(|e| e.to_string())?;
    let sources = load_sources(&yaml, &taxonomy).map_err(|e| e.to_string())?;
    let store = connect(2).await?;
    let report = store
        .sync_register(&register_entries(&sources))
        .await
        .map_err(|e| e.to_string())?;
    println!(
        "{} sources ({} removed), {} endpoints added, {} disabled",
        report.sources, report.removed, report.endpoints_added, report.endpoints_disabled
    );
    Ok(())
}

async fn work(args: &[String]) -> Result<(), String> {
    let mut config = WorkerConfig::default();
    if let Ok(id) = std::env::var("ATENA_WORKER") {
        config.worker_id = id;
    }
    let mut once = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--once" => once = true,
            "--concurrency" => {
                config.concurrency = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|n| *n > 0)
                    .ok_or("--concurrency needs a positive number")?;
            }
            other => return Err(format!("unknown option {other}\n\n{USAGE}")),
        }
    }
    let store = connect(
        u32::try_from(config.concurrency)
            .unwrap_or(64)
            .saturating_add(4),
    )
    .await?;
    let fetcher = Fetcher::new(Config::default()).map_err(|e| e.to_string())?;
    let taxonomy = Taxonomy::bundled().map_err(|e| e.to_string())?;
    let pipeline = Arc::new(Pipeline::new(
        store,
        Arc::new(fetcher),
        Arc::new(taxonomy),
        PipelineConfig::default(),
    ));
    let (stop_tx, stop_rx) = watch::channel(false);
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            tracing::info!("stopping: finishing running checks");
            let _ = stop_tx.send(true);
        }
    });
    tracing::info!(worker = %config.worker_id, concurrency = config.concurrency, once, "started");
    let stats = run(pipeline, config, stop_rx, once).await;
    println!(
        "{} checks on {} hosts, {} store errors",
        stats.checks(),
        stats.hosts,
        stats.store_errors
    );
    for (outcome, count) in &stats.outcomes {
        println!("  {outcome}: {count}");
    }
    Ok(())
}

async fn fetch_one(url: &str) -> Result<(), String> {
    let url = Url::parse(url).map_err(|e| format!("{url}: {e}"))?;
    let fetcher = Fetcher::new(Config::default()).map_err(|e| e.to_string())?;
    let page = match fetcher
        .fetch(&url, &Validators::default())
        .await
        .map_err(|e| e.to_string())?
    {
        Fetched::Page(page) => page,
        Fetched::NotModified { url } => return Err(format!("{url}: not modified")),
    };
    let doc = atena_extract::extract(&page.body, page.content_type.as_deref(), &page.url)
        .map_err(|e| e.to_string())?;
    println!("URL:          {}", page.url);
    println!(
        "Status:       {} ({} bytes, {})",
        page.status,
        page.body.len(),
        page.content_type.as_deref().unwrap_or("no type")
    );
    println!("Format:       {:?}", doc.format);
    println!("Title:        {}", doc.title.as_deref().unwrap_or("–"));
    match &doc.language {
        Some(l) => println!(
            "Language:     {} ({:?}, {:.2})",
            l.code, l.source, l.confidence
        ),
        None => println!("Language:     –"),
    }
    println!(
        "TDM reserved: {:?} / {:?}",
        page.tdm.reserved, doc.tdm_reservation
    );
    println!(
        "Links:        {}   Feeds: {}",
        doc.links.len(),
        doc.feeds.len()
    );
    let preview: String = doc.text.chars().take(600).collect();
    println!(
        "\n{preview}{}",
        if doc.text.chars().count() > 600 {
            " …"
        } else {
            ""
        }
    );
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let level = match std::env::var("ATENA_LOG").as_deref() {
        Ok("error") => Level::ERROR,
        Ok("warn") => Level::WARN,
        Ok("debug") => Level::DEBUG,
        _ => Level::INFO,
    };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("migrate") => connect(2).await.map(|_| println!("database is up to date")),
        Some("sync") => sync(args.get(1).map(String::as_str)).await,
        Some("run") => work(&args[1..]).await,
        Some("fetch") => match args.get(1) {
            Some(url) => fetch_one(url).await,
            None => Err(format!("fetch needs a URL\n\n{USAGE}")),
        },
        Some("help" | "--help" | "-h") | None => {
            print!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("unknown command {other}\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
