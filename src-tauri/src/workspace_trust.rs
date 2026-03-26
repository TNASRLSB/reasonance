use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Trusted,
    ReadOnly,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEntry {
    pub hash: String,
    pub path: String,
    pub level: TrustLevel,
    pub trusted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderInfo {
    pub name: String,
    pub path: String,
    pub has_git: bool,
    pub file_count: usize,
}

#[derive(Debug, Serialize)]
pub struct TrustCheckResult {
    pub level: Option<TrustLevel>,
    pub needs_prompt: bool,
    pub folder_info: Option<FolderInfo>,
    pub rename_hint: Option<String>,
}

/// Directories that are too broad to trust.
const BLOCKED_PARENTS: &[&str] = &["/", "/tmp", "/usr", "/var"];

pub struct TrustStore {
    store_path: PathBuf,
    entries: Mutex<Vec<TrustEntry>>,
}

impl TrustStore {
    pub fn new(store_path: PathBuf) -> Self {
        let entries = match fs::read_to_string(&store_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        Self {
            store_path,
            entries: Mutex::new(entries),
        }
    }

    pub fn hash_path(canonical: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn is_blocked_broad_dir(path: &str) -> bool {
        let p = path.trim_end_matches('/');
        if BLOCKED_PARENTS.iter().any(|b| p == b.trim_end_matches('/')) {
            return true;
        }
        if let Some(home) = dirs::home_dir() {
            if p == home.to_string_lossy().trim_end_matches('/') {
                return true;
            }
        }
        false
    }

    pub fn check_trust(&self, path: &str) -> TrustCheckResult {
        self.check_trust_with_expiration(path, None)
    }

    pub fn check_trust_with_expiration(&self, path: &str, expiration_days: Option<u64>) -> TrustCheckResult {
        let canonical = match fs::canonicalize(path) {
            Ok(p) => p,
            Err(_) => {
                return TrustCheckResult {
                    level: None,
                    needs_prompt: true,
                    folder_info: Self::folder_info(path).ok(),
                    rename_hint: None,
                };
            }
        };
        let canonical_str = canonical.to_string_lossy().to_string();
        let basename = Path::new(path).file_name()
            .map(|n| n.to_string_lossy().to_string());

        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let hash = Self::hash_path(&canonical_str);
        if let Some(entry) = entries.iter().find(|e| e.hash == hash) {
            if let Some(days) = expiration_days {
                if Self::is_expired(&entry.trusted_at, days) {
                    return TrustCheckResult {
                        level: None,
                        needs_prompt: true,
                        folder_info: Self::folder_info(path).ok(),
                        rename_hint: None,
                    };
                }
            }
            return TrustCheckResult {
                level: Some(entry.level),
                needs_prompt: false,
                folder_info: None,
                rename_hint: None,
            };
        }

        let mut current = canonical.as_path();
        while let Some(parent) = current.parent() {
            let parent_str = parent.to_string_lossy().to_string();
            let parent_hash = Self::hash_path(&parent_str);
            if let Some(entry) = entries.iter().find(|e| e.hash == parent_hash) {
                if let Some(days) = expiration_days {
                    if Self::is_expired(&entry.trusted_at, days) {
                        break;
                    }
                }
                return TrustCheckResult {
                    level: Some(entry.level),
                    needs_prompt: false,
                    folder_info: None,
                    rename_hint: None,
                };
            }
            if parent == Path::new("/") || parent == Path::new("") {
                break;
            }
            current = parent;
        }

        let rename_hint = basename.and_then(|bn| {
            entries.iter().find(|e| {
                let entry_basename = Path::new(&e.path).file_name()
                    .map(|n| n.to_string_lossy().to_string());
                entry_basename.as_deref() == Some(&bn) && e.path != canonical_str
            }).map(|e| format!("A folder named '{}' was previously trusted at {}", bn, e.path))
        });

        TrustCheckResult {
            level: None,
            needs_prompt: true,
            folder_info: Self::folder_info(path).ok(),
            rename_hint,
        }
    }

    fn is_expired(trusted_at: &str, expiration_days: u64) -> bool {
        match chrono::DateTime::parse_from_rfc3339(trusted_at) {
            Ok(dt) => {
                let now = chrono::Utc::now();
                let elapsed = now.signed_duration_since(dt);
                elapsed.num_days() > expiration_days as i64
            }
            Err(_) => false,
        }
    }

    pub fn set_trust(&self, path: &str, level: TrustLevel) -> Result<(), crate::error::ReasonanceError> {
        let canonical = fs::canonicalize(path)
            .map_err(|e| crate::error::ReasonanceError::io(format!("canonicalize '{}'", path), e))?;
        let canonical_str = canonical.to_string_lossy().to_string();

        if Self::is_blocked_broad_dir(&canonical_str) {
            return Err(crate::error::ReasonanceError::Security {
                message: "This directory is too broad to trust. Please trust specific project folders instead.".to_string(),
                code: crate::error::SecurityErrorCode::BlockedWorkspace,
            });
        }

        let hash = Self::hash_path(&canonical_str);
        let now = chrono::Utc::now().to_rfc3339();

        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = entries.iter_mut().find(|e| e.hash == hash) {
            existing.level = level;
            existing.trusted_at = now;
        } else {
            entries.push(TrustEntry {
                hash,
                path: canonical_str,
                level,
                trusted_at: now,
            });
        }

        self.save(&entries)
    }

    pub fn revoke_trust(&self, hash: &str) -> Result<(), crate::error::ReasonanceError> {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.retain(|e| e.hash != hash);
        self.save(&entries)
    }

    pub fn list_trusted(&self) -> Vec<TrustEntry> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn folder_info(path: &str) -> Result<FolderInfo, String> {
        let p = Path::new(path);
        let name = p.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        let has_git = p.join(".git").exists();
        let file_count = fs::read_dir(p)
            .map(|entries| entries.count())
            .unwrap_or(0);
        Ok(FolderInfo {
            name,
            path: path.to_string(),
            has_git,
            file_count,
        })
    }

    fn save(&self, entries: &[TrustEntry]) -> Result<(), crate::error::ReasonanceError> {
        let json = serde_json::to_string_pretty(entries)?;
        if let Some(parent) = self.store_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&self.store_path, json)
            .map_err(|e| crate::error::ReasonanceError::io("write trust store", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_store(dir: &Path) -> TrustStore {
        let store_path = dir.join("trusted-workspaces.json");
        TrustStore::new(store_path)
    }

    #[test]
    fn test_hash_is_deterministic() {
        let h1 = TrustStore::hash_path("/home/user/project");
        let h2 = TrustStore::hash_path("/home/user/project");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_differs_for_different_paths() {
        let h1 = TrustStore::hash_path("/home/user/project-a");
        let h2 = TrustStore::hash_path("/home/user/project-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_empty_store_returns_needs_prompt() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(tmp.path());
        let result = store.check_trust(tmp.path().to_str().unwrap());
        assert!(result.needs_prompt);
        assert!(result.level.is_none());
    }

    #[test]
    fn test_set_and_check_trust() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(tmp.path());
        let project = tmp.path().join("my-project");
        fs::create_dir(&project).unwrap();

        store.set_trust(project.to_str().unwrap(), TrustLevel::Trusted).unwrap();

        let result = store.check_trust(project.to_str().unwrap());
        assert!(!result.needs_prompt);
        assert_eq!(result.level, Some(TrustLevel::Trusted));
    }

    #[test]
    fn test_revoke_trust() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(tmp.path());
        let project = tmp.path().join("my-project");
        fs::create_dir(&project).unwrap();

        store.set_trust(project.to_str().unwrap(), TrustLevel::Trusted).unwrap();
        let hash = TrustStore::hash_path(&fs::canonicalize(&project).unwrap().to_string_lossy());
        store.revoke_trust(&hash).unwrap();

        let result = store.check_trust(project.to_str().unwrap());
        assert!(result.needs_prompt);
    }

    #[test]
    fn test_parent_inheritance() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(tmp.path());
        let parent = tmp.path().join("workspace");
        let child = parent.join("sub-project");
        fs::create_dir_all(&child).unwrap();

        store.set_trust(parent.to_str().unwrap(), TrustLevel::Trusted).unwrap();

        let result = store.check_trust(child.to_str().unwrap());
        assert!(!result.needs_prompt);
        assert_eq!(result.level, Some(TrustLevel::Trusted));
    }

    #[test]
    fn test_blocked_broad_directories() {
        assert!(TrustStore::is_blocked_broad_dir("/"));
        assert!(TrustStore::is_blocked_broad_dir("/tmp"));
        assert!(TrustStore::is_blocked_broad_dir("/usr"));
        assert!(!TrustStore::is_blocked_broad_dir("/home/user/project"));
    }

    #[test]
    fn test_persistence_across_instances() {
        let tmp = TempDir::new().unwrap();
        let store_path = tmp.path().join("trusted-workspaces.json");
        let project = tmp.path().join("my-project");
        fs::create_dir(&project).unwrap();

        {
            let store = TrustStore::new(store_path.clone());
            store.set_trust(project.to_str().unwrap(), TrustLevel::ReadOnly).unwrap();
        }

        let store2 = TrustStore::new(store_path);
        let result = store2.check_trust(project.to_str().unwrap());
        assert_eq!(result.level, Some(TrustLevel::ReadOnly));
    }

    #[test]
    fn test_corrupted_store_degrades_gracefully() {
        let tmp = TempDir::new().unwrap();
        let store_path = tmp.path().join("trusted-workspaces.json");
        fs::write(&store_path, "NOT VALID JSON").unwrap();

        let store = TrustStore::new(store_path);
        let result = store.check_trust("/some/path");
        assert!(result.needs_prompt);
    }

    #[test]
    fn test_folder_info() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("my-project");
        fs::create_dir(&project).unwrap();
        fs::write(project.join("file1.txt"), "").unwrap();
        fs::write(project.join("file2.txt"), "").unwrap();
        fs::create_dir(project.join(".git")).unwrap();

        let info = TrustStore::folder_info(project.to_str().unwrap()).unwrap();
        assert_eq!(info.name, "my-project");
        assert!(info.has_git);
        assert!(info.file_count >= 3);
    }
}
