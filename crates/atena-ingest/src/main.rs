//! `atena-ingest`: database migrations, register synchronisation, the crawler worker, proposals of
//! sources, the search index and a one-off fetch for checking a single URL.

mod commands;
mod searching;

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use atena_core::Taxonomy;
use atena_core::legality::{SourceFacts, permission};
use atena_core::safety::for_terminal;
use atena_core::source::{BUNDLED_SOURCES, load_sources};
use atena_crawl::{Config, Fetched, Fetcher, Validators};
use atena_extract::Parser;
use atena_ingest::{Pipeline, PipelineConfig, WorkerConfig, register_entries, run};
use atena_store::Store;
use tokio::sync::watch;
use tracing::Level;
use url::Url;

const USAGE: &str = "\
atena-ingest – ATENA crawler and ingestion

USAGE:
    atena-ingest migrate              apply database migrations
    atena-ingest sync [REGISTER | --shared [URL]]
                                      load the source register (default: the bundled data/sources.yaml;
                                      --shared: download the shared register of the ATENA repository –
                                      only on this explicit command)
    atena-ingest run [--once] [--concurrency N]
                                      check due endpoints; --once stops when nothing is due
    atena-ingest fetch URL            fetch and extract one URL and print a summary (no database)
    atena-ingest legality [REGISTER]  what the legality principle allows for every source (no database)
    atena-ingest assess [FILE] [--json] [--concurrency N]
                                      read addresses or DOIs, one per line (FILE or standard input),
                                      and propose licence and terms for each (no database)
    atena-ingest import [FILE] [--country CC] [--no-share]
                                      propose sources: addresses or DOIs, one per line; each is assessed
                                      and stored as a proposal, nothing is crawled until confirmed
    atena-ingest imports [proposed|confirmed|rejected|merged|all]
                                      list imported sources with their proposals
    atena-ingest confirm ID... [--licence L] [--terms T] [--country CC] [--by NAME] --reason TEXT
                                      a person's decision (default: the proposal); crawled if allowed
    atena-ingest reject ID... [--by NAME] --reason TEXT
                                      reject imported sources and remove what was collected
    atena-ingest share FILE           write the proposals waiting to be shared into a list file for the
                                      editor; ATENA itself sends nothing anywhere
    atena-ingest proposals FILE...    editor: read list files from other installations and assess them
    atena-ingest publish ID --id NEW_ID --authority A1…E [--priority P0…P3] [--languages sk,en]
                         [--name TEXT] [--register PATH]
                                      editor: add a confirmed import to the register file
    atena-ingest index [--rebuild] [--batch N]
                                      bring the search index up to date with the database (run does
                                      it every minute); --rebuild: build it anew
    atena-ingest search WORDS... [--lang LL] [--label SCHEME/CONCEPT]... [--country CC]
                        [--limit N] [--page N] [--facets] [--json]
                                      search the index (no database); --lang: language of the words;
                                      --label also takes language/LL and source/ID
    atena-ingest forget URL --reason TEXT [--prefix]
                                      erase everything about URL (with --prefix: every URL below it)
                                      and never fetch it again – for erasure requests (GDPR Art. 17)

ENVIRONMENT:
    ATENA_DATABASE_URL   PostgreSQL connection, e.g. postgres://atena@localhost/atena (required except for fetch)
    ATENA_INDEX          directory of the search index (default: ./atena-index)
    ATENA_WORKER         name of this worker in host leases (default: host name and process ID)
    ATENA_LOG            error, warn, info (default) or debug
    ATENA_SHARE          0 keeps imported addresses to this installation (default: kept for sharing)
    ATENA_EDITOR         name recorded with confirmations and rejections (or --by)
    ATENA_SHARED_REGISTER  address of the shared register (default: data/sources.yaml on GitHub)
";

async fn connect(max_connections: u32) -> Result<Store, String> {
    let url = std::env::var("ATENA_DATABASE_URL")
        .map_err(|_| "ATENA_DATABASE_URL is not set".to_string())?;
    let store = Store::connect(&url, max_connections)
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
        let p = permission(SourceFacts::from(s), false);
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

async fn sync(args: &[String]) -> Result<(), String> {
    let sources = match args.first().map(String::as_str) {
        Some("--shared") => {
            let text = commands::shared_register(args.get(1).map(String::as_str)).await?;
            let taxonomy = Taxonomy::bundled().map_err(|e| e.to_string())?;
            load_sources(&text, &taxonomy).map_err(|e| format!("shared register: {e}"))?
        }
        path => register(path)?,
    };
    let store = connect(2).await?;
    let report = store
        .sync_register(&register_entries(&sources))
        .await
        .map_err(|e| e.to_string())?;
    // the legality principle applies retroactively: what a source may no longer keep is removed
    let (mut purged, mut restricted) = (0, 0);
    for source in &sources {
        let allowed = permission(SourceFacts::from(source), false);
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
        "{} sources ({} removed, {} imports taken over), {} endpoints added, {} disabled; legality: \
         {purged} documents of sources that may not be crawled removed, texts of {restricted} \
         versions reduced to metadata",
        report.sources,
        report.removed,
        report.merged,
        report.endpoints_added,
        report.endpoints_disabled
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
    // the search index follows what the checks store
    let (indexed_tx, indexed_rx) = watch::channel(false);
    let indexer = searching::keep_index_fresh(store.clone(), indexed_rx);
    let pipeline = Arc::new(Pipeline::new(
        store,
        Arc::new(fetcher),
        Arc::new(taxonomy),
        PipelineConfig {
            parser: parser(),
            ..PipelineConfig::default()
        },
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
    let _ = indexed_tx.send(true);
    if let Some(indexer) = indexer {
        let _ = indexer.await;
    }
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
    let doc = parser()
        .extract(page.body.clone(), page.content_type.as_deref(), &page.url)
        .await
        .map_err(|e| e.to_string())?;
    println!("URL:          {}", page.url);
    println!(
        "Status:       {} ({} bytes, {})",
        page.status,
        page.body.len(),
        for_terminal(page.content_type.as_deref().unwrap_or("no type"))
    );
    println!("Format:       {:?}", doc.format);
    println!(
        "Title:        {}",
        for_terminal(doc.title.as_deref().unwrap_or("–"))
    );
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
        "\n{}{}",
        for_terminal(&preview),
        if doc.text.chars().count() > 600 {
            " …"
        } else {
            ""
        }
    );
    Ok(())
}

/// Reads one item per line from `path`, or from standard input without one.
fn read_lines(path: Option<&str>) -> Result<String, String> {
    match path {
        Some(p) => std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}")),
        None => std::io::read_to_string(std::io::stdin()).map_err(|e| e.to_string()),
    }
}

/// What a proposal allows, in the words of the legality report.
fn allows(country: Option<&str>, licence: &str, terms: &str) -> (bool, bool, bool) {
    let (Some(country), licence, terms) = (
        country,
        atena_core::Licence::parse(licence),
        atena_core::Terms::parse(terms),
    ) else {
        return (false, false, false);
    };
    let p = permission(
        SourceFacts {
            country,
            licence: &licence,
            terms,
        },
        false,
    );
    (p.crawl, p.store_text, p.show_full_text)
}

async fn assess(args: &[String]) -> Result<(), String> {
    let (mut path, mut json, mut concurrency) = (None, false, 4usize);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--concurrency" => {
                concurrency = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|n| (1..=32).contains(n))
                    .ok_or("--concurrency needs a number from 1 to 32")?;
            }
            other if path.is_none() && !other.starts_with("--") => path = Some(other),
            other => return Err(format!("unexpected argument {other}\n\n{USAGE}")),
        }
    }
    let parsed = atena_assess::parse_input(&read_lines(path)?);
    for r in &parsed.rejected {
        eprintln!("line {}: {:?}: {}", r.line, r.reason, for_terminal(&r.text));
    }
    let fetcher = Arc::new(Fetcher::new(Config::default()).map_err(|e| e.to_string())?);
    let assessments = atena_assess::Assessor::new(fetcher)
        .with_parser(parser())
        .assess_all(&parsed.items, concurrency)
        .await;
    if json {
        let out = serde_json::to_string_pretty(&assessments).map_err(|e| e.to_string())?;
        println!("{out}");
        return Ok(());
    }
    let yes = |b: bool| if b { "yes" } else { "–" };
    println!(
        "{:<5} {:<8} {:<7} {:<12} {:<9} {:<5} {:<5} {:<5}  address",
        "line", "kind", "country", "licence", "terms", "crawl", "text", "show"
    );
    for a in &assessments {
        let p = &a.proposal;
        let (crawl, text, show) = allows(a.country.as_deref(), &p.licence, &p.terms);
        println!(
            "{:<5} {:<8} {:<7} {:<12} {:<9} {:<5} {:<5} {:<5}  {}",
            a.line,
            a.kind
                .map(|k| format!("{k:?}").to_lowercase())
                .unwrap_or_else(|| "–".into()),
            a.country.as_deref().unwrap_or("?"),
            p.licence,
            p.terms,
            yes(crawl),
            yes(text),
            yes(show),
            a.url
        );
        println!("      {}", for_terminal(&p.rights.finding));
    }
    println!(
        "\n{} items assessed, {} lines not accepted. Proposals are not applied: a person confirms each one.",
        assessments.len(),
        parsed.rejected.len()
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
        "{normalized}{}: {} documents and {} versions erased; the address is blocked",
        if prefix {
            " (and everything below)"
        } else {
            ""
        },
        report.documents,
        report.versions
    );
    Ok(())
}

/// Every allocation of this program is counted, so that a parsing worker can be held to its
/// memory limit (`atena_extract::Parser`): beyond it, the worker stops and nothing is kept.
#[global_allocator]
static ALLOCATOR: cap::Cap<std::alloc::System> = cap::Cap::new(std::alloc::System, usize::MAX);

/// The limited child process that parses one document (see `atena_extract::Parser`).
fn parse_worker(limit: Option<&str>) -> ExitCode {
    let limit = limit
        .and_then(|l| l.parse().ok())
        .unwrap_or(atena_extract::MEMORY_LIMIT);
    if ALLOCATOR.set_limit(limit).is_err() {
        return ExitCode::FAILURE;
    }
    match atena_extract::serve() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

/// Documents are parsed in a limited child process of this program.
pub(crate) fn parser() -> Parser {
    match std::env::current_exe() {
        Ok(program) => Parser::isolated(program, PARSE_TIME),
        Err(e) => {
            tracing::warn!("parsing in this process, without a memory limit: {e}");
            Parser::InProcess {
                time_limit: PARSE_TIME,
            }
        }
    }
}

/// Longest time for parsing one document.
const PARSE_TIME: Duration = Duration::from_secs(60);

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some(atena_extract::WORKER_ARG) {
        return parse_worker(args.get(1).map(String::as_str));
    }
    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime.block_on(command(args)),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn command(args: Vec<String>) -> ExitCode {
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
    let result = match args.first().map(String::as_str) {
        Some("migrate") => connect(2).await.map(|_| println!("database is up to date")),
        Some("sync") => sync(&args[1..]).await,
        Some("run") => work(&args[1..]).await,
        Some("fetch") => match args.get(1) {
            Some(url) => fetch_one(url).await,
            None => Err(format!("fetch needs a URL\n\n{USAGE}")),
        },
        Some("forget") => forget(&args[1..]).await,
        Some("legality") => legality_report(args.get(1).map(String::as_str)),
        Some("search") => searching::search(&args[1..]),
        Some("index") => match connect(4).await {
            Ok(store) => searching::index(&store, &args[1..]).await,
            Err(e) => Err(e),
        },
        Some("assess") => assess(&args[1..]).await,
        Some(
            cmd @ ("import" | "imports" | "confirm" | "reject" | "share" | "proposals" | "publish"),
        ) => match connect(4).await {
            Ok(store) => match cmd {
                "import" => commands::import(&store, &args[1..]).await,
                "imports" => commands::list(&store, &args[1..]).await,
                "confirm" => commands::confirm(&store, &args[1..]).await,
                "reject" => commands::reject(&store, &args[1..]).await,
                "share" => commands::share(&store, &args[1..]).await,
                "proposals" => commands::proposals(&store, &args[1..]).await,
                _ => commands::publish(&store, &args[1..]).await,
            },
            Err(e) => Err(e),
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
