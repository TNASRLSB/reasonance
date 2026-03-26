use crate::agent_event::AgentEvent;
use crate::error::ReasonanceError;
use crate::transport::StructuredAgentTransport;
use crate::transport::request::{AgentRequest, SessionStatus};
use log::{info, error, debug};
use tauri::State;

#[tauri::command]
pub async fn agent_send(
    request: AgentRequest,
    transport: State<'_, StructuredAgentTransport>,
    trust_store: State<'_, crate::workspace_trust::TrustStore>,
) -> Result<String, ReasonanceError> {
    info!("cmd::agent_send(session_id={:?}, provider={})", request.session_id, request.provider);
    transport.send(request, &trust_store).map_err(|e| {
        error!("cmd::agent_send failed: {}", e);
        e
    })
}

#[tauri::command]
pub async fn agent_stop(
    session_id: String,
    transport: State<'_, StructuredAgentTransport>,
) -> Result<(), ReasonanceError> {
    info!("cmd::agent_stop(session_id={})", session_id);
    transport.stop(&session_id)
}

#[tauri::command]
pub async fn agent_get_events(
    session_id: String,
    transport: State<'_, StructuredAgentTransport>,
) -> Result<Vec<AgentEvent>, ReasonanceError> {
    debug!("cmd::agent_get_events(session_id={})", session_id);
    Ok(transport.get_events(&session_id))
}

#[tauri::command]
pub async fn agent_get_session_status(
    session_id: String,
    transport: State<'_, StructuredAgentTransport>,
) -> Result<SessionStatus, ReasonanceError> {
    debug!("cmd::agent_get_session_status(session_id={})", session_id);
    transport.get_status(&session_id)
}

#[tauri::command]
pub async fn agent_list_sessions(
    transport: State<'_, StructuredAgentTransport>,
) -> Result<Vec<String>, ReasonanceError> {
    info!("cmd::agent_list_sessions called");
    Ok(transport.active_sessions())
}
