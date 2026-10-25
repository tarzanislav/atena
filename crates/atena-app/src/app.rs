//! The operations of the desktop app: what the user interface may ask for, each one checked
//! here, whatever the interface sent.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use atena_assess::{Assessor, parse_input};
use atena_core::legality::{SourceFacts, permission, profile};
use atena_core::safety::clean_text;
use atena_core::source::{BUNDLED_SOURCES, load_sources};
use atena_core::taxonomy::BUNDLED_TAXONOMY;
use atena_core::{Licence, Taxonomy, Terms};
use atena_crawl::{Config, Fetcher};
use atena_ingest::PipelineConfig;
use atena_ingest::classifying::reclassify;
use atena_ingest::import::{Choice, Proposed, confirm, propose};
use atena_ingest::indexing::keep_fresh;
use atena_ingest::register::{fetch_shared, sync};
use atena_ingest::share::{self, Received, write_list};
use atena_ingest::urls::normalize;
use atena_search::{DEFAULT_BATCH, SearchError, SearchIndex, SearchRequest, rebuild};
use atena_store::{ImportState, Store, Via, hash, hex};
use tokio::sync::{Mutex, Notify, watch};
use tokio::task::JoinHandle;
use url::Url;

use crate::AppError;
use crate::crawl::Crawl;
use crate::settings::Settings;
use crate::views::{
    ConfirmedView, CrawlView, DocumentView, ErasureView, HitView, ImportView, OverviewView,
    PendingView, PrivateCreatedView, PrivateStatusView, ProposalView, ProposedView, ReceivedView,
    SearchQuery, SearchView, SourceView, SyncView, TallyView, TaxonomyView, ToolView, VersionView,
};
use atena_vault::{DataKey, Keychain, Vault};
use jiff::Timestamp;

mod archive;
mod library;
pub use library::PrivateFetch;

/// "New" in the overview: first seen within this many days.
pub const NEW_DAYS: u32 = 7;
/// Hits per page of a search.
pub const PER_PAGE: usize = 20;
/// Largest text the user may paste for an import or a list.
pub const MAX_INPUT_BYTES: usize = 1024 * 1024;
/// Longest reason of a decision.
pub const MAX_REASON_CHARS: usize = 2_000;
/// Items assessed at the same time during an import.
const IMPORT_CONCURRENCY: usize = 4;
/// Longest wait for the crawler to finish its checks when the app closes.
const CLOSE_WAIT: Duration = Duration::from_secs(30);

/// The application: the database, the search index and the crawler of one installation.
pub struct App {
    store: Store,
    /// The private account of this installation (`private/` in the app's folder).
    vault: Vault,
    /// The open private account: its name and data key, until it is locked or ATENA closes.
    private: std::sync::Mutex<Option<(String, DataKey)>>,
    /// The register last synced: the bundled one, or the shared one when the user fetched it.
    register: std::sync::RwLock<Vec<atena_core::Source>>,
    index: Arc<SearchIndex>,
    taxonomy: Arc<Taxonomy>,
    folder: PathBuf,
    settings: std::sync::Mutex<Settings>,
    crawl: Crawl,
    nudge: Arc<Notify>,
    indexer: Mutex<Option<(watch::Sender<bool>, JoinHandle<()>)>>,
    /// How copies for the user's own use are fetched and read (made when first needed).
    private_fetch: std::sync::Mutex<Option<PrivateFetch>>,
    /// One change of the private library at a time.
    library_lock: Mutex<()>,
}

fn internal(e: impl std::fmt::Display) -> AppError {
    AppError::new("internal", e.to_string())
}

/// A reason given by the person: required, cleaned and of limited length.
fn reason(text: &str) -> Result<String, AppError> {
    let text = clean_text(text).trim().to_string();
    if text.is_empty() {
        return Err(AppError::new("reason-missing", "a reason is required"));
    }
    if text.chars().count() > MAX_REASON_CHARS {
        return Err(AppError::new(
            "input-too-long",
            format!("a reason has at most {MAX_REASON_CHARS} characters"),
        ));
    }
    Ok(text)
}

fn input(text: &str) -> Result<&str, AppError> {
    if text.len() > MAX_INPUT_BYTES {
        return Err(AppError::new(
            "input-too-long",
            format!("at most {MAX_INPUT_BYTES} bytes at a time"),
        ));
    }
    Ok(text)
}

/// A country code the legality principle knows, in upper case.
fn country(code: &str) -> Result<String, AppError> {
    let code = code.trim().to_ascii_uppercase();
    if profile(&code).is_none() {
        return Err(AppError::new(
            "decision-country",
            format!("country {code} has no legal profile"),
        ));
    }
    Ok(code)
}

/// Syncs the register that came with the app when it differs from the one synced last time
/// (after an update of the app); the legality principle applies retroactively.
async fn sync_bundled_register(
    store: &Store,
    taxonomy: &Taxonomy,
    folder: &Path,
) -> Result<Vec<atena_core::Source>, AppError> {
    let sources = load_sources(BUNDLED_SOURCES, taxonomy).map_err(internal)?;
    let marker = folder.join("register.synced");
    let fingerprint = hex(&hash(BUNDLED_SOURCES.as_bytes()));
    if std::fs::read_to_string(&marker).is_ok_and(|m| m.trim() == fingerprint) {
        return Ok(sources);
    }
    let summary = sync(store, &sources).await?;
    tracing::info!(
        sources = summary.report.sources,
        purged = summary.purged,
        restricted = summary.restricted,
        "the register of this version of ATENA is synced"
    );
    std::fs::write(&marker, fingerprint).map_err(|e| AppError::io(&marker, e))?;
    Ok(sources)
}

/// Classifies the stored documents again when the taxonomy of this version of ATENA differs from
/// the one they were classified with – in the background, so the window opens at once. The marker
/// is written only after a complete run, so an interrupted one is repeated at the next start.
fn reclassify_when_changed(
    store: Store,
    taxonomy: Arc<Taxonomy>,
    folder: &Path,
    nudge: Arc<Notify>,
) {
    let marker = folder.join("taxonomy.classified");
    let fingerprint = hex(&hash(BUNDLED_TAXONOMY.as_bytes()));
    if std::fs::read_to_string(&marker).is_ok_and(|m| m.trim() == fingerprint) {
        return;
    }
    tokio::spawn(async move {
        let threshold = PipelineConfig::default().classify_threshold;
        match reclassify(&store, &taxonomy, threshold).await {
            Ok(report) => {
                tracing::info!(
                    documents = report.documents,
                    changed = report.changed,
                    "documents classified with the taxonomy of this version of ATENA"
                );
                nudge.notify_one();
                if let Err(e) = std::fs::write(&marker, fingerprint) {
                    tracing::warn!("{}: {e}", marker.display());
                }
            }
            Err(e) => tracing::warn!(error = %e, "classifying the documents again failed"),
        }
    });
}

/// The search index in `path`; built anew when it was made by another version of ATENA.
async fn open_index(store: &Store, path: &Path) -> Result<SearchIndex, AppError> {
    match SearchIndex::open(path) {
        Ok(index) => Ok(index),
        Err(SearchError::Outdated(_)) => {
            tracing::info!("the search index is rebuilt for this version of ATENA");
            let (index, _) = rebuild(store, path, DEFAULT_BATCH).await?;
            Ok(index)
        }
        Err(e) => Err(e.into()),
    }
}

/// The account opened by a persistent sign-in, if it is on and still fits.
fn remembered(vault: &Vault) -> Option<(String, DataKey)> {
    match vault.remembered() {
        Ok(Some(key)) => {
            let name = vault.account().ok().flatten()?.name;
            tracing::info!("the private part was opened by the persistent sign-in");
            Some((name, key))
        }
        Ok(None) => None,
        Err(e) => {
            tracing::warn!(error = %e, "the persistent sign-in could not open the private part");
            None
        }
    }
}

impl App {
    /// Opens the application on `store` with its files in `folder`: brings the database up to
    /// date, syncs the bundled register when it changed and keeps the search index fresh. A
    /// persistent sign-in (Windows Credential Manager) opens the private part at once.
    pub async fn open(store: Store, folder: &Path) -> Result<Self, AppError> {
        Self::open_with_keychain(store, folder, Keychain::system()).await
    }

    /// [`App::open`] with another credential store for the persistent sign-in (`None`: none).
    pub async fn open_with_keychain(
        store: Store,
        folder: &Path,
        keychain: Option<Keychain>,
    ) -> Result<Self, AppError> {
        std::fs::create_dir_all(folder).map_err(|e| AppError::io(folder, e))?;
        store.migrate().await?;
        // only this installation uses the database: leases of an app that crashed are void
        store.release_all_hosts().await?;
        let taxonomy = Arc::new(Taxonomy::bundled().map_err(internal)?);
        let register = sync_bundled_register(&store, &taxonomy, folder).await?;
        let index = Arc::new(open_index(&store, &folder.join("index")).await?);
        let nudge = Arc::new(Notify::new());
        reclassify_when_changed(
            store.clone(),
            Arc::clone(&taxonomy),
            folder,
            Arc::clone(&nudge),
        );
        let vault = Vault::new(folder.join("private")).with_keychain(keychain);
        // files of the private part and of the archive opened last time are not left behind
        library::wipe_opened(folder);
        archive::wipe_opened(folder);
        // the register may have changed the rights of archived documents
        if let Err(e) = archive::prune(&store).await {
            tracing::warn!(error = %e, "the archive could not be checked");
        }
        let private = remembered(&vault);
        let (stop, stop_rx) = watch::channel(false);
        let indexer = keep_fresh(
            store.clone(),
            Arc::clone(&index),
            Arc::clone(&nudge),
            stop_rx,
        );
        Ok(Self {
            settings: std::sync::Mutex::new(Settings::load(&folder.join("settings.json"))),
            register: std::sync::RwLock::new(register),
            vault,
            private: std::sync::Mutex::new(private),
            store,
            index,
            taxonomy,
            folder: folder.to_path_buf(),
            crawl: Crawl::default(),
            nudge,
            indexer: Mutex::new(Some((stop, indexer))),
            private_fetch: std::sync::Mutex::new(None),
            library_lock: Mutex::new(()),
        })
    }

    fn assessor(&self) -> Result<Assessor, AppError> {
        let fetcher =
            Fetcher::new(Config::default()).map_err(|e| AppError::new("crawl", e.to_string()))?;
        Ok(Assessor::new(Arc::new(fetcher)).with_parser(atena_ingest::parser()))
    }

    /// The settings.
    pub fn settings(&self) -> Settings {
        self.settings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Checks, saves and applies new settings; returns them as kept.
    pub fn set_settings(&self, settings: Settings) -> Result<Settings, AppError> {
        let settings = settings.checked();
        settings.save(&self.folder.join("settings.json"))?;
        *self
            .settings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = settings.clone();
        Ok(settings)
    }

    /// The person deciding, from the settings.
    fn editor(&self) -> Result<String, AppError> {
        let editor = self.settings().editor;
        if editor.is_empty() {
            return Err(AppError::new(
                "editor-missing",
                "a decision needs a name: set it in the settings",
            ));
        }
        Ok(editor)
    }

    /// The overview.
    pub async fn overview(&self) -> Result<OverviewView, AppError> {
        let o = self.store.overview().await?;
        Ok(OverviewView {
            sources: o.sources,
            sources_crawled: o.sources_crawled,
            proposals: o.proposals,
            documents: o.documents,
            versions: o.versions,
            searchable: self.index.len(),
            endpoints: o.endpoints,
            endpoints_due: o.endpoints_due,
            last_check: o.last_check,
            checks_last_day: o.checks_last_day,
            waiting_to_share: o.waiting_to_share,
            crawling: self.crawl.is_running().await,
            archived_files: o.archived_files,
            archived_bytes: o.archived_bytes,
            gone: o.gone,
        })
    }

    /// Every source with what the legality principle allows for it.
    pub async fn sources(&self) -> Result<Vec<SourceView>, AppError> {
        Ok(self
            .store
            .source_summaries()
            .await?
            .into_iter()
            .map(SourceView::from)
            .collect())
    }

    fn private_session(&self) -> std::sync::MutexGuard<'_, Option<(String, DataKey)>> {
        self.private
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Runs a key derivation (deliberately slow) off the async threads.
    async fn vault<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Vault) -> Result<T, atena_vault::VaultError> + Send + 'static,
    ) -> Result<T, AppError> {
        let vault = self.vault.clone();
        Ok(tokio::task::spawn_blocking(move || work(&vault))
            .await
            .map_err(internal)??)
    }

    /// The private account: whether there is one and whether it is open.
    pub async fn private_status(&self) -> Result<PrivateStatusView, AppError> {
        let account = self.vault.account()?;
        let unlocked = self.private_session().is_some();
        Ok(PrivateStatusView {
            exists: account.is_some(),
            name: account.map(|a| a.name),
            unlocked,
            remembered: self.vault.is_remembered(),
            remember_available: self.vault.can_remember(),
        })
    }

    fn open_private(&self, name: String, key: DataKey) {
        *self.private_session() = Some((name, key));
    }

    /// Makes the private account of this installation and opens it; the recovery key is returned
    /// once to be shown to the user.
    pub async fn private_create(
        &self,
        name: String,
        password: String,
    ) -> Result<PrivateCreatedView, AppError> {
        let (key, recovery) = self.vault(move |v| v.create(&name, &password)).await?;
        let name = self.vault.account()?.map(|a| a.name).unwrap_or_default();
        self.open_private(name, key);
        Ok(PrivateCreatedView {
            recovery_key: recovery.as_str().to_string(),
            status: self.private_status().await?,
        })
    }

    /// Opens the private account.
    pub async fn private_unlock(
        &self,
        name: String,
        password: String,
    ) -> Result<PrivateStatusView, AppError> {
        let key = self.vault(move |v| v.unlock(&name, &password)).await?;
        let name = self.vault.account()?.map(|a| a.name).unwrap_or_default();
        self.open_private(name, key);
        self.private_status().await
    }

    /// Closes the private account; its key is wiped from memory, opened files are deleted.
    pub async fn private_lock(&self) -> Result<PrivateStatusView, AppError> {
        self.private_session().take();
        library::wipe_opened(&self.folder);
        self.private_status().await
    }

    /// Sets a new password with the recovery key and opens the account.
    pub async fn private_recover(
        &self,
        recovery_key: String,
        new_password: String,
    ) -> Result<PrivateStatusView, AppError> {
        let key = self
            .vault(move |v| v.recover(&recovery_key, &new_password))
            .await?;
        let name = self.vault.account()?.map(|a| a.name).unwrap_or_default();
        self.open_private(name, key);
        self.private_status().await
    }

    /// Turns the persistent sign-in on for the open account: the user ticked that they do it at
    /// their own risk (whoever signs in to their Windows account opens the private part).
    pub async fn private_remember(&self) -> Result<PrivateStatusView, AppError> {
        {
            let session = self.private_session();
            let (_, key) = session.as_ref().ok_or_else(|| {
                AppError::new("private-locked", "the private account is not open")
            })?;
            self.vault.remember(key)?;
        }
        self.private_status().await
    }

    /// Turns the persistent sign-in off; the account stays as it is (open or locked).
    pub async fn private_forget(&self) -> Result<PrivateStatusView, AppError> {
        self.vault.forget()?;
        self.private_status().await
    }

    /// Changes the password of the open account.
    pub async fn private_change_password(&self, old: String, new: String) -> Result<(), AppError> {
        let name = self
            .private_session()
            .as_ref()
            .map(|(name, _)| name.clone())
            .ok_or_else(|| AppError::new("private-locked", "the private account is not open"))?;
        self.vault(move |v| v.change_password(&name, &old, &new))
            .await
    }

    /// The tools of the register (dictionaries, corpora, applications – linked, never crawled),
    /// described in `language`.
    pub fn tools(&self, language: &str) -> Vec<ToolView> {
        self.register
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|s| s.is_tool())
            .map(|s| ToolView::new(s, language))
            .collect()
    }

    /// The taxonomy with labels in `language`.
    pub fn taxonomy(&self, language: &str) -> TaxonomyView {
        TaxonomyView::new(&self.taxonomy, language)
    }

    /// Searches the index.
    pub async fn search(&self, query: SearchQuery) -> Result<SearchView, AppError> {
        let page = query.page.max(1);
        let language = query
            .language
            .filter(|l| self.taxonomy.languages.iter().any(|known| known == l));
        let request = SearchRequest {
            text: query.text,
            language,
            filters: query.filters,
            limit: PER_PAGE,
            offset: (page - 1)
                .saturating_mul(PER_PAGE)
                .min(atena_search::MAX_OFFSET),
        };
        let (index, taxonomy) = (Arc::clone(&self.index), Arc::clone(&self.taxonomy));
        let results = tokio::task::spawn_blocking(move || index.search(&taxonomy, &request))
            .await
            .map_err(internal)??;
        Ok(SearchView::new(results, page, PER_PAGE))
    }

    /// The overview's counts within the scope `filters` (`region/sk`, or `region/abroad` with
    /// `language/xx`): documents per section, all and those first seen in the last
    /// [`NEW_DAYS`] days, and the sizes of the regions and languages.
    pub async fn tally(&self, filters: Vec<String>) -> Result<TallyView, AppError> {
        let since = Timestamp::now()
            .checked_sub(jiff::SignedDuration::from_hours(24 * i64::from(NEW_DAYS)))
            .map_err(internal)?;
        let (index, taxonomy) = (Arc::clone(&self.index), Arc::clone(&self.taxonomy));
        let tally =
            tokio::task::spawn_blocking(move || index.tally(&taxonomy, "section", &filters, since))
                .await
                .map_err(internal)??;
        Ok(TallyView::new(tally, "section", NEW_DAYS))
    }

    /// The newest documents that match `filters` (`section/changes`, `country/SK` …; all without
    /// filters), at most `limit` (up to two pages).
    pub async fn newest(
        &self,
        filters: Vec<String>,
        limit: usize,
    ) -> Result<Vec<HitView>, AppError> {
        let limit = limit.clamp(1, 2 * PER_PAGE);
        let (index, taxonomy) = (Arc::clone(&self.index), Arc::clone(&self.taxonomy));
        let hits = tokio::task::spawn_blocking(move || index.newest(&taxonomy, &filters, limit))
            .await
            .map_err(internal)??;
        Ok(hits.into_iter().map(HitView::from).collect())
    }

    /// The card of document `id`: what search may show about it, its versions, and its text where
    /// the legality principle allows showing it in full. `None` for a document search may not
    /// show (removed, `noindex`, of a source that may not be crawled).
    pub async fn document(&self, id: i64) -> Result<Option<DocumentView>, AppError> {
        let Some(row) = self.store.search_rows(&[id]).await?.into_iter().next() else {
            return Ok(None);
        };
        let licence = Licence::parse(&row.licence);
        let allowed = permission(
            SourceFacts {
                country: &row.country,
                licence: &licence,
                terms: Terms::parse(&row.terms),
            },
            row.tdm_reserved,
        );
        if !row.source_enabled || row.noindex || !allowed.crawl {
            return Ok(None);
        }
        // what ATENA may show in full stays in its archive when the publisher removes it
        let archive = allowed.show_full_text && !row.noarchive;
        if row.status != "active" && !(archive && row.body.is_some()) {
            return Ok(None);
        }
        let text = archive.then_some(row.body).flatten();
        let archived_file = if archive {
            self.store.archived_file(id).await?.map(Into::into)
        } else {
            None
        };
        let versions = self
            .store
            .versions(id)
            .await?
            .into_iter()
            .map(VersionView::from)
            .collect();
        Ok(Some(DocumentView {
            id,
            title: row.title.unwrap_or_default(),
            url: row.url,
            source_id: row.source_id,
            source_name: row.source_name,
            country: row.country,
            trust: atena_core::scope::trust_of(&row.authority).into(),
            authority: row.authority,
            licence: row.licence,
            language: row.language,
            published: row.published_at,
            last_changed: row.last_changed_at,
            labels: row
                .labels
                .into_iter()
                .map(|(scheme, concept)| format!("{scheme}/{concept}"))
                .collect(),
            text,
            gone_at: (row.status == "gone").then_some(row.gone_at).flatten(),
            archived_file,
            versions,
        }))
    }

    /// Assesses the addresses or DOIs in `text` (one per line) and stores them as proposals;
    /// `country` applies to lines without one. Nothing is crawled until a person confirms.
    pub async fn import(
        &self,
        text: &str,
        country_code: Option<&str>,
    ) -> Result<ImportView, AppError> {
        let mut parsed = parse_input(input(text)?);
        if let Some(code) = country_code.filter(|c| !c.trim().is_empty()) {
            let code = country(code)?;
            for item in parsed.items.iter_mut().filter(|i| i.country.is_none()) {
                item.country = Some(code.clone());
            }
        }
        let proposed = propose(
            &self.store,
            &self.assessor()?,
            &parsed.items,
            Via::Local,
            IMPORT_CONCURRENCY,
        )
        .await?;
        let lines: Vec<String> = proposed.iter().filter_map(Proposed::share_line).collect();
        let kept_for_sharing = if self.settings().share_proposals && !lines.is_empty() {
            self.store
                .outbox_add(&write_list(lines.iter().map(String::as_str)))
                .await?;
            lines.len()
        } else {
            0
        };
        Ok(ImportView {
            proposed: proposed.iter().map(ProposedView::from).collect(),
            rejected: parsed.rejected,
            kept_for_sharing,
        })
    }

    /// Imported sources in `state` (`proposed`, `confirmed`, `rejected`, `merged`), or all.
    pub async fn proposals(&self, state: Option<&str>) -> Result<Vec<ProposalView>, AppError> {
        let state = match state {
            None | Some("all") => None,
            Some("proposed") => Some(ImportState::Proposed),
            Some("confirmed") => Some(ImportState::Confirmed),
            Some("rejected") => Some(ImportState::Rejected),
            Some("merged") => Some(ImportState::Merged),
            Some(other) => return Err(AppError::new("not-found", format!("state {other}"))),
        };
        Ok(self
            .store
            .imported(state)
            .await?
            .into_iter()
            .map(ProposalView::from)
            .collect())
    }

    /// The person's decision to confirm the imported source `id`: as proposed unless `choice`
    /// says otherwise. Crawled only as far as the legality principle allows.
    pub async fn confirm(
        &self,
        id: &str,
        mut choice: Choice,
        why: &str,
    ) -> Result<ConfirmedView, AppError> {
        let editor = self.editor()?;
        let why = reason(why)?;
        choice.country = choice.country.as_deref().map(country).transpose()?;
        let done = confirm(&self.store, id, &choice, &editor, &why).await?;
        Ok(ConfirmedView {
            licence: done.licence,
            terms: done.terms,
            country: done.country,
            crawl: done.crawl,
        })
    }

    /// The person's decision to reject the imported source `id`; what was collected from it is
    /// removed.
    pub async fn reject(&self, id: &str, why: &str) -> Result<(), AppError> {
        let editor = self.editor()?;
        let why = reason(why)?;
        if !self.store.reject_import(id, &editor, &why).await? {
            return Err(AppError::new("not-found", id));
        }
        self.nudge.notify_one();
        Ok(())
    }

    /// The list of proposals waiting to be handed to the editor, if any.
    pub async fn pending_share(&self) -> Result<Option<PendingView>, AppError> {
        Ok(share::pending(&self.store).await?.map(|p| PendingView {
            list: p.list,
            addresses: p.addresses,
            entries: p.entries,
        }))
    }

    /// The user handed the list over: these entries no longer wait.
    pub async fn handed_over(&self, entries: &[i64]) -> Result<(), AppError> {
        Ok(share::handed_over(&self.store, entries).await?)
    }

    /// Editor: reads a list from another installation and assesses every address here.
    pub async fn receive_list(&self, text: &str) -> Result<ReceivedView, AppError> {
        let received = share::receive(
            &self.store,
            &self.assessor()?,
            input(text)?,
            IMPORT_CONCURRENCY,
        )
        .await?;
        Ok(match received {
            Received::NotAList => ReceivedView {
                status: "not-a-list",
                list: None,
                proposed: Vec::new(),
                rejected: Vec::new(),
            },
            Received::ReadBefore(id) => ReceivedView {
                status: "read-before",
                list: Some(id),
                proposed: Vec::new(),
                rejected: Vec::new(),
            },
            Received::Read {
                id,
                proposed,
                rejected,
            } => ReceivedView {
                status: "read",
                list: Some(id),
                proposed: proposed.iter().map(ProposedView::from).collect(),
                rejected,
            },
        })
    }

    /// Downloads the shared register of the ATENA repository and syncs it – only on the user's
    /// explicit command; the legality principle applies retroactively.
    pub async fn sync_shared_register(&self) -> Result<SyncView, AppError> {
        let sources = fetch_shared(None, &self.taxonomy).await?;
        let summary = sync(&self.store, &sources).await?;
        *self
            .register
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = sources;
        self.nudge.notify_one();
        Ok(summary.into())
    }

    /// Erases everything about `url` (with `prefix`: every address below it) and never fetches
    /// it again – for erasure requests (GDPR Art. 17).
    pub async fn forget(
        &self,
        url: &str,
        prefix: bool,
        why: &str,
    ) -> Result<ErasureView, AppError> {
        let why = reason(why)?;
        let parsed = Url::parse(url.trim())
            .ok()
            .filter(|u| matches!(u.scheme(), "http" | "https") && u.host().is_some())
            .ok_or_else(|| {
                AppError::new("url", url.trim().chars().take(200).collect::<String>())
            })?;
        let report = self
            .store
            .forget(normalize(&parsed).as_str(), prefix, &why)
            .await?;
        self.nudge.notify_one();
        Ok(ErasureView {
            documents: report.documents,
            versions: report.versions,
        })
    }

    /// Starts the crawler (on the user's command only).
    pub async fn crawl_start(&self) -> Result<CrawlView, AppError> {
        let settings = self.settings();
        self.crawl
            .start(
                &self.store,
                &self.taxonomy,
                settings.concurrency,
                settings.keep_originals,
                Arc::clone(&self.nudge),
            )
            .await?;
        Ok(self.crawl.status().await)
    }

    /// Asks the crawler to stop; the checks under way finish first.
    pub async fn crawl_stop(&self) -> CrawlView {
        self.crawl.request_stop().await;
        self.crawl.status().await
    }

    /// The state of the crawler.
    pub async fn crawl_status(&self) -> CrawlView {
        self.crawl.status().await
    }

    /// Ends everything cleanly: the crawler finishes its checks (at most [`CLOSE_WAIT`]), the
    /// index takes what they stored, the database connections close.
    pub async fn close(&self) {
        // the private key leaves memory first, opened private files the disk
        self.private_session().take();
        library::wipe_opened(&self.folder);
        archive::wipe_opened(&self.folder);
        if tokio::time::timeout(CLOSE_WAIT, self.crawl.stop())
            .await
            .is_err()
        {
            tracing::warn!("the crawler did not finish in time");
        }
        if let Some((stop, indexer)) = self.indexer.lock().await.take() {
            let _ = stop.send(true);
            let _ = indexer.await;
        }
        self.store.pool().close().await;
    }

    /// The database (for tests and tools).
    pub fn store(&self) -> &Store {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_are_checked() {
        assert_eq!(reason("  read on 24 Oct  ").unwrap(), "read on 24 Oct");
        assert_eq!(reason(" \u{200b} ").unwrap_err().code, "reason-missing");
        assert_eq!(
            reason(&"a".repeat(MAX_REASON_CHARS + 1)).unwrap_err().code,
            "input-too-long"
        );
        assert_eq!(country("sk").unwrap(), "SK");
        assert_eq!(country("XX").unwrap_err().code, "decision-country");
        assert_eq!(
            input(&"x".repeat(MAX_INPUT_BYTES + 1)).unwrap_err().code,
            "input-too-long"
        );
    }
}
