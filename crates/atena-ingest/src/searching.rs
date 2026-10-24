//! The commands for the search index: `index` and `search`, and keeping the index up to date while
//! the crawler runs.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

use atena_core::Taxonomy;
use atena_core::safety::for_terminal;
use atena_search::{DEFAULT_BATCH, Results, SearchIndex, SearchRequest, Snippet, rebuild, update};
use atena_store::Store;
use tokio::sync::watch;
use tokio::task::JoinHandle;

/// How often the running crawler brings the index up to date.
const INDEX_EVERY: Duration = Duration::from_secs(60);
/// Hits per page by default.
const DEFAULT_LIMIT: usize = 10;

/// Directory of the search index: `ATENA_INDEX` or `./atena-index`.
pub fn index_path() -> PathBuf {
    std::env::var("ATENA_INDEX")
        .ok()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| "atena-index".into())
        .into()
}

fn number<T: std::str::FromStr>(value: Option<&String>, option: &str) -> Result<T, String> {
    value
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| format!("{option} needs a positive number"))
}

/// `index [--rebuild] [--batch N]`
pub async fn index(store: &Store, args: &[String]) -> Result<(), String> {
    let mut rebuild_all = false;
    let mut batch = DEFAULT_BATCH;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--rebuild" => rebuild_all = true,
            "--batch" => batch = number::<i64>(args.next(), "--batch")?.max(1),
            other => return Err(format!("unknown option {other}")),
        }
    }
    let path = index_path();
    let (index, report) = if rebuild_all {
        rebuild(store, &path, batch).await
    } else {
        let index = SearchIndex::open(&path).map_err(|e| e.to_string())?;
        update(store, &index, batch).await.map(|r| (index, r))
    }
    .map_err(|e| e.to_string())?;
    println!(
        "indexed: {}, removed or left out: {}; searchable now: {} ({})",
        report.indexed,
        report.left_out,
        index.len(),
        path.display()
    );
    Ok(())
}

/// Brings the index up to date every minute and once more after `stop`; failures are logged and
/// the crawler goes on. Await the handle to let the last update finish.
pub fn keep_index_fresh(store: Store, mut stop: watch::Receiver<bool>) -> Option<JoinHandle<()>> {
    let path = index_path();
    let index = match SearchIndex::open(&path) {
        Ok(index) => index,
        Err(e) => {
            tracing::warn!("search index not updated while crawling: {e}");
            return None;
        }
    };
    Some(tokio::spawn(async move {
        loop {
            match update(&store, &index, DEFAULT_BATCH).await {
                Ok(r) if r.indexed + r.left_out > 0 => {
                    tracing::info!(
                        indexed = r.indexed,
                        left_out = r.left_out,
                        "search index updated"
                    );
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("updating the search index: {e}"),
            }
            tokio::select! {
                _ = tokio::time::sleep(INDEX_EVERY) => {}
                _ = stop.changed() => break,
            }
        }
        // what the last checks queued
        if let Err(e) = update(&store, &index, DEFAULT_BATCH).await {
            tracing::warn!("updating the search index: {e}");
        }
    }))
}

/// `search WORDS... [--lang LL] [--label SCHEME/CONCEPT]... [--country CC] [--limit N] [--page N]
/// [--facets] [--json]`
pub fn search(args: &[String]) -> Result<(), String> {
    let mut request = SearchRequest {
        limit: DEFAULT_LIMIT,
        ..SearchRequest::default()
    };
    let mut words = Vec::new();
    let mut page = 1usize;
    let (mut json, mut facets) = (false, false);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--lang" => request.language = Some(args.next().ok_or("--lang needs a code")?.clone()),
            "--label" => request
                .filters
                .push(args.next().ok_or("--label needs SCHEME/CONCEPT")?.clone()),
            "--country" => request.filters.push(format!(
                "country/{}",
                args.next()
                    .ok_or("--country needs a code")?
                    .to_ascii_uppercase()
            )),
            "--limit" => request.limit = number(args.next(), "--limit")?,
            "--page" => page = number::<usize>(args.next(), "--page")?.max(1),
            "--json" => json = true,
            "--facets" => facets = true,
            option if option.starts_with("--") => return Err(format!("unknown option {option}")),
            word => words.push(word.to_string()),
        }
    }
    request.text = words.join(" ");
    request.offset = (page - 1)
        .saturating_mul(request.limit.clamp(1, atena_search::MAX_LIMIT))
        .min(atena_search::MAX_OFFSET);
    let path = index_path();
    if !path.exists() {
        return Err(format!(
            "{}: no search index yet – run atena-ingest index",
            path.display()
        ));
    }
    let index = SearchIndex::open(&path).map_err(|e| e.to_string())?;
    let taxonomy = Taxonomy::bundled().map_err(|e| e.to_string())?;
    let results = index
        .search(&taxonomy, &request)
        .map_err(|e| e.to_string())?;
    if json {
        let text = serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?;
        println!("{text}");
    } else {
        print_results(&results, request.offset, facets);
    }
    Ok(())
}

fn marked(snippet: &Snippet) -> String {
    let (open, close) = if std::io::stdout().is_terminal() {
        ("\x1b[1m", "\x1b[0m")
    } else {
        ("*", "*")
    };
    let safe = |text: &str| for_terminal(text).into_owned();
    let mut out = String::new();
    let mut at = 0;
    for range in &snippet.highlighted {
        out.push_str(&safe(&snippet.text[at..range.start]));
        out.push_str(open);
        out.push_str(&safe(&snippet.text[range.clone()]));
        out.push_str(close);
        at = range.end;
    }
    out.push_str(&safe(&snippet.text[at..]));
    // the markers are added after the text is made safe, so they are the only control codes
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn print_results(results: &Results, offset: usize, facets: bool) {
    let noun = if results.total == 1 {
        "document"
    } else {
        "documents"
    };
    print!("{} {noun}", results.total);
    if !results.all_words {
        print!(" (none has all the words; these have some of them)");
    }
    println!();
    if !results.concepts.is_empty() {
        let concepts: Vec<String> = results
            .concepts
            .iter()
            .map(|c| format!("{}/{} ({:.2})", c.scheme, c.concept, c.score))
            .collect();
        println!("Concepts also searched: {}", concepts.join(", "));
    }
    for (n, hit) in results.hits.iter().enumerate() {
        let title = if hit.title.is_empty() {
            "(no title)"
        } else {
            &hit.title
        };
        println!("\n{}. {}", offset + n + 1, for_terminal(title));
        println!("   {}", hit.url);
        let mut about = vec![
            hit.source_name.clone(),
            hit.country.clone(),
            hit.authority.clone(),
        ];
        about.extend(hit.language.clone());
        about.extend(hit.published.map(|p| p.strftime("%Y-%m-%d").to_string()));
        println!("   {}", for_terminal(&about.join(" · ")));
        let labels: Vec<&String> = hit
            .labels
            .iter()
            .filter(|l| !l.starts_with("country/") && !l.starts_with("authority/"))
            .collect();
        if !labels.is_empty() {
            let labels: Vec<&str> = labels.iter().map(|l| l.as_str()).collect();
            println!("   {}", labels.join(", "));
        }
        match &hit.snippet {
            Some(s) => println!("   {}", marked(s)),
            None => println!(
                "   (licence {}: the text is not shown, only the link)",
                hit.licence
            ),
        }
    }
    if facets {
        println!();
        let mut namespace = "";
        for f in &results.facets {
            if f.namespace != namespace {
                namespace = &f.namespace;
                print!("\n{namespace}:");
            }
            print!(" {} {}", f.value, f.count);
        }
        println!();
    }
}
