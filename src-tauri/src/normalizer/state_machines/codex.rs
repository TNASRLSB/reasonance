use super::accumulator::{TextAccumulator, TimedFlush, ToolInputAccumulator};
use super::StateMachine;
use crate::agent_event::{AgentEvent, AgentEventType, EventContent};
use std::time::Duration;

const FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

/// Codex state machine.
/// JSON-RPC v2 protocol. Text deltas (AgentMessageDeltaNotification) accumulate
/// until a non-delta event. ItemCompletedNotification events (reasoning,
/// commandExecution, mcpToolCall) arrive pre-assembled and pass through after
/// flushing any pending text.
pub struct CodexStateMachine {
    text_accumulator: TextAccumulator,
    tool_accumulator: ToolInputAccumulator,
    timed_flush: TimedFlush,
    provider: String,
}

impl CodexStateMachine {
    pub fn new() -> Self {
        Self {
            text_accumulator: TextAccumulator::new(),
            tool_accumulator: ToolInputAccumulator::new(),
            timed_flush: TimedFlush::new(FLUSH_TIMEOUT),
            provider: "codex".to_string(),
        }
    }

    fn flush_pending_text(&mut self) -> Option<AgentEvent> {
        if !self.text_accumulator.is_empty() {
            let text = self.text_accumulator.take();
            Some(AgentEvent::text(&text, &self.provider))
        } else {
            None
        }
    }

    fn flush_all_with_incomplete(&mut self) -> Vec<AgentEvent> {
        let mut result = Vec::new();
        if !self.text_accumulator.is_empty() {
            let text = self.text_accumulator.take();
            let mut event = AgentEvent::text(&text, &self.provider);
            event.metadata.incomplete = Some(true);
            result.push(event);
        }
        if let Some(mut tool_event) = self.tool_accumulator.finalize() {
            tool_event.metadata.incomplete = Some(true);
            result.push(tool_event);
        }
        result
    }
}

impl StateMachine for CodexStateMachine {
    fn process(&mut self, event: AgentEvent) -> Vec<AgentEvent> {
        // Check timeout
        if self.timed_flush.is_expired()
            && (!self.text_accumulator.is_empty() || self.tool_accumulator.is_active())
        {
            let mut flushed = self.flush_all_with_incomplete();
            flushed.push(event);
            return flushed;
        }
        self.timed_flush.touch();

        match event.event_type {
            AgentEventType::Text => {
                // Accumulate text deltas
                if let EventContent::Text { ref value } = event.content {
                    self.text_accumulator.push(value);
                }
                vec![]
            }
            AgentEventType::Thinking | AgentEventType::ToolUse | AgentEventType::ToolResult => {
                let mut result = Vec::new();
                if let Some(text_event) = self.flush_pending_text() {
                    result.push(text_event);
                }
                result.push(event);
                result
            }
            AgentEventType::Usage | AgentEventType::Done | AgentEventType::Error => {
                let mut result = Vec::new();
                if let Some(text_event) = self.flush_pending_text() {
                    result.push(text_event);
                }
                result.push(event);
                result
            }
            _ => vec![event],
        }
    }

    fn reset(&mut self) {
        self.text_accumulator = TextAccumulator::new();
        self.tool_accumulator.reset();
        self.timed_flush = TimedFlush::new(FLUSH_TIMEOUT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::AgentEventType;

    #[test]
    fn test_single_text_delta_accumulates() {
        let mut sm = CodexStateMachine::new();
        let event = AgentEvent::text("hello", "codex");
        let result = sm.process(event);
        assert_eq!(result.len(), 0, "text delta should accumulate, not emit");
    }

    #[test]
    fn test_text_deltas_flush_on_done() {
        let mut sm = CodexStateMachine::new();

        let result = sm.process(AgentEvent::text("hello ", "codex"));
        assert_eq!(result.len(), 0);

        let result = sm.process(AgentEvent::text("world", "codex"));
        assert_eq!(result.len(), 0);

        let done = AgentEvent::done("", "codex");
        let result = sm.process(done);

        assert_eq!(result.len(), 2, "should emit flushed text + done");
        assert_eq!(result[0].event_type, AgentEventType::Text);
        if let EventContent::Text { ref value } = result[0].content {
            assert_eq!(value, "hello world");
        } else {
            panic!("expected text content");
        }
        assert_eq!(result[1].event_type, AgentEventType::Done);
    }

    #[test]
    fn test_thinking_flushes_pending_text() {
        let mut sm = CodexStateMachine::new();

        let result = sm.process(AgentEvent::text("pending", "codex"));
        assert_eq!(result.len(), 0);

        let thinking = AgentEvent::thinking("step 1", "codex");
        let result = sm.process(thinking);

        assert_eq!(result.len(), 2, "should emit flushed text + thinking");
        assert_eq!(result[0].event_type, AgentEventType::Text);
        assert_eq!(result[1].event_type, AgentEventType::Thinking);
    }

    #[test]
    fn test_tool_use_passes_through() {
        let mut sm = CodexStateMachine::new();

        // Tool use events are pre-assembled by rules and pass through directly
        let tool = AgentEvent::tool_use("bash", r#"{"cmd":"ls"}"#, "codex");
        let result = sm.process(tool);

        assert_eq!(result.len(), 1, "tool_use should pass through directly");
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
    }

    #[test]
    fn test_reset_clears_all() {
        let mut sm = CodexStateMachine::new();

        // Accumulate some text
        sm.process(AgentEvent::text("pending text", "codex"));
        assert!(!sm.text_accumulator.is_empty());

        sm.reset();
        assert!(sm.text_accumulator.is_empty());

        // After reset, Done should emit only Done (no accumulated text)
        let done = AgentEvent::done("", "codex");
        let result = sm.process(done);

        assert_eq!(result.len(), 1, "after reset, done should emit only done");
        assert_eq!(result[0].event_type, AgentEventType::Done);
    }

    #[test]
    fn test_timeout_flush_emits_incomplete() {
        let mut sm = CodexStateMachine::new();
        sm.timed_flush = TimedFlush::new(Duration::from_millis(1));

        // Accumulate some text
        sm.process(AgentEvent::text("partial", "codex"));

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(5));

        // Next event triggers timeout flush
        let next = AgentEvent::text("next", "codex");
        let result = sm.process(next);

        assert!(
            result.len() >= 2,
            "should have flushed incomplete text + next event"
        );
        assert_eq!(result[0].event_type, AgentEventType::Text);
        assert_eq!(result[0].metadata.incomplete, Some(true));
    }
}
