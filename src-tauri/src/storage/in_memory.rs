//! In-memory storage backend for testing.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

use super::{StorageBackend, TransactionId};
use crate::error::ReasonanceError;

/// A fast, in-memory implementation of `StorageBackend`.
///
/// Data lives only for the lifetime of the struct — intended for tests
/// and ephemeral caches.
type NamespaceMap<V> = HashMap<String, HashMap<String, V>>;

/// Buffered writes for a single in-flight transaction.
struct PendingTx {
    namespace: String,
    puts: Vec<(String, Vec<u8>)>,
    appends: Vec<(String, Vec<u8>)>,
}

pub struct InMemoryBackend {
    data: Mutex<NamespaceMap<Vec<u8>>>,
    streams: Mutex<NamespaceMap<Vec<Vec<u8>>>>,
    versions: Mutex<HashMap<String, u32>>,
    transactions: Mutex<HashMap<String, PendingTx>>,
}

impl InMemoryBackend {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
            streams: Mutex::new(HashMap::new()),
            versions: Mutex::new(HashMap::new()),
            transactions: Mutex::new(HashMap::new()),
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

    async fn append(&self, namespace: &str, key: &str, line: &[u8]) -> Result<(), ReasonanceError> {
        let mut guard = self
            .streams
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        guard
            .entry(namespace.to_string())
            .or_default()
            .entry(key.to_string())
            .or_default()
            .push(line.to_vec());
        Ok(())
    }

    async fn read_stream(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Vec<Vec<u8>>, ReasonanceError> {
        let guard = self
            .streams
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        Ok(guard
            .get(namespace)
            .and_then(|ns| ns.get(key))
            .cloned()
            .unwrap_or_default())
    }

    async fn migrate(&self, namespace: &str, version: u32) -> Result<(), ReasonanceError> {
        let mut guard = self
            .versions
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        guard.insert(namespace.to_string(), version);
        Ok(())
    }

    async fn rollback(&self, namespace: &str, version: u32) -> Result<(), ReasonanceError> {
        let mut guard = self
            .versions
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        guard.insert(namespace.to_string(), version);
        Ok(())
    }

    async fn get_version(&self, namespace: &str) -> Result<u32, ReasonanceError> {
        let guard = self
            .versions
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        Ok(guard.get(namespace).copied().unwrap_or(0))
    }

    async fn begin_transaction(&self, namespace: &str) -> Result<TransactionId, ReasonanceError> {
        let id = uuid::Uuid::new_v4().to_string();
        let pending = PendingTx {
            namespace: namespace.to_string(),
            puts: Vec::new(),
            appends: Vec::new(),
        };
        let mut guard = self
            .transactions
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        guard.insert(id.clone(), pending);
        Ok(id)
    }

    async fn tx_put(
        &self,
        tx: &TransactionId,
        key: &str,
        value: &[u8],
    ) -> Result<(), ReasonanceError> {
        let mut guard = self
            .transactions
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        let pending = guard
            .get_mut(tx)
            .ok_or_else(|| ReasonanceError::not_found("transaction", tx))?;
        pending.puts.push((key.to_string(), value.to_vec()));
        Ok(())
    }

    async fn tx_append(
        &self,
        tx: &TransactionId,
        key: &str,
        line: &[u8],
    ) -> Result<(), ReasonanceError> {
        let mut guard = self
            .transactions
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        let pending = guard
            .get_mut(tx)
            .ok_or_else(|| ReasonanceError::not_found("transaction", tx))?;
        pending.appends.push((key.to_string(), line.to_vec()));
        Ok(())
    }

    async fn commit(&self, tx: TransactionId) -> Result<(), ReasonanceError> {
        let pending = {
            let mut guard = self
                .transactions
                .lock()
                .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
            guard
                .remove(&tx)
                .ok_or_else(|| ReasonanceError::not_found("transaction", &tx))?
        };
        let ns = &pending.namespace;
        for (key, value) in &pending.puts {
            self.put(ns, key, value).await?;
        }
        for (key, line) in &pending.appends {
            self.append(ns, key, line).await?;
        }
        Ok(())
    }

    async fn rollback_transaction(&self, tx: TransactionId) -> Result<(), ReasonanceError> {
        let mut guard = self
            .transactions
            .lock()
            .map_err(|e| ReasonanceError::internal(format!("lock poisoned: {e}")))?;
        guard
            .remove(&tx)
            .ok_or_else(|| ReasonanceError::not_found("transaction", &tx))?;
        Ok(())
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

    #[tokio::test]
    async fn transaction_commit_applies_writes() {
        let backend = InMemoryBackend::new();
        let tx = backend.begin_transaction("ns").await.unwrap();
        backend.tx_put(&tx, "key1", b"val1").await.unwrap();
        backend.tx_append(&tx, "log", b"line1").await.unwrap();
        assert!(backend.get("ns", "key1").await.unwrap().is_none());
        backend.commit(tx).await.unwrap();
        assert_eq!(
            backend.get("ns", "key1").await.unwrap(),
            Some(b"val1".to_vec())
        );
        assert_eq!(backend.read_stream("ns", "log").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn transaction_rollback_discards_writes() {
        let backend = InMemoryBackend::new();
        let tx = backend.begin_transaction("ns").await.unwrap();
        backend.tx_put(&tx, "key1", b"val1").await.unwrap();
        backend.rollback_transaction(tx).await.unwrap();
        assert!(backend.get("ns", "key1").await.unwrap().is_none());
    }
}
