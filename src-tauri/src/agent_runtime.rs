use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentState { Idle, Queued, Running, Success, Failed, Retrying, Fallback, Error, Skipped }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstance {
    pub id: String,
    pub node_id: String,
    pub workflow_path: String,
    pub state: AgentState,
    pub pty_id: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub fallback_agent: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error_message: Option<String>,
    #[serde(default)]
    pub output_buffer: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub from: String,
    pub to: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

pub struct AgentRuntime {
    pub agents: Arc<Mutex<HashMap<String, AgentInstance>>>,
    pub messages: Arc<Mutex<Vec<AgentMessage>>>,
}

impl AgentRuntime {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn create_agent(&self, node_id: &str, workflow_path: &str, max_retries: u32, fallback_agent: Option<String>) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        info!("Agent created: id={}, node_id={}, workflow={}, max_retries={}", id, node_id, workflow_path, max_retries);
        let agent = AgentInstance {
            id: id.clone(), node_id: node_id.to_string(), workflow_path: workflow_path.to_string(),
            state: AgentState::Idle, pty_id: None, retry_count: 0, max_retries,
            fallback_agent, started_at: None, finished_at: None, error_message: None, output_buffer: Vec::new(),
        };
        self.agents.lock().unwrap_or_else(|e| e.into_inner()).insert(id.clone(), agent);
        id
    }

    pub fn transition(&self, agent_id: &str, new_state: AgentState) -> Result<AgentState, String> {
        let mut agents = self.agents.lock().unwrap_or_else(|e| e.into_inner());
        let agent = agents.get_mut(agent_id).ok_or_else(|| {
            error!("Agent state transition failed: agent {} not found", agent_id);
            format!("Agent {} not found", agent_id)
        })?;
        let valid = match (&agent.state, &new_state) {
            (AgentState::Idle, AgentState::Queued) => true,
            (AgentState::Queued, AgentState::Running) => true,
            (AgentState::Running, AgentState::Success) => true,
            (AgentState::Running, AgentState::Failed) => true,
            (AgentState::Failed, AgentState::Retrying) => agent.retry_count < agent.max_retries,
            (AgentState::Failed, AgentState::Fallback) => agent.fallback_agent.is_some(),
            (AgentState::Failed, AgentState::Error) => true,
            (AgentState::Retrying, AgentState::Running) => true,
            (AgentState::Fallback, AgentState::Running) => true,
            (AgentState::Idle, AgentState::Skipped) => true,
            _ => false,
        };
        if !valid {
            warn!("Invalid state transition: {:?} -> {:?} for agent {}", agent.state, new_state, agent_id);
            return Err(format!("Invalid transition: {:?} -> {:?} for agent {}", agent.state, new_state, agent_id));
        }
        info!("Agent state transition: id={}, {:?} -> {:?}", agent_id, agent.state, new_state);
        let now = chrono::Utc::now().to_rfc3339();
        match &new_state {
            AgentState::Running => { if agent.started_at.is_none() { agent.started_at = Some(now); } }
            AgentState::Retrying => { agent.retry_count += 1; }
            AgentState::Success | AgentState::Error | AgentState::Skipped => { agent.finished_at = Some(now); }
            _ => {}
        }
        agent.state = new_state.clone();
        Ok(new_state)
    }

    pub fn set_pty(&self, agent_id: &str, pty_id: &str) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap_or_else(|e| e.into_inner());
        let agent = agents.get_mut(agent_id).ok_or_else(|| format!("Agent {} not found", agent_id))?;
        agent.pty_id = Some(pty_id.to_string());
        Ok(())
    }

    pub fn set_error(&self, agent_id: &str, message: &str) -> Result<(), String> {
        error!("Agent error set: id={}, message='{}'", agent_id, message);
        let mut agents = self.agents.lock().unwrap_or_else(|e| e.into_inner());
        let agent = agents.get_mut(agent_id).ok_or_else(|| format!("Agent {} not found", agent_id))?;
        agent.error_message = Some(message.to_string());
        Ok(())
    }

    pub fn get_agent(&self, agent_id: &str) -> Option<AgentInstance> {
        self.agents.lock().unwrap_or_else(|e| e.into_inner()).get(agent_id).cloned()
    }

    pub fn get_workflow_agents(&self, workflow_path: &str) -> Vec<AgentInstance> {
        self.agents.lock().unwrap_or_else(|e| e.into_inner()).values().filter(|a| a.workflow_path == workflow_path).cloned().collect()
    }

    pub fn remove_agent(&self, agent_id: &str) -> Result<(), String> {
        info!("Agent removed: id={}", agent_id);
        self.agents.lock().unwrap_or_else(|e| e.into_inner()).remove(agent_id).ok_or_else(|| format!("Agent {} not found", agent_id))?;
        Ok(())
    }

    pub fn remove_workflow_agents(&self, workflow_path: &str) {
        debug!("Removing all agents for workflow={}", workflow_path);
        self.agents.lock().unwrap_or_else(|e| e.into_inner()).retain(|_, a| a.workflow_path != workflow_path);
    }

    pub fn send_message(&self, from: &str, to: &str, payload: serde_json::Value) {
        debug!("Agent message: from={}, to={}", from, to);
        let msg = AgentMessage {
            from: from.to_string(), to: to.to_string(), payload,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.messages.lock().unwrap_or_else(|e| e.into_inner()).push(msg);
    }

    const MAX_OUTPUT_LINES: usize = 200;

    pub fn append_output(&self, agent_id: &str, line: &str) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap_or_else(|e| e.into_inner());
        let agent = agents.get_mut(agent_id).ok_or_else(|| format!("Agent {} not found", agent_id))?;
        agent.output_buffer.push(line.to_string());
        if agent.output_buffer.len() > Self::MAX_OUTPUT_LINES {
            let drain_count = agent.output_buffer.len() - Self::MAX_OUTPUT_LINES;
            agent.output_buffer.drain(..drain_count);
        }
        Ok(())
    }

    pub fn get_output(&self, agent_id: &str) -> Result<Vec<String>, String> {
        let agents = self.agents.lock().unwrap_or_else(|e| e.into_inner());
        let agent = agents.get(agent_id).ok_or_else(|| format!("Agent {} not found", agent_id))?;
        Ok(agent.output_buffer.clone())
    }

    pub fn get_messages_for(&self, agent_id: &str) -> Vec<AgentMessage> {
        self.messages.lock().unwrap_or_else(|e| e.into_inner()).iter().filter(|m| m.to == agent_id).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_agent() {
        let runtime = AgentRuntime::new();
        let id = runtime.create_agent("node-1", "wf-path", 3, None);
        let agent = runtime.get_agent(&id).unwrap();
        assert_eq!(agent.node_id, "node-1");
        assert_eq!(agent.state, AgentState::Idle);
        assert_eq!(agent.max_retries, 3);
        assert_eq!(agent.retry_count, 0);
    }

    #[test]
    fn test_valid_transitions() {
        let runtime = AgentRuntime::new();
        let id = runtime.create_agent("node-1", "wf", 2, None);

        assert_eq!(runtime.transition(&id, AgentState::Queued).unwrap(), AgentState::Queued);
        assert_eq!(runtime.transition(&id, AgentState::Running).unwrap(), AgentState::Running);
        assert_eq!(runtime.transition(&id, AgentState::Success).unwrap(), AgentState::Success);
    }

    #[test]
    fn test_invalid_transition() {
        let runtime = AgentRuntime::new();
        let id = runtime.create_agent("node-1", "wf", 0, None);

        // Can't go directly from Idle to Running
        let result = runtime.transition(&id, AgentState::Running);
        assert!(result.is_err());
    }

    #[test]
    fn test_retry_transition() {
        let runtime = AgentRuntime::new();
        let id = runtime.create_agent("node-1", "wf", 2, None);

        runtime.transition(&id, AgentState::Queued).unwrap();
        runtime.transition(&id, AgentState::Running).unwrap();
        runtime.transition(&id, AgentState::Failed).unwrap();
        runtime.transition(&id, AgentState::Retrying).unwrap();

        let agent = runtime.get_agent(&id).unwrap();
        assert_eq!(agent.retry_count, 1);
    }

    #[test]
    fn test_retry_exhausted() {
        let runtime = AgentRuntime::new();
        let id = runtime.create_agent("node-1", "wf", 1, None);

        runtime.transition(&id, AgentState::Queued).unwrap();
        runtime.transition(&id, AgentState::Running).unwrap();
        runtime.transition(&id, AgentState::Failed).unwrap();
        runtime.transition(&id, AgentState::Retrying).unwrap(); // retry_count becomes 1
        runtime.transition(&id, AgentState::Running).unwrap();
        runtime.transition(&id, AgentState::Failed).unwrap();

        // Can't retry again (retry_count 1 >= max_retries 1)
        let result = runtime.transition(&id, AgentState::Retrying);
        assert!(result.is_err());
    }

    #[test]
    fn test_fallback_requires_fallback_agent() {
        let runtime = AgentRuntime::new();

        // No fallback agent configured
        let id1 = runtime.create_agent("node-1", "wf", 0, None);
        runtime.transition(&id1, AgentState::Queued).unwrap();
        runtime.transition(&id1, AgentState::Running).unwrap();
        runtime.transition(&id1, AgentState::Failed).unwrap();
        let result = runtime.transition(&id1, AgentState::Fallback);
        assert!(result.is_err());

        // With fallback agent configured
        let id2 = runtime.create_agent("node-2", "wf", 0, Some("backup".to_string()));
        runtime.transition(&id2, AgentState::Queued).unwrap();
        runtime.transition(&id2, AgentState::Running).unwrap();
        runtime.transition(&id2, AgentState::Failed).unwrap();
        let result = runtime.transition(&id2, AgentState::Fallback);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_pty_and_error() {
        let runtime = AgentRuntime::new();
        let id = runtime.create_agent("node-1", "wf", 0, None);

        runtime.set_pty(&id, "pty-123").unwrap();
        let agent = runtime.get_agent(&id).unwrap();
        assert_eq!(agent.pty_id, Some("pty-123".to_string()));

        runtime.set_error(&id, "something broke").unwrap();
        let agent = runtime.get_agent(&id).unwrap();
        assert_eq!(agent.error_message, Some("something broke".to_string()));
    }

    #[test]
    fn test_workflow_agents() {
        let runtime = AgentRuntime::new();
        runtime.create_agent("n1", "wf-a", 0, None);
        runtime.create_agent("n2", "wf-a", 0, None);
        runtime.create_agent("n3", "wf-b", 0, None);

        let agents_a = runtime.get_workflow_agents("wf-a");
        assert_eq!(agents_a.len(), 2);

        let agents_b = runtime.get_workflow_agents("wf-b");
        assert_eq!(agents_b.len(), 1);
    }

    #[test]
    fn test_remove_agent() {
        let runtime = AgentRuntime::new();
        let id = runtime.create_agent("node-1", "wf", 0, None);
        assert!(runtime.get_agent(&id).is_some());
        runtime.remove_agent(&id).unwrap();
        assert!(runtime.get_agent(&id).is_none());
    }

    #[test]
    fn test_remove_workflow_agents() {
        let runtime = AgentRuntime::new();
        runtime.create_agent("n1", "wf-a", 0, None);
        runtime.create_agent("n2", "wf-a", 0, None);
        runtime.create_agent("n3", "wf-b", 0, None);

        runtime.remove_workflow_agents("wf-a");
        assert_eq!(runtime.get_workflow_agents("wf-a").len(), 0);
        assert_eq!(runtime.get_workflow_agents("wf-b").len(), 1);
    }

    #[test]
    fn test_skipped_transition() {
        let runtime = AgentRuntime::new();
        let id = runtime.create_agent("node-1", "wf", 0, None);

        // Idle → Skipped is valid
        let result = runtime.transition(&id, AgentState::Skipped);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), AgentState::Skipped);

        let agent = runtime.get_agent(&id).unwrap();
        assert!(agent.finished_at.is_some());
    }

    #[test]
    fn test_messaging() {
        let runtime = AgentRuntime::new();
        runtime.send_message("agent-1", "agent-2", serde_json::json!({"text": "hello"}));
        runtime.send_message("agent-3", "agent-2", serde_json::json!({"text": "world"}));

        let msgs = runtime.get_messages_for("agent-2");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].from, "agent-1");
        assert_eq!(msgs[1].from, "agent-3");

        let msgs_1 = runtime.get_messages_for("agent-1");
        assert!(msgs_1.is_empty());
    }

    #[test]
    fn test_timestamps_set_on_transition() {
        let runtime = AgentRuntime::new();
        let id = runtime.create_agent("node-1", "wf", 0, None);

        let agent = runtime.get_agent(&id).unwrap();
        assert!(agent.started_at.is_none());
        assert!(agent.finished_at.is_none());

        runtime.transition(&id, AgentState::Queued).unwrap();
        runtime.transition(&id, AgentState::Running).unwrap();
        let agent = runtime.get_agent(&id).unwrap();
        assert!(agent.started_at.is_some());

        runtime.transition(&id, AgentState::Success).unwrap();
        let agent = runtime.get_agent(&id).unwrap();
        assert!(agent.finished_at.is_some());
    }
}
