use crate::error::ReasonanceError;
use crate::file_ops::FileOpsManager;
use log::info;
use std::sync::Mutex;
use tauri::State;

/// Managed state wrapping the FileOpsManager.
/// Initialized with a default path; re-initialized when the project root changes
/// via `file_ops_set_project`.
pub struct FileOpsState(pub Mutex<FileOpsManager>);

impl FileOpsState {
    pub fn new() -> Self {
        // Default to a temp-like path; the frontend should call file_ops_set_project
        // once a folder is opened to point the trash at the correct location.
        let default_root = std::env::temp_dir().join("reasonance-file-ops");
        Self(Mutex::new(FileOpsManager::new(&default_root)))
    }
}

/// Re-initialize the FileOpsManager with a new project root.
/// Called when the user opens a project folder.
#[tauri::command]
pub fn file_ops_set_project(
    path: String,
    state: State<'_, FileOpsState>,
) -> Result<(), ReasonanceError> {
    info!("cmd::file_ops_set_project(path={})", path);
    let root = std::path::Path::new(&path);
    if !root.exists() {
        return Err(ReasonanceError::not_found("directory", &path));
    }
    let mut mgr = state.0.lock().unwrap();
    *mgr = FileOpsManager::new(root);
    Ok(())
}

/// Delete a file by moving it to the project's .reasonance/.trash/ directory.
#[tauri::command]
pub fn file_ops_delete(
    path: String,
    state: State<'_, FileOpsState>,
) -> Result<(), ReasonanceError> {
    info!("cmd::file_ops_delete(path={})", path);
    let mgr = state.0.lock().unwrap();
    mgr.delete_file(&path)
}

/// Undo the last file operation.
/// Returns a description of what was undone, or null if the stack is empty.
#[tauri::command]
pub fn file_ops_undo(
    state: State<'_, FileOpsState>,
) -> Result<Option<String>, ReasonanceError> {
    info!("cmd::file_ops_undo");
    let mgr = state.0.lock().unwrap();
    mgr.undo()
}

/// Record that a file was created, so it can be undone later.
#[tauri::command]
pub fn file_ops_record_create(
    path: String,
    state: State<'_, FileOpsState>,
) -> Result<(), ReasonanceError> {
    info!("cmd::file_ops_record_create(path={})", path);
    let mgr = state.0.lock().unwrap();
    mgr.record_create(&path);
    Ok(())
}

/// Record that a file was renamed, so it can be undone later.
#[tauri::command]
pub fn file_ops_record_rename(
    old_path: String,
    new_path: String,
    state: State<'_, FileOpsState>,
) -> Result<(), ReasonanceError> {
    info!("cmd::file_ops_record_rename(old={}, new={})", old_path, new_path);
    let mgr = state.0.lock().unwrap();
    mgr.record_rename(&old_path, &new_path);
    Ok(())
}
