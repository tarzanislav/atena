//! The commands the interface may call (each also listed in `build.rs` and granted in
//! `capabilities/main.json`). They only pass the call to `atena_app::App`, which checks every
//! input; errors reach the interface as a code and a detail.

use std::sync::Arc;

use atena_app::views::{
    ChoiceInput, ConfirmedView, CrawlView, DecisionOptions, DocumentView, ErasureView, HitView,
    ImportView, OverviewView, PendingView, ProposalView, ReceivedView, SearchQuery, SearchView,
    SourceView, SyncView, TallyView, TaxonomyView,
};
use atena_app::{App, AppError, Settings};
use serde::Serialize;
use tauri::State;

use crate::desktop::{Phase, Shared};

type Answer<T> = Result<T, AppError>;

async fn app(shared: &State<'_, Shared>) -> Answer<Arc<App>> {
    shared.app().await
}

/// How far the start is.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupView {
    /// `starting`, `ready`, `failed` or `closing`.
    state: &'static str,
    /// Why it failed.
    error: Option<AppError>,
    /// The settings (readable before the start finishes, for the interface's language).
    settings: Settings,
    /// Version of ATENA.
    version: &'static str,
}

/// How far the start is, and the settings.
#[tauri::command]
pub async fn startup(shared: State<'_, Shared>) -> Answer<StartupView> {
    let (state, error, settings) = match &*shared.phase.read().await {
        Phase::Starting => (
            "starting",
            None,
            Settings::load(&shared.folder.join("settings.json")),
        ),
        Phase::Ready(backend) => ("ready", None, backend.app().settings()),
        Phase::Failed(e) => (
            "failed",
            Some(e.clone()),
            Settings::load(&shared.folder.join("settings.json")),
        ),
        Phase::Closing => ("closing", None, Settings::default()),
    };
    Ok(StartupView {
        state,
        error,
        settings,
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// The overview.
#[tauri::command]
pub async fn overview(shared: State<'_, Shared>) -> Answer<OverviewView> {
    app(&shared).await?.overview().await
}

/// Every source.
#[tauri::command]
pub async fn sources(shared: State<'_, Shared>) -> Answer<Vec<SourceView>> {
    app(&shared).await?.sources().await
}

/// The taxonomy in `language`.
#[tauri::command]
pub async fn taxonomy(shared: State<'_, Shared>, language: String) -> Answer<TaxonomyView> {
    Ok(app(&shared).await?.taxonomy(&language))
}

/// A search.
#[tauri::command]
pub async fn search(shared: State<'_, Shared>, query: SearchQuery) -> Answer<SearchView> {
    app(&shared).await?.search(query).await
}

/// The overview's counts within a scope.
#[tauri::command]
pub async fn tally(shared: State<'_, Shared>, filters: Vec<String>) -> Answer<TallyView> {
    app(&shared).await?.tally(filters).await
}

/// The newest documents.
#[tauri::command]
pub async fn newest(
    shared: State<'_, Shared>,
    filters: Vec<String>,
    limit: usize,
) -> Answer<Vec<HitView>> {
    app(&shared).await?.newest(filters, limit).await
}

/// A document card.
#[tauri::command]
pub async fn document(shared: State<'_, Shared>, id: i64) -> Answer<Option<DocumentView>> {
    app(&shared).await?.document(id).await
}

/// Proposes addresses or DOIs.
#[tauri::command]
pub async fn import_addresses(
    shared: State<'_, Shared>,
    text: String,
    country: Option<String>,
) -> Answer<ImportView> {
    app(&shared).await?.import(&text, country.as_deref()).await
}

/// Imported sources in a state.
#[tauri::command]
pub async fn proposals(
    shared: State<'_, Shared>,
    state: Option<String>,
) -> Answer<Vec<ProposalView>> {
    app(&shared).await?.proposals(state.as_deref()).await
}

/// What a decision may choose.
#[tauri::command]
pub fn decision_options() -> DecisionOptions {
    DecisionOptions::new()
}

/// Confirms a proposal.
#[tauri::command]
pub async fn confirm_proposal(
    shared: State<'_, Shared>,
    id: String,
    choice: ChoiceInput,
    reason: String,
) -> Answer<ConfirmedView> {
    app(&shared)
        .await?
        .confirm(&id, choice.into(), &reason)
        .await
}

/// Rejects a proposal.
#[tauri::command]
pub async fn reject_proposal(shared: State<'_, Shared>, id: String, reason: String) -> Answer<()> {
    app(&shared).await?.reject(&id, &reason).await
}

/// The list waiting for the editor.
#[tauri::command]
pub async fn pending_share(shared: State<'_, Shared>) -> Answer<Option<PendingView>> {
    app(&shared).await?.pending_share().await
}

/// The list was handed over.
#[tauri::command]
pub async fn handed_over(shared: State<'_, Shared>, entries: Vec<i64>) -> Answer<()> {
    app(&shared).await?.handed_over(&entries).await
}

/// Editor: reads a received list.
#[tauri::command]
pub async fn receive_list(shared: State<'_, Shared>, text: String) -> Answer<ReceivedView> {
    app(&shared).await?.receive_list(&text).await
}

/// Syncs the shared register (on the user's command).
#[tauri::command]
pub async fn sync_shared_register(shared: State<'_, Shared>) -> Answer<SyncView> {
    app(&shared).await?.sync_shared_register().await
}

/// An erasure request.
#[tauri::command]
pub async fn forget(
    shared: State<'_, Shared>,
    url: String,
    prefix: bool,
    reason: String,
) -> Answer<ErasureView> {
    app(&shared).await?.forget(&url, prefix, &reason).await
}

/// Starts the crawler.
#[tauri::command]
pub async fn crawl_start(shared: State<'_, Shared>) -> Answer<CrawlView> {
    app(&shared).await?.crawl_start().await
}

/// Asks the crawler to stop.
#[tauri::command]
pub async fn crawl_stop(shared: State<'_, Shared>) -> Answer<CrawlView> {
    Ok(app(&shared).await?.crawl_stop().await)
}

/// The state of the crawler.
#[tauri::command]
pub async fn crawl_status(shared: State<'_, Shared>) -> Answer<CrawlView> {
    Ok(app(&shared).await?.crawl_status().await)
}

/// The settings.
#[tauri::command]
pub async fn get_settings(shared: State<'_, Shared>) -> Answer<Settings> {
    Ok(app(&shared).await?.settings())
}

/// Saves the settings; returns them as kept.
#[tauri::command]
pub async fn set_settings(shared: State<'_, Shared>, settings: Settings) -> Answer<Settings> {
    app(&shared).await?.set_settings(settings)
}

/// Opens the original of a document in the user's own browser – only an `http` or `https`
/// address without credentials.
#[tauri::command]
pub fn open_link(url: String) -> Answer<()> {
    let bad = || AppError::new("url", url.chars().take(200).collect::<String>());
    if !atena_app::openable(&url) {
        return Err(bad());
    }
    open::that_detached(&url).map_err(|e| {
        tracing::warn!("opening {url}: {e}");
        bad()
    })
}
