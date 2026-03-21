use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentState { Idle, Queued, Running, Success, Failed, Retrying, Fallback, Error }

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
        let agent = AgentInstance {
            id: id.clone(), node_id: node_id.to_string(), workflow_path: workflow_path.to_string(),
            state: AgentState::Idle, pty_id: None, retry_count: 0, max_retries,
            fallback_agent, started_at: None, finished_at: None, error_message: None,
        };
        self.agents.lock().unwrap().insert(id.clone(), agent);
        id
    }

    pub fn transition(&self, agent_id: &str, new_state: AgentState) -> Result<AgentState, String> {
        let mut agents = self.agents.lock().unwrap();
        let agent = agents.get_mut(agent_id).ok_or_else(|| format!("Agent {} not found", agent_id))?;
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
            _ => false,
        };
        if !valid {
            return Err(format!("Invalid transition: {:?} -> {:?} for agent {}", agent.state, new_state, agent_id));
        }
        let now = chrono::Utc::now().to_rfc3339();
        match &new_state {
            AgentState::Running => { if agent.started_at.is_none() { agent.started_at = Some(now); } }
            AgentState::Retrying => { agent.retry_count += 1; }
            AgentState::Success | AgentState::Error => { agent.finished_at = Some(now); }
            _ => {}
        }
        agent.state = new_state.clone();
        Ok(new_state)
    }

    pub fn set_pty(&self, agent_id: &str, pty_id: &str) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap();
        let agent = agents.get_mut(agent_id).ok_or_else(|| format!("Agent {} not found", agent_id))?;
        agent.pty_id = Some(pty_id.to_string());
        Ok(())
    }

    pub fn set_error(&self, agent_id: &str, message: &str) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap();
        let agent = agents.get_mut(agent_id).ok_or_else(|| format!("Agent {} not found", agent_id))?;
        agent.error_message = Some(message.to_string());
        Ok(())
    }

    pub fn get_agent(&self, agent_id: &str) -> Option<AgentInstance> {
        self.agents.lock().unwrap().get(agent_id).cloned()
    }

    pub fn get_workflow_agents(&self, workflow_path: &str) -> Vec<AgentInstance> {
        self.agents.lock().unwrap().values().filter(|a| a.workflow_path == workflow_path).cloned().collect()
    }

    pub fn remove_agent(&self, agent_id: &str) -> Result<(), String> {
        self.agents.lock().unwrap().remove(agent_id).ok_or_else(|| format!("Agent {} not found", agent_id))?;
        Ok(())
    }

    pub fn remove_workflow_agents(&self, workflow_path: &str) {
        self.agents.lock().unwrap().retain(|_, a| a.workflow_path != workflow_path);
    }

    pub fn send_message(&self, from: &str, to: &str, payload: serde_json::Value) {
        let msg = AgentMessage {
            from: from.to_string(), to: to.to_string(), payload,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.messages.lock().unwrap().push(msg);
    }

    pub fn get_messages_for(&self, agent_id: &str) -> Vec<AgentMessage> {
        self.messages.lock().unwrap().iter().filter(|m| m.to == agent_id).cloned().collect()
    }
}
