//! JSON file storage backend with atomic writes.

use async_trait::async_trait;
use std::path::{Path, PathBuf};

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

/// Atomic file write: write to `.tmp`, then rename.
///
/// Rename is atomic on most local filesystems, ensuring the destination
/// file is never seen in a partial state.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), ReasonanceError> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)
        .map_err(|e| ReasonanceError::io("atomic write temp file", e))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| ReasonanceError::io("atomic write rename", e))?;
    Ok(())
}

/// Append a complete line to a JSONL file with fsync.
///
/// Creates the file if it does not exist. Each call appends exactly one
/// newline-terminated line and flushes to disk before returning.
pub fn safe_append(path: &Path, line: &str) -> Result<(), ReasonanceError> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| ReasonanceError::io("open JSONL for append", e))?;
    writeln!(file, "{}", line)
        .map_err(|e| ReasonanceError::io("append to JSONL", e))?;
    file.sync_data()
        .map_err(|e| ReasonanceError::io("fsync JSONL", e))?;
    Ok(())
}

/// Validate a JSONL file: truncate any partial last line.
///
/// Each non-empty line must be valid JSON. The first line that fails
/// JSON parsing (or a read error) marks the end of the valid region;
/// the file is truncated there.
///
/// Returns the number of valid (successfully parsed) lines.
pub fn validate_jsonl(path: &Path) -> Result<usize, ReasonanceError> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open(path)
        .map_err(|e| ReasonanceError::io("open JSONL for validation", e))?;
    let reader = BufReader::new(file);

    let mut last_valid_pos: u64 = 0;
    let mut valid_count = 0;

    for line in reader.lines() {
        match line {
            Ok(l) => {
                if l.trim().is_empty() {
                    continue;
                }
                if serde_json::from_str::<serde_json::Value>(&l).is_ok() {
                    last_valid_pos += l.len() as u64 + 1; // +1 for '\n'
                    valid_count += 1;
                } else {
                    // Partial or corrupt line — stop here
                    break;
                }
            }
            Err(_) => break,
        }
    }

    // Truncate file to last valid position
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| ReasonanceError::io("open JSONL for truncation", e))?;
    file.set_len(last_valid_pos)
        .map_err(|e| ReasonanceError::io("truncate JSONL", e))?;

    Ok(valid_count)
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

    // ── atomic_write ──────────────────────────────────────────────────────────

    #[test]
    fn atomic_write_creates_file_with_correct_content() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("out.json");
        atomic_write(&path, b"{\"ok\":true}").unwrap();
        let content = std::fs::read(&path).unwrap();
        assert_eq!(content, b"{\"ok\":true}");
        // No leftover .tmp file
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn atomic_write_overwrites_existing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("out.json");
        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, b"second-longer").unwrap();
        let content = std::fs::read(&path).unwrap();
        assert_eq!(content, b"second-longer");
    }

    // ── safe_append ───────────────────────────────────────────────────────────

    #[test]
    fn safe_append_creates_file_and_appends_lines() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        safe_append(&path, r#"{"id":1}"#).unwrap();
        safe_append(&path, r#"{"id":2}"#).unwrap();
        safe_append(&path, r#"{"id":3}"#).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], r#"{"id":1}"#);
        assert_eq!(lines[1], r#"{"id":2}"#);
        assert_eq!(lines[2], r#"{"id":3}"#);
    }

    // ── validate_jsonl ────────────────────────────────────────────────────────

    #[test]
    fn validate_jsonl_valid_file_returns_correct_count() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("log.jsonl");
        safe_append(&path, r#"{"a":1}"#).unwrap();
        safe_append(&path, r#"{"b":2}"#).unwrap();
        safe_append(&path, r#"{"c":3}"#).unwrap();

        let count = validate_jsonl(&path).unwrap();
        assert_eq!(count, 3);

        // File content must be unchanged
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), 3);
    }

    #[test]
    fn validate_jsonl_partial_last_line_is_truncated() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("log.jsonl");

        // Write two complete lines then a partial/corrupt one
        use std::io::Write;
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"ok":true}}"#).unwrap();
        writeln!(f, r#"{{"ok":true}}"#).unwrap();
        write!(f, r#"{{"partial":"#).unwrap(); // no closing brace or newline
        drop(f);

        let count = validate_jsonl(&path).unwrap();
        assert_eq!(count, 2);

        // File must contain exactly 2 complete lines
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), 2);
        // Partial line must be gone
        assert!(!content.contains("partial"));
    }

    #[test]
    fn validate_jsonl_empty_file_returns_zero() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.jsonl");
        std::fs::File::create(&path).unwrap();

        let count = validate_jsonl(&path).unwrap();
        assert_eq!(count, 0);

        // File remains empty
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
    }
}
