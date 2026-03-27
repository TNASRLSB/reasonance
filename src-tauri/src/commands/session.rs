use crate::agent_event::AgentEvent;
use crate::error::ReasonanceError;
use crate::transport::session_handle::{SessionHandle, SessionSummary, ViewMode};
use crate::transport::session_manager::SessionManager;
use log::{debug, error, info};
use tauri::State;

#[tauri::command]
pub async fn session_create(
    provider: String,
    model: String,
    session_manager: State<'_, SessionManager>,
) -> Result<String, ReasonanceError> {
    info!(
        "cmd::session_create(provider={}, model={})",
        provider, model
    );
    let result = session_manager.create_session(&provider, &model).await;
    match &result {
        Ok(id) => debug!("cmd::session_create created session_id={}", id),
        Err(e) => error!("cmd::session_create failed: {}", e),
    }
    result
}

#[tauri::command]
pub async fn session_restore(
    session_id: String,
    session_manager: State<'_, SessionManager>,
) -> Result<SessionHandle, ReasonanceError> {
    info!("cmd::session_restore(session_id={})", session_id);
    let (handle, _events) = session_manager
        .restore_session(&session_id)
        .await
        .map_err(|e| {
            error!(
                "cmd::session_restore failed for session_id={}: {}",
                session_id, e
            );
            e
        })?;
    Ok(handle)
}

#[tauri::command]
pub async fn session_get_events(
    session_id: String,
    session_manager: State<'_, SessionManager>,
) -> Result<Vec<AgentEvent>, ReasonanceError> {
    debug!("cmd::session_get_events(session_id={})", session_id);
    let store = session_manager.store();
    store.read_events(&session_id).await
}

#[tauri::command]
pub async fn session_list(
    session_manager: State<'_, SessionManager>,
) -> Result<Vec<SessionSummary>, ReasonanceError> {
    info!("cmd::session_list called");
    Ok(session_manager.list_sessions())
}

#[tauri::command]
pub async fn session_delete(
    session_id: String,
    session_manager: State<'_, SessionManager>,
) -> Result<(), ReasonanceError> {
    info!("cmd::session_delete(session_id={})", session_id);
    session_manager.delete_session(&session_id).await
}

/// Maximum allowed length for a session title.
const MAX_SESSION_TITLE_LENGTH: usize = 200;

#[tauri::command]
pub async fn session_rename(
    session_id: String,
    title: String,
    session_manager: State<'_, SessionManager>,
) -> Result<(), ReasonanceError> {
    info!("cmd::session_rename(session_id={})", session_id);
    if title.len() > MAX_SESSION_TITLE_LENGTH {
        error!(
            "cmd::session_rename title too long ({} chars) for session_id={}",
            title.len(),
            session_id
        );
        return Err(ReasonanceError::validation(
            "title",
            format!(
                "Session title too long ({} chars). Maximum allowed is {} characters.",
                title.len(),
                MAX_SESSION_TITLE_LENGTH,
            ),
        ));
    }
    session_manager.rename_session(&session_id, &title).await
}

#[tauri::command]
pub async fn session_fork(
    session_id: String,
    fork_event_index: u32,
    session_manager: State<'_, SessionManager>,
) -> Result<String, ReasonanceError> {
    info!(
        "cmd::session_fork(session_id={}, fork_event_index={})",
        session_id, fork_event_index
    );
    session_manager
        .fork_session(&session_id, fork_event_index)
        .await
}

#[tauri::command]
pub async fn session_set_view_mode(
    session_id: String,
    mode: ViewMode,
    session_manager: State<'_, SessionManager>,
) -> Result<(), ReasonanceError> {
    info!(
        "cmd::session_set_view_mode(session_id={}, mode={:?})",
        session_id, mode
    );
    session_manager.set_view_mode(&session_id, mode).await
}
