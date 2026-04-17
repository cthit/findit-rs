use std::{fmt::Display, future::Future};

use tokio::{
    sync::{Mutex, RwLock},
    time::{Duration, Instant},
};

pub struct SnapshotCache<T> {
    label: &'static str,
    ttl: Duration,
    retry_after_failure: Duration,
    state: RwLock<CacheState<T>>,
    refresh_lock: Mutex<()>,
}

struct CacheState<T> {
    snapshot: Option<CachedSnapshot<T>>,
    last_refresh_attempt: Option<Instant>,
}

struct CachedSnapshot<T> {
    value: T,
    fetched_at: Instant,
}

impl<T> SnapshotCache<T>
where
    T: Clone + Default,
{
    pub fn new(label: &'static str, ttl: Duration, retry_after_failure: Duration) -> Self {
        assert!(ttl > Duration::ZERO, "ttl must be greater than zero");
        assert!(
            retry_after_failure > Duration::ZERO,
            "retry_after_failure must be greater than zero"
        );

        Self {
            label,
            ttl,
            retry_after_failure,
            state: RwLock::new(CacheState {
                snapshot: None,
                last_refresh_attempt: None,
            }),
            refresh_lock: Mutex::new(()),
        }
    }

    pub async fn load_or_refresh<F, Fut, E>(&self, fetch: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Display,
    {
        let now = Instant::now();
        if let Some(value) = self.cached_or_backoff(now).await {
            return value;
        }

        let _refresh_guard = self.refresh_lock.lock().await;

        let now = Instant::now();
        if let Some(value) = self.cached_or_backoff(now).await {
            return value;
        }

        self.refresh_snapshot(fetch).await
    }

    async fn cached_or_backoff(&self, now: Instant) -> Option<T> {
        let state = self.state.read().await;

        if let Some(snapshot) = state.snapshot.as_ref() {
            if now.saturating_duration_since(snapshot.fetched_at) < self.ttl {
                return Some(snapshot.value.clone());
            }

            if let Some(last_attempt) = state.last_refresh_attempt {
                if now.saturating_duration_since(last_attempt) < self.retry_after_failure {
                    return Some(snapshot.value.clone());
                }
            }
        } else if let Some(last_attempt) = state.last_refresh_attempt {
            if now.saturating_duration_since(last_attempt) < self.retry_after_failure {
                return Some(T::default());
            }
        }

        None
    }

    async fn refresh_snapshot<F, Fut, E>(&self, fetch: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Display,
    {
        match fetch().await {
            Ok(value) => {
                let finished_at = Instant::now();
                let mut state = self.state.write().await;
                state.snapshot = Some(CachedSnapshot {
                    value: value.clone(),
                    fetched_at: finished_at,
                });
                state.last_refresh_attempt = Some(finished_at);
                value
            }
            Err(err) => self.handle_refresh_failure(err).await,
        }
    }

    async fn handle_refresh_failure<E: Display>(&self, err: E) -> T {
        eprintln!("findIT {} refresh failed: {err}", self.label);

        let now = Instant::now();
        let mut state = self.state.write().await;
        state.last_refresh_attempt = Some(now);

        if let Some(snapshot) = state.snapshot.as_ref() {
            eprintln!(
                "findIT {} cache returning stale snapshot after refresh failure",
                self.label
            );
            snapshot.value.clone()
        } else {
            eprintln!(
                "findIT {} cache returning empty snapshot after refresh failure",
                self.label
            );
            T::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::sync::Notify;
    use tokio::time::advance;

    #[tokio::test(start_paused = true)]
    async fn returns_cached_data_within_ttl() {
        let cache =
            SnapshotCache::new("test cache", Duration::from_secs(5), Duration::from_secs(2));
        let calls = Arc::new(AtomicUsize::new(0));

        let first = cache
            .load_or_refresh({
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok::<Vec<i32>, &str>(vec![1])
                    }
                }
            })
            .await;
        assert_eq!(first, vec![1]);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let second = cache
            .load_or_refresh({
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok::<Vec<i32>, &str>(vec![2])
                    }
                }
            })
            .await;
        assert_eq!(second, vec![1]);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn expired_requests_share_a_single_refresh() {
        let cache = Arc::new(SnapshotCache::new(
            "test cache",
            Duration::from_secs(5),
            Duration::from_secs(2),
        ));
        let calls = Arc::new(AtomicUsize::new(0));
        let release = Arc::new(Notify::new());

        let initial = cache
            .load_or_refresh({
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok::<Vec<i32>, &str>(vec![1])
                    }
                }
            })
            .await;
        assert_eq!(initial, vec![1]);

        advance(Duration::from_secs(6)).await;

        let mut handles = Vec::new();
        for _ in 0..10 {
            let cache = cache.clone();
            let calls = calls.clone();
            let release = release.clone();
            handles.push(tokio::spawn(async move {
                cache
                    .load_or_refresh(move || {
                        let calls = calls.clone();
                        let release = release.clone();
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            release.notified().await;
                            Ok::<Vec<i32>, &str>(vec![2])
                        }
                    })
                    .await
            }));
        }

        tokio::task::yield_now().await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        release.notify_waiters();

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.expect("task failed"));
        }

        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|value| value == &vec![2]));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn failure_with_snapshot_returns_stale_until_retry_window() {
        let cache =
            SnapshotCache::new("test cache", Duration::from_secs(5), Duration::from_secs(2));
        let calls = Arc::new(AtomicUsize::new(0));

        let seed = cache
            .load_or_refresh({
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok::<Vec<i32>, &str>(vec![1])
                    }
                }
            })
            .await;
        assert_eq!(seed, vec![1]);

        advance(Duration::from_secs(6)).await;

        let stale = cache
            .load_or_refresh({
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err::<Vec<i32>, &str>("boom")
                    }
                }
            })
            .await;
        assert_eq!(stale, vec![1]);
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        let still_stale = cache
            .load_or_refresh({
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok::<Vec<i32>, &str>(vec![3])
                    }
                }
            })
            .await;
        assert_eq!(still_stale, vec![1]);
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        advance(Duration::from_secs(3)).await;
        let refreshed = cache
            .load_or_refresh({
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok::<Vec<i32>, &str>(vec![4])
                    }
                }
            })
            .await;
        assert_eq!(refreshed, vec![4]);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn failure_without_snapshot_returns_empty_and_backoffs() {
        let cache =
            SnapshotCache::new("test cache", Duration::from_secs(5), Duration::from_secs(2));
        let calls = Arc::new(AtomicUsize::new(0));

        let empty = cache
            .load_or_refresh({
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err::<Vec<i32>, &str>("boom")
                    }
                }
            })
            .await;
        assert!(empty.is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let still_empty = cache
            .load_or_refresh({
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok::<Vec<i32>, &str>(vec![3])
                    }
                }
            })
            .await;
        assert!(still_empty.is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        advance(Duration::from_secs(3)).await;
        let seeded = cache
            .load_or_refresh({
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok::<Vec<i32>, &str>(vec![5])
                    }
                }
            })
            .await;
        assert_eq!(seeded, vec![5]);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
