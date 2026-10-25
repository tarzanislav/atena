//! The private library of the open account: saving documents for the user, reading them offline,
//! opening kept files and removing them.
//!
//! What may be saved, and how (the author's decisions of 25 Oct 2026):
//! * only documents of the public part whose source ATENA may visit at all (a legal profile, terms
//!   that do not forbid automated access, an enabled source);
//! * a document the public part keeps and shows in full anyway (official texts, open licences) is
//!   saved as a **reference** – never a second copy;
//! * anything else is a **copy for the user's own use** (in Slovakia § 42 of the Copyright Act
//!   185/2015): fetched from the publisher on the user's click, as the crawler fetches (robots.txt
//!   and the pace of the server respected), refused when the publisher asks for no stored copies
//!   (`noarchive`), read by the isolated reader – which refuses dangerous files – and encrypted.
//!   The file is kept for PDF and office documents, the text for all.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use atena_core::legality::{SourceFacts, permission};
use atena_core::safety::clean_text;
use atena_core::{Licence, Terms};
use atena_crawl::{Config, FetchError, Fetched, Fetcher, IndexRules, Validators};
use atena_extract::{ExtractError, Format, Parser};
use atena_store::{hash, hex};
use atena_vault::{Added, DataKey, ItemFacts, Kind, Library};
use url::Url;

use super::{App, internal};
use crate::AppError;
use crate::views::{PrivateItemView, PrivateReadView, PrivateSavedView};

/// Folder of the private files the user opened (decrypted while they are open).
const OPENED: &str = "private-open";
/// Most documents removed at once.
const MAX_REMOVE: usize = 1_000;

/// How copies for the user's own use are fetched and read.
#[derive(Debug, Clone)]
pub struct PrivateFetch {
    fetcher: Arc<Fetcher>,
    parser: Parser,
}

impl PrivateFetch {
    /// Fetching with `config` and reading with `parser` (tests and tools use a local server).
    pub fn new(config: Config, parser: Parser) -> Result<Self, AppError> {
        let fetcher = Fetcher::new(config).map_err(|e| AppError::new("crawl", e.to_string()))?;
        Ok(Self {
            fetcher: Arc::new(fetcher),
            parser,
        })
    }
}

/// Deletes the private files decrypted for viewing; a file still open in a viewer stays until the
/// next time (Windows does not delete open files).
pub(super) fn wipe_opened(folder: &Path) {
    let opened = folder.join(OPENED);
    if opened.exists()
        && let Err(e) = std::fs::remove_dir_all(&opened)
    {
        tracing::warn!("{}: {e}", opened.display());
    }
}

fn refused(reason: &'static str) -> PrivateSavedView {
    PrivateSavedView {
        status: "refused",
        reason: Some(reason),
        item: None,
    }
}

fn not_found() -> AppError {
    AppError::new("not-found", "no such item in the private library")
}

/// A file name for an opened copy: letters, digits, spaces and dashes of the title, then the start
/// of the item's identifier and the extension of its type.
fn file_name(title: &str, id: &str, media_type: Option<&str>) -> String {
    let stem: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(60)
        .collect();
    let stem = stem.trim();
    let extension = match media_type {
        Some("application/pdf") => "pdf",
        Some(t) if t == Format::Docx.media_type() => "docx",
        Some(t) if t == Format::Odt.media_type() => "odt",
        _ => "bin",
    };
    let short = id.get(..8).unwrap_or(id);
    if stem.is_empty() {
        format!("atena-{short}.{extension}")
    } else {
        format!("{stem} ({short}).{extension}")
    }
}

impl App {
    /// The data key of the open account (a copy, wiped when dropped).
    fn private_key(&self) -> Result<DataKey, AppError> {
        self.private_session()
            .as_ref()
            .map(|(_, key)| key.clone())
            .ok_or_else(|| AppError::new("private-locked", "the private account is not open"))
    }

    fn library(&self) -> Library {
        Library::new(self.folder.join("private").join("library"))
    }

    /// Runs work on the library off the async threads.
    async fn with_library<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Library, &DataKey) -> Result<T, atena_vault::VaultError> + Send + 'static,
    ) -> Result<T, AppError> {
        let key = self.private_key()?;
        let library = self.library();
        Ok(tokio::task::spawn_blocking(move || work(&library, &key))
            .await
            .map_err(internal)??)
    }

    /// Uses another way of fetching and reading copies (tests and tools).
    pub fn set_private_fetch(&self, fetch: PrivateFetch) {
        *self
            .private_fetch
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(fetch);
    }

    fn private_fetch(&self) -> Result<PrivateFetch, AppError> {
        let mut slot = self
            .private_fetch
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(fetch) = slot.as_ref() {
            return Ok(fetch.clone());
        }
        let fetch = PrivateFetch::new(Config::default(), atena_ingest::parser())?;
        *slot = Some(fetch.clone());
        Ok(fetch)
    }

    /// The documents the user saved, newest first.
    pub async fn private_library(&self) -> Result<Vec<PrivateItemView>, AppError> {
        let items = self.with_library(|l, k| l.items(k)).await?;
        Ok(items.into_iter().map(PrivateItemView::from).collect())
    }

    /// Saves a document of the public part for the user: a reference or a copy for their own use.
    /// A refusal is an answer, not an error, so a list can be saved one by one.
    pub async fn private_save(&self, document_id: i64) -> Result<PrivateSavedView, AppError> {
        // the account must be open before anything is fetched
        self.private_key()?;
        let Some(row) = self
            .store
            .search_rows(&[document_id])
            .await?
            .into_iter()
            .next()
        else {
            return Ok(refused("gone"));
        };
        let url = row.url.clone();
        if let Some(item) = self
            .with_library(move |l, k| l.find(k, &url, Some(document_id)))
            .await?
        {
            return Ok(PrivateSavedView {
                status: "already",
                reason: None,
                item: Some(item.into()),
            });
        }
        let licence = Licence::parse(&row.licence);
        let terms = Terms::parse(&row.terms);
        let allowed = permission(
            SourceFacts {
                country: &row.country,
                licence: &licence,
                terms,
            },
            row.tdm_reserved,
        );
        if !row.source_enabled || !allowed.crawl || terms == Terms::Forbidden {
            return Ok(refused("not-allowed"));
        }
        // a removed document can still be referred to where ATENA's archive keeps it
        let archived = allowed.show_full_text && !row.noarchive && row.body.is_some();
        if row.status != "active" && !archived {
            return Ok(refused("gone"));
        }
        let mut facts = ItemFacts {
            document_id: Some(document_id),
            url: row.url.clone(),
            title: row.title.clone().unwrap_or_default(),
            source_id: row.source_id.clone(),
            source_name: row.source_name.clone(),
            licence: row.licence.clone(),
            authority: row.authority.clone(),
            language: row.language.clone(),
            labels: row
                .labels
                .iter()
                .map(|(scheme, concept)| format!("{scheme}/{concept}"))
                .collect(),
            fingerprint: None,
            media_type: None,
        };

        // the public part keeps and shows it: a reference, never a second copy
        if allowed.show_full_text
            && !row.noarchive
            && let Some(body) = row.body.as_deref()
        {
            facts.fingerprint = Some(hex(&hash(body.as_bytes())));
            return self.add(facts, Kind::Reference, None, None).await;
        }

        // a copy for the user's own use, fetched from the publisher now
        if row.noarchive {
            return Ok(refused("noarchive"));
        }
        let Ok(address) = Url::parse(&row.url) else {
            return Ok(refused("gone"));
        };
        let fetch = self.private_fetch()?;
        let page = match fetch.fetcher.fetch(&address, &Validators::default()).await {
            Ok(Fetched::Page(page)) => page,
            Ok(Fetched::NotModified { .. }) => return Ok(refused("unavailable")),
            Err(FetchError::Robots { .. }) => return Ok(refused("robots")),
            Err(FetchError::RetryLater { .. }) => return Ok(refused("busy")),
            Err(e) => {
                tracing::info!("a private copy of {} was not fetched: {e}", row.url);
                return Ok(refused("unavailable"));
            }
        };
        if page.index.noarchive {
            return Ok(refused("noarchive"));
        }
        let document = match fetch
            .parser
            .extract(page.body.clone(), page.content_type.as_deref(), &page.url)
            .await
        {
            Ok(document) => document,
            Err(ExtractError::Dangerous(why)) => {
                tracing::warn!("a private copy of {} was refused: {why}", row.url);
                return Ok(refused("dangerous"));
            }
            Err(e) => {
                tracing::info!("a private copy of {} cannot be read: {e}", row.url);
                return Ok(refused("unreadable"));
            }
        };
        let mut rules = IndexRules::default();
        for list in &document.robots {
            rules.add_list(list);
        }
        if rules.noarchive {
            return Ok(refused("noarchive"));
        }
        let text = clean_text(&document.text).into_owned();
        if text.trim().is_empty() {
            return Ok(refused("unreadable"));
        }
        let keep_file = matches!(document.format, Format::Pdf | Format::Docx | Format::Odt);
        if facts.title.trim().is_empty()
            && let Some(title) = document.title.as_deref()
        {
            facts.title = clean_text(title).trim().to_string();
        }
        facts.fingerprint = Some(hex(&hash(text.as_bytes())));
        facts.media_type = keep_file.then(|| document.format.media_type().to_string());
        let file = keep_file.then(|| page.body.to_vec());
        self.add(facts, Kind::Copy, Some(text), file).await
    }

    async fn add(
        &self,
        facts: ItemFacts,
        kind: Kind,
        text: Option<String>,
        file: Option<Vec<u8>>,
    ) -> Result<PrivateSavedView, AppError> {
        let _one_at_a_time = self.library_lock.lock().await;
        let added = self
            .with_library(move |l, k| l.add(k, facts, kind, text.as_deref(), file.as_deref()))
            .await?;
        Ok(match added {
            Added::New(item) => PrivateSavedView {
                status: "saved",
                reason: None,
                item: Some(item.into()),
            },
            Added::Already(item) => PrivateSavedView {
                status: "already",
                reason: None,
                item: Some(item.into()),
            },
        })
    }

    /// A saved document with its text, for reading offline.
    pub async fn private_read(&self, id: String) -> Result<PrivateReadView, AppError> {
        let wanted = id.clone();
        let item = self
            .with_library(move |l, k| Ok(l.items(k)?.into_iter().find(|item| item.id == wanted)))
            .await?
            .ok_or_else(not_found)?;
        // a reference reads the public part, with its archived file and whether it left the web
        let card = match (item.kind, item.facts.document_id) {
            (Kind::Reference, Some(document_id)) => self.document(document_id).await?,
            _ => None,
        };
        let text = match item.kind {
            Kind::Copy => self
                .with_library(move |l, k| l.text(k, &id))
                .await?
                .map(|text| text.to_string()),
            Kind::Reference => card.as_ref().and_then(|c| c.text.clone()),
        };
        Ok(PrivateReadView {
            item: item.into(),
            text,
            archived_file: card.as_ref().and_then(|c| c.archived_file.clone()),
            gone_at: card.and_then(|c| c.gone_at),
        })
    }

    /// Decrypts the kept file of a copy into a private folder for the system's viewer; returns its
    /// path. The file is deleted when the private part is locked or ATENA closes.
    pub async fn private_open_file(&self, id: String) -> Result<PathBuf, AppError> {
        let wanted = id.clone();
        let item = self
            .with_library(move |l, k| Ok(l.items(k)?.into_iter().find(|item| item.id == wanted)))
            .await?
            .ok_or_else(not_found)?;
        let bytes = self
            .with_library(move |l, k| l.file(k, &id))
            .await?
            .ok_or_else(not_found)?;
        let folder = self.folder.join(OPENED);
        let path = folder.join(file_name(
            &item.facts.title,
            &item.id,
            item.facts.media_type.as_deref(),
        ));
        tokio::task::spawn_blocking(move || -> Result<PathBuf, AppError> {
            std::fs::create_dir_all(&folder).map_err(|e| AppError::io(&folder, e))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700))
                    .map_err(|e| AppError::io(&folder, e))?;
            }
            std::fs::write(&path, &*bytes).map_err(|e| AppError::io(&path, e))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                    .map_err(|e| AppError::io(&path, e))?;
            }
            Ok(path)
        })
        .await
        .map_err(internal)?
    }

    /// Removes saved documents; returns how many were removed.
    pub async fn private_remove(&self, ids: Vec<String>) -> Result<usize, AppError> {
        if ids.len() > MAX_REMOVE {
            return Err(AppError::new(
                "input-too-long",
                "too many documents at once",
            ));
        }
        let _one_at_a_time = self.library_lock.lock().await;
        self.with_library(move |l, k| l.remove(k, &ids)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opened_files_get_plain_names() {
        let id = "0123456789abcdef0123456789abcdef";
        assert_eq!(
            file_name("Metodika: hodnotenie / 2026", id, Some("application/pdf")),
            "Metodika_ hodnotenie _ 2026 (01234567).pdf"
        );
        assert_eq!(
            file_name("..\\..\\x", id, Some(Format::Docx.media_type())),
            "______x (01234567).docx"
        );
        assert_eq!(file_name("  ", id, None), "atena-01234567.bin");
    }
}
