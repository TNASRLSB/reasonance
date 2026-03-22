use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub id: String,
    pub provider: String,
    pub timestamp: u64,
    pub checksum: String,
}

pub struct NormalizerVersionStore {
    base_dir: PathBuf,
    index: Mutex<HashMap<String, Vec<VersionEntry>>>,
}

impl NormalizerVersionStore {
    pub fn new(base_dir: &Path) -> Self {
        let _ = std::fs::create_dir_all(base_dir);
        let index = Self::load_index(base_dir);
        Self {
            base_dir: base_dir.to_path_buf(),
            index: Mutex::new(index),
        }
    }

    pub fn backup(&self, provider: &str, toml_content: &str) -> Result<String, String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let checksum = format!("{:x}", content_hash(toml_content));
        let id = format!("{}-{}", timestamp, &checksum[..8]);

        let provider_dir = self.base_dir.join(provider);
        std::fs::create_dir_all(&provider_dir).map_err(|e| e.to_string())?;

        let file_path = provider_dir.join(format!("{}.toml", id));
        std::fs::write(&file_path, toml_content).map_err(|e| e.to_string())?;

        let entry = VersionEntry {
            id: id.clone(),
            provider: provider.to_string(),
            timestamp,
            checksum,
        };

        let mut index = self.index.lock().unwrap();
        index.entry(provider.to_string()).or_default().push(entry);
        self.save_index(&index)?;

        Ok(id)
    }

    pub fn restore(&self, provider: &str, version_id: &str) -> Result<String, String> {
        let file_path = self.base_dir.join(provider).join(format!("{}.toml", version_id));
        std::fs::read_to_string(&file_path)
            .map_err(|_| format!("Version {} not found for {}", version_id, provider))
    }

    pub fn list_versions(&self, provider: &str) -> Vec<VersionEntry> {
        let index = self.index.lock().unwrap();
        index.get(provider).cloned().unwrap_or_default()
    }

    pub fn current(&self, provider: &str) -> Option<VersionEntry> {
        let index = self.index.lock().unwrap();
        index.get(provider).and_then(|v| v.last().cloned())
    }

    fn load_index(base_dir: &Path) -> HashMap<String, Vec<VersionEntry>> {
        let index_path = base_dir.join("index.json");
        if let Ok(content) = std::fs::read_to_string(&index_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        }
    }

    fn save_index(&self, index: &HashMap<String, Vec<VersionEntry>>) -> Result<(), String> {
        let index_path = self.base_dir.join("index.json");
        let json = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
        std::fs::write(&index_path, json).map_err(|e| e.to_string())
    }
}

/// Simple hash for checksums (not cryptographic — just for dedup)
fn content_hash(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_toml() -> &'static str {
        r#"[cli]
name = "testprovider"
binary = "test"
programmatic_args = ["-p", "{prompt}"]

[[rules]]
name = "text"
when = 'type == "text"'
emit = "text"
"#
    }

    #[test]
    fn test_version_store_creation() {
        let dir = TempDir::new().unwrap();
        let store = NormalizerVersionStore::new(dir.path());
        assert!(store.list_versions("testprovider").is_empty());
    }

    #[test]
    fn test_backup_and_list() {
        let dir = TempDir::new().unwrap();
        let store = NormalizerVersionStore::new(dir.path());
        let version_id = store.backup("testprovider", sample_toml()).unwrap();
        assert!(!version_id.is_empty());
        let versions = store.list_versions("testprovider");
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].id, version_id);
    }

    #[test]
    fn test_restore_version() {
        let dir = TempDir::new().unwrap();
        let store = NormalizerVersionStore::new(dir.path());
        let v1 = store.backup("testprovider", sample_toml()).unwrap();
        let modified = sample_toml().replace("testprovider", "modified");
        let _v2 = store.backup("testprovider", &modified).unwrap();

        let restored = store.restore("testprovider", &v1).unwrap();
        assert!(restored.contains("testprovider"));
        assert!(!restored.contains("modified"));
    }

    #[test]
    fn test_restore_nonexistent_fails() {
        let dir = TempDir::new().unwrap();
        let store = NormalizerVersionStore::new(dir.path());
        let result = store.restore("testprovider", "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_current_returns_latest() {
        let dir = TempDir::new().unwrap();
        let store = NormalizerVersionStore::new(dir.path());
        let _v1 = store.backup("testprovider", "v1 content").unwrap();
        let v2 = store.backup("testprovider", "v2 content").unwrap();
        let current = store.current("testprovider").unwrap();
        assert_eq!(current.id, v2);
    }
}
