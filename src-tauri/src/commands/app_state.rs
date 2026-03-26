use crate::app_state_store::{AppState, AppStateStore, ProjectState};
use crate::error::ReasonanceError;
use tauri::State;

/// Return the current app-level state.
#[tauri::command]
pub fn get_app_state(store: State<'_, AppStateStore>) -> Result<AppState, ReasonanceError> {
    Ok(store.get_app_state())
}

/// Persist app-level state.
#[tauri::command]
pub fn save_app_state(
    store: State<'_, AppStateStore>,
    state: AppState,
) -> Result<(), ReasonanceError> {
    store.save_app_state(&state)
}

/// Return the state for a specific project.
#[tauri::command]
pub fn get_project_state(
    store: State<'_, AppStateStore>,
    project_id: String,
) -> Result<ProjectState, ReasonanceError> {
    Ok(store.get_project_state(&project_id))
}

/// Persist the state for a specific project.
#[tauri::command]
pub fn save_project_state(
    store: State<'_, AppStateStore>,
    project_id: String,
    state: ProjectState,
) -> Result<(), ReasonanceError> {
    store.save_project_state(&project_id, &state)
}
