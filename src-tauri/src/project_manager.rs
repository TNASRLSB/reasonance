use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use log::{debug, error, info};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub root_path: PathBuf,
    pub trust_level: String, // "trusted", "read_only", "blocked"
}

pub struct ProjectsState(pub Mutex<HashMap<String, ProjectState>>);

impl ProjectsState {
    pub fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

pub struct ActiveProjectState(pub Mutex<Option<String>>);

impl ActiveProjectState {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }
}

#[tauri::command]
pub fn add_project(
    id: String,
    root_path: String,
    trust_level: String,
    state: tauri::State<'_, ProjectsState>,
) -> Result<(), String> {
    info!("cmd::add_project(id={}, root_path={}, trust_level={})", id, root_path, trust_level);
    let mut projects = state.0.lock().map_err(|e| {
        error!("cmd::add_project failed to acquire lock: {}", e);
        e.to_string()
    })?;
    projects.insert(
        id.clone(),
        ProjectState {
            root_path: PathBuf::from(root_path),
            trust_level,
        },
    );
    debug!("cmd::add_project inserted project id={}", id);
    Ok(())
}

#[tauri::command]
pub fn remove_project(
    id: String,
    state: tauri::State<'_, ProjectsState>,
) -> Result<(), String> {
    info!("cmd::remove_project(id={})", id);
    let mut projects = state.0.lock().map_err(|e| {
        error!("cmd::remove_project failed to acquire lock: {}", e);
        e.to_string()
    })?;
    projects.remove(&id);
    debug!("cmd::remove_project removed project id={}", id);
    Ok(())
}

#[tauri::command]
pub fn set_active_project(
    id: String,
    projects_state: tauri::State<'_, ProjectsState>,
    active_state: tauri::State<'_, ActiveProjectState>,
) -> Result<(), String> {
    info!("cmd::set_active_project(id={})", id);
    // Verify project exists
    let projects = projects_state.0.lock().map_err(|e| {
        error!("cmd::set_active_project failed to acquire projects lock: {}", e);
        e.to_string()
    })?;
    if !projects.contains_key(&id) {
        error!("cmd::set_active_project failed: project {} not found", id);
        return Err(format!("Project {} not found", id));
    }
    drop(projects);

    let mut active = active_state.0.lock().map_err(|e| {
        error!("cmd::set_active_project failed to acquire active lock: {}", e);
        e.to_string()
    })?;
    *active = Some(id.clone());
    debug!("cmd::set_active_project set active to id={}", id);
    Ok(())
}

#[tauri::command]
pub fn get_project_root(
    project_id: String,
    state: tauri::State<'_, ProjectsState>,
) -> Result<String, String> {
    debug!("cmd::get_project_root(project_id={})", project_id);
    let projects = state.0.lock().map_err(|e| {
        error!("cmd::get_project_root failed to acquire lock: {}", e);
        e.to_string()
    })?;
    let project = projects.get(&project_id).ok_or_else(|| {
        error!("cmd::get_project_root failed: project {} not found", project_id);
        format!("Project {} not found", project_id)
    })?;
    Ok(project.root_path.to_string_lossy().to_string())
}
