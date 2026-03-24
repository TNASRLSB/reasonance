use crate::agent_runtime::{AgentInstance, AgentMessage, AgentRuntime, AgentState};
use crate::agent_memory::{AgentMemoryStore, MemoryEntry};
use log::{info, error, debug};
use tauri::State;

#[tauri::command]
pub fn create_agent(node_id: String, workflow_path: String, max_retries: u32, fallback_agent: Option<String>, runtime: State<'_, AgentRuntime>) -> String {
    info!("cmd::create_agent(node_id={}, workflow_path={})", node_id, workflow_path);
    let id = runtime.create_agent(&node_id, &workflow_path, max_retries, fallback_agent);
    debug!("cmd::create_agent created agent_id={}", id);
    id
}

#[tauri::command]
pub fn transition_agent(agent_id: String, new_state: AgentState, runtime: State<'_, AgentRuntime>) -> Result<AgentState, String> {
    info!("cmd::transition_agent(agent_id={}, new_state={:?})", agent_id, new_state);
    runtime.transition(&agent_id, new_state).map_err(|e| {
        error!("cmd::transition_agent failed for {}: {}", agent_id, e);
        e
    })
}

#[tauri::command]
pub fn set_agent_pty(agent_id: String, pty_id: String, runtime: State<'_, AgentRuntime>) -> Result<(), String> {
    info!("cmd::set_agent_pty(agent_id={}, pty_id={})", agent_id, pty_id);
    runtime.set_pty(&agent_id, &pty_id)
}

#[tauri::command]
pub fn set_agent_error(agent_id: String, message: String, runtime: State<'_, AgentRuntime>) -> Result<(), String> {
    error!("cmd::set_agent_error(agent_id={}, message={})", agent_id, message);
    runtime.set_error(&agent_id, &message)
}

#[tauri::command]
pub fn get_agent(agent_id: String, runtime: State<'_, AgentRuntime>) -> Option<AgentInstance> {
    debug!("cmd::get_agent(agent_id={})", agent_id);
    runtime.get_agent(&agent_id)
}

#[tauri::command]
pub fn get_workflow_agents(workflow_path: String, runtime: State<'_, AgentRuntime>) -> Vec<AgentInstance> {
    debug!("cmd::get_workflow_agents(workflow_path={})", workflow_path);
    runtime.get_workflow_agents(&workflow_path)
}

#[tauri::command]
pub fn remove_agent(agent_id: String, runtime: State<'_, AgentRuntime>) -> Result<(), String> {
    info!("cmd::remove_agent(agent_id={})", agent_id);
    runtime.remove_agent(&agent_id)
}

#[tauri::command]
pub fn stop_workflow_agents(workflow_path: String, runtime: State<'_, AgentRuntime>) {
    info!("cmd::stop_workflow_agents(workflow_path={})", workflow_path);
    runtime.remove_workflow_agents(&workflow_path);
}

#[tauri::command]
pub fn send_agent_message(from: String, to: String, payload: serde_json::Value, runtime: State<'_, AgentRuntime>) {
    info!("cmd::send_agent_message(from={}, to={})", from, to);
    runtime.send_message(&from, &to, payload);
}

#[tauri::command]
pub fn get_agent_messages(agent_id: String, runtime: State<'_, AgentRuntime>) -> Vec<AgentMessage> {
    debug!("cmd::get_agent_messages(agent_id={})", agent_id);
    runtime.get_messages_for(&agent_id)
}

#[tauri::command]
pub fn get_agent_memory(node_id: String, workflow_path: String, persist: String) -> Result<Vec<MemoryEntry>, String> {
    info!("cmd::get_agent_memory(node_id={}, persist={})", node_id, persist);
    let mem_path = match persist.as_str() {
        "global" => AgentMemoryStore::global_memory_path(&node_id),
        _ => AgentMemoryStore::workflow_memory_path(&workflow_path, &node_id),
    };
    let path_str = mem_path.to_str().unwrap_or("");
    match AgentMemoryStore::load(path_str) {
        Ok(store) => Ok(store.entries),
        Err(_) => Ok(Vec::new()),
    }
}
