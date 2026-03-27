use crate::agent_event::{AgentEvent, ErrorSeverity};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub prompt: String,
    pub provider: String,
    pub model: Option<String>,
    #[serde(default)]
    pub context: Vec<ContextItem>,
    pub session_id: Option<String>,
    pub system_prompt: Option<String>,
    pub max_tokens: Option<u64>,
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Working directory for the CLI process (typically the project root).
    #[serde(default)]
    pub cwd: Option<String>,
    /// When true, append permission_args (e.g. --dangerously-skip-permissions).
    #[serde(default)]
    pub yolo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContextItem {
    File {
        path: String,
        content: String,
        language: Option<String>,
    },
    Selection {
        file_path: String,
        start_line: u32,
        end_line: u32,
        content: String,
    },
    PreviousOutput {
        agent_id: String,
        events: Vec<AgentEvent>,
    },
    Custom {
        label: String,
        content: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliMode {
    Structured,
    BasicPrint,
    PtyOnly,
    DirectApi,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Idle,
    Resumable,
    Terminated,
    Error { severity: ErrorSeverity },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)] // Roadmap: used for agent command protocol
pub enum AgentCommand {
    Stop,
    Cancel,
    Pause,
    Resume,
    Interrupt { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_request_serialization() {
        let req = AgentRequest {
            prompt: "Hello".to_string(),
            provider: "claude".to_string(),
            model: Some("claude-sonnet-4-6".to_string()),
            context: vec![],
            session_id: None,
            system_prompt: None,
            max_tokens: None,
            allowed_tools: None,
            cwd: None,
            yolo: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: AgentRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.prompt, "Hello");
        assert_eq!(deserialized.provider, "claude");
    }

    #[test]
    fn test_context_item_file() {
        let item = ContextItem::File {
            path: "src/main.rs".to_string(),
            content: "fn main() {}".to_string(),
            language: Some("rust".to_string()),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"file\""));
    }

    #[test]
    fn test_cli_mode_serialization() {
        let mode = CliMode::Structured;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"structured\"");
    }

    #[test]
    fn test_session_status_variants() {
        let active = SessionStatus::Active;
        let error = SessionStatus::Error {
            severity: ErrorSeverity::Fatal,
        };
        assert_eq!(active, SessionStatus::Active);
        assert_ne!(active, SessionStatus::Terminated);
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("fatal"));
    }

    #[test]
    fn test_agent_command_serialization() {
        let cmd = AgentCommand::Interrupt {
            message: "stop now".to_string(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("interrupt"));
        assert!(json.contains("stop now"));
    }
}
