use crate::agent_runtime::{AgentInstance, AgentMessage, AgentRuntime, AgentState};
use tauri::State;

#[tauri::command]
pub fn create_agent(node_id: String, workflow_path: String, max_retries: u32, fallback_agent: Option<String>, runtime: State<'_, AgentRuntime>) -> String {
    runtime.create_agent(&node_id, &workflow_path, max_retries, fallback_agent)
}

#[tauri::command]
pub fn transition_agent(agent_id: String, new_state: AgentState, runtime: State<'_, AgentRuntime>) -> Result<AgentState, String> {
    runtime.transition(&agent_id, new_state)
}

#[tauri::command]
pub fn set_agent_pty(agent_id: String, pty_id: String, runtime: State<'_, AgentRuntime>) -> Result<(), String> {
    runtime.set_pty(&agent_id, &pty_id)
}

#[tauri::command]
pub fn set_agent_error(agent_id: String, message: String, runtime: State<'_, AgentRuntime>) -> Result<(), String> {
    runtime.set_error(&agent_id, &message)
}

#[tauri::command]
pub fn get_agent(agent_id: String, runtime: State<'_, AgentRuntime>) -> Option<AgentInstance> {
    runtime.get_agent(&agent_id)
}

#[tauri::command]
pub fn get_workflow_agents(workflow_path: String, runtime: State<'_, AgentRuntime>) -> Vec<AgentInstance> {
    runtime.get_workflow_agents(&workflow_path)
}

#[tauri::command]
pub fn remove_agent(agent_id: String, runtime: State<'_, AgentRuntime>) -> Result<(), String> {
    runtime.remove_agent(&agent_id)
}

#[tauri::command]
pub fn stop_workflow_agents(workflow_path: String, runtime: State<'_, AgentRuntime>) {
    runtime.remove_workflow_agents(&workflow_path);
}

#[tauri::command]
pub fn send_agent_message(from: String, to: String, payload: serde_json::Value, runtime: State<'_, AgentRuntime>) {
    runtime.send_message(&from, &to, payload);
}

#[tauri::command]
pub fn get_agent_messages(agent_id: String, runtime: State<'_, AgentRuntime>) -> Vec<AgentMessage> {
    runtime.get_messages_for(&agent_id)
}
