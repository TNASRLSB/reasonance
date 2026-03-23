use crate::agent_event::AgentEvent;
use crate::transport::request::{AgentRequest, CliMode, SessionStatus};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AgentSession {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub status: SessionStatus,
    pub cli_mode: CliMode,
    pub request: AgentRequest,
    pub events: Vec<AgentEvent>,
    pub created_at: u64,
    pub cli_session_id: Option<String>,
    pub abort_handle: Option<tokio::task::AbortHandle>,
}

impl AgentSession {
    pub fn new(request: AgentRequest, cli_mode: CliMode) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            id: request.session_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()),
            provider: request.provider.clone(),
            model: request.model.clone().unwrap_or_default(),
            status: SessionStatus::Active,
            cli_mode,
            request,
            events: Vec::new(),
            created_at: now,
            cli_session_id: None,
            abort_handle: None,
        }
    }

    pub fn set_abort_handle(&mut self, handle: tokio::task::AbortHandle) {
        self.abort_handle = Some(handle);
    }

    pub fn add_event(&mut self, event: AgentEvent) {
        self.events.push(event);
    }

    pub fn set_status(&mut self, status: SessionStatus) {
        self.status = status;
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> AgentRequest {
        AgentRequest {
            prompt: "Hello".to_string(),
            provider: "claude".to_string(),
            model: Some("claude-sonnet-4-6".to_string()),
            context: vec![],
            session_id: None,
            system_prompt: None,
            max_tokens: None,
            allowed_tools: None,
        }
    }

    #[test]
    fn test_session_creation() {
        let session = AgentSession::new(sample_request(), CliMode::Structured);
        assert!(!session.id.is_empty());
        assert_eq!(session.provider, "claude");
        assert_eq!(session.model, "claude-sonnet-4-6");
        assert_eq!(session.status, SessionStatus::Active);
        assert_eq!(session.cli_mode, CliMode::Structured);
        assert!(session.created_at > 0);
    }

    #[test]
    fn test_session_add_events() {
        let mut session = AgentSession::new(sample_request(), CliMode::Structured);
        assert_eq!(session.event_count(), 0);

        session.add_event(AgentEvent::text("hello", "claude"));
        session.add_event(AgentEvent::text("world", "claude"));
        assert_eq!(session.event_count(), 2);
    }

    #[test]
    fn test_session_status_transition() {
        let mut session = AgentSession::new(sample_request(), CliMode::Structured);
        assert_eq!(session.status, SessionStatus::Active);

        session.set_status(SessionStatus::Terminated);
        assert_eq!(session.status, SessionStatus::Terminated);
    }

    #[test]
    fn test_session_default_model() {
        let mut req = sample_request();
        req.model = None;
        let session = AgentSession::new(req, CliMode::Structured);
        assert_eq!(session.model, "");
    }
}
