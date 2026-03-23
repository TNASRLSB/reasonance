use crate::transport::request::SessionStatus;
#[allow(unused_imports)]
use log::{info, warn, error, debug, trace};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    Chat,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionSource {
    User,
    Workflow { run_id: String, node_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkInfo {
    pub parent_session_id: String,
    pub fork_event_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHandle {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub cli_session_id: Option<String>,
    pub status: SessionStatus,
    pub title: String,
    pub created_at: u64,
    pub last_active_at: u64,
    pub event_count: u32,
    pub view_mode: ViewMode,
    pub source: SessionSource,
    pub forked_from: Option<ForkInfo>,
}

/// Lightweight summary for listing sessions (no events loaded).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub title: String,
    pub status: SessionStatus,
    pub created_at: u64,
    pub last_active_at: u64,
    pub event_count: u32,
    pub source: SessionSource,
}

impl SessionHandle {
    pub fn new(provider: &str, model: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let id = Uuid::new_v4().to_string();
        debug!("SessionHandle: created new handle id={} provider={} model={}", id, provider, model);

        Self {
            id,
            provider: provider.to_string(),
            model: model.to_string(),
            cli_session_id: None,
            status: SessionStatus::Active,
            title: String::new(),
            created_at: now,
            last_active_at: now,
            event_count: 0,
            view_mode: ViewMode::Chat,
            source: SessionSource::User,
            forked_from: None,
        }
    }

    pub fn to_summary(&self) -> SessionSummary {
        SessionSummary {
            id: self.id.clone(),
            provider: self.provider.clone(),
            model: self.model.clone(),
            title: self.title.clone(),
            status: self.status.clone(),
            created_at: self.created_at,
            last_active_at: self.last_active_at,
            event_count: self.event_count,
            source: self.source.clone(),
        }
    }

    pub fn touch(&mut self) {
        self.last_active_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_handle_creation() {
        let handle = SessionHandle::new("claude", "claude-sonnet-4-6");
        assert!(!handle.id.is_empty());
        assert_eq!(handle.provider, "claude");
        assert_eq!(handle.model, "claude-sonnet-4-6");
        assert_eq!(handle.status, SessionStatus::Active);
        assert_eq!(handle.view_mode, ViewMode::Chat);
        assert_eq!(handle.source, SessionSource::User);
        assert!(handle.created_at > 0);
        assert_eq!(handle.event_count, 0);
    }

    #[test]
    fn test_session_handle_serialization() {
        let handle = SessionHandle::new("claude", "claude-sonnet-4-6");
        let json = serde_json::to_string(&handle).unwrap();
        let deserialized: SessionHandle = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, handle.id);
        assert_eq!(deserialized.provider, "claude");
    }

    #[test]
    fn test_to_summary() {
        let handle = SessionHandle::new("claude", "claude-sonnet-4-6");
        let summary = handle.to_summary();
        assert_eq!(summary.id, handle.id);
        assert_eq!(summary.provider, "claude");
        assert_eq!(summary.event_count, 0);
    }

    #[test]
    fn test_fork_info_serialization() {
        let fork = ForkInfo {
            parent_session_id: "parent-123".to_string(),
            fork_event_index: 42,
        };
        let json = serde_json::to_string(&fork).unwrap();
        assert!(json.contains("parent-123"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_view_mode_serialization() {
        let chat = ViewMode::Chat;
        let terminal = ViewMode::Terminal;
        assert_eq!(serde_json::to_string(&chat).unwrap(), "\"chat\"");
        assert_eq!(serde_json::to_string(&terminal).unwrap(), "\"terminal\"");
    }

    #[test]
    fn test_session_source_variants() {
        let user = SessionSource::User;
        let workflow = SessionSource::Workflow {
            run_id: "run-1".to_string(),
            node_id: "n-1".to_string(),
        };
        assert_eq!(user, SessionSource::User);
        let json = serde_json::to_string(&workflow).unwrap();
        assert!(json.contains("workflow"));
        assert!(json.contains("run-1"));
    }
}
