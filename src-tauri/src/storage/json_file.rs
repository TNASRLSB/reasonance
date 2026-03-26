//! JSON file storage backend with atomic writes.

use async_trait::async_trait;
use std::path::PathBuf;

use crate::error::ReasonanceError;
use super::StorageBackend;

/// A filesystem-backed implementation of `StorageBackend`.
///
/// Each namespace maps to a subdirectory under the base path.
/// Each key maps to a file within that subdirectory.
/// Writes are atomic: data is written to a `.tmp` file then renamed.
pub struct JsonFileBackend {
    base_dir: PathBuf,
}

impl JsonFileBackend {
    /// Create a new file-backed store rooted at `base_dir`.
    ///
    /// The directory is created if it doesn't exist.
    pub fn new(base_dir: impl Into<PathBuf>) -> Result<Self, ReasonanceError> {
        let base_dir = base_dir.into();
        std::fs::create_dir_all(&base_dir).map_err(|e| {
            ReasonanceError::io("creating storage base directory", e)
        })?;
        Ok(Self { base_dir })
    }

    /// Build the path for a key within a namespace, sanitizing against path traversal.
    fn key_path(&self, namespace: &str, key: &str) -> PathBuf {
        let safe_ns = sanitize_path_component(namespace);
        let safe_key = sanitize_path_component(key);
        self.base_dir.join(safe_ns).join(safe_key)
    }

    /// Ensure the namespace directory exists.
    fn ensure_ns_dir(&self, namespace: &str) -> Result<PathBuf, ReasonanceError> {
        let safe_ns = sanitize_path_component(namespace);
        let dir = self.base_dir.join(safe_ns);
        std::fs::create_dir_all(&dir).map_err(|e| {
            ReasonanceError::io("creating namespace directory", e)
        })?;
        Ok(dir)
    }
}

/// Sanitize a string for safe use as a filesystem path component.
///
/// Replaces `/`, `\`, and `..` sequences with `_` to prevent path traversal.
/// Also strips leading dots to avoid hidden files on Unix.
fn sanitize_path_component(s: &str) -> String {
    let sanitized = s
        .replace("..", "_")
        .replace('/', "_")
        .replace('\\', "_");
    // Strip leading dots
    let trimmed = sanitized.trim_start_matches('.');
    if trimmed.is_empty() {
        "_".to_string()
    } else {
        trimmed.to_string()
    }
}

#[async_trait]
impl StorageBackend for JsonFileBackend {
    async fn get(&self, namespace: &str, key: &str) -> Result<Option<Vec<u8>>, ReasonanceError> {
        let path = self.key_path(namespace, key);
        match std::fs::read(&path) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ReasonanceError::io("reading storage key", e)),
        }
    }

    async fn put(&self, namespace: &str, key: &str, value: &[u8]) -> Result<(), ReasonanceError> {
        let path = self.key_path(namespace, key);
        self.ensure_ns_dir(namespace)?;

        // Atomic write: write to .tmp then rename
        let tmp_path = path.with_extension("tmp");
        std::fs::write(&tmp_path, value).map_err(|e| {
            ReasonanceError::io("writing temp file", e)
        })?;
        std::fs::rename(&tmp_path, &path).map_err(|e| {
            ReasonanceError::io("atomic rename", e)
        })?;

        Ok(())
    }

    async fn delete(&self, namespace: &str, key: &str) -> Result<bool, ReasonanceError> {
        let path = self.key_path(namespace, key);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(ReasonanceError::io("deleting storage key", e)),
        }
    }

    async fn list_keys(
        &self,
        namespace: &str,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, ReasonanceError> {
        let safe_ns = sanitize_path_component(namespace);
        let dir = self.base_dir.join(safe_ns);

        if !dir.is_dir() {
            return Ok(Vec::new());
        }

        let entries = std::fs::read_dir(&dir).map_err(|e| {
            ReasonanceError::io("listing storage keys", e)
        })?;

        let mut keys = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| ReasonanceError::io("reading dir entry", e))?;
            if let Some(name) = entry.file_name().to_str() {
                // Skip temp files
                if name.ends_with(".tmp") {
                    continue;
                }
                match prefix {
                    Some(p) if !name.starts_with(p) => continue,
                    _ => keys.push(name.to_string()),
                }
            }
        }

        Ok(keys)
    }

    async fn exists(&self, namespace: &str, key: &str) -> Result<bool, ReasonanceError> {
        let path = self.key_path(namespace, key);
        Ok(path.is_file())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn make_backend() -> (JsonFileBackend, TempDir) {
        let tmp = TempDir::new().unwrap();
        let backend = JsonFileBackend::new(tmp.path()).unwrap();
        (backend, tmp)
    }

    #[tokio::test]
    async fn put_and_get() {
        let (backend, _tmp) = make_backend();
        backend.put("ns", "key1", b"hello world").await.unwrap();
        let val = backend.get("ns", "key1").await.unwrap();
        assert_eq!(val, Some(b"hello world".to_vec()));
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let (backend, _tmp) = make_backend();
        assert_eq!(backend.get("ns", "nope").await.unwrap(), None);
    }

    #[tokio::test]
    async fn delete_existing() {
        let (backend, _tmp) = make_backend();
        backend.put("ns", "k", b"data").await.unwrap();
        assert!(backend.delete("ns", "k").await.unwrap());
        assert_eq!(backend.get("ns", "k").await.unwrap(), None);
    }

    #[tokio::test]
    async fn delete_missing() {
        let (backend, _tmp) = make_backend();
        assert!(!backend.delete("ns", "nope").await.unwrap());
    }

    #[tokio::test]
    async fn list_keys_with_prefix() {
        let (backend, _tmp) = make_backend();
        backend.put("ns", "user-alice", b"1").await.unwrap();
        backend.put("ns", "user-bob", b"2").await.unwrap();
        backend.put("ns", "config-x", b"3").await.unwrap();

        let mut users = backend.list_keys("ns", Some("user-")).await.unwrap();
        users.sort();
        assert_eq!(users, vec!["user-alice", "user-bob"]);
    }

    #[tokio::test]
    async fn list_keys_no_prefix() {
        let (backend, _tmp) = make_backend();
        backend.put("ns", "a", b"1").await.unwrap();
        backend.put("ns", "b", b"2").await.unwrap();

        let all = backend.list_keys("ns", None).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn exists_check() {
        let (backend, _tmp) = make_backend();
        assert!(!backend.exists("ns", "k").await.unwrap());
        backend.put("ns", "k", b"v").await.unwrap();
        assert!(backend.exists("ns", "k").await.unwrap());
    }

    #[tokio::test]
    async fn namespace_isolation() {
        let (backend, _tmp) = make_backend();
        backend.put("ns1", "key", b"one").await.unwrap();
        backend.put("ns2", "key", b"two").await.unwrap();
        assert_eq!(backend.get("ns1", "key").await.unwrap(), Some(b"one".to_vec()));
        assert_eq!(backend.get("ns2", "key").await.unwrap(), Some(b"two".to_vec()));
    }

    #[tokio::test]
    async fn path_traversal_sanitized() {
        let (backend, tmp) = make_backend();
        // Attempt path traversal via key
        backend.put("ns", "../../../etc/passwd", b"nope").await.unwrap();
        // Should NOT create file outside base dir
        assert!(!Path::new("/etc/passwd_nope").exists());
        // File should be inside base dir under sanitized name
        let val = backend.get("ns", "../../../etc/passwd").await.unwrap();
        assert_eq!(val, Some(b"nope".to_vec()));

        // Verify the file is actually inside the temp dir
        let entries: Vec<_> = std::fs::read_dir(tmp.path().join("ns"))
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn atomic_write_no_partial() {
        let (backend, _tmp) = make_backend();
        // Write and then overwrite — second value should be complete
        backend.put("ns", "key", b"first").await.unwrap();
        backend.put("ns", "key", b"second-longer-value").await.unwrap();
        let val = backend.get("ns", "key").await.unwrap();
        assert_eq!(val, Some(b"second-longer-value".to_vec()));
    }

    #[test]
    fn sanitize_various_inputs() {
        assert_eq!(sanitize_path_component("normal"), "normal");
        assert_eq!(sanitize_path_component("../etc"), "__etc");
        assert_eq!(sanitize_path_component("foo/bar"), "foo_bar");
        assert_eq!(sanitize_path_component("foo\\bar"), "foo_bar");
        assert_eq!(sanitize_path_component(".."), "_");
        assert_eq!(sanitize_path_component(".hidden"), "hidden");
        assert_eq!(sanitize_path_component("..."), "_.");
    }
}
