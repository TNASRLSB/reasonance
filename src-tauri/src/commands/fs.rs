use crate::fs_watcher::FsWatcherState;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, State};

// ── Project root state ────────────────────────────────────────────────────────

/// Holds the canonicalized project root path set by the frontend on folder open.
pub struct ProjectRootState(pub Mutex<Option<PathBuf>>);

impl ProjectRootState {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }
}

/// Set (or clear) the project root. Called by the frontend whenever a folder is opened.
#[tauri::command]
pub fn set_project_root(path: String, state: State<'_, ProjectRootState>) -> Result<(), String> {
    let canonical = if path.is_empty() {
        None
    } else {
        Some(
            std::fs::canonicalize(&path)
                .map_err(|e| format!("Cannot canonicalize project root '{}': {}", path, e))?,
        )
    };
    *state.0.lock().unwrap() = canonical;
    Ok(())
}

// ── Path validation helpers ───────────────────────────────────────────────────

/// Returns the user config directory for Reasonance (used for reading config files).
fn reasonance_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("reasonance"))
}

/// Validate that `path` is safe for reading:
/// - Inside the project root, OR
/// - Inside the user's Reasonance config directory.
/// If no project root is set yet, only config-dir reads are allowed.
fn validate_read_path(path: &Path, state: &ProjectRootState) -> Result<(), String> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|e| format!("Cannot resolve path '{}': {}", path.display(), e))?;

    // Allow reads from Reasonance config dir (e.g. llms.toml)
    if let Some(config_dir) = reasonance_config_dir() {
        if canonical.starts_with(&config_dir) {
            return Ok(());
        }
    }

    let root_lock = state.0.lock().unwrap();
    if let Some(root) = root_lock.as_ref() {
        if canonical.starts_with(root) {
            return Ok(());
        }
        return Err(format!(
            "Access denied: '{}' is outside the project root",
            path.display()
        ));
    }

    // No project root set yet — only config dir was allowed (already checked above)
    Err(format!(
        "Access denied: no project root is set and '{}' is not in the config directory",
        path.display()
    ))
}

/// Validate that `path` is safe for writing:
/// - Must be inside the project root.
fn validate_write_path(path: &Path, state: &ProjectRootState) -> Result<(), String> {
    // For write we require the parent to exist to canonicalize;
    // if the file itself doesn't exist yet, canonicalize the parent.
    let canonical = if path.exists() {
        std::fs::canonicalize(path)
            .map_err(|e| format!("Cannot resolve path '{}': {}", path.display(), e))?
    } else {
        let parent = path
            .parent()
            .ok_or_else(|| format!("No parent directory for '{}'", path.display()))?;
        let canon_parent = std::fs::canonicalize(parent)
            .map_err(|e| format!("Cannot resolve parent '{}': {}", parent.display(), e))?;
        canon_parent.join(path.file_name().unwrap_or_default())
    };

    let root_lock = state.0.lock().unwrap();
    if let Some(root) = root_lock.as_ref() {
        if canonical.starts_with(root) {
            return Ok(());
        }
        return Err(format!(
            "Access denied: '{}' is outside the project root",
            path.display()
        ));
    }

    Err("Access denied: no project root is set".to_string())
}

// ── File commands ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
    pub is_gitignored: bool,
}

#[tauri::command]
pub fn read_file(path: String, state: State<'_, ProjectRootState>) -> Result<String, String> {
    validate_read_path(Path::new(&path), &state)?;
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_file(
    path: String,
    content: String,
    state: State<'_, ProjectRootState>,
) -> Result<(), String> {
    validate_write_path(Path::new(&path), &state)?;
    fs::write(&path, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_dir(path: String, respect_gitignore: bool) -> Result<Vec<FileEntry>, String> {
    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;

    let gitignore = if respect_gitignore {
        ignore::gitignore::Gitignore::new(Path::new(&path).join(".gitignore")).0
    } else {
        ignore::gitignore::Gitignore::empty()
    };

    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = entry.metadata().map_err(|e| e.to_string())?;

        let is_ignored = if respect_gitignore {
            let matched = gitignore.matched_path_or_any_parents(&entry.path(), metadata.is_dir());
            matched.is_ignore()
        } else {
            false
        };

        let modified = metadata
            .modified()
            .map_err(|e| e.to_string())?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        result.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified,
            is_gitignored: is_ignored,
        });
    }
    result.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(result)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrepResult {
    pub path: String,
    pub line_number: usize,
    pub line: String,
}

#[tauri::command]
pub fn grep_files(
    path: String,
    pattern: String,
    respect_gitignore: bool,
) -> Result<Vec<GrepResult>, String> {
    use ignore::WalkBuilder;
    use std::io::BufRead;

    let mut results = Vec::new();
    let walker = WalkBuilder::new(&path)
        .git_ignore(respect_gitignore)
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }
        let file_path = entry.path().to_owned();
        if let Ok(file) = std::fs::File::open(&file_path) {
            let reader = std::io::BufReader::new(file);
            for (i, line_result) in reader.lines().enumerate() {
                if let Ok(line) = line_result {
                    if line.contains(&pattern) {
                        results.push(GrepResult {
                            path: file_path.to_string_lossy().to_string(),
                            line_number: i + 1,
                            line,
                        });
                        if results.len() >= 500 {
                            return Ok(results);
                        }
                    }
                }
            }
        }
    }
    Ok(results)
}

#[tauri::command]
pub fn start_watching(
    path: String,
    app: AppHandle,
    state: State<'_, FsWatcherState>,
) -> Result<(), String> {
    crate::fs_watcher::start_watching(&path, app, &state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_temp_dir() -> TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    fn make_root_state(root: Option<&Path>) -> ProjectRootState {
        let state = ProjectRootState::new();
        if let Some(r) = root {
            *state.0.lock().unwrap() = Some(std::fs::canonicalize(r).unwrap());
        }
        state
    }

    // ── Path validation tests ─────────────────────────────────────────────

    #[test]
    fn validate_read_allows_file_in_project_root() {
        let dir = setup_temp_dir();
        let file = dir.path().join("hello.txt");
        std::fs::write(&file, "hello").unwrap();
        let state = make_root_state(Some(dir.path()));
        assert!(validate_read_path(&file, &state).is_ok());
    }

    #[test]
    fn validate_read_rejects_file_outside_project_root() {
        let dir = setup_temp_dir();
        let other = setup_temp_dir();
        let file = other.path().join("secret.txt");
        std::fs::write(&file, "secret").unwrap();
        let state = make_root_state(Some(dir.path()));
        assert!(validate_read_path(&file, &state).is_err());
    }

    #[test]
    fn validate_read_rejects_when_no_root_set() {
        let dir = setup_temp_dir();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, "data").unwrap();
        let state = make_root_state(None);
        assert!(validate_read_path(&file, &state).is_err());
    }

    #[test]
    fn validate_write_allows_file_in_project_root() {
        let dir = setup_temp_dir();
        let file = dir.path().join("output.txt");
        let state = make_root_state(Some(dir.path()));
        assert!(validate_write_path(&file, &state).is_ok());
    }

    #[test]
    fn validate_write_rejects_file_outside_project_root() {
        let dir = setup_temp_dir();
        let other = setup_temp_dir();
        let file = other.path().join("danger.txt");
        std::fs::write(&file, "x").unwrap();
        let state = make_root_state(Some(dir.path()));
        assert!(validate_write_path(&file, &state).is_err());
    }

    #[test]
    fn validate_write_rejects_when_no_root_set() {
        let dir = setup_temp_dir();
        let file = dir.path().join("file.txt");
        let state = make_root_state(None);
        assert!(validate_write_path(&file, &state).is_err());
    }

    // ── Existing functional tests (adapted to provide state) ──────────────

    #[test]
    fn read_file_existing() {
        let dir = setup_temp_dir();
        let file_path = dir.path().join("hello.txt");
        std::fs::write(&file_path, "hello world").unwrap();
        let state = make_root_state(Some(dir.path()));

        // Directly call the underlying logic to avoid needing Tauri State<>
        validate_read_path(&file_path, &state).unwrap();
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn write_file_creates_file() {
        let dir = setup_temp_dir();
        let file_path = dir.path().join("output.txt");
        let state = make_root_state(Some(dir.path()));

        validate_write_path(&file_path, &state).unwrap();
        fs::write(&file_path, "written content").unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "written content");
    }

    #[test]
    fn write_file_overwrites_existing() {
        let dir = setup_temp_dir();
        let file_path = dir.path().join("overwrite.txt");
        std::fs::write(&file_path, "original").unwrap();
        let state = make_root_state(Some(dir.path()));

        validate_write_path(&file_path, &state).unwrap();
        fs::write(&file_path, "new content").unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn list_dir_returns_entries() {
        let dir = setup_temp_dir();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let entries = list_dir(dir.path().to_string_lossy().to_string(), false).unwrap();
        assert_eq!(entries.len(), 3);

        // Dirs should come before files
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "subdir");
    }

    #[test]
    fn list_dir_sorts_dirs_first_then_alphabetical() {
        let dir = setup_temp_dir();
        std::fs::write(dir.path().join("z.txt"), "z").unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::create_dir(dir.path().join("mydir")).unwrap();

        let entries = list_dir(dir.path().to_string_lossy().to_string(), false).unwrap();
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "a.txt");
        assert_eq!(entries[2].name, "z.txt");
    }

    #[test]
    fn list_dir_nonexistent_returns_err() {
        let result = list_dir("/nonexistent/dir/path".to_string(), false);
        assert!(result.is_err());
    }

    #[test]
    fn grep_files_finds_matches() {
        let dir = setup_temp_dir();
        std::fs::write(dir.path().join("file1.txt"), "hello world\nfoo bar\n").unwrap();
        std::fs::write(dir.path().join("file2.txt"), "no match here\n").unwrap();

        let results = grep_files(
            dir.path().to_string_lossy().to_string(),
            "hello".to_string(),
            false,
        )
        .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line_number, 1);
        assert!(results[0].line.contains("hello world"));
    }

    #[test]
    fn grep_files_no_match_returns_empty() {
        let dir = setup_temp_dir();
        std::fs::write(dir.path().join("file.txt"), "nothing to find here\n").unwrap();

        let results = grep_files(
            dir.path().to_string_lossy().to_string(),
            "ZZZNOMATCH".to_string(),
            false,
        )
        .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn file_entry_fields_are_populated() {
        let dir = setup_temp_dir();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content here").unwrap();

        let entries = list_dir(dir.path().to_string_lossy().to_string(), false).unwrap();
        let entry = entries.iter().find(|e| e.name == "test.txt").unwrap();

        assert!(!entry.is_dir);
        assert_eq!(entry.size, 12); // "content here" = 12 bytes
        assert!(entry.modified > 0);
        assert!(!entry.is_gitignored);
    }
}
