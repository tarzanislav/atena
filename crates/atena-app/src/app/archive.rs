//! ATENA's archive of original files (the author's decision of 25 Oct 2026): official texts and
//! openly licensed PDF and office documents keep their original file, so they stay available
//! after the publisher removes them. The pipeline decides what is archived; here the files are
//! opened, and the archive is checked again when the rights of a source change.

use std::path::{Path, PathBuf};

use atena_core::legality::{SourceFacts, permission};
use atena_core::{Licence, Terms};
use atena_store::Store;

use super::{App, internal};
use crate::AppError;

/// Folder of the archived files the user opened.
const OPENED: &str = "archive-open";

/// Deletes the archived files copied out for viewing (a file still open in a viewer stays until
/// the next time).
pub(super) fn wipe_opened(folder: &Path) {
    let opened = folder.join(OPENED);
    if opened.exists()
        && let Err(e) = std::fs::remove_dir_all(&opened)
    {
        tracing::warn!("{}: {e}", opened.display());
    }
}

/// Deletes archived files that may no longer be kept: the source's licence or terms changed in
/// the register, or the publisher reserved the text or asked for no stored copies since.
pub(super) async fn prune(store: &Store) -> Result<u64, AppError> {
    let rights = store.archive_rights().await?;
    let withdrawn: Vec<i64> = rights
        .into_iter()
        .filter(|r| {
            let licence = Licence::parse(&r.licence);
            let allowed = permission(
                SourceFacts {
                    country: &r.country,
                    licence: &licence,
                    terms: Terms::parse(&r.terms),
                },
                r.tdm_reserved,
            );
            !allowed.show_full_text || r.noarchive
        })
        .map(|r| r.document_id)
        .collect();
    if withdrawn.is_empty() {
        return Ok(0);
    }
    let deleted = store.delete_files(&withdrawn).await?;
    tracing::info!(deleted, "archived files whose rights changed were deleted");
    Ok(deleted)
}

/// A file name for an opened archived file: letters, digits, spaces and dashes of the title, the
/// document's ID and the extension of its type.
pub(super) fn file_name(title: &str, document_id: i64, media_type: &str) -> String {
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
    let extension = match media_type {
        "application/pdf" => "pdf",
        t if t == atena_extract::Format::Docx.media_type() => "docx",
        t if t == atena_extract::Format::Odt.media_type() => "odt",
        _ => "bin",
    };
    match stem.trim() {
        "" => format!("atena-{document_id}.{extension}"),
        stem => format!("{stem} ({document_id}).{extension}"),
    }
}

impl App {
    /// Copies the archived original of a document out for the system's viewer; returns its path.
    /// Only what may be shown in full is opened (the same check as the document card).
    pub async fn open_archived_file(&self, document_id: i64) -> Result<PathBuf, AppError> {
        let not_found = || AppError::new("not-found", format!("no archived file of {document_id}"));
        let card = self.document(document_id).await?.ok_or_else(not_found)?;
        if card.archived_file.is_none() {
            return Err(not_found());
        }
        let (file, content) = self
            .store
            .archived_content(document_id)
            .await?
            .ok_or_else(not_found)?;
        let folder = self.folder.join(OPENED);
        let path = folder.join(file_name(&card.title, document_id, &file.media_type));
        tokio::task::spawn_blocking(move || -> Result<PathBuf, AppError> {
            std::fs::create_dir_all(&folder).map_err(|e| AppError::io(&folder, e))?;
            std::fs::write(&path, &content).map_err(|e| AppError::io(&path, e))?;
            Ok(path)
        })
        .await
        .map_err(internal)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opened_archive_files_get_plain_names() {
        assert_eq!(
            file_name("Zákon č. 245/2008 Z. z.", 42, "application/pdf"),
            "Zákon č_ 245_2008 Z_ z_ (42).pdf"
        );
        assert_eq!(file_name("", 7, "application/x-unknown"), "atena-7.bin");
    }
}
