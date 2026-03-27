//! In-memory storage backend for testing.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

use super::StorageBackend;
use crate::error::ReasonanceError;

/// A fast, in-memory implementation of `StorageBackend`.
///
/// Data lives only for the lifetime of the struct — intended for tests
/// and ephemeral caches.
pub struct InMemoryBackend {
    data: Mutex<HashMap<String, HashMap<String, Vec<u8>>>>,
}

impl InMemoryBackend {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for InMemoryBackend {
    async fn get(&self, namespace: &str, key: &str) -> Result<Option<Vec<u8>>, ReasonanceError> {
        let guard = self
            .data
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        Ok(guard.get(namespace).and_then(|ns| ns.get(key)).cloned())
    }

    async fn put(&self, namespace: &str, key: &str, value: &[u8]) -> Result<(), ReasonanceError> {
        let mut guard = self
            .data
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        guard
            .entry(namespace.to_string())
            .or_default()
            .insert(key.to_string(), value.to_vec());
        Ok(())
    }

    async fn delete(&self, namespace: &str, key: &str) -> Result<bool, ReasonanceError> {
        let mut guard = self
            .data
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        Ok(guard
            .get_mut(namespace)
            .map(|ns| ns.remove(key).is_some())
            .unwrap_or(false))
    }

    async fn list_keys(
        &self,
        namespace: &str,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, ReasonanceError> {
        let guard = self
            .data
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        let keys = match guard.get(namespace) {
            Some(ns) => ns
                .keys()
                .filter(|k| match prefix {
                    Some(p) => k.starts_with(p),
                    None => true,
                })
                .cloned()
                .collect(),
            None => Vec::new(),
        };
        Ok(keys)
    }

    async fn exists(&self, namespace: &str, key: &str) -> Result<bool, ReasonanceError> {
        let guard = self
            .data
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        Ok(guard
            .get(namespace)
            .map(|ns| ns.contains_key(key))
            .unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn put_and_get() {
        let backend = InMemoryBackend::new();
        backend.put("ns", "key1", b"hello").await.unwrap();
        let val = backend.get("ns", "key1").await.unwrap();
        assert_eq!(val, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn get_missing_key() {
        let backend = InMemoryBackend::new();
        let val = backend.get("ns", "nonexistent").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn get_missing_namespace() {
        let backend = InMemoryBackend::new();
        let val = backend.get("no-such-ns", "key").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn delete_existing_key() {
        let backend = InMemoryBackend::new();
        backend.put("ns", "key1", b"data").await.unwrap();
        assert!(backend.delete("ns", "key1").await.unwrap());
        assert_eq!(backend.get("ns", "key1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn delete_missing_key() {
        let backend = InMemoryBackend::new();
        assert!(!backend.delete("ns", "nope").await.unwrap());
    }

    #[tokio::test]
    async fn list_keys_with_prefix() {
        let backend = InMemoryBackend::new();
        backend.put("ns", "user:alice", b"1").await.unwrap();
        backend.put("ns", "user:bob", b"2").await.unwrap();
        backend.put("ns", "config:theme", b"3").await.unwrap();

        let mut users = backend.list_keys("ns", Some("user:")).await.unwrap();
        users.sort();
        assert_eq!(users, vec!["user:alice", "user:bob"]);

        let configs = backend.list_keys("ns", Some("config:")).await.unwrap();
        assert_eq!(configs, vec!["config:theme"]);
    }

    #[tokio::test]
    async fn list_keys_no_prefix() {
        let backend = InMemoryBackend::new();
        backend.put("ns", "a", b"1").await.unwrap();
        backend.put("ns", "b", b"2").await.unwrap();

        let all = backend.list_keys("ns", None).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn exists_check() {
        let backend = InMemoryBackend::new();
        assert!(!backend.exists("ns", "key").await.unwrap());
        backend.put("ns", "key", b"val").await.unwrap();
        assert!(backend.exists("ns", "key").await.unwrap());
    }

    #[tokio::test]
    async fn namespace_isolation() {
        let backend = InMemoryBackend::new();
        backend.put("ns1", "key", b"value1").await.unwrap();
        backend.put("ns2", "key", b"value2").await.unwrap();

        assert_eq!(
            backend.get("ns1", "key").await.unwrap(),
            Some(b"value1".to_vec())
        );
        assert_eq!(
            backend.get("ns2", "key").await.unwrap(),
            Some(b"value2".to_vec())
        );
    }
}
