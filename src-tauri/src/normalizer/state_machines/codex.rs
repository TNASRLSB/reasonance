use super::StateMachine;
use crate::agent_event::AgentEvent;

/// Codex state machine (v0.118.0+).
/// `codex exec --json` emits fully assembled item events (item.completed,
/// item.started) rather than streaming deltas. Text arrives as complete
/// `agent_message` items, reasoning as `reasoning` items, and tool use as
/// `command_execution` / `file_change` items. All events pass through
/// directly — no accumulation is needed.
pub struct CodexStateMachine {
    provider: String,
}

impl CodexStateMachine {
    pub fn new() -> Self {
        Self {
            provider: "codex".to_string(),
        }
    }
}

impl StateMachine for CodexStateMachine {
    fn process(&mut self, event: AgentEvent) -> Vec<AgentEvent> {
        // All events are pre-assembled; pass through directly.
        log::trace!("CodexStateMachine: pass-through {:?}", event.event_type);
        vec![event]
    }

    fn reset(&mut self) {
        // No state to clear.
        let _ = &self.provider;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::AgentEventType;

    #[test]
    fn test_text_passes_through() {
        let mut sm = CodexStateMachine::new();
        let event = AgentEvent::text("Hello from Codex", "codex");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Text);
    }

    #[test]
    fn test_thinking_passes_through() {
        let mut sm = CodexStateMachine::new();
        let event = AgentEvent::thinking("Step by step analysis", "codex");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Thinking);
    }

    #[test]
    fn test_tool_use_passes_through() {
        let mut sm = CodexStateMachine::new();
        let event = AgentEvent::tool_use("ls", r#"{"cmd":"ls"}"#, "codex");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
    }

    #[test]
    fn test_done_passes_through() {
        let mut sm = CodexStateMachine::new();
        let event = AgentEvent::done("", "codex");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Done);
    }

    #[test]
    fn test_usage_passes_through() {
        let mut sm = CodexStateMachine::new();
        let event = AgentEvent::usage(10, 5, "codex");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Usage);
    }

    #[test]
    fn test_error_passes_through() {
        let mut sm = CodexStateMachine::new();
        let event = AgentEvent::error(
            "rate limit",
            "rate_limit",
            crate::agent_event::ErrorSeverity::Recoverable,
            "codex",
        );
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Error);
    }

    #[test]
    fn test_reset_is_noop() {
        let mut sm = CodexStateMachine::new();
        sm.process(AgentEvent::text("hello", "codex"));
        sm.reset();
        // After reset, events still pass through
        let result = sm.process(AgentEvent::done("", "codex"));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Done);
    }
}
