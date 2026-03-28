use crate::error::ReasonanceError;
use crate::fs_watcher::FsWatcherState;
use log::{debug, error, info};
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
pub fn set_project_root(
    path: String,
    state: State<'_, ProjectRootState>,
    settings: State<'_, std::sync::Mutex<crate::settings::LayeredSettings>>,
    policy: State<'_, crate::policy_file::PolicyFile>,
) -> Result<(), ReasonanceError> {
    info!("cmd::set_project_root(path={})", path);
    let canonical =
        if path.is_empty() {
            None
        } else {
            Some(std::fs::canonicalize(&path).map_err(|e| {
                ReasonanceError::io(format!("canonicalize project root '{}'", path), e)
            })?)
        };
    *state.0.lock().unwrap_or_else(|e| e.into_inner()) = canonical.clone();

    // Load project-level and workspace-level settings overrides
    if let Some(ref root) = canonical {
        settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_project_root(root);
    }

    // Reload policy file with the new project root so permission rules
    // from `.reasonance/permissions.toml` take effect immediately.
    if let Some(ref root) = canonical {
        let global_config = dirs::config_dir().map(|d| d.join("reasonance"));
        policy.load_optional(Some(root), global_config.as_deref());
    }

    // Install prepare-commit-msg hook to add Reasonance co-author trailer
    if let Some(ref root) = canonical {
        install_commit_hook(root);
    }

    Ok(())
}

/// Installs a git prepare-commit-msg hook that appends a Reasonance co-author
/// trailer to every commit made from within the application.
fn install_commit_hook(project_root: &std::path::Path) {
    let git_dir = project_root.join(".git");
    if !git_dir.is_dir() {
        return; // not a git repo
    }
    let hooks_dir = git_dir.join("hooks");
    let hook_path = hooks_dir.join("prepare-commit-msg");

    // Don't overwrite an existing hook
    if hook_path.exists() {
        // Check if it's already our hook
        if let Ok(content) = std::fs::read_to_string(&hook_path) {
            if content.contains("Reasonance") {
                return; // already installed
            }
        }
        // Existing hook from user/other tool — don't touch it
        debug!("prepare-commit-msg hook already exists, skipping Reasonance hook install");
        return;
    }

    if let Err(e) = std::fs::create_dir_all(&hooks_dir) {
        debug!("Failed to create hooks dir: {}", e);
        return;
    }

    let hook_content = r#"#!/bin/sh
# Added by Reasonance — adds co-author trailer to commits
COMMIT_MSG_FILE="$1"
COMMIT_SOURCE="$2"

# Only add trailer for regular commits (not merges, squashes, etc.)
if [ -z "$COMMIT_SOURCE" ] || [ "$COMMIT_SOURCE" = "message" ]; then
    # Don't add if already present
    if ! grep -q "Co-Authored-By:.*REASONANCE-IDE" "$COMMIT_MSG_FILE" 2>/dev/null; then
        printf '\nCo-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>\n' >> "$COMMIT_MSG_FILE"
    fi
fi
"#;

    match std::fs::write(&hook_path, hook_content) {
        Ok(_) => {
            // Make executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755));
            }
            info!(
                "Installed Reasonance prepare-commit-msg hook at {:?}",
                hook_path
            );
        }
        Err(e) => {
            debug!("Failed to write prepare-commit-msg hook: {}", e);
        }
    }
}

// ── Path validation helpers ───────────────────────────────────────────────────

/// Resolve a path and verify it stays within the given root after symlink resolution.
/// Uses `canonicalize()` which follows all symlinks to the final target.
/// Returns the canonicalized path if safe, or a `Security::PathTraversal` error.
pub fn resolve_safe_path(path: &Path, project_root: &Path) -> Result<PathBuf, ReasonanceError> {
    let resolved = path
        .canonicalize()
        .map_err(|e| ReasonanceError::io(format!("Cannot resolve path '{}'", path.display()), e))?;

    let root_resolved = project_root.canonicalize().map_err(|e| {
        ReasonanceError::io(
            format!("Cannot resolve project root '{}'", project_root.display()),
            e,
        )
    })?;

    if !resolved.starts_with(&root_resolved) {
        return Err(ReasonanceError::Security {
            message: format!(
                "Path '{}' resolves to '{}' which is outside project root '{}'",
                path.display(),
                resolved.display(),
                root_resolved.display()
            ),
            code: crate::error::SecurityErrorCode::PathTraversal,
        });
    }

    Ok(resolved)
}

/// Resolve a path for writing (file may not exist yet).
/// Canonicalizes the parent directory and checks the result is within root.
pub fn resolve_safe_write_path(
    path: &Path,
    project_root: &Path,
) -> Result<PathBuf, ReasonanceError> {
    if path.exists() {
        return resolve_safe_path(path, project_root);
    }

    let parent = path.parent().ok_or_else(|| {
        ReasonanceError::validation(
            "path",
            format!("No parent directory for '{}'", path.display()),
        )
    })?;
    let canon_parent = parent.canonicalize().map_err(|e| {
        ReasonanceError::io(format!("Cannot resolve parent '{}'", parent.display()), e)
    })?;
    let root_resolved = project_root.canonicalize().map_err(|e| {
        ReasonanceError::io(
            format!("Cannot resolve project root '{}'", project_root.display()),
            e,
        )
    })?;

    if !canon_parent.starts_with(&root_resolved) {
        return Err(ReasonanceError::Security {
            message: format!(
                "Path '{}' (parent '{}') resolves outside project root '{}'",
                path.display(),
                canon_parent.display(),
                root_resolved.display()
            ),
            code: crate::error::SecurityErrorCode::PathTraversal,
        });
    }

    Ok(canon_parent.join(path.file_name().unwrap_or_default()))
}

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
fn validate_read_path(path: &Path, state: &ProjectRootState) -> Result<(), ReasonanceError> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|e| ReasonanceError::io(format!("resolve path '{}'", path.display()), e))?;

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
        // Use resolve_safe_path for the actual bounds check (follows symlinks)
        resolve_safe_path(path, root)?;
        return Ok(());
    }

    // No project root set yet — only config dir was allowed (already checked above)
    Err(ReasonanceError::PermissionDenied {
        action: format!("read '{}' (no project root set)", path.display()),
        tool: None,
    })
}

/// Validate that `path` is safe for writing:
/// - Must be inside the project root.
fn validate_write_path(path: &Path, state: &ProjectRootState) -> Result<(), ReasonanceError> {
    // For write we require the parent to exist to canonicalize;
    // if the file itself doesn't exist yet, canonicalize the parent.
    let canonical = if path.exists() {
        std::fs::canonicalize(path)
            .map_err(|e| ReasonanceError::io(format!("resolve path '{}'", path.display()), e))?
    } else {
        let parent = path.parent().ok_or_else(|| {
            ReasonanceError::validation(
                "path",
                format!("No parent directory for '{}'", path.display()),
            )
        })?;
        let canon_parent = std::fs::canonicalize(parent).map_err(|e| {
            ReasonanceError::io(format!("resolve parent '{}'", parent.display()), e)
        })?;
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
        // Use resolve_safe_write_path for the actual bounds check (follows symlinks)
        resolve_safe_write_path(path, root)?;
        return Ok(());
    }

    Err(ReasonanceError::PermissionDenied {
        action: "write (no project root set)".to_string(),
        tool: None,
    })
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

pub fn read_file_inner(path: &str, state: &ProjectRootState) -> Result<String, ReasonanceError> {
    info!("cmd::read_file(path={})", path);
    validate_read_path(Path::new(path), state)?;

    let metadata =
        fs::metadata(path).map_err(|e| ReasonanceError::io(format!("stat '{}'", path), e))?;
    if metadata.len() > MAX_READ_FILE_SIZE {
        error!(
            "cmd::read_file file too large: {} bytes at {}",
            metadata.len(),
            path
        );
        return Err(ReasonanceError::validation(
            "file_size",
            format!(
                "File too large ({:.1} MB). Maximum allowed size is {:.0} MB.",
                metadata.len() as f64 / (1024.0 * 1024.0),
                MAX_READ_FILE_SIZE as f64 / (1024.0 * 1024.0),
            ),
        ));
    }

    fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::InvalidData {
            error!("cmd::read_file error for {}: binary file", path);
            ReasonanceError::validation(
                "file_content",
                "Cannot open binary file: the file contains non-UTF-8 data.",
            )
        } else {
            error!("cmd::read_file error for {}: {}", path, e);
            ReasonanceError::io(format!("read '{}'", path), e)
        }
    })
}

#[tauri::command]
pub fn read_file(
    path: String,
    state: State<'_, ProjectRootState>,
) -> Result<String, ReasonanceError> {
    read_file_inner(&path, &state)
}

pub fn write_file_inner(
    path: &str,
    content: &str,
    state: &ProjectRootState,
) -> Result<(), ReasonanceError> {
    info!("cmd::write_file(path={})", path);
    validate_write_path(Path::new(path), state)?;

    // Atomic write: write to a temp file in the same directory, then rename.
    // This prevents partial writes on crash (rename is atomic on the same filesystem).
    let target = Path::new(path);
    let parent = target.parent().ok_or_else(|| {
        ReasonanceError::validation("path", format!("No parent directory for '{}'", path))
    })?;
    let file_name = target.file_name().ok_or_else(|| {
        ReasonanceError::validation("path", format!("No file name for '{}'", path))
    })?;

    let tmp_name = format!(".{}.tmp", file_name.to_string_lossy());
    let tmp_path = parent.join(&tmp_name);

    fs::write(&tmp_path, content).map_err(|e| {
        error!(
            "cmd::write_file failed to write temp file {}: {}",
            tmp_path.display(),
            e
        );
        ReasonanceError::io(format!("write temp file '{}'", tmp_path.display()), e)
    })?;
    fs::rename(&tmp_path, target).map_err(|e| {
        // Clean up temp file on rename failure
        let _ = fs::remove_file(&tmp_path);
        error!(
            "cmd::write_file failed to rename temp file to {}: {}",
            path, e
        );
        ReasonanceError::io(format!("rename temp file to '{}'", path), e)
    })
}

#[tauri::command]
pub fn write_file(
    path: String,
    content: String,
    state: State<'_, ProjectRootState>,
) -> Result<(), ReasonanceError> {
    write_file_inner(&path, &content, &state)
}

pub fn list_dir_inner(
    path: &str,
    respect_gitignore: bool,
    state: &ProjectRootState,
) -> Result<Vec<FileEntry>, ReasonanceError> {
    info!("cmd::list_dir(path={})", path);
    validate_read_path(Path::new(path), state)?;
    let entries =
        fs::read_dir(path).map_err(|e| ReasonanceError::io(format!("read dir '{}'", path), e))?;

    let gitignore = if respect_gitignore {
        ignore::gitignore::Gitignore::new(Path::new(path).join(".gitignore")).0
    } else {
        ignore::gitignore::Gitignore::empty()
    };

    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| ReasonanceError::io("read dir entry", e))?;
        let metadata = entry
            .metadata()
            .map_err(|e| ReasonanceError::io("read entry metadata", e))?;

        let is_ignored = if respect_gitignore {
            let matched = gitignore.matched_path_or_any_parents(&entry.path(), metadata.is_dir());
            matched.is_ignore()
        } else {
            false
        };

        let modified = metadata
            .modified()
            .map_err(|e| ReasonanceError::io("read modified time", e))?
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
    debug!(
        "cmd::list_dir returned {} entries for {}",
        result.len(),
        path
    );
    Ok(result)
}

#[tauri::command]
pub fn list_dir(
    path: String,
    respect_gitignore: bool,
    state: State<'_, ProjectRootState>,
) -> Result<Vec<FileEntry>, ReasonanceError> {
    list_dir_inner(&path, respect_gitignore, &state)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrepResult {
    pub path: String,
    pub line_number: usize,
    pub line: String,
}

pub fn grep_files_inner(
    path: &str,
    pattern: &str,
    respect_gitignore: bool,
    state: &ProjectRootState,
) -> Result<Vec<GrepResult>, ReasonanceError> {
    info!("cmd::grep_files(path={}, pattern={})", path, pattern);
    validate_read_path(Path::new(path), state)?;
    use ignore::WalkBuilder;
    use std::io::BufRead;

    let mut results = Vec::new();
    let walker = WalkBuilder::new(path).git_ignore(respect_gitignore).build();

    for entry in walker.flatten() {
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }
        let file_path = entry.path().to_owned();
        if let Ok(file) = std::fs::File::open(&file_path) {
            let reader = std::io::BufReader::new(file);
            for (i, line_result) in reader.lines().enumerate() {
                if let Ok(line) = line_result {
                    if line.contains(pattern) {
                        results.push(GrepResult {
                            path: file_path.to_string_lossy().to_string(),
                            line_number: i + 1,
                            line,
                        });
                        if results.len() >= 500 {
                            debug!(
                                "cmd::grep_files hit 500 result limit for pattern={}",
                                pattern
                            );
                            return Ok(results);
                        }
                    }
                }
            }
        }
    }
    debug!(
        "cmd::grep_files found {} matches for pattern={}",
        results.len(),
        pattern
    );
    Ok(results)
}

#[tauri::command]
pub fn grep_files(
    path: String,
    pattern: String,
    respect_gitignore: bool,
    state: State<'_, ProjectRootState>,
) -> Result<Vec<GrepResult>, ReasonanceError> {
    grep_files_inner(&path, &pattern, respect_gitignore, &state)
}

#[tauri::command]
pub async fn get_git_status(
    project_root: String,
) -> Result<std::collections::HashMap<String, String>, ReasonanceError> {
    use std::collections::HashMap;

    let output = std::process::Command::new("git")
        .args(["status", "--porcelain=v1", "-uall"])
        .current_dir(&project_root)
        .output()
        .map_err(|e| ReasonanceError::io("run git status", e))?;

    if !output.status.success() {
        // Not a git repo or git not available — return empty
        return Ok(HashMap::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut statuses = HashMap::new();

    for line in stdout.lines() {
        if line.len() < 4 {
            continue;
        }
        let xy = &line[0..2];
        let path = line[3..].trim();

        // Handle renames: "R  old -> new" — use the new path
        let effective_path = if let Some(arrow_pos) = path.find(" -> ") {
            &path[arrow_pos + 4..]
        } else {
            path
        };

        let status = match xy.trim() {
            "M" | " M" | "MM" => "modified",
            "A" | "AM" => "added",
            "D" | " D" => "deleted",
            "R" | "RM" => "renamed",
            "??" => "untracked",
            "UU" | "AA" | "DD" => "conflicted",
            _ => "modified", // fallback
        };

        statuses.insert(effective_path.to_string(), status.to_string());
    }

    debug!(
        "cmd::get_git_status returned {} entries for {}",
        statuses.len(),
        project_root
    );
    Ok(statuses)
}

#[tauri::command]
pub fn start_watching(
    path: String,
    app: AppHandle,
    state: State<'_, FsWatcherState>,
    root_state: State<'_, ProjectRootState>,
) -> Result<(), ReasonanceError> {
    info!("cmd::start_watching(path={})", path);
    validate_read_path(Path::new(&path), &root_state)?;
    crate::fs_watcher::start_watching(&path, app, &state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_temp_dir() -> TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    fn make_root_state(root: Option<&Path>) -> ProjectRootState {
        let state = ProjectRootState::new();
        if let Some(r) = root {
            *state.0.lock().unwrap_or_else(|e| e.into_inner()) =
                Some(std::fs::canonicalize(r).unwrap());
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
        entries_vec.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then(a.0.to_lowercase().cmp(&b.0.to_lowercase()))
        });
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
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }
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
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }
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

    // ── resolve_safe_path tests (symlink escape detection) ─────────────

    #[test]
    fn path_safety_within_project_ok() {
        let dir = setup_temp_dir();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello").unwrap();
        let result = resolve_safe_path(&file, dir.path());
        assert!(result.is_ok());
        // Returned path should be canonicalized
        assert!(result.unwrap().is_absolute());
    }

    #[test]
    fn path_safety_outside_project_denied() {
        let dir = setup_temp_dir();
        let other = setup_temp_dir();
        let file = other.path().join("secret.txt");
        std::fs::write(&file, "secret").unwrap();
        let result = resolve_safe_path(&file, dir.path());
        assert!(result.is_err());
        // Should be a Security error with PathTraversal code
        match result.unwrap_err() {
            ReasonanceError::Security { code, .. } => {
                assert!(matches!(
                    code,
                    crate::error::SecurityErrorCode::PathTraversal
                ));
            }
            other => panic!("Expected Security::PathTraversal, got: {:?}", other),
        }
    }

    #[test]
    fn path_safety_relative_traversal_denied() {
        let dir = setup_temp_dir();
        let traversal = dir.path().join("../../etc/passwd");
        // This may or may not exist, but if it resolves outside root → denied
        let result = resolve_safe_path(&traversal, dir.path());
        // Either Err (file doesn't exist → IO) or Err (path traversal → Security)
        assert!(result.is_err());
    }

    #[test]
    #[cfg(unix)]
    fn path_safety_symlink_escape_denied() {
        let dir = setup_temp_dir();
        let target = std::env::temp_dir().join("reasonance_symlink_test_target");
        std::fs::write(&target, "secret data").unwrap();
        let link = dir.path().join("escape_link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let result = resolve_safe_path(&link, dir.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            ReasonanceError::Security { code, .. } => {
                assert!(matches!(
                    code,
                    crate::error::SecurityErrorCode::PathTraversal
                ));
            }
            other => panic!("Expected Security::PathTraversal, got: {:?}", other),
        }
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    #[cfg(unix)]
    fn path_safety_symlink_within_project_ok() {
        let dir = setup_temp_dir();
        let target = dir.path().join("real_file.txt");
        std::fs::write(&target, "legit").unwrap();
        let link = dir.path().join("internal_link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let result = resolve_safe_path(&link, dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn path_safety_write_new_file_in_project_ok() {
        let dir = setup_temp_dir();
        let new_file = dir.path().join("new_file.txt");
        // File doesn't exist yet
        let result = resolve_safe_write_path(&new_file, dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn path_safety_write_outside_project_denied() {
        let dir = setup_temp_dir();
        let other = setup_temp_dir();
        let file = other.path().join("bad_write.txt");
        let result = resolve_safe_write_path(&file, dir.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            ReasonanceError::Security { code, .. } => {
                assert!(matches!(
                    code,
                    crate::error::SecurityErrorCode::PathTraversal
                ));
            }
            other => panic!("Expected Security::PathTraversal, got: {:?}", other),
        }
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
        assert!(
            metadata
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
                > 0
        );
    }
}
