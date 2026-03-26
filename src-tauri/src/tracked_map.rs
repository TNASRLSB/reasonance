//! `TrackedMap<K, V>` — a lifecycle-aware HashMap wrapper.
//!
//! Values are stored as `Arc<Mutex<V>>`. Callers receive a `WeakHandle<V>`
//! that can detect when the map has dropped its strong reference (after
//! `remove`) without keeping the value alive themselves.
//!
//! The `sweep_exclusive` method removes every entry for which the map is
//! the **only** remaining `Arc` owner (`strong_count == 1`), providing a
//! simple GC-style cleanup for idle resources.

use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex, Weak},
};

// ---------------------------------------------------------------------------
// WeakHandle
// ---------------------------------------------------------------------------

/// A non-owning handle to a value stored in a [`TrackedMap`].
///
/// Holds a `Weak<Mutex<V>>` so it does **not** extend the value's lifetime.
/// The `id` string is kept for logging / debugging after the value has died.
#[derive(Clone)]
pub struct WeakHandle<V> {
    inner: Weak<Mutex<V>>,
    pub id: String,
}

impl<V> WeakHandle<V> {
    fn new(arc: &Arc<Mutex<V>>, id: impl Into<String>) -> Self {
        Self {
            inner: Arc::downgrade(arc),
            id: id.into(),
        }
    }

    /// Attempt to upgrade to a strong reference.
    /// Returns `None` if the map (or all other owners) have dropped the value.
    pub fn upgrade(&self) -> Option<Arc<Mutex<V>>> {
        self.inner.upgrade()
    }

    /// Returns `true` while at least one strong `Arc` still exists.
    pub fn is_alive(&self) -> bool {
        self.inner.strong_count() > 0
    }
}

// ---------------------------------------------------------------------------
// TrackedMap
// ---------------------------------------------------------------------------

/// A `HashMap` whose values are `Arc<Mutex<V>>`.
///
/// Every insertion hands back a [`WeakHandle`] so callers can observe the
/// value's lifecycle without keeping it alive. `sweep_exclusive` removes
/// entries that have no external strong references.
pub struct TrackedMap<K, V> {
    inner: HashMap<K, Arc<Mutex<V>>>,
}

impl<K, V> TrackedMap<K, V>
where
    K: Eq + Hash,
{
    /// Create an empty map.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Insert `value` under `key`, identified by `id`.
    ///
    /// Stores the value in an `Arc<Mutex<V>>` and returns a [`WeakHandle`]
    /// pointing to it. Any previous value for that key is silently replaced.
    pub fn insert(&mut self, key: K, value: V, id: impl Into<String>) -> WeakHandle<V> {
        let arc = Arc::new(Mutex::new(value));
        let handle = WeakHandle::new(&arc, id);
        self.inner.insert(key, arc);
        handle
    }

    /// Retrieve a clone of the `Arc` stored under `key`, if present.
    pub fn get(&self, key: &K) -> Option<Arc<Mutex<V>>> {
        self.inner.get(key).cloned()
    }

    /// Remove the entry for `key`.
    ///
    /// Returns `true` if an entry was removed.  After this the `Arc`'s
    /// strong count drops by 1; if no external holder exists it reaches 0
    /// and the value is freed.
    pub fn remove(&mut self, key: &K) -> bool {
        self.inner.remove(key).is_some()
    }

    /// Number of entries currently in the map.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` when the map contains no entries.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Remove every entry for which the map is the **sole** `Arc` owner
    /// (`strong_count == 1`).
    ///
    /// Returns the number of entries swept.
    pub fn sweep_exclusive(&mut self) -> usize {
        let before = self.inner.len();
        self.inner.retain(|_, arc| Arc::strong_count(arc) > 1);
        before - self.inner.len()
    }

    /// Iterate over all stored `Arc<Mutex<V>>` values.
    pub fn values(&self) -> impl Iterator<Item = &Arc<Mutex<V>>> {
        self.inner.values()
    }

    /// Iterate over all keys.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.inner.keys()
    }

    /// Iterate over `(key, Arc<Mutex<V>>)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &Arc<Mutex<V>>)> {
        self.inner.iter()
    }

    /// Retain only entries for which `f(key, arc)` returns `true`.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut Arc<Mutex<V>>) -> bool,
    {
        self.inner.retain(f);
    }
}

impl<K, V> Default for TrackedMap<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // 1. insert → get → value correct
    #[test]
    fn test_insert_and_get_returns_correct_value() {
        let mut map: TrackedMap<&str, i32> = TrackedMap::new();
        map.insert("answer", 42, "answer-id");

        let arc = map.get(&"answer").expect("should be present");
        let value = *arc.lock().unwrap();
        assert_eq!(value, 42);
    }

    // 2. weak handle alive before remove, dead after remove (when no external Arc)
    #[test]
    fn test_weak_handle_alive_then_dead_after_remove() {
        let mut map: TrackedMap<&str, String> = TrackedMap::new();
        let handle = map.insert("key", "hello".to_string(), "key-id");

        assert!(handle.is_alive(), "handle should be alive after insert");
        assert!(handle.upgrade().is_some());

        map.remove(&"key");

        assert!(!handle.is_alive(), "handle should be dead after remove");
        assert!(handle.upgrade().is_none());
    }

    // 3. sweep_exclusive behaviour
    #[test]
    fn test_sweep_exclusive_removes_only_entries_without_external_refs() {
        let mut map: TrackedMap<&str, u32> = TrackedMap::new();

        // "solo"  — no external holder → should be swept
        map.insert("solo", 1, "solo");

        // "held"  — we keep an external Arc → should survive
        let _external = map.get(&"held").unwrap_or_else(|| {
            // first insert it, then retrieve the Arc
            map.insert("held", 2, "held");
            map.get(&"held").unwrap()
        });

        assert_eq!(map.len(), 2);

        let swept = map.sweep_exclusive();

        assert_eq!(swept, 1, "exactly one entry swept");
        assert_eq!(map.len(), 1, "held entry survives");
        assert!(map.get(&"held").is_some());
        assert!(map.get(&"solo").is_none());
    }

    // 4. len and is_empty
    #[test]
    fn test_len_and_is_empty() {
        let mut map: TrackedMap<u32, &str> = TrackedMap::new();

        assert!(map.is_empty());
        assert_eq!(map.len(), 0);

        map.insert(1, "a", "a");
        map.insert(2, "b", "b");
        assert!(!map.is_empty());
        assert_eq!(map.len(), 2);

        map.remove(&1);
        assert_eq!(map.len(), 1);

        map.remove(&2);
        assert!(map.is_empty());
    }
}
