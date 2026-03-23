use crate::agent_event::{AgentEvent, AgentEventType, AgentEventMetadata, EventContent, ErrorSeverity};
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

        // Build AgentEvent(s) from rule + JSON
        let raw_events = if let Some(ref blocks_path) = rule.content_blocks {
            self.build_events_from_blocks(rule, &value, blocks_path)
        } else {
            vec![self.build_event(rule, &value)]
        };

        for ev in &raw_events {
            log::debug!("Pipeline[{}]: built event type={:?} content_len={}", self.provider, ev.event_type,
                match &ev.content { crate::agent_event::EventContent::Text { value } => value.len(), _ => 0 });
        }

        // Stage 2 & 3: State Machine + Content Parser for each event
        let mut results = Vec::new();
        for event in raw_events {
            let enriched = self.state_machine.process(event);
            results.extend(enriched.into_iter().map(|e| self.enrich_content(e)));
        }
        results
    }

    /// Reset the pipeline state (e.g., new session).
    pub fn reset(&mut self) {
        self.state_machine.reset();
    }

    /// Build multiple events from an array of content blocks (e.g. Claude's message.content[]).
    /// Each block's `type` field determines the event type:
    ///   "thinking" → Thinking (content from .thinking)
    ///   "text"     → Text     (content from .text)
    ///   "tool_use" → ToolUse  (tool_name from .name, content from .input as JSON)
    ///   "tool_result" → ToolResult (content from .content)
    fn build_events_from_blocks(&self, rule: &Rule, value: &serde_json::Value, blocks_path: &str) -> Vec<AgentEvent> {
        let blocks = match resolve_path(value, blocks_path) {
            Some(arr) if arr.is_array() => arr.as_array().unwrap(),
            _ => {
                log::warn!("Pipeline[{}]: content_blocks path '{}' not found or not array, falling back to single event",
                    self.provider, blocks_path);
                return vec![self.build_event(rule, value)];
            }
        };

        let model = rule.mappings.get("model")
            .and_then(|path| resolve_path(value, path))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut events = Vec::new();
        for block in blocks {
            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

            let (event_type, content, tool_name) = match block_type {
                "thinking" => {
                    let text = block.get("thinking")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if text.is_empty() { continue; }
                    (AgentEventType::Thinking, EventContent::Text { value: text.to_string() }, None)
                }
                "text" => {
                    let text = block.get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if text.is_empty() { continue; }
                    (AgentEventType::Text, EventContent::Text { value: text.to_string() }, None)
                }
                "tool_use" => {
                    let name = block.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let input = block.get("input")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    (AgentEventType::ToolUse, EventContent::Json { value: input }, Some(name))
                }
                "tool_result" => {
                    let text = block.get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    (AgentEventType::ToolResult, EventContent::Text { value: text.to_string() }, None)
                }
                _ => {
                    log::trace!("Pipeline[{}]: skipping unknown content block type '{}'", self.provider, block_type);
                    continue;
                }
            };

            let metadata = AgentEventMetadata {
                session_id: None,
                input_tokens: None,
                output_tokens: None,
                tool_name,
                model: model.clone(),
                provider: self.provider.clone(),
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

            events.push(AgentEvent {
                id: Uuid::new_v4().to_string(),
                parent_id: None,
                event_type,
                content,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                metadata,
            });
        }

        if events.is_empty() {
            log::warn!("Pipeline[{}]: content_blocks produced 0 events, falling back to single event", self.provider);
            vec![self.build_event(rule, value)]
        } else {
            events
        }
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
                content_blocks: None,
            },
            Rule {
                name: "done".into(),
                when: r#"type == "message_stop""#.into(),
                emit: "done".into(),
                mappings: Default::default(),
                content_blocks: None,
            },
            Rule {
                name: "usage".into(),
                when: r#"type == "message_delta""#.into(),
                emit: "usage".into(),
                mappings: [
                    ("input_tokens".to_string(), "usage.input_tokens".to_string()),
                    ("output_tokens".to_string(), "usage.output_tokens".to_string()),
                ].into(),
                content_blocks: None,
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
                content_blocks: None,
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
                content_blocks: None,
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
                content_blocks: None,
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

    #[test]
    fn test_pipeline_content_blocks_thinking_and_text() {
        let rules = vec![
            Rule {
                name: "assistant_content".into(),
                when: r#"type == "assistant" && exists(message.content)"#.into(),
                emit: "text".into(),
                mappings: [("model".to_string(), "message.model".to_string())].into(),
                content_blocks: Some("message.content".to_string()),
            },
        ];
        let mut pipeline = NormalizerPipeline::new(
            rules,
            Box::new(GenericStateMachine::new()),
            "claude".to_string(),
        );
        let input = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-20250514","content":[{"type":"thinking","thinking":"Let me think about this..."},{"type":"text","text":"The answer is 4."}]}}"#;
        let events = pipeline.process(input);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, AgentEventType::Thinking);
        assert!(matches!(&events[0].content, EventContent::Text { value } if value == "Let me think about this..."));
        assert_eq!(events[0].metadata.model, Some("claude-sonnet-4-20250514".to_string()));
        assert_eq!(events[1].event_type, AgentEventType::Text);
        assert!(matches!(&events[1].content, EventContent::Text { value } if value == "The answer is 4."));
    }

    #[test]
    fn test_pipeline_content_blocks_with_tool_use() {
        let rules = vec![
            Rule {
                name: "assistant_content".into(),
                when: r#"type == "assistant" && exists(message.content)"#.into(),
                emit: "text".into(),
                mappings: [("model".to_string(), "message.model".to_string())].into(),
                content_blocks: Some("message.content".to_string()),
            },
        ];
        let mut pipeline = NormalizerPipeline::new(
            rules,
            Box::new(GenericStateMachine::new()),
            "claude".to_string(),
        );
        let input = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-20250514","content":[{"type":"text","text":"I'll read that file."},{"type":"tool_use","id":"tu_1","name":"Read","input":{"path":"src/main.rs"}}]}}"#;
        let events = pipeline.process(input);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, AgentEventType::Text);
        assert_eq!(events[1].event_type, AgentEventType::ToolUse);
        assert_eq!(events[1].metadata.tool_name, Some("Read".to_string()));
        assert!(matches!(&events[1].content, EventContent::Json { value } if value.get("path").is_some()));
    }

    #[test]
    fn test_pipeline_content_blocks_empty_thinking_skipped() {
        let rules = vec![
            Rule {
                name: "assistant_content".into(),
                when: r#"type == "assistant" && exists(message.content)"#.into(),
                emit: "text".into(),
                mappings: Default::default(),
                content_blocks: Some("message.content".to_string()),
            },
        ];
        let mut pipeline = NormalizerPipeline::new(
            rules,
            Box::new(GenericStateMachine::new()),
            "claude".to_string(),
        );
        let input = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":""},{"type":"text","text":"Hello"}]}}"#;
        let events = pipeline.process(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AgentEventType::Text);
    }
}
