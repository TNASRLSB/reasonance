use super::StateMachine;
use crate::agent_event::{AgentEvent, AgentEventType, EventContent};

/// Claude-specific state machine.
/// Handles content_block lifecycle: start → N deltas → stop.
/// Tool use events are accumulated until the block is complete.
pub struct ClaudeStateMachine {
    pending_tool: Option<AgentEvent>,
    accumulated_input: String,
}

impl ClaudeStateMachine {
    pub fn new() -> Self {
        Self {
            pending_tool: None,
            accumulated_input: String::new(),
        }
    }
}

impl StateMachine for ClaudeStateMachine {
    fn process(&mut self, event: AgentEvent) -> Vec<AgentEvent> {
        match event.event_type {
            AgentEventType::ToolUse => {
                if self.pending_tool.is_none() {
                    self.pending_tool = Some(event);
                    self.accumulated_input.clear();
                    vec![]
                } else {
                    if let EventContent::Text { ref value } = event.content {
                        self.accumulated_input.push_str(value);
                    } else if let EventContent::Json { ref value } = event.content {
                        self.accumulated_input.push_str(&value.to_string());
                    }
                    vec![]
                }
            }
            AgentEventType::Status => {
                if let Some(mut tool_event) = self.pending_tool.take() {
                    if !self.accumulated_input.is_empty() {
                        let parsed = serde_json::from_str(&self.accumulated_input)
                            .unwrap_or(serde_json::Value::String(self.accumulated_input.clone()));
                        tool_event.content = EventContent::Json { value: parsed };
                    }
                    self.accumulated_input.clear();
                    vec![tool_event]
                } else {
                    vec![event]
                }
            }
            _ => vec![event],
        }
    }

    fn reset(&mut self) {
        self.pending_tool = None;
        self.accumulated_input.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_events_pass_through() {
        let mut sm = ClaudeStateMachine::new();
        let event = AgentEvent::text("hello", "claude");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_tool_use_accumulates() {
        let mut sm = ClaudeStateMachine::new();

        // Start tool use
        let start = AgentEvent::tool_use("read_file", "{}", "claude");
        let result = sm.process(start);
        assert_eq!(result.len(), 0); // accumulating

        // Append input chunks
        let mut chunk = AgentEvent::text(r#"{"path":"#, "claude");
        chunk.event_type = AgentEventType::ToolUse;
        let result = sm.process(chunk);
        assert_eq!(result.len(), 0); // still accumulating

        // Finish — emits accumulated tool_use
        let done = AgentEvent::status("content_block_stop", "claude");
        let result = sm.process(done);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
    }

    #[test]
    fn test_reset_clears_accumulator() {
        let mut sm = ClaudeStateMachine::new();
        let start = AgentEvent::tool_use("read_file", "{}", "claude");
        sm.process(start);
        sm.reset();
        // After reset, no pending events
        let text = AgentEvent::text("hello", "claude");
        let result = sm.process(text);
        assert_eq!(result.len(), 1); // passes through normally
    }

    #[test]
    fn test_done_and_usage_pass_through() {
        let mut sm = ClaudeStateMachine::new();

        let usage = AgentEvent::usage(100, 200, "claude");
        assert_eq!(sm.process(usage).len(), 1);

        let done = AgentEvent::done("session-1", "claude");
        assert_eq!(sm.process(done).len(), 1);
    }

    #[test]
    fn test_error_passes_through() {
        let mut sm = ClaudeStateMachine::new();
        let error = AgentEvent::error(
            "bad",
            "err",
            crate::agent_event::ErrorSeverity::Fatal,
            "claude",
        );
        assert_eq!(sm.process(error).len(), 1);
    }
}
