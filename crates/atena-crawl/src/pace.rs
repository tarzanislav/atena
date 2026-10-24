//! Per-host pacing and per-origin caches.
//!
//! A host gets one request at a time, at least `delay` apart (measured from the start of one
//! request to the start of the next). After `429` or `503` the host is left alone until the
//! `Retry-After` time; callers are told so at once instead of waiting.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use tokio::time::Instant;

/// Hosts and origins kept in memory before idle ones are forgotten.
const MAX_TRACKED: usize = 10_000;

#[derive(Debug)]
struct HostState {
    next: Instant,
    backoff_until: Option<Instant>,
}

/// Hands out turns per host.
#[derive(Debug, Default)]
pub(crate) struct Pacer {
    hosts: Mutex<HashMap<String, Arc<AsyncMutex<HostState>>>>,
}

/// The right to send one request to a host; held until the response body has been read.
pub(crate) struct Turn {
    state: OwnedMutexGuard<HostState>,
}

impl Turn {
    /// Leaves the host alone until `until` (after `429` or `503`).
    pub(crate) fn back_off(&mut self, until: Instant) {
        self.state.backoff_until = Some(until);
        self.state.next = self.state.next.max(until);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // a panic elsewhere must not stop the crawler; the maps stay consistent at every step
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Pacer {
    /// Waits for the host's turn. `Err(remaining)` when the host asked to be left alone longer.
    pub(crate) async fn turn(&self, host: &str, delay: Duration) -> Result<Turn, Duration> {
        let cell = {
            let mut hosts = lock(&self.hosts);
            if hosts.len() >= MAX_TRACKED {
                // forget hosts nobody waits for and that need no pause any more
                let now = Instant::now();
                hosts.retain(|_, cell| {
                    Arc::strong_count(cell) > 1
                        || cell.try_lock().map_or(true, |s| {
                            s.next > now || s.backoff_until.is_some_and(|u| u > now)
                        })
                });
            }
            let state = HostState {
                next: Instant::now(),
                backoff_until: None,
            };
            Arc::clone(
                hosts
                    .entry(host.to_string())
                    .or_insert_with(|| Arc::new(AsyncMutex::new(state))),
            )
        };
        let mut state = cell.lock_owned().await;
        if let Some(until) = state.backoff_until {
            let now = Instant::now();
            if until > now {
                return Err(until - now);
            }
            state.backoff_until = None;
        }
        tokio::time::sleep_until(state.next).await;
        state.next = Instant::now() + delay;
        Ok(Turn { state })
    }
}

#[derive(Debug, Clone)]
struct Entry<T> {
    value: T,
    expires: Instant,
}

type Cell<T> = Arc<AsyncMutex<Option<Entry<T>>>>;

/// A per-origin cache with one fill at a time per key (robots.txt, tdmrep.json).
#[derive(Debug)]
pub(crate) struct OriginCache<T> {
    cells: Mutex<HashMap<String, Cell<T>>>,
}

impl<T> Default for OriginCache<T> {
    fn default() -> Self {
        Self {
            cells: Mutex::new(HashMap::new()),
        }
    }
}

impl<T: Clone> OriginCache<T> {
    /// The cached value, or a fresh one from `fill`, which receives the expired value (if any)
    /// and returns the new value with its lifetime.
    pub(crate) async fn get<F, Fut>(&self, origin: &str, fill: F) -> T
    where
        F: FnOnce(Option<T>) -> Fut,
        Fut: Future<Output = (T, Duration)>,
    {
        let cell = {
            let mut cells = lock(&self.cells);
            if cells.len() >= MAX_TRACKED {
                // forget expired entries nobody is filling
                let now = Instant::now();
                cells.retain(|_, cell| {
                    Arc::strong_count(cell) > 1
                        || cell
                            .try_lock()
                            .map_or(true, |s| s.as_ref().is_some_and(|e| e.expires > now))
                });
            }
            Arc::clone(cells.entry(origin.to_string()).or_default())
        };
        let mut slot = cell.lock().await;
        match slot.as_ref() {
            Some(entry) if entry.expires > Instant::now() => entry.value.clone(),
            _ => {
                let stale = slot.take().map(|e| e.value);
                let (value, ttl) = fill(stale).await;
                *slot = Some(Entry {
                    value: value.clone(),
                    expires: Instant::now() + ttl,
                });
                value
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn idle_hosts_are_forgotten() {
        let pacer = Pacer::default();
        for i in 0..MAX_TRACKED + 5 {
            drop(
                pacer
                    .turn(&format!("h{i}.example"), Duration::ZERO)
                    .await
                    .unwrap(),
            );
        }
        assert!(lock(&pacer.hosts).len() < MAX_TRACKED);
        let cache = OriginCache::<u8>::default();
        for i in 0..MAX_TRACKED + 5 {
            cache
                .get(&format!("o{i}"), |_| async { (1, Duration::ZERO) })
                .await;
        }
        assert!(lock(&cache.cells).len() < MAX_TRACKED);
    }

    #[tokio::test(start_paused = true)]
    async fn turns_are_spaced_and_back_off_is_reported() {
        let pacer = Pacer::default();
        let start = Instant::now();
        for _ in 0..3 {
            drop(
                pacer
                    .turn("a.example", Duration::from_secs(5))
                    .await
                    .unwrap(),
            );
        }
        assert_eq!(Instant::now() - start, Duration::from_secs(10));
        // another host is not delayed
        drop(
            pacer
                .turn("b.example", Duration::from_secs(5))
                .await
                .unwrap(),
        );
        assert_eq!(Instant::now() - start, Duration::from_secs(10));

        let mut turn = pacer
            .turn("a.example", Duration::from_secs(5))
            .await
            .unwrap();
        turn.back_off(Instant::now() + Duration::from_secs(120));
        drop(turn);
        assert_eq!(
            pacer.turn("a.example", Duration::ZERO).await.err(),
            Some(Duration::from_secs(120))
        );
        tokio::time::advance(Duration::from_secs(121)).await;
        assert!(pacer.turn("a.example", Duration::ZERO).await.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn cache_fills_once_and_passes_the_stale_value() {
        let cache: OriginCache<u32> = OriginCache::default();
        let fills = std::sync::atomic::AtomicU32::new(0);
        let get = || {
            cache.get("o", |old| {
                fills.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async move { (old.unwrap_or(0) + 1, Duration::from_secs(60)) }
            })
        };
        assert_eq!(get().await, 1);
        assert_eq!(get().await, 1);
        assert_eq!(fills.load(std::sync::atomic::Ordering::SeqCst), 1);
        tokio::time::advance(Duration::from_secs(61)).await;
        assert_eq!(get().await, 2);
    }
}
