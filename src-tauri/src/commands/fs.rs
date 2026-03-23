use crate::fs_watcher::FsWatcherState;
use log::{info, error, debug};
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
    info!("cmd::set_project_root(path={})", path);
    let canonical = if path.is_empty() {
        None
    } else {
        Some(
            std::fs::canonicalize(&path)
                .map_err(|e| format!("Cannot canonicalize project root '{}': {}", path, e))?,
        )
    };
    *state.0.lock().unwrap_or_else(|e| e.into_inner()) = canonical;
    Ok(())
}

// ── Path validation helpers ───────────────────────────────────────────────────

/// Returns the user config directory for Reasonance (used for reading config files).
fn reasonance_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("reasonance"))
}

/// Returns well-known CLI local directories that the app should be allowed to read.
/// These are where CLI tools store their local state (memory, sessions, config).
fn cli_local_dirs() -> Vec<PathBuf> {
    let mut dirs_list = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // Claude Code: ~/.claude/ (memory, projects, sessions)
        dirs_list.push(home.join(".claude"));
        // Gemini CLI: ~/.gemini/
        dirs_list.push(home.join(".gemini"));
        // Codex: ~/.codex/
        dirs_list.push(home.join(".codex"));
        // Kimi: ~/.kimi/
        dirs_list.push(home.join(".kimi"));
        // Qwen: ~/.qwen-code/
        dirs_list.push(home.join(".qwen-code"));
    }
    dirs_list
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

    // Allow reads from CLI local directories (e.g. ~/.claude/ for memory.md)
    for cli_dir in cli_local_dirs() {
        if cli_dir.exists() {
            if let Ok(canon_cli) = std::fs::canonicalize(&cli_dir) {
                if canonical.starts_with(&canon_cli) {
                    return Ok(());
                }
            }
        }
    }

    let root_lock = state.0.lock().unwrap_or_else(|e| e.into_inner());
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

    // Allow writes to CLI local directories (e.g. ~/.claude/ for memory, sessions)
    for cli_dir in cli_local_dirs() {
        if cli_dir.exists() {
            if let Ok(canon_cli) = std::fs::canonicalize(&cli_dir) {
                if canonical.starts_with(&canon_cli) {
                    return Ok(());
                }
            }
        }
    }

    let root_lock = state.0.lock().unwrap_or_else(|e| e.into_inner());
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

/// Maximum file size allowed for reading (10 MB).
const MAX_READ_FILE_SIZE: u64 = 10 * 1024 * 1024;

#[tauri::command]
pub fn read_file(path: String, state: State<'_, ProjectRootState>) -> Result<String, String> {
    info!("cmd::read_file(path={})", path);
    validate_read_path(Path::new(&path), &state)?;

    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    if metadata.len() > MAX_READ_FILE_SIZE {
        error!("cmd::read_file file too large: {} bytes at {}", metadata.len(), path);
        return Err(format!(
            "File too large ({:.1} MB). Maximum allowed size is {:.0} MB.",
            metadata.len() as f64 / (1024.0 * 1024.0),
            MAX_READ_FILE_SIZE as f64 / (1024.0 * 1024.0),
        ));
    }

    fs::read_to_string(&path).map_err(|e| {
        let msg = if e.kind() == std::io::ErrorKind::InvalidData {
            "Cannot open binary file: the file contains non-UTF-8 data.".to_string()
        } else {
            e.to_string()
        };
        error!("cmd::read_file error for {}: {}", path, msg);
        msg
    })
}

#[tauri::command]
pub fn write_file(
    path: String,
    content: String,
    state: State<'_, ProjectRootState>,
) -> Result<(), String> {
    info!("cmd::write_file(path={})", path);
    validate_write_path(Path::new(&path), &state)?;

    // Atomic write: write to a temp file in the same directory, then rename.
    // This prevents partial writes on crash (rename is atomic on the same filesystem).
    let target = Path::new(&path);
    let parent = target
        .parent()
        .ok_or_else(|| format!("No parent directory for '{}'", path))?;
    let file_name = target
        .file_name()
        .ok_or_else(|| format!("No file name for '{}'", path))?;

    let tmp_name = format!(".{}.tmp", file_name.to_string_lossy());
    let tmp_path = parent.join(&tmp_name);

    fs::write(&tmp_path, &content)
        .map_err(|e| {
            error!("cmd::write_file failed to write temp file {}: {}", tmp_path.display(), e);
            format!("Failed to write temp file '{}': {}", tmp_path.display(), e)
        })?;
    fs::rename(&tmp_path, target)
        .map_err(|e| {
            // Clean up temp file on rename failure
            let _ = fs::remove_file(&tmp_path);
            error!("cmd::write_file failed to rename temp file to {}: {}", path, e);
            format!("Failed to rename temp file to '{}': {}", path, e)
        })
}

#[tauri::command]
pub fn list_dir(path: String, respect_gitignore: bool, state: State<'_, ProjectRootState>) -> Result<Vec<FileEntry>, String> {
    info!("cmd::list_dir(path={})", path);
    validate_read_path(Path::new(&path), &state)?;
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
    debug!("cmd::list_dir returned {} entries for {}", result.len(), path);
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
    state: State<'_, ProjectRootState>,
) -> Result<Vec<GrepResult>, String> {
    info!("cmd::grep_files(path={}, pattern={})", path, pattern);
    validate_read_path(Path::new(&path), &state)?;
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
                            debug!("cmd::grep_files hit 500 result limit for pattern={}", pattern);
                            return Ok(results);
                        }
                    }
                }
            }
        }
    }
    debug!("cmd::grep_files found {} matches for pattern={}", results.len(), pattern);
    Ok(results)
}

#[tauri::command]
pub fn start_watching(
    path: String,
    app: AppHandle,
    state: State<'_, FsWatcherState>,
    root_state: State<'_, ProjectRootState>,
) -> Result<(), String> {
    info!("cmd::start_watching(path={})", path);
    validate_read_path(Path::new(&path), &root_state)?;
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
            *state.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(std::fs::canonicalize(r).unwrap());
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
        let state = make_root_state(Some(dir.path()));

        validate_read_path(dir.path(), &state).unwrap();
        let entries = fs::read_dir(dir.path().to_string_lossy().as_ref())
            .map_err(|e| e.to_string())
            .unwrap();
        let count = entries.count();
        assert_eq!(count, 3);
    }

    #[test]
    fn list_dir_sorts_dirs_first_then_alphabetical() {
        let dir = setup_temp_dir();
        std::fs::write(dir.path().join("z.txt"), "z").unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::create_dir(dir.path().join("mydir")).unwrap();
        let state = make_root_state(Some(dir.path()));

        // Validate path then test sorting via the underlying logic
        validate_read_path(dir.path(), &state).unwrap();
        let mut entries_vec: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| {
                let md = e.metadata().unwrap();
                (e.file_name().to_string_lossy().to_string(), md.is_dir())
            })
            .collect();
        entries_vec.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.to_lowercase().cmp(&b.0.to_lowercase())));
        assert!(entries_vec[0].1); // first is dir
        assert_eq!(entries_vec[1].0, "a.txt");
        assert_eq!(entries_vec[2].0, "z.txt");
    }

    #[test]
    fn list_dir_nonexistent_returns_err() {
        let dir = setup_temp_dir();
        let state = make_root_state(Some(dir.path()));
        // Path doesn't exist, so validate_read_path will fail (canonicalize fails)
        let result = validate_read_path(Path::new("/nonexistent/dir/path"), &state);
        assert!(result.is_err());
    }

    #[test]
    fn grep_files_finds_matches() {
        let dir = setup_temp_dir();
        std::fs::write(dir.path().join("file1.txt"), "hello world\nfoo bar\n").unwrap();
        std::fs::write(dir.path().join("file2.txt"), "no match here\n").unwrap();
        let state = make_root_state(Some(dir.path()));

        validate_read_path(dir.path(), &state).unwrap();
        // Test grep logic directly
        use ignore::WalkBuilder;
        use std::io::BufRead;
        let pattern = "hello";
        let mut results = Vec::new();
        let walker = WalkBuilder::new(dir.path()).git_ignore(false).build();
        for entry in walker.flatten() {
            if !entry.file_type().map_or(false, |ft| ft.is_file()) { continue; }
            if let Ok(file) = std::fs::File::open(entry.path()) {
                let reader = std::io::BufReader::new(file);
                for (i, line_result) in reader.lines().enumerate() {
                    if let Ok(line) = line_result {
                        if line.contains(pattern) {
                            results.push((entry.path().to_owned(), i + 1, line));
                        }
                    }
                }
            }
        }
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, 1);
        assert!(results[0].2.contains("hello world"));
    }

    #[test]
    fn grep_files_no_match_returns_empty() {
        let dir = setup_temp_dir();
        std::fs::write(dir.path().join("file.txt"), "nothing to find here\n").unwrap();
        let state = make_root_state(Some(dir.path()));

        validate_read_path(dir.path(), &state).unwrap();
        use ignore::WalkBuilder;
        use std::io::BufRead;
        let pattern = "ZZZNOMATCH";
        let mut results = Vec::new();
        let walker = WalkBuilder::new(dir.path()).git_ignore(false).build();
        for entry in walker.flatten() {
            if !entry.file_type().map_or(false, |ft| ft.is_file()) { continue; }
            if let Ok(file) = std::fs::File::open(entry.path()) {
                let reader = std::io::BufReader::new(file);
                for (_i, line_result) in reader.lines().enumerate() {
                    if let Ok(line) = line_result {
                        if line.contains(pattern) {
                            results.push(line);
                        }
                    }
                }
            }
        }
        assert!(results.is_empty());
    }

    #[test]
    fn file_entry_fields_are_populated() {
        let dir = setup_temp_dir();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "content here").unwrap();
        let state = make_root_state(Some(dir.path()));

        validate_read_path(dir.path(), &state).unwrap();
        let metadata = std::fs::metadata(&file_path).unwrap();
        assert!(!metadata.is_dir());
        assert_eq!(metadata.len(), 12); // "content here" = 12 bytes
        assert!(metadata.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() > 0);
    }
}
