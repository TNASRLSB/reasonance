use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::error::ReasonanceError;
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
) -> Result<(), ReasonanceError> {
    info!("cmd::add_project(id={}, root_path={}, trust_level={})", id, root_path, trust_level);
    let mut projects = state.0.lock().unwrap_or_else(|e| e.into_inner());
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
) -> Result<(), ReasonanceError> {
    info!("cmd::remove_project(id={})", id);
    let mut projects = state.0.lock().unwrap_or_else(|e| e.into_inner());
    projects.remove(&id);
    debug!("cmd::remove_project removed project id={}", id);
    Ok(())
}

#[tauri::command]
pub fn set_active_project(
    id: String,
    projects_state: tauri::State<'_, ProjectsState>,
    active_state: tauri::State<'_, ActiveProjectState>,
) -> Result<(), ReasonanceError> {
    info!("cmd::set_active_project(id={})", id);
    // Verify project exists
    let projects = projects_state.0.lock().unwrap_or_else(|e| e.into_inner());
    if !projects.contains_key(&id) {
        error!("cmd::set_active_project failed: project {} not found", id);
        return Err(ReasonanceError::not_found("project", &id));
    }
    drop(projects);

    let mut active = active_state.0.lock().unwrap_or_else(|e| e.into_inner());
    *active = Some(id.clone());
    debug!("cmd::set_active_project set active to id={}", id);
    Ok(())
}

#[tauri::command]
pub fn get_project_root(
    project_id: String,
    state: tauri::State<'_, ProjectsState>,
) -> Result<String, ReasonanceError> {
    debug!("cmd::get_project_root(project_id={})", project_id);
    let projects = state.0.lock().unwrap_or_else(|e| e.into_inner());
    let project = projects.get(&project_id).ok_or_else(|| {
        error!("cmd::get_project_root failed: project {} not found", project_id);
        ReasonanceError::not_found("project", &project_id)
    })?;
    Ok(project.root_path.to_string_lossy().to_string())
}
