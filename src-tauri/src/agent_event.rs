use log::trace;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventType {
    Text,
    ToolUse,
    ToolResult,
    Thinking,
    Error,
    Status,
    Usage,
    Metrics,
    PermissionDenial,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventContent {
    Text { value: String },
    Code { language: String, source: String },
    Diff { file_path: String, hunks: Vec<DiffHunk> },
    FileRef { path: String, action: FileAction },
    Json { value: serde_json::Value },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    Read,
    Write,
    Create,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub new_start: u32,
    pub old_lines: Vec<String>,
    pub new_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Recoverable,
    Degraded,
    Fatal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetrics {
    pub tokens_so_far: u64,
    pub elapsed_ms: u64,
    pub tokens_per_second: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventMetadata {
    pub session_id: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub tool_name: Option<String>,
    pub model: Option<String>,
    pub provider: String,
    pub error_severity: Option<ErrorSeverity>,
    pub error_code: Option<String>,
    pub stream_metrics: Option<StreamMetrics>,
    #[serde(default)]
    pub incomplete: Option<bool>,
    #[serde(default)]
    pub cache_creation_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_tokens: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub duration_api_ms: Option<u64>,
    #[serde(default)]
    pub num_turns: Option<u32>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub context_usage: Option<f64>,
    #[serde(default)]
    pub context_tokens: Option<u64>,
    #[serde(default)]
    pub max_context_tokens: Option<u64>,
    #[serde(default)]
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: String,
    pub parent_id: Option<String>,
    pub event_type: AgentEventType,
    pub content: EventContent,
    pub timestamp: u64,
    pub metadata: AgentEventMetadata,
}

impl AgentEvent {
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn base_metadata(provider: &str) -> AgentEventMetadata {
        AgentEventMetadata {
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            tool_name: None,
            model: None,
            provider: provider.to_string(),
            error_severity: None,
            error_code: None,
            stream_metrics: None,
            incomplete: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            duration_ms: None,
            duration_api_ms: None,
            num_turns: None,
            stop_reason: None,
            context_usage: None,
            context_tokens: None,
            max_context_tokens: None,
            total_cost_usd: None,
        }
    }

    pub fn text(content: &str, provider: &str) -> Self {
        trace!("AgentEvent::text created for provider='{}'", provider);
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::Text,
            content: EventContent::Text { value: content.to_string() },
            timestamp: Self::now(),
            metadata: Self::base_metadata(provider),
        }
    }

    pub fn thinking(content: &str, provider: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::Thinking,
            content: EventContent::Text { value: content.to_string() },
            timestamp: Self::now(),
            metadata: Self::base_metadata(provider),
        }
    }

    pub fn error(message: &str, code: &str, severity: ErrorSeverity, provider: &str) -> Self {
        trace!("AgentEvent::error created for provider='{}', code='{}', severity={:?}", provider, code, severity);
        let mut meta = Self::base_metadata(provider);
        meta.error_severity = Some(severity);
        meta.error_code = Some(code.to_string());
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::Error,
            content: EventContent::Text { value: message.to_string() },
            timestamp: Self::now(),
            metadata: meta,
        }
    }

    pub fn tool_use(tool_name: &str, input: &str, provider: &str) -> Self {
        trace!("AgentEvent::tool_use created: tool='{}', provider='{}'", tool_name, provider);
        let mut meta = Self::base_metadata(provider);
        meta.tool_name = Some(tool_name.to_string());
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::ToolUse,
            content: EventContent::Json {
                value: serde_json::from_str(input).unwrap_or(serde_json::Value::String(input.to_string())),
            },
            timestamp: Self::now(),
            metadata: meta,
        }
    }

    pub fn tool_result(content: &str, parent_id: &str, provider: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id: Some(parent_id.to_string()),
            event_type: AgentEventType::ToolResult,
            content: EventContent::Text { value: content.to_string() },
            timestamp: Self::now(),
            metadata: Self::base_metadata(provider),
        }
    }

    pub fn usage(input_tokens: u64, output_tokens: u64, provider: &str) -> Self {
        let mut meta = Self::base_metadata(provider);
        meta.input_tokens = Some(input_tokens);
        meta.output_tokens = Some(output_tokens);
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::Usage,
            content: EventContent::Text { value: String::new() },
            timestamp: Self::now(),
            metadata: meta,
        }
    }

    pub fn done(session_id: &str, provider: &str) -> Self {
        trace!("AgentEvent::done created for session='{}', provider='{}'", session_id, provider);
        let mut meta = Self::base_metadata(provider);
        meta.session_id = Some(session_id.to_string());
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::Done,
            content: EventContent::Text { value: String::new() },
            timestamp: Self::now(),
            metadata: meta,
        }
    }

    pub fn metrics(metrics: StreamMetrics, provider: &str) -> Self {
        let mut meta = Self::base_metadata(provider);
        meta.stream_metrics = Some(metrics);
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::Metrics,
            content: EventContent::Text { value: String::new() },
            timestamp: Self::now(),
            metadata: meta,
        }
    }

    pub fn permission_denial(denials_json: serde_json::Value, provider: &str) -> Self {
        trace!("AgentEvent::permission_denial created for provider='{}'", provider);
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::PermissionDenial,
            content: EventContent::Json { value: denials_json },
            timestamp: Self::now(),
            metadata: Self::base_metadata(provider),
        }
    }

    pub fn status(status_text: &str, provider: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::Status,
            content: EventContent::Text { value: status_text.to_string() },
            timestamp: Self::now(),
            metadata: Self::base_metadata(provider),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_event_creation() {
        let event = AgentEvent::text("hello world", "claude");
        assert_eq!(event.event_type, AgentEventType::Text);
        assert!(matches!(event.content, EventContent::Text { ref value } if value == "hello world"));
        assert_eq!(event.metadata.provider, "claude");
        assert!(!event.id.is_empty());
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_error_event_with_severity() {
        let event = AgentEvent::error(
            "rate limit exceeded",
            "overloaded",
            ErrorSeverity::Recoverable,
            "claude",
        );
        assert_eq!(event.event_type, AgentEventType::Error);
        assert_eq!(event.metadata.error_severity, Some(ErrorSeverity::Recoverable));
        assert_eq!(event.metadata.error_code, Some("overloaded".to_string()));
    }

    #[test]
    fn test_tool_use_with_parent_id() {
        let parent = AgentEvent::tool_use("read_file", r#"{"path":"test.rs"}"#, "claude");
        let child = AgentEvent::tool_result("file contents here", &parent.id, "claude");
        assert_eq!(child.parent_id, Some(parent.id.clone()));
    }

    #[test]
    fn test_usage_event() {
        let event = AgentEvent::usage(100, 250, "claude");
        assert_eq!(event.metadata.input_tokens, Some(100));
        assert_eq!(event.metadata.output_tokens, Some(250));
    }

    #[test]
    fn test_done_event() {
        let event = AgentEvent::done("session-123", "claude");
        assert_eq!(event.event_type, AgentEventType::Done);
        assert_eq!(event.metadata.session_id, Some("session-123".to_string()));
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let event = AgentEvent::text("hello", "claude");
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: AgentEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, event.id);
        assert_eq!(deserialized.event_type, event.event_type);
    }

    #[test]
    fn test_event_content_code() {
        let content = EventContent::Code {
            language: "rust".to_string(),
            source: "fn main() {}".to_string(),
        };
        assert!(matches!(content, EventContent::Code { ref language, .. } if language == "rust"));
    }

    #[test]
    fn test_event_content_diff() {
        let content = EventContent::Diff {
            file_path: "src/main.rs".to_string(),
            hunks: vec![DiffHunk {
                old_start: 1,
                new_start: 1,
                old_lines: vec!["old line".to_string()],
                new_lines: vec!["new line".to_string()],
            }],
        };
        assert!(matches!(content, EventContent::Diff { .. }));
    }

    #[test]
    fn test_metadata_new_fields_default_none() {
        let meta = AgentEventMetadata {
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            tool_name: None,
            model: None,
            provider: "claude".to_string(),
            error_severity: None,
            error_code: None,
            stream_metrics: None,
            incomplete: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            duration_ms: None,
            duration_api_ms: None,
            num_turns: None,
            stop_reason: None,
            context_usage: None,
            context_tokens: None,
            max_context_tokens: None,
            total_cost_usd: None,
        };
        assert!(meta.cache_creation_tokens.is_none());
        assert!(meta.total_cost_usd.is_none());
    }

    #[test]
    fn test_metadata_serialization_with_new_fields() {
        let mut meta = AgentEvent::base_metadata("claude");
        meta.cache_creation_tokens = Some(500);
        meta.cache_read_tokens = Some(1000);
        meta.duration_ms = Some(4105);
        meta.total_cost_usd = Some(0.055);
        meta.num_turns = Some(3);
        meta.stop_reason = Some("end_turn".to_string());

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("cache_creation_tokens"));
        assert!(json.contains("500"));
        assert!(json.contains("total_cost_usd"));

        let deserialized: AgentEventMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cache_creation_tokens, Some(500));
        assert_eq!(deserialized.total_cost_usd, Some(0.055));
        assert_eq!(deserialized.num_turns, Some(3));
    }

    #[test]
    fn test_metadata_deserialization_without_new_fields() {
        // Old JSON without new fields should deserialize with None defaults
        let json = r#"{"provider":"claude"}"#;
        let meta: AgentEventMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.provider, "claude");
        assert!(meta.cache_creation_tokens.is_none());
        assert!(meta.duration_ms.is_none());
        assert!(meta.total_cost_usd.is_none());
    }
}
