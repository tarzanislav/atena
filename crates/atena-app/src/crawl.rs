//! The crawler inside the app: started and stopped by the user, never by itself.

use std::sync::Arc;

use atena_core::Taxonomy;
use atena_crawl::{Config, Fetcher};
use atena_ingest::{Pipeline, PipelineConfig, Progress, RunStats, WorkerConfig, run_observed};
use atena_store::Store;
use jiff::Timestamp;
use tokio::sync::{Mutex, Notify, watch};
use tokio::task::JoinHandle;

use crate::AppError;
use crate::views::CrawlView;

struct Running {
    stop: watch::Sender<bool>,
    progress: Progress,
    task: JoinHandle<RunStats>,
    started: Timestamp,
}

/// Starts, watches and stops the crawler; at most one runs.
#[derive(Default)]
pub struct Crawl {
    running: Mutex<Option<Running>>,
}

impl Crawl {
    /// Starts the crawler with `concurrency` hosts at a time; nothing happens when it runs already.
    /// `finished` is notified when it ends, so the search index takes the last checks at once.
    pub async fn start(
        &self,
        store: &Store,
        taxonomy: &Arc<Taxonomy>,
        concurrency: usize,
        finished: Arc<Notify>,
    ) -> Result<(), AppError> {
        let mut running = self.running.lock().await;
        if running.as_ref().is_some_and(|r| !r.task.is_finished()) {
            return Ok(());
        }
        let fetcher =
            Fetcher::new(Config::default()).map_err(|e| AppError::new("crawl", e.to_string()))?;
        let pipeline = Arc::new(Pipeline::new(
            store.clone(),
            Arc::new(fetcher),
            Arc::clone(taxonomy),
            PipelineConfig {
                parser: atena_ingest::parser(),
                ..PipelineConfig::default()
            },
        ));
        let config = WorkerConfig {
            worker_id: format!("desktop-{}", std::process::id()),
            concurrency,
            ..WorkerConfig::default()
        };
        let (stop, stop_rx) = watch::channel(false);
        let progress = Progress::default();
        let watched = progress.clone();
        let task = tokio::spawn(async move {
            let stats = run_observed(pipeline, config, stop_rx, false, watched).await;
            finished.notify_one();
            stats
        });
        *running = Some(Running {
            stop,
            progress,
            task,
            started: Timestamp::now(),
        });
        Ok(())
    }

    /// Asks the crawler to stop: the checks under way finish, then it ends.
    pub async fn request_stop(&self) {
        if let Some(r) = self.running.lock().await.as_ref() {
            let _ = r.stop.send(true);
        }
    }

    /// Stops the crawler and waits until it has ended.
    pub async fn stop(&self) -> Option<RunStats> {
        let running = self.running.lock().await.take()?;
        let _ = running.stop.send(true);
        running.task.await.ok()
    }

    /// The state of the crawler.
    pub async fn status(&self) -> CrawlView {
        let running = self.running.lock().await;
        let Some(r) = running.as_ref() else {
            return CrawlView::default();
        };
        let stats = r.progress.snapshot();
        CrawlView {
            running: !r.task.is_finished(),
            stopping: *r.stop.borrow() && !r.task.is_finished(),
            started: Some(r.started),
            checks: stats.checks(),
            hosts: stats.hosts,
            store_errors: stats.store_errors,
            outcomes: stats.outcomes,
        }
    }

    /// The crawler runs.
    pub async fn is_running(&self) -> bool {
        self.running
            .lock()
            .await
            .as_ref()
            .is_some_and(|r| !r.task.is_finished())
    }
}
