//! `atena-ingest`: database migrations, register synchronisation, the crawler worker and a
//! one-off fetch for checking a single URL.

use std::process::ExitCode;
use std::sync::Arc;

use atena_core::Taxonomy;
use atena_core::legality::{SourceFacts, permission};
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
    atena-ingest legality [REGISTER]  what the legality principle allows for every source (no database)
    atena-ingest forget URL --reason TEXT [--prefix]
                                      erase everything about URL (with --prefix: every URL below it)
                                      and never fetch it again – for erasure requests (GDPR Art. 17)

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

fn register(path: Option<&str>) -> Result<Vec<atena_core::Source>, String> {
    let yaml = match path {
        Some(p) => std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?,
        None => BUNDLED_SOURCES.to_string(),
    };
    let taxonomy = Taxonomy::bundled().map_err(|e| e.to_string())?;
    load_sources(&yaml, &taxonomy).map_err(|e| e.to_string())
}

/// Prints what the legality principle allows for every source of the register.
fn legality_report(path: Option<&str>) -> Result<(), String> {
    println!("{}\n", atena_core::PRINCIPLE);
    println!(
        "{:<7} {:<7} {:<12} {:<9} {:<5} {:<5} {:<5}  reason",
        "source", "country", "licence", "terms", "crawl", "text", "show"
    );
    let yes = |b: bool| if b { "yes" } else { "–" };
    let (mut crawl, mut text, mut show) = (0, 0, 0);
    let sources = register(path)?;
    for s in &sources {
        let p = permission(SourceFacts::from(s), false, true);
        crawl += usize::from(p.crawl);
        text += usize::from(p.store_text);
        show += usize::from(p.show_full_text);
        println!(
            "{:<7} {:<7} {:<12} {:<9} {:<5} {:<5} {:<5}  {}",
            s.id,
            s.country,
            s.licence.as_str(),
            s.terms.as_str(),
            yes(p.crawl),
            yes(p.store_text),
            yes(p.show_full_text),
            p.reasons.join("; ")
        );
    }
    println!(
        "\n{} sources: {crawl} may be crawled, {text} may keep texts, {show} may be shown in full",
        sources.len()
    );
    Ok(())
}

async fn sync(path: Option<&str>) -> Result<(), String> {
    let sources = register(path)?;
    let store = connect(2).await?;
    let report = store
        .sync_register(&register_entries(&sources))
        .await
        .map_err(|e| e.to_string())?;
    // the legality principle applies retroactively: what a source may no longer keep is removed
    let (mut purged, mut restricted) = (0, 0);
    for source in &sources {
        let allowed = permission(SourceFacts::from(source), false, false);
        if !allowed.crawl {
            purged += store
                .purge_source(&source.id)
                .await
                .map_err(|e| e.to_string())?;
        } else if !allowed.store_text {
            restricted += store
                .restrict_source(&source.id)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    println!(
        "{} sources ({} removed), {} endpoints added, {} disabled; legality: {purged} documents of \
         sources that may not be crawled removed, texts of {restricted} versions reduced to metadata",
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
        "TDM reserved: {:?} / {:?}{}",
        page.tdm.reserved,
        doc.tdm_reservation,
        if page.tdm.refused_to_ai_crawlers {
            " – robots.txt refuses AI crawlers"
        } else {
            ""
        }
    );
    println!(
        "Links:        {}   Videos: {}   Feeds: {}",
        doc.links.len(),
        doc.videos.len(),
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

async fn forget(args: &[String]) -> Result<(), String> {
    let mut url = None;
    let mut reason = None;
    let mut prefix = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--prefix" => prefix = true,
            "--reason" => reason = Some(args.next().ok_or("--reason needs a text")?.clone()),
            other if url.is_none() && !other.starts_with("--") => url = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other}\n\n{USAGE}")),
        }
    }
    let url = url.ok_or_else(|| format!("forget needs a URL\n\n{USAGE}"))?;
    let reason = reason.ok_or("forget needs --reason (e.g. the request it fulfils)")?;
    let parsed = Url::parse(&url).map_err(|e| format!("{url}: {e}"))?;
    let normalized = atena_ingest::urls::normalize(&parsed).to_string();
    let store = connect(2).await?;
    let report = store
        .forget(&normalized, prefix, &reason)
        .await
        .map_err(|e| e.to_string())?;
    println!(
        "{normalized}{}: {} documents, {} versions and {} files erased; the address is blocked",
        if prefix {
            " (and everything below)"
        } else {
            ""
        },
        report.documents,
        report.versions,
        report.files
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
        Some("forget") => forget(&args[1..]).await,
        Some("legality") => legality_report(args.get(1).map(String::as_str)),
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
