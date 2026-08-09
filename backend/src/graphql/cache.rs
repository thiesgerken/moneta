//! A short-lived cache in front of the three expensive analysis queries.
//!
//! `history` and `overviews` take two to three seconds against real data, and the analysis
//! tabs ask for the same window again on every mount — a tab switch, a reload, the other
//! person opening the same page. Nothing about that data changes between two of those, so
//! the second one is paid for nothing.
//!
//! Correctness comes from [`AnalysisCache::clear`] on every write rather than from the TTL:
//! a mutation empties the whole thing, so the next read recomputes. The TTL is the backstop
//! for the writes this process does not see — the hourly `periodic --run` job is a separate
//! process with its own connection, and nothing tells the server it ran.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};

const TTL: Duration = Duration::from_secs(600);

/// How much a date in a key is rounded down, in seconds.
///
/// Without this the cache would never hit: every range the frontend asks for ends at
/// `Date.now()`, so an exact key is a fresh one on every page load. Rounding the key means an
/// entry can answer a request for a window ending up to ten minutes after the one it was
/// computed for, which is invisible on a chart of months and no worse than the TTL already
/// allows.
const BUCKET_SECONDS: i64 = 600;

/// Small because the pod has 512Mi and an entry is a few hundred kilobytes — ten years of
/// overviews is 216 KB as JSON. Two people looking at a handful of windows never come close;
/// dragging the range around does, and there dropping everything is fine. The point is to
/// bound the map, not to keep the best of it, and a cache miss beats an OOMKill.
const CAPACITY: usize = 32;

/// The value alongside when it was computed. `Any` because the three resolvers return three
/// different shapes and one map is less to think about than three.
type Entry = (Instant, Arc<dyn Any + Send + Sync>);

#[derive(Default)]
pub struct AnalysisCache {
    entries: Mutex<HashMap<String, Entry>>,
}

impl AnalysisCache {
    /// Empty it. Called from the mutation root, so any write invalidates every analysis
    /// answer rather than trying to work out which windows it touched — an expense moved
    /// across a year changes months at both ends, and the balances after it.
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    fn get<T: Send + Sync + 'static>(&self, key: &str) -> Option<Arc<T>> {
        let mut entries = self.entries.lock().unwrap();
        let (stored, value) = entries.get(key)?;

        if stored.elapsed() > TTL {
            entries.remove(key);
            return None;
        }

        value.clone().downcast().ok()
    }

    fn put<T: Send + Sync + 'static>(&self, key: String, value: Arc<T>) {
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= CAPACITY {
            entries.clear();
        }
        entries.insert(key, (Instant::now(), value));
    }
}

/// A date as it appears in a cache key. See [`BUCKET_SECONDS`].
pub fn bucket(date: DateTime<Utc>) -> i64 {
    date.timestamp().div_euclid(BUCKET_SECONDS)
}

/// Run `compute` unless the answer is already there.
///
/// Concurrent misses on the same key both compute; the loser's result is simply overwritten.
/// Holding the lock across the work instead would mean holding it across a database round
/// trip, which is the one thing worth avoiding here.
pub async fn cached<T, F>(cache: &AnalysisCache, key: String, compute: F) -> juniper::FieldResult<T>
where
    T: Clone + Send + Sync + 'static,
    F: Future<Output = juniper::FieldResult<T>>,
{
    if let Some(hit) = cache.get::<T>(&key) {
        return Ok((*hit).clone());
    }

    let value = compute.await?;
    cache.put(key, Arc::new(value.clone()));
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn serves_a_hit_and_forgets_it_on_a_write() {
        let cache = AnalysisCache::default();
        let calls = std::sync::atomic::AtomicUsize::new(0);

        let run = || {
            cached(&cache, "k".to_string(), async {
                Ok(calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1)
            })
        };

        assert_eq!(run().await.unwrap(), 1);
        assert_eq!(run().await.unwrap(), 1, "the second read is the cached one");

        cache.clear();
        assert_eq!(run().await.unwrap(), 2, "a write forces a recompute");
    }

    #[tokio::test]
    async fn a_failure_is_not_cached() {
        let cache = AnalysisCache::default();

        let failed: juniper::FieldResult<i32> =
            cached(&cache, "k".to_string(), async { Err("no".into()) }).await;
        assert!(failed.is_err());

        assert_eq!(
            cached(&cache, "k".to_string(), async { Ok(7) })
                .await
                .unwrap(),
            7
        );
    }

    #[test]
    fn a_key_ignores_the_seconds_a_page_load_happened_to_start_at() {
        let at = |s: &str| bucket(s.parse::<DateTime<Utc>>().unwrap());

        assert_eq!(at("2026-08-09T12:00:01Z"), at("2026-08-09T12:09:59Z"));
        assert_ne!(at("2026-08-09T12:00:01Z"), at("2026-08-09T12:10:01Z"));
    }
}
