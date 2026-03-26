use crate::agent_comms::{AgentCommsBus, AgentMessage, ChannelType};
use crate::error::ReasonanceError;
use log::{debug, info};
use tauri::State;

#[tauri::command]
pub fn agent_publish_message(
    from: String,
    channel: ChannelType,
    payload: serde_json::Value,
    reply_to: Option<String>,
    ttl_secs: Option<u64>,
    bus: State<'_, AgentCommsBus>,
) -> Result<String, ReasonanceError> {
    info!("cmd::agent_publish_message(from={})", from);
    let msg = AgentMessage {
        id: uuid::Uuid::new_v4().to_string(),
        from,
        channel,
        payload,
        timestamp: chrono::Utc::now().to_rfc3339(),
        reply_to,
        ttl_secs,
    };
    let id = msg.id.clone();
    bus.publish(msg)?;
    Ok(id)
}

#[tauri::command]
pub fn agent_get_messages(
    node_id: String,
    since_id: Option<String>,
    bus: State<'_, AgentCommsBus>,
) -> Vec<AgentMessage> {
    debug!("cmd::agent_get_messages(node_id={}, since_id={:?})", node_id, since_id);
    bus.get_messages(&node_id, since_id.as_deref())
}

#[tauri::command]
pub fn agent_get_topic_messages(
    topic: String,
    since_id: Option<String>,
    bus: State<'_, AgentCommsBus>,
) -> Vec<AgentMessage> {
    debug!("cmd::agent_get_topic_messages(topic={}, since_id={:?})", topic, since_id);
    bus.get_topic_messages(&topic, since_id.as_deref())
}

#[tauri::command]
pub fn agent_get_broadcast_messages(
    workflow_id: String,
    since_id: Option<String>,
    bus: State<'_, AgentCommsBus>,
) -> Vec<AgentMessage> {
    debug!("cmd::agent_get_broadcast_messages(workflow_id={})", workflow_id);
    bus.get_broadcast_messages(&workflow_id, since_id.as_deref())
}

#[tauri::command]
pub fn agent_sweep_messages(bus: State<'_, AgentCommsBus>) -> usize {
    info!("cmd::agent_sweep_messages");
    bus.sweep_expired()
}

#[tauri::command]
pub fn agent_clear_workflow_messages(workflow_id: String, bus: State<'_, AgentCommsBus>) {
    info!("cmd::agent_clear_workflow_messages(workflow_id={})", workflow_id);
    bus.clear_workflow(&workflow_id);
}
