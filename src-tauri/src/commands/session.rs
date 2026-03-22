use crate::agent_event::AgentEvent;
use crate::transport::session_handle::{SessionHandle, SessionSummary, ViewMode};
use crate::transport::session_manager::SessionManager;
use tauri::State;

#[tauri::command]
pub async fn session_create(
    provider: String,
    model: String,
    session_manager: State<'_, SessionManager>,
) -> Result<String, String> {
    session_manager.create_session(&provider, &model)
}

#[tauri::command]
pub async fn session_restore(
    session_id: String,
    session_manager: State<'_, SessionManager>,
) -> Result<SessionHandle, String> {
    let (handle, _events) = session_manager.restore_session(&session_id)?;
    Ok(handle)
}

#[tauri::command]
pub async fn session_get_events(
    session_id: String,
    session_manager: State<'_, SessionManager>,
) -> Result<Vec<AgentEvent>, String> {
    let store = session_manager.store();
    store.read_events(&session_id)
}

#[tauri::command]
pub async fn session_list(
    session_manager: State<'_, SessionManager>,
) -> Result<Vec<SessionSummary>, String> {
    Ok(session_manager.list_sessions())
}

#[tauri::command]
pub async fn session_delete(
    session_id: String,
    session_manager: State<'_, SessionManager>,
) -> Result<(), String> {
    session_manager.delete_session(&session_id)
}

#[tauri::command]
pub async fn session_rename(
    session_id: String,
    title: String,
    session_manager: State<'_, SessionManager>,
) -> Result<(), String> {
    session_manager.rename_session(&session_id, &title)
}

#[tauri::command]
pub async fn session_fork(
    session_id: String,
    fork_event_index: u32,
    session_manager: State<'_, SessionManager>,
) -> Result<String, String> {
    session_manager.fork_session(&session_id, fork_event_index)
}

#[tauri::command]
pub async fn session_set_view_mode(
    session_id: String,
    mode: ViewMode,
    session_manager: State<'_, SessionManager>,
) -> Result<(), String> {
    session_manager.set_view_mode(&session_id, mode)
}
