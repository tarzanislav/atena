//! The worker: leases due hosts, checks their endpoints one after another, releases them.
//!
//! Up to `concurrency` hosts are worked on at the same time, each by its own task; within a host
//! the fetcher's pacing spaces the requests. A host is given back after `host_batch` endpoints or
//! `host_budget`, so long queues of one publisher do not block the others, and immediately when it
//! asks for rest (`429`/`503`). Shutdown is graceful: running checks finish, hosts are released.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use atena_store::{HostLease, StoreError};
use tokio::sync::watch;
use tokio::task::JoinSet;

use crate::pipeline::Pipeline;

/// Settings of a worker.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Name of this worker in host leases (default: host name and process ID).
    pub worker_id: String,
    /// Hosts worked on at the same time (default 16).
    pub concurrency: usize,
    /// Endpoints of one host per lease (default 20).
    pub host_batch: i64,
    /// Time spent on one host per lease (default 10 min).
    pub host_budget: Duration,
    /// Lease length; longer than the budget plus one request (default 20 min).
    pub lease: Duration,
    /// Pause when nothing is due (default 10 s).
    pub idle_wait: Duration,
    /// How long the check log is kept (default 90 days); older records are deleted hourly.
    pub log_retention: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "worker".into());
        Self {
            worker_id: format!("{host}-{}", std::process::id()),
            concurrency: 16,
            host_batch: 20,
            host_budget: Duration::from_secs(600),
            lease: Duration::from_secs(1_200),
            idle_wait: Duration::from_secs(10),
            log_retention: Duration::from_secs(90 * 86_400),
        }
    }
}

/// Counts of outcomes of one run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunStats {
    /// Checks per outcome.
    pub outcomes: BTreeMap<String, u64>,
    /// Hosts processed.
    pub hosts: u64,
    /// Errors of the database or file store.
    pub store_errors: u64,
}

impl RunStats {
    /// All checks.
    pub fn checks(&self) -> u64 {
        self.outcomes.values().sum()
    }
}

async fn work_on_host(
    pipeline: Arc<Pipeline>,
    config: WorkerConfig,
    host: HostLease,
    stats: Arc<Mutex<RunStats>>,
    stop: watch::Receiver<bool>,
) -> Result<(), StoreError> {
    let store = pipeline.store();
    let started = Instant::now();
    let mut backoff = None;
    let endpoints = store.due_endpoints(host.id, config.host_batch).await;
    let result = match endpoints {
        Ok(endpoints) => {
            let mut result = Ok(());
            for ep in endpoints {
                if *stop.borrow() || started.elapsed() > config.host_budget {
                    break;
                }
                match pipeline.check(&ep).await {
                    Ok(outcome) => {
                        let mut s = stats
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        *s.outcomes.entry(outcome.outcome).or_default() += 1;
                        if outcome.backoff.is_some() {
                            backoff = outcome.backoff;
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!(endpoint = ep.id, error = %e, "store error");
                        result = Err(e);
                        break;
                    }
                }
            }
            result
        }
        Err(e) => Err(e),
    };
    // the lease is given back in every case; if this fails too, it expires on its own
    let released = store
        .release_host(host.id, &config.worker_id, backoff)
        .await;
    stats
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .hosts += 1;
    result.and(released)
}

/// Waits until `stop` changes. When the sender is gone no stop can come any more, so this waits
/// forever instead of returning at once (which would turn the loops below into busy loops).
async fn stop_requested(stop: &mut watch::Receiver<bool>) {
    if stop.changed().await.is_err() {
        std::future::pending::<()>().await;
    }
}

/// The counts of a running worker, for watching it while it works.
#[derive(Debug, Clone, Default)]
pub struct Progress(Arc<Mutex<RunStats>>);

impl Progress {
    /// The counts so far.
    pub fn snapshot(&self) -> RunStats {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

/// Runs until `stop` becomes `true`; with `once`, returns as soon as nothing is due any more.
///
/// A check whose extraction exceeds its time limit is abandoned, but its worker thread runs on
/// until the library returns; repeated failures switch a discovered endpoint off.
pub async fn run(
    pipeline: Arc<Pipeline>,
    config: WorkerConfig,
    stop: watch::Receiver<bool>,
    once: bool,
) -> RunStats {
    run_observed(pipeline, config, stop, once, Progress::default()).await
}

/// [`run`], counting into `progress` as it goes.
pub async fn run_observed(
    pipeline: Arc<Pipeline>,
    config: WorkerConfig,
    mut stop: watch::Receiver<bool>,
    once: bool,
    progress: Progress,
) -> RunStats {
    let stats = progress.0;
    let mut tasks: JoinSet<Result<(), StoreError>> = JoinSet::new();
    let mut last_prune: Option<Instant> = None;
    loop {
        if *stop.borrow() {
            break;
        }
        // storage limitation: old check records are deleted (at start and then hourly)
        if last_prune.is_none_or(|t| t.elapsed() > Duration::from_secs(3_600)) {
            match pipeline.store().prune_fetch_log(config.log_retention).await {
                Ok(n) if n > 0 => tracing::info!(deleted = n, "old check records deleted"),
                Ok(_) => {}
                Err(e) => tracing::error!(error = %e, "pruning the check log failed"),
            }
            last_prune = Some(Instant::now());
        }
        let free = config.concurrency.saturating_sub(tasks.len());
        let leased = if free > 0 {
            match pipeline
                .store()
                .lease_hosts(
                    &config.worker_id,
                    i64::try_from(free).unwrap_or(i64::MAX),
                    config.lease,
                )
                .await
            {
                Ok(hosts) => hosts,
                Err(e) => {
                    tracing::error!(error = %e, "leasing hosts failed");
                    stats
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .store_errors += 1;
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        let got_work = !leased.is_empty();
        for host in leased {
            tasks.spawn(work_on_host(
                Arc::clone(&pipeline),
                config.clone(),
                host,
                Arc::clone(&stats),
                stop.clone(),
            ));
        }
        if tasks.is_empty() && !got_work {
            if once {
                break;
            }
            tokio::select! {
                () = tokio::time::sleep(config.idle_wait) => {}
                () = stop_requested(&mut stop) => {}
            }
            continue;
        }
        // wait for a task to finish, but look for new hosts now and then while slots are free
        tokio::select! {
            Some(done) = tasks.join_next() => record(&stats, done),
            () = tokio::time::sleep(Duration::from_secs(1)), if tasks.len() < config.concurrency => {}
            () = stop_requested(&mut stop) => {}
        }
    }
    while let Some(done) = tasks.join_next().await {
        record(&stats, done);
    }
    stats
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

fn record(stats: &Mutex<RunStats>, done: Result<Result<(), StoreError>, tokio::task::JoinError>) {
    let failed = match done {
        Ok(Ok(())) => false,
        Ok(Err(e)) => {
            tracing::error!(error = %e, "host task failed");
            true
        }
        Err(e) => {
            tracing::error!(error = %e, "host task panicked");
            true
        }
    };
    if failed {
        stats
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .store_errors += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn a_dropped_stop_sender_does_not_end_the_wait() {
        let (tx, mut rx) = watch::channel(false);
        drop(tx);
        let waited = tokio::time::timeout(Duration::from_secs(3600), stop_requested(&mut rx)).await;
        assert!(
            waited.is_err(),
            "the wait must not return when the sender is gone"
        );
    }
}
