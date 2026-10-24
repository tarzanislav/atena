//! The commands for proposals: import, review, sharing and publishing in the shared register.

use std::sync::Arc;

use atena_assess::{Assessment, Assessor, Item, MAX_ITEMS, Target, parse_input};
use atena_core::Taxonomy;
use atena_core::safety::for_terminal;
use atena_core::source::load_sources;
use atena_crawl::{Config, Fetcher, ServiceClient};
use atena_ingest::import::{decision, to_import};
use atena_ingest::share::{HEADER, list_id, read_list, write_list};
use atena_store::{ImportOutcome, ImportState, Store, Via};
use url::Url;

/// The shared register all installations read.
pub const DEFAULT_SHARED_REGISTER: &str =
    "https://raw.githubusercontent.com/tarzanislav/atena/main/data/sources.yaml";

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Imported addresses are kept for sharing unless `ATENA_SHARE` is `0`, `off`, `no` or `false`.
fn sharing_enabled() -> bool {
    !matches!(
        std::env::var("ATENA_SHARE")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "0" | "off" | "no" | "false"
    )
}

fn service() -> Result<ServiceClient, String> {
    ServiceClient::new(&Config::default()).map_err(|e| e.to_string())
}

fn assessor() -> Result<Assessor, String> {
    Ok(Assessor::new(Arc::new(
        Fetcher::new(Config::default()).map_err(|e| e.to_string())?,
    )))
}

/// The person deciding: `--by` or `ATENA_EDITOR`.
fn person(by: Option<String>) -> Result<String, String> {
    by.or_else(|| std::env::var("ATENA_EDITOR").ok())
        .filter(|b| !b.trim().is_empty())
        .ok_or_else(|| "say who decides: --by NAME or ATENA_EDITOR".to_string())
}

/// The line an item is shared as: its address or DOI and its country.
fn share_line(a: &Assessment) -> String {
    let target = match &a.target {
        Target::Url(u) => u.to_string(),
        Target::Doi(doi) => doi.clone(),
    };
    match &a.country {
        Some(c) => format!("{target} {c}"),
        None => target,
    }
}

fn short(outcome: &ImportOutcome) -> String {
    match outcome {
        ImportOutcome::New => "new".into(),
        ImportOutcome::Reassessed => "reassessed".into(),
        ImportOutcome::AlreadyDecided(state) => format!("already {}", state.as_str()),
        ImportOutcome::InRegister(id) => format!("in the register ({id})"),
        ImportOutcome::Blocked => "blocked (erasure request)".into(),
    }
}

/// Assesses and stores items; returns the lines worth sharing.
async fn assess_and_store(
    store: &Store,
    items: &[Item],
    via: Via,
    concurrency: usize,
) -> Result<Vec<String>, String> {
    let assessments = assessor()?.assess_all(items, concurrency).await;
    let mut shareable = Vec::new();
    println!(
        "{:<5} {:<13} {:<20} {:<7} {:<12} {:<9}  address",
        "line", "id", "result", "country", "proposal", "terms"
    );
    for a in &assessments {
        let Some((source, auto)) = to_import(a, via) else {
            println!(
                "{:<5} {:<13} {:<20} {:<7} {:<12} {:<9}  {}",
                a.line, "–", "page about people", "", "", "", a.url
            );
            continue;
        };
        let outcome = store
            .import_source(&source, &auto)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{:<5} {:<13} {:<20} {:<7} {:<12} {:<9}  {}",
            a.line,
            source.id,
            short(&outcome),
            a.country.as_deref().unwrap_or("?"),
            auto.licence,
            auto.terms,
            source.url
        );
        println!("      {}", for_terminal(&auto.finding));
        if matches!(outcome, ImportOutcome::New | ImportOutcome::Reassessed) {
            shareable.push(share_line(a));
        }
    }
    Ok(shareable)
}

/// `share FILE` – writes every list waiting to be shared into one list file and marks them
/// handed over. Nothing is sent anywhere: the user hands the file to the editor as they choose.
pub async fn share(store: &Store, args: &[String]) -> Result<(), String> {
    let [path] = args else {
        return Err("share needs the name of the file to write".into());
    };
    if std::path::Path::new(path).exists() {
        return Err(format!("{path} exists; choose a new file"));
    }
    let pending = store.outbox_pending().await.map_err(|e| e.to_string())?;
    let lines: Vec<String> = pending
        .iter()
        .filter_map(|m| read_list(&m.body))
        .flatten()
        .collect();
    if lines.is_empty() {
        println!("No proposals are waiting to be shared.");
        return Ok(());
    }
    let list = write_list(lines.iter().map(String::as_str));
    std::fs::write(path, &list).map_err(|e| format!("{path}: {e}"))?;
    for message in &pending {
        store
            .outbox_sent(message.id)
            .await
            .map_err(|e| e.to_string())?;
    }
    println!(
        "{} addresses written to {path} – addresses and country codes only. Hand the file to the \
         ATENA editor as you choose; ATENA itself sends nothing anywhere.",
        list.lines().count() - 1
    );
    Ok(())
}

/// `import [FILE] [--country CC] [--concurrency N] [--no-share]`
pub async fn import(store: &Store, args: &[String]) -> Result<(), String> {
    let (mut path, mut country, mut concurrency, mut share) = (None, None, 4, sharing_enabled());
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--country" => {
                country = Some(args.next().ok_or("--country needs a code")?.to_uppercase())
            }
            "--concurrency" => {
                concurrency = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|n| (1..=32).contains(n))
                    .ok_or("--concurrency needs a number from 1 to 32")?;
            }
            "--no-share" => share = false,
            other if path.is_none() && !other.starts_with("--") => path = Some(other.to_string()),
            other => return Err(format!("unexpected argument {other}")),
        }
    }
    let text = crate::read_lines(path.as_deref())?;
    let mut parsed = parse_input(&text);
    for item in &mut parsed.items {
        if item.country.is_none() {
            item.country.clone_from(&country);
        }
    }
    for r in &parsed.rejected {
        eprintln!("line {}: {:?}: {}", r.line, r.reason, for_terminal(&r.text));
    }
    let lines = assess_and_store(store, &parsed.items, Via::Local, concurrency).await?;
    println!(
        "\nNothing is crawled until a person confirms: atena-ingest confirm ID --by NAME --reason TEXT"
    );
    if share && !lines.is_empty() {
        store
            .outbox_add(&write_list(lines.iter().map(String::as_str)))
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{} addresses are kept for the shared register; `atena-ingest share FILE` writes them \
             into a file for the ATENA editor. Nothing is sent anywhere. ATENA_SHARE=0 or \
             --no-share keeps them to this installation.",
            lines.len()
        );
    }
    Ok(())
}

/// `imports [proposed|confirmed|rejected|merged|all]`
pub async fn list(store: &Store, args: &[String]) -> Result<(), String> {
    let state = match args.first().map(String::as_str).unwrap_or("proposed") {
        "proposed" => Some(ImportState::Proposed),
        "confirmed" => Some(ImportState::Confirmed),
        "rejected" => Some(ImportState::Rejected),
        "merged" => Some(ImportState::Merged),
        "all" => None,
        other => return Err(format!("unknown state {other}")),
    };
    let sources = store.imported(state).await.map_err(|e| e.to_string())?;
    println!(
        "{:<13} {:<10} {:<7} {:<7} {:<24} {:<24}  address",
        "id", "state", "via", "country", "decided", "proposed"
    );
    for s in &sources {
        let proposed = s
            .proposal
            .as_ref()
            .map(|p| format!("{} / {}", p.licence, p.terms))
            .unwrap_or_default();
        println!(
            "{:<13} {:<10} {:<7} {:<7} {:<24} {:<24}  {}",
            s.id,
            s.state.as_str(),
            if s.via == Via::Shared {
                "shared"
            } else {
                "local"
            },
            if s.country.is_empty() {
                "?"
            } else {
                &s.country
            },
            format!("{} / {}", s.licence, s.terms),
            proposed,
            s.url
        );
        if let Some(p) = &s.proposal {
            println!("              {}", for_terminal(&p.finding));
        }
    }
    println!("\n{} sources", sources.len());
    Ok(())
}

/// `confirm ID... [--licence L] [--terms T] [--country CC] [--by NAME] --reason TEXT`
pub async fn confirm(store: &Store, args: &[String]) -> Result<(), String> {
    let (mut ids, mut licence, mut terms, mut country, mut by, mut reason) =
        (Vec::new(), None, None, None, None, None);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().cloned().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--licence" => licence = Some(value("--licence")?),
            "--terms" => terms = Some(value("--terms")?),
            "--country" => country = Some(value("--country")?.to_uppercase()),
            "--by" => by = Some(value("--by")?),
            "--reason" => reason = Some(value("--reason")?),
            other if !other.starts_with("--") => ids.push(other.to_string()),
            other => return Err(format!("unexpected argument {other}")),
        }
    }
    let by = person(by)?;
    let reason = reason.ok_or("say why: --reason TEXT (what you read and when)")?;
    if ids.is_empty() {
        return Err("name the sources to confirm (see: atena-ingest imports)".into());
    }
    let all = store.imported(None).await.map_err(|e| e.to_string())?;
    for id in &ids {
        let source = all
            .iter()
            .find(|s| &s.id == id)
            .ok_or_else(|| format!("{id}: no such imported source"))?;
        let proposal = source.proposal.as_ref();
        let pick = |given: &Option<String>, proposed: Option<String>, current: &str| {
            given
                .clone()
                .or(proposed)
                .unwrap_or_else(|| current.to_string())
        };
        let country = pick(
            &country,
            proposal.and_then(|p| p.country.clone()),
            &source.country,
        );
        let licence = pick(
            &licence,
            proposal.map(|p| p.licence.clone()),
            &source.licence,
        );
        let terms = pick(&terms, proposal.map(|p| p.terms.clone()), &source.terms);
        let page = proposal.map_or(source.url.clone(), |p| p.rights_page.clone());
        let (record, crawl) = decision(&by, &country, &licence, &terms, &page, &reason)
            .map_err(|e| format!("{id}: {e}"))?;
        store
            .confirm_import(id, &record, crawl)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{id}: confirmed as {} / {} ({country}); {}",
            record.licence,
            record.terms,
            if crawl {
                "it will be crawled"
            } else {
                "it is not crawled"
            }
        );
    }
    Ok(())
}

/// `reject ID... [--by NAME] --reason TEXT`
pub async fn reject(store: &Store, args: &[String]) -> Result<(), String> {
    let (mut ids, mut by, mut reason) = (Vec::new(), None, None);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--by" => by = args.next().cloned(),
            "--reason" => reason = args.next().cloned(),
            other if !other.starts_with("--") => ids.push(other.to_string()),
            other => return Err(format!("unexpected argument {other}")),
        }
    }
    let by = person(by)?;
    let reason = reason.ok_or("say why: --reason TEXT")?;
    for id in &ids {
        if store
            .reject_import(id, &by, &reason)
            .await
            .map_err(|e| e.to_string())?
        {
            println!("{id}: rejected; everything collected from it is removed");
        } else {
            println!("{id}: no such imported source");
        }
    }
    Ok(())
}

/// `proposals FILE…` – on the editor's installation: reads list files from other installations,
/// assesses every address itself and stores them as proposals. A list read before is skipped.
pub async fn proposals(store: &Store, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("proposals needs one or more list files".into());
    }
    for path in args {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let Some(lines) = read_list(&text) else {
            eprintln!("{path}: not an ATENA list (the first line must be {HEADER})");
            continue;
        };
        let id = list_id(&lines);
        if !store
            .inbox_known(std::slice::from_ref(&id))
            .await
            .map_err(|e| e.to_string())?
            .is_empty()
        {
            println!("{path}: read before ({id})");
            continue;
        }
        println!("\n{path}: {} lines", lines.len());
        let mut accepted = 0;
        // an import takes at most MAX_ITEMS lines at a time
        for chunk in lines.chunks(MAX_ITEMS) {
            let parsed = parse_input(&chunk.join("\n"));
            for r in &parsed.rejected {
                eprintln!("{:?}: {}", r.reason, for_terminal(&r.text));
            }
            assess_and_store(store, &parsed.items, Via::Shared, 4).await?;
            accepted += parsed.items.len();
        }
        let count = i32::try_from(accepted).unwrap_or(i32::MAX);
        store
            .inbox_add(&id, &lines.join("\n"), count)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn yaml_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into())
}

/// `publish ID --id NEW_ID --authority A2 [--priority P2] [--languages sk,en] [--name TEXT]
/// [--register data/sources.yaml]` – appends a confirmed import to the register file, which is
/// then proposed to the shared register as a pull request.
pub async fn publish(store: &Store, args: &[String]) -> Result<(), String> {
    let mut source_id = None;
    let (mut new_id, mut authority, mut priority, mut languages, mut name) =
        (None, None, "P2".to_string(), None, None);
    let mut path = "data/sources.yaml".to_string();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |n: &str| args.next().cloned().ok_or(format!("{n} needs a value"));
        match arg.as_str() {
            "--id" => new_id = Some(value("--id")?),
            "--authority" => authority = Some(value("--authority")?),
            "--priority" => priority = value("--priority")?,
            "--languages" => languages = Some(value("--languages")?),
            "--name" => name = Some(value("--name")?),
            "--register" => path = value("--register")?,
            other if source_id.is_none() && !other.starts_with("--") => {
                source_id = Some(other.to_string())
            }
            other => return Err(format!("unexpected argument {other}")),
        }
    }
    let source_id = source_id.ok_or("name the imported source to publish")?;
    let new_id = new_id.ok_or("give the register ID: --id SK25")?;
    let authority = authority.ok_or("give the authority level: --authority A1…E")?;
    let source = store
        .imported(Some(ImportState::Confirmed))
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|s| s.id == source_id)
        .ok_or_else(|| format!("{source_id}: no confirmed imported source"))?;
    let decided = store
        .assessments(&source_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|a| a.by_person.is_some())
        .ok_or_else(|| format!("{source_id}: no decision of a person"))?;
    let languages: Vec<String> = languages
        .unwrap_or_else(|| default_language(&source.country).to_string())
        .split(',')
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let mut entry = format!(
        "- id: {new_id}\n  name: {}\n  url: {}\n  country: {}\n  languages:\n",
        yaml_string(name.as_deref().unwrap_or(&source.name)),
        yaml_string(&source.url),
        source.country
    );
    for l in &languages {
        entry.push_str(&format!("  - {l}\n"));
    }
    let interval: i32 = register_interval(&source.connector);
    entry.push_str(&format!(
        "  authority: {authority}\n  priority: {priority}\n  connector: {}\n  interval_s: {interval}\n  review: new\n",
        source.connector
    ));
    if source.connector == "rss" {
        entry.push_str(&format!("  feed: {}\n", yaml_string(&source.url)));
    }
    if source.licence != "copyrighted" {
        entry.push_str(&format!("  licence: {}\n", source.licence));
    }
    if source.terms != "unknown" {
        entry.push_str(&format!("  terms: {}\n", source.terms));
    }
    let checked = jiff::Timestamp::now()
        .to_zoned(jiff::tz::TimeZone::UTC)
        .date()
        .to_string();
    entry.push_str(&format!(
        "  rights:\n    checked: {checked}\n    page: {}\n    finding: {}\n",
        yaml_string(&decided.rights_page),
        yaml_string(&format!(
            "{} (decided by {})",
            decided.finding,
            decided.by_person.unwrap_or_default()
        ))
    ));
    let current = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let mut updated = current.clone();
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&entry);
    let taxonomy = Taxonomy::bundled().map_err(|e| e.to_string())?;
    load_sources(&updated, &taxonomy)
        .map_err(|e| format!("the entry would break the register: {e}"))?;
    std::fs::write(&path, updated).map_err(|e| format!("{path}: {e}"))?;
    println!("{new_id} added to {path}; propose it to the shared register with a pull request.");
    Ok(())
}

/// The check interval the register uses for a connector.
fn register_interval(connector: &str) -> i32 {
    match connector {
        "rss" => 3_600,
        "doi" => 30 * 86_400,
        _ => 86_400,
    }
}

/// The usual language of a country, for a register entry without `--languages`.
fn default_language(country: &str) -> &'static str {
    match country {
        "SK" => "sk",
        "CZ" => "cs",
        "PL" => "pl",
        "DE" | "AT" => "de",
        "HU" => "hu",
        "FI" => "fi",
        "EE" => "et",
        "NL" => "nl",
        "FR" => "fr",
        "SE" => "sv",
        "IT" => "it",
        "JP" => "ja",
        "KR" => "ko",
        "CN" => "zh",
        "RU" => "ru",
        _ => "en",
    }
}

/// The shared register as text, from `ATENA_SHARED_REGISTER` or the ATENA repository.
pub async fn shared_register(url: Option<&str>) -> Result<String, String> {
    let text = url
        .map(str::to_string)
        .unwrap_or_else(|| env_or("ATENA_SHARED_REGISTER", DEFAULT_SHARED_REGISTER));
    let url = Url::parse(&text).map_err(|e| format!("{text}: {e}"))?;
    let body = service()?.get(&url, &[]).await.map_err(|e| e.to_string())?;
    String::from_utf8(body.to_vec()).map_err(|_| format!("{url}: not UTF-8 text"))
}
