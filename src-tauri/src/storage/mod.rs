//! Storage abstraction layer.
//!
//! Provides a `StorageBackend` trait for key-value persistence with namespace
//! isolation, plus a `TypedStore<T>` wrapper for automatic serde serialization.

mod in_memory;
mod json_file;

pub use in_memory::InMemoryBackend;
pub use json_file::JsonFileBackend;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use std::sync::Arc;

use crate::error::ReasonanceError;

/// Async key-value storage backend with namespace isolation.
///
/// All keys and namespaces are plain strings. Values are opaque byte slices.
/// Implementations must be `Send + Sync` for use across async tasks.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Retrieve the value for `key` within `namespace`, or `None` if absent.
    async fn get(&self, namespace: &str, key: &str) -> Result<Option<Vec<u8>>, ReasonanceError>;

    /// Store `value` under `key` within `namespace`, overwriting any previous value.
    async fn put(&self, namespace: &str, key: &str, value: &[u8]) -> Result<(), ReasonanceError>;

    /// Delete `key` from `namespace`. Returns `true` if the key existed.
    async fn delete(&self, namespace: &str, key: &str) -> Result<bool, ReasonanceError>;

    /// List all keys in `namespace`, optionally filtered by a string prefix.
    async fn list_keys(
        &self,
        namespace: &str,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, ReasonanceError>;

    /// Check whether `key` exists in `namespace`.
    async fn exists(&self, namespace: &str, key: &str) -> Result<bool, ReasonanceError>;
}

/// A typed wrapper around a `StorageBackend` that handles JSON serialization.
///
/// All values are serialized to/from JSON bytes transparently.
pub struct TypedStore<T: Serialize + DeserializeOwned> {
    backend: Arc<dyn StorageBackend>,
    namespace: String,
    _phantom: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> TypedStore<T> {
    /// Create a new typed store for the given namespace.
    pub fn new(backend: Arc<dyn StorageBackend>, namespace: &str) -> Self {
        Self {
            backend,
            namespace: namespace.to_string(),
            _phantom: PhantomData,
        }
    }

    /// Retrieve and deserialize the value for `key`, or `None` if absent.
    pub async fn get(&self, key: &str) -> Result<Option<T>, ReasonanceError> {
        match self.backend.get(&self.namespace, key).await? {
            Some(bytes) => {
                let value: T = serde_json::from_slice(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Serialize and store `value` under `key`.
    pub async fn put(&self, key: &str, value: &T) -> Result<(), ReasonanceError> {
        let bytes = serde_json::to_vec(value).map_err(|e| ReasonanceError::Serialization {
            context: "TypedStore::put".to_string(),
            message: e.to_string(),
        })?;
        self.backend.put(&self.namespace, key, &bytes).await
    }

    /// Delete `key`. Returns `true` if the key existed.
    pub async fn delete(&self, key: &str) -> Result<bool, ReasonanceError> {
        self.backend.delete(&self.namespace, key).await
    }

    /// List all keys, optionally filtered by prefix.
    pub async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, ReasonanceError> {
        self.backend.list_keys(&self.namespace, prefix).await
    }

    /// Check whether `key` exists.
    pub async fn exists(&self, key: &str) -> Result<bool, ReasonanceError> {
        self.backend.exists(&self.namespace, key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
    struct TestItem {
        name: String,
        count: u32,
    }

    #[tokio::test]
    async fn typed_store_put_and_get() {
        let backend = Arc::new(InMemoryBackend::new());
        let store: TypedStore<TestItem> = TypedStore::new(backend, "test");

        let item = TestItem {
            name: "widget".to_string(),
            count: 42,
        };
        store.put("item1", &item).await.unwrap();
        let retrieved = store.get("item1").await.unwrap();
        assert_eq!(retrieved, Some(item));
    }

    #[tokio::test]
    async fn typed_store_get_missing() {
        let backend = Arc::new(InMemoryBackend::new());
        let store: TypedStore<TestItem> = TypedStore::new(backend, "test");
        assert_eq!(store.get("nonexistent").await.unwrap(), None);
    }

    #[tokio::test]
    async fn typed_store_delete() {
        let backend = Arc::new(InMemoryBackend::new());
        let store: TypedStore<TestItem> = TypedStore::new(backend, "test");

        let item = TestItem {
            name: "temp".to_string(),
            count: 1,
        };
        store.put("k", &item).await.unwrap();
        assert!(store.delete("k").await.unwrap());
        assert_eq!(store.get("k").await.unwrap(), None);
        // Delete again returns false
        assert!(!store.delete("k").await.unwrap());
    }

    #[tokio::test]
    async fn typed_store_list_and_exists() {
        let backend = Arc::new(InMemoryBackend::new());
        let store: TypedStore<TestItem> = TypedStore::new(backend, "ns");

        let item = TestItem {
            name: "x".to_string(),
            count: 0,
        };
        store.put("alpha-1", &item).await.unwrap();
        store.put("alpha-2", &item).await.unwrap();
        store.put("beta-1", &item).await.unwrap();

        assert!(store.exists("alpha-1").await.unwrap());
        assert!(!store.exists("gamma").await.unwrap());

        let mut keys = store.list_keys(Some("alpha")).await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["alpha-1", "alpha-2"]);

        let all = store.list_keys(None).await.unwrap();
        assert_eq!(all.len(), 3);
    }
}
