//! Keeping the search index in step with the database while ATENA runs.

use std::sync::Arc;
use std::time::Duration;

use atena_search::{DEFAULT_BATCH, SearchIndex, update};
use atena_store::Store;
use tokio::sync::{Notify, watch};
use tokio::task::JoinHandle;

/// How often the index is brought up to date without being asked.
pub const INDEX_EVERY: Duration = Duration::from_secs(60);

/// Brings `index` up to date every [`INDEX_EVERY`], whenever `nudge` is notified, and once more
/// when `stop` becomes `true`; failures are logged and the work goes on. Await the handle to let
/// the last update finish. This task should be the only one writing to the index.
pub fn keep_fresh(
    store: Store,
    index: Arc<SearchIndex>,
    nudge: Arc<Notify>,
    mut stop: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
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
            if *stop.borrow() {
                break;
            }
            tokio::select! {
                () = tokio::time::sleep(INDEX_EVERY) => {}
                () = nudge.notified() => {}
                changed = stop.changed() => {
                    if changed.is_err() {
                        // no stop can come any more: the owner is gone
                        break;
                    }
                }
            }
        }
    })
}
