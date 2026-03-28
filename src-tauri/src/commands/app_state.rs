use crate::app_state_store::{AppState, AppStateStore, ProjectState};
use crate::error::ReasonanceError;
use tauri::State;

/// Return the current app-level state.
pub fn get_app_state_inner(store: &AppStateStore) -> Result<AppState, ReasonanceError> {
    Ok(store.get_app_state())
}

#[tauri::command]
pub fn get_app_state(store: State<'_, AppStateStore>) -> Result<AppState, ReasonanceError> {
    get_app_state_inner(&store)
}

/// Persist app-level state.
pub fn save_app_state_inner(
    store: &AppStateStore,
    state: &AppState,
) -> Result<(), ReasonanceError> {
    store.save_app_state(state)
}

#[tauri::command]
pub fn save_app_state(
    store: State<'_, AppStateStore>,
    state: AppState,
) -> Result<(), ReasonanceError> {
    save_app_state_inner(&store, &state)
}

/// Return the state for a specific project.
pub fn get_project_state_inner(
    store: &AppStateStore,
    project_id: &str,
) -> Result<ProjectState, ReasonanceError> {
    Ok(store.get_project_state(project_id))
}

#[tauri::command]
pub fn get_project_state(
    store: State<'_, AppStateStore>,
    project_id: String,
) -> Result<ProjectState, ReasonanceError> {
    get_project_state_inner(&store, &project_id)
}

/// Persist the state for a specific project.
pub fn save_project_state_inner(
    store: &AppStateStore,
    project_id: &str,
    state: &ProjectState,
) -> Result<(), ReasonanceError> {
    store.save_project_state(project_id, state)
}

#[tauri::command]
pub fn save_project_state(
    store: State<'_, AppStateStore>,
    project_id: String,
    state: ProjectState,
) -> Result<(), ReasonanceError> {
    save_project_state_inner(&store, &project_id, &state)
}
