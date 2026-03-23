use crate::agent_event::{AgentEvent, AgentEventType, AgentEventMetadata, EventContent, ErrorSeverity, StreamMetrics};
use crate::normalizer::rules_engine::{Rule, find_matching_rule, resolve_path};
use crate::normalizer::content_parser::parse_content;
use crate::normalizer::state_machines::StateMachine;
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

/// Three-stage normalizer pipeline: Rules → State Machine → Content Parser.
pub struct NormalizerPipeline {
    rules: Vec<Rule>,
    state_machine: Box<dyn StateMachine>,
    provider: String,
}

impl NormalizerPipeline {
    pub fn new(rules: Vec<Rule>, state_machine: Box<dyn StateMachine>, provider: String) -> Self {
        Self { rules, state_machine, provider }
    }

    /// Process a single line of JSON from the CLI stdout.
    /// Returns 0 or more AgentEvents.
    pub fn process(&mut self, raw_json: &str) -> Vec<AgentEvent> {
        // Parse JSON
        let value: serde_json::Value = match serde_json::from_str(raw_json) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        // Stage 1: Rules Engine — find matching rule
        let rule = match find_matching_rule(&self.rules, &value) {
            Some(r) => {
                log::debug!("Pipeline[{}]: matched rule '{}' → emit '{}'", self.provider, r.name, r.emit);
                r
            }
            None => {
                let json_type = value.get("type").and_then(|t| t.as_str()).unwrap_or("?");
                log::trace!("Pipeline[{}]: no rule matched for type='{}'", self.provider, json_type);
                return vec![];
            }
        };

        // Build AgentEvent from rule + JSON
        let event = self.build_event(rule, &value);
        log::debug!("Pipeline[{}]: built event type={:?} content_len={}", self.provider, event.event_type,
            match &event.content { crate::agent_event::EventContent::Text { value } => value.len(), _ => 0 });

        // Stage 2: State Machine — accumulate/flush
        let events = self.state_machine.process(event);

        // Stage 3: Content Parser — enrich text content
        events.into_iter().map(|e| self.enrich_content(e)).collect()
    }

    /// Reset the pipeline state (e.g., new session).
    pub fn reset(&mut self) {
        self.state_machine.reset();
    }

    fn build_event(&self, rule: &Rule, value: &serde_json::Value) -> AgentEvent {
        let event_type = match rule.emit.as_str() {
            "text" => AgentEventType::Text,
            "thinking" => AgentEventType::Thinking,
            "tool_use" => AgentEventType::ToolUse,
            "tool_result" => AgentEventType::ToolResult,
            "error" => AgentEventType::Error,
            "status" => AgentEventType::Status,
            "usage" => AgentEventType::Usage,
            "metrics" => AgentEventType::Metrics,
            "done" => AgentEventType::Done,
            _ => AgentEventType::Text,
        };

        // Extract content from mapping
        let content_str = rule.mappings.get("content")
            .and_then(|path| resolve_path(value, path))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let content = EventContent::Text { value: content_str };

        // Build metadata
        let metadata = AgentEventMetadata {
            session_id: rule.mappings.get("session_id")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            input_tokens: rule.mappings.get("input_tokens")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            output_tokens: rule.mappings.get("output_tokens")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            tool_name: rule.mappings.get("tool_name")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            model: rule.mappings.get("model")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            provider: self.provider.clone(),
            error_severity: rule.mappings.get("severity").map(|s| match s.as_str() {
                "recoverable" => ErrorSeverity::Recoverable,
                "degraded" => ErrorSeverity::Degraded,
                _ => ErrorSeverity::Fatal,
            }),
            error_code: rule.mappings.get("error_code")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            stream_metrics: None,
            incomplete: None,
            cache_creation_tokens: rule.mappings.get("cache_creation_tokens")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            cache_read_tokens: rule.mappings.get("cache_read_tokens")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            duration_ms: rule.mappings.get("duration_ms")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            duration_api_ms: rule.mappings.get("duration_api_ms")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            num_turns: rule.mappings.get("num_turns")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64().map(|n| n as u32)),
            stop_reason: rule.mappings.get("stop_reason")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            context_usage: rule.mappings.get("context_usage")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_f64()),
            context_tokens: rule.mappings.get("context_tokens")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            max_context_tokens: rule.mappings.get("max_context_tokens")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            total_cost_usd: rule.mappings.get("total_cost_usd")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_f64()),
        };

        let parent_id = rule.mappings.get("parent_id")
            .and_then(|path| resolve_path(value, path))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        AgentEvent {
            id: Uuid::new_v4().to_string(),
            parent_id,
            event_type,
            content,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            metadata,
        }
    }

    /// Stage 3: Content Parser — detects code blocks, diffs in text content.
    fn enrich_content(&self, mut event: AgentEvent) -> AgentEvent {
        if event.event_type == AgentEventType::Text {
            if let EventContent::Text { ref value } = event.content {
                if !value.is_empty() {
                    event.content = parse_content(value);
                }
            }
        }
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalizer::state_machines::generic::GenericStateMachine;
    use serde_json::json;

    fn claude_text_rules() -> Vec<Rule> {
        vec![
            Rule {
                name: "text".into(),
                when: r#"type == "content_block_delta" && delta.type == "text_delta""#.into(),
                emit: "text".into(),
                mappings: [("content".to_string(), "delta.text".to_string())].into(),
            },
            Rule {
                name: "done".into(),
                when: r#"type == "message_stop""#.into(),
                emit: "done".into(),
                mappings: Default::default(),
            },
            Rule {
                name: "usage".into(),
                when: r#"type == "message_delta""#.into(),
                emit: "usage".into(),
                mappings: [
                    ("input_tokens".to_string(), "usage.input_tokens".to_string()),
                    ("output_tokens".to_string(), "usage.output_tokens".to_string()),
                ].into(),
            },
            Rule {
                name: "error".into(),
                when: r#"type == "error""#.into(),
                emit: "error".into(),
                mappings: [
                    ("content".to_string(), "error.message".to_string()),
                    ("error_code".to_string(), "error.type".to_string()),
                    ("severity".to_string(), "fatal".to_string()),
                ].into(),
            },
        ]
    }

    #[test]
    fn test_pipeline_text_event() {
        let mut pipeline = NormalizerPipeline::new(
            claude_text_rules(),
            Box::new(GenericStateMachine::new()),
            "claude".to_string(),
        );
        let input = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}"#;
        let events = pipeline.process(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AgentEventType::Text);
        assert!(matches!(&events[0].content, EventContent::Text { value } if value == "Hello"));
    }

    #[test]
    fn test_pipeline_done_event() {
        let mut pipeline = NormalizerPipeline::new(
            claude_text_rules(),
            Box::new(GenericStateMachine::new()),
            "claude".to_string(),
        );
        let input = r#"{"type":"message_stop"}"#;
        let events = pipeline.process(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AgentEventType::Done);
    }

    #[test]
    fn test_pipeline_usage_event() {
        let mut pipeline = NormalizerPipeline::new(
            claude_text_rules(),
            Box::new(GenericStateMachine::new()),
            "claude".to_string(),
        );
        let input = r#"{"type":"message_delta","usage":{"input_tokens":100,"output_tokens":250}}"#;
        let events = pipeline.process(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AgentEventType::Usage);
        assert_eq!(events[0].metadata.input_tokens, Some(100));
        assert_eq!(events[0].metadata.output_tokens, Some(250));
    }

    #[test]
    fn test_pipeline_no_match_returns_empty() {
        let mut pipeline = NormalizerPipeline::new(
            claude_text_rules(),
            Box::new(GenericStateMachine::new()),
            "claude".to_string(),
        );
        let input = r#"{"type":"ping"}"#;
        let events = pipeline.process(input);
        assert!(events.is_empty());
    }

    #[test]
    fn test_pipeline_invalid_json_returns_empty() {
        let mut pipeline = NormalizerPipeline::new(
            claude_text_rules(),
            Box::new(GenericStateMachine::new()),
            "claude".to_string(),
        );
        let events = pipeline.process("not json at all");
        assert!(events.is_empty());
    }

    #[test]
    fn test_pipeline_error_event() {
        let mut pipeline = NormalizerPipeline::new(
            claude_text_rules(),
            Box::new(GenericStateMachine::new()),
            "claude".to_string(),
        );
        let input = r#"{"type":"error","error":{"type":"overloaded","message":"Server busy"}}"#;
        let events = pipeline.process(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AgentEventType::Error);
        assert_eq!(events[0].metadata.error_code, Some("overloaded".to_string()));
    }

    #[test]
    fn test_pipeline_code_block_in_text() {
        let mut pipeline = NormalizerPipeline::new(
            claude_text_rules(),
            Box::new(GenericStateMachine::new()),
            "claude".to_string(),
        );
        let input = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"```rust\nfn main() {}\n```"}}"#;
        let events = pipeline.process(input);
        assert_eq!(events.len(), 1);
        // Content parser detects code block
        assert!(matches!(&events[0].content, EventContent::Code { language, .. } if language == "rust"));
    }

    #[test]
    fn test_pipeline_extracts_cache_and_duration() {
        let rules = vec![
            Rule {
                name: "result".into(),
                when: r#"type == "result""#.into(),
                emit: "usage".into(),
                mappings: [
                    ("cache_creation_tokens".to_string(), "usage.cache_creation_input_tokens".to_string()),
                    ("cache_read_tokens".to_string(), "usage.cache_read_input_tokens".to_string()),
                    ("duration_ms".to_string(), "duration_ms".to_string()),
                    ("duration_api_ms".to_string(), "duration_api_ms".to_string()),
                    ("num_turns".to_string(), "num_turns".to_string()),
                    ("stop_reason".to_string(), "stop_reason".to_string()),
                    ("total_cost_usd".to_string(), "total_cost_usd".to_string()),
                ].into(),
            },
        ];
        let mut pipeline = NormalizerPipeline::new(
            rules,
            Box::new(GenericStateMachine::new()),
            "claude".to_string(),
        );
        let input = r#"{"type":"result","duration_ms":4105,"duration_api_ms":4089,"num_turns":1,"stop_reason":"end_turn","total_cost_usd":0.055,"usage":{"cache_creation_input_tokens":7727,"cache_read_input_tokens":15092}}"#;
        let events = pipeline.process(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].metadata.cache_creation_tokens, Some(7727));
        assert_eq!(events[0].metadata.cache_read_tokens, Some(15092));
        assert_eq!(events[0].metadata.duration_ms, Some(4105));
        assert_eq!(events[0].metadata.duration_api_ms, Some(4089));
        assert_eq!(events[0].metadata.num_turns, Some(1));
        assert_eq!(events[0].metadata.stop_reason, Some("end_turn".to_string()));
        assert_eq!(events[0].metadata.total_cost_usd, Some(0.055));
    }

    #[test]
    fn test_pipeline_extracts_context_usage() {
        let rules = vec![
            Rule {
                name: "context".into(),
                when: r#"type == "metrics""#.into(),
                emit: "metrics".into(),
                mappings: [
                    ("context_usage".to_string(), "context_usage".to_string()),
                    ("context_tokens".to_string(), "context_tokens".to_string()),
                    ("max_context_tokens".to_string(), "max_context_tokens".to_string()),
                ].into(),
            },
        ];
        let mut pipeline = NormalizerPipeline::new(
            rules,
            Box::new(GenericStateMachine::new()),
            "kimi".to_string(),
        );
        let input = r#"{"type":"metrics","context_usage":0.75,"context_tokens":96000,"max_context_tokens":128000}"#;
        let events = pipeline.process(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].metadata.context_usage, Some(0.75));
        assert_eq!(events[0].metadata.context_tokens, Some(96000));
        assert_eq!(events[0].metadata.max_context_tokens, Some(128000));
    }
}
