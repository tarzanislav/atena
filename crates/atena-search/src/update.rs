//! Keeping the index up to date from the database's search queue.

use std::collections::HashMap;
use std::path::Path;

use atena_store::Store;
use serde::Serialize;

use crate::Result;
use crate::document::{SearchDocument, searchable};
use crate::index::SearchIndex;

/// Documents taken from the queue at a time (one commit each).
pub const DEFAULT_BATCH: i64 = 500;

/// What an update did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct UpdateReport {
    /// Documents written into the index.
    pub indexed: u64,
    /// Documents removed or left out: erased, gone, or not to be found (see `searchable`).
    pub left_out: u64,
    /// Commits made.
    pub commits: u64,
}

/// Takes the queued documents, writes or removes them, commits, and only then marks them done.
///
/// Documents queued again meanwhile are taken again; a crawler running alongside cannot keep the
/// update going for ever – what it queues after the update started may wait for the next one.
pub async fn update(store: &Store, index: &SearchIndex, batch: i64) -> Result<UpdateReport> {
    let batch = batch.max(1);
    let pending = store.search_queue_len().await?;
    let mut report = UpdateReport::default();
    if pending == 0 {
        return Ok(report);
    }
    let mut writer = index.writer()?;
    for _ in 0..pending / batch + 2 {
        let entries = store.search_queue_take(batch).await?;
        if entries.is_empty() {
            break;
        }
        let ids: Vec<i64> = entries.iter().map(|e| e.document_id).collect();
        let documents: HashMap<i64, SearchDocument> = store
            .search_rows(&ids)
            .await?
            .iter()
            .filter_map(searchable)
            .map(|d| (d.id, d))
            .collect();
        for id in &ids {
            let document = documents.get(id);
            index.put(&writer, *id, document)?;
            if document.is_some() {
                report.indexed += 1;
            } else {
                report.left_out += 1;
            }
        }
        index.commit(&mut writer)?;
        report.commits += 1;
        store.search_queue_done(&entries).await?;
    }
    writer.wait_merging_threads()?;
    Ok(report)
}

/// Deletes the index in `path` (only an ATENA index), queues every document and indexes them.
pub async fn rebuild(
    store: &Store,
    path: &Path,
    batch: i64,
) -> Result<(SearchIndex, UpdateReport)> {
    let index = SearchIndex::recreate(path)?;
    store.search_queue_all().await?;
    let report = update(store, &index, batch).await?;
    Ok((index, report))
}
