use crate::agent_event::AgentEvent;
use crate::transport::StructuredAgentTransport;
use crate::transport::request::{AgentRequest, SessionStatus};
use tauri::State;

#[tauri::command]
pub async fn agent_send(
    request: AgentRequest,
    transport: State<'_, StructuredAgentTransport>,
) -> Result<String, String> {
    transport.send(request)
}

#[tauri::command]
pub async fn agent_stop(
    session_id: String,
    transport: State<'_, StructuredAgentTransport>,
) -> Result<(), String> {
    transport.stop(&session_id)
}

#[tauri::command]
pub async fn agent_get_events(
    session_id: String,
    transport: State<'_, StructuredAgentTransport>,
) -> Result<Vec<AgentEvent>, String> {
    Ok(transport.get_events(&session_id))
}

#[tauri::command]
pub async fn agent_get_session_status(
    session_id: String,
    transport: State<'_, StructuredAgentTransport>,
) -> Result<SessionStatus, String> {
    transport.get_status(&session_id)
}

#[tauri::command]
pub async fn agent_list_sessions(
    transport: State<'_, StructuredAgentTransport>,
) -> Result<Vec<String>, String> {
    Ok(transport.active_sessions())
}
