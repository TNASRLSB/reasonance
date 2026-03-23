use crate::commands::fs::ProjectRootState;
use crate::workflow_store::{Workflow, WorkflowStore};
use log::{info, error, debug};
use std::path::{Path, PathBuf};
use tauri::State;

// ── Workflow path validation ─────────────────────────────────────────────────

/// Returns the global workflows directory (`~/.config/reasonance/workflows/`).
fn global_workflows_dir() -> PathBuf {
    WorkflowStore::global_dir()
}

/// Returns the project workflows directory (`{project_root}/.reasonance/workflows/`).
fn project_workflows_dir(project_root: &Path) -> PathBuf {
    project_root.join(".reasonance").join("workflows")
}

/// Validate that a workflow path is within an allowed directory:
/// 1. `{project_root}/.reasonance/workflows/` (project workflows)
/// 2. `~/.config/reasonance/workflows/` (global workflows)
///
/// `for_write` controls whether we allow the file to not yet exist (write case).
fn validate_workflow_path(
    path: &Path,
    state: &ProjectRootState,
    for_write: bool,
) -> Result<(), String> {
    let canonical = if path.exists() {
        std::fs::canonicalize(path)
            .map_err(|e| format!("Cannot resolve path '{}': {}", path.display(), e))?
    } else if for_write {
        let parent = path
            .parent()
            .ok_or_else(|| format!("No parent directory for '{}'", path.display()))?;
        let canon_parent = std::fs::canonicalize(parent)
            .map_err(|e| format!("Cannot resolve parent '{}': {}", parent.display(), e))?;
        canon_parent.join(path.file_name().unwrap_or_default())
    } else {
        return Err(format!("Path does not exist: '{}'", path.display()));
    };

    // Check global workflows dir
    let global_dir = global_workflows_dir();
    if let Ok(canon_global) = std::fs::canonicalize(&global_dir) {
        if canonical.starts_with(&canon_global) {
            return Ok(());
        }
    }

    // Check project workflows dir
    let root_lock = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(root) = root_lock.as_ref() {
        let project_wf_dir = project_workflows_dir(root);
        if let Ok(canon_project) = std::fs::canonicalize(&project_wf_dir) {
            if canonical.starts_with(&canon_project) {
                return Ok(());
            }
        }
    }

    Err(format!(
        "Access denied: '{}' is not in a valid workflows directory",
        path.display()
    ))
}

/// Validate that a directory path is an allowed workflows directory.
fn validate_workflow_dir(dir: &Path, state: &ProjectRootState) -> Result<(), String> {
    let canonical = std::fs::canonicalize(dir)
        .map_err(|e| format!("Cannot resolve directory '{}': {}", dir.display(), e))?;

    // Check global workflows dir
    let global_dir = global_workflows_dir();
    if let Ok(canon_global) = std::fs::canonicalize(&global_dir) {
        if canonical.starts_with(&canon_global) || canonical == canon_global {
            return Ok(());
        }
    }

    // Check project workflows dir
    let root_lock = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(root) = root_lock.as_ref() {
        let project_wf_dir = project_workflows_dir(root);
        if let Ok(canon_project) = std::fs::canonicalize(&project_wf_dir) {
            if canonical.starts_with(&canon_project) || canonical == canon_project {
                return Ok(());
            }
        }
    }

    Err(format!(
        "Access denied: '{}' is not a valid workflows directory",
        dir.display()
    ))
}

// ── Workflow commands ────────────────────────────────────────────────────────

#[tauri::command]
pub fn load_workflow(
    file_path: String,
    store: State<'_, WorkflowStore>,
    state: State<'_, ProjectRootState>,
) -> Result<Workflow, String> {
    info!("cmd::load_workflow(path={})", file_path);
    validate_workflow_path(Path::new(&file_path), &state, false)?;
    store.load(&file_path).map_err(|e| {
        error!("cmd::load_workflow failed for {}: {}", file_path, e);
        e
    })
}

#[tauri::command]
pub fn save_workflow(
    file_path: String,
    workflow: Workflow,
    store: State<'_, WorkflowStore>,
    state: State<'_, ProjectRootState>,
) -> Result<(), String> {
    info!("cmd::save_workflow(path={})", file_path);
    validate_workflow_path(Path::new(&file_path), &state, true)?;
    store.save(&file_path, &workflow)
}

#[tauri::command]
pub fn list_workflows(dir: String, state: State<'_, ProjectRootState>) -> Result<Vec<String>, String> {
    info!("cmd::list_workflows(dir={})", dir);
    validate_workflow_dir(Path::new(&dir), &state)?;
    WorkflowStore::list_workflows(&dir)
}

#[tauri::command]
pub fn delete_workflow(
    file_path: String,
    store: State<'_, WorkflowStore>,
    state: State<'_, ProjectRootState>,
) -> Result<(), String> {
    info!("cmd::delete_workflow(path={})", file_path);
    validate_workflow_path(Path::new(&file_path), &state, false)?;
    store.delete(&file_path)
}

#[tauri::command]
pub fn create_workflow(
    name: String,
    file_path: String,
    store: State<'_, WorkflowStore>,
    state: State<'_, ProjectRootState>,
) -> Result<Workflow, String> {
    info!("cmd::create_workflow(name={}, path={})", name, file_path);
    validate_workflow_path(Path::new(&file_path), &state, true)?;
    let workflow = WorkflowStore::create_empty(&name);
    store.save(&file_path, &workflow)?;
    Ok(workflow)
}

#[tauri::command]
pub fn get_workflow(
    file_path: String,
    store: State<'_, WorkflowStore>,
    state: State<'_, ProjectRootState>,
) -> Result<Option<Workflow>, String> {
    debug!("cmd::get_workflow(path={})", file_path);
    validate_workflow_path(Path::new(&file_path), &state, false)?;
    Ok(store.get(&file_path))
}

#[tauri::command]
pub fn duplicate_workflow(
    store: State<'_, WorkflowStore>,
    source_path: String,
    dest_path: String,
    state: State<'_, ProjectRootState>,
) -> Result<Workflow, String> {
    info!("cmd::duplicate_workflow(source={}, dest={})", source_path, dest_path);
    validate_workflow_path(Path::new(&source_path), &state, false)?;
    validate_workflow_path(Path::new(&dest_path), &state, true)?;
    let mut wf = store.load(&source_path)?;
    let now = chrono::Utc::now().format("%Y-%m-%d").to_string();
    wf.name = format!("{} (copy)", wf.name);
    wf.created = Some(now.clone());
    wf.modified = Some(now);
    store.save(&dest_path, &wf)?;
    Ok(wf)
}

#[tauri::command]
pub fn save_to_global(
    store: State<'_, WorkflowStore>,
    workflow_path: String,
    state: State<'_, ProjectRootState>,
) -> Result<String, String> {
    info!("cmd::save_to_global(path={})", workflow_path);
    validate_workflow_path(Path::new(&workflow_path), &state, false)?;
    let wf = store.load(&workflow_path)?;
    let global_dir = WorkflowStore::global_dir();
    std::fs::create_dir_all(&global_dir)
        .map_err(|e| format!("Failed to create global dir: {}", e))?;
    let filename = std::path::Path::new(&workflow_path)
        .file_name()
        .ok_or("Invalid path")?
        .to_str()
        .ok_or("Invalid filename")?;
    let dest = global_dir.join(filename);
    let dest_str = dest.to_str().ok_or("Invalid destination path")?.to_string();
    store.save(&dest_str, &wf)?;
    Ok(dest_str)
}

#[tauri::command]
pub fn list_global_workflows() -> Result<Vec<String>, String> {
    info!("cmd::list_global_workflows called");
    let global_dir = WorkflowStore::global_dir();
    let dir_str = global_dir.to_str().ok_or("Invalid global dir path")?;
    WorkflowStore::list_workflows(dir_str)
}
