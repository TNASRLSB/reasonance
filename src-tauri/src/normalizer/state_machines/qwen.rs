use super::accumulator::{TimedFlush, ToolInputAccumulator};
use super::StateMachine;
use crate::agent_event::{AgentEvent, AgentEventType, EventContent};
use std::time::Duration;

const FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

/// Qwen state machine.
/// Uses Claude-like content_block lifecycle: tool_use start → N input deltas → status (content_block_stop) → flush.
/// Key difference from Gemini: flushes only on Status events, not on any non-tool event.
/// No thinking support (unlike Kimi).
pub struct QwenStateMachine {
    tool_accumulator: ToolInputAccumulator,
    timed_flush: TimedFlush,
}

impl QwenStateMachine {
    pub fn new() -> Self {
        Self {
            tool_accumulator: ToolInputAccumulator::new(),
            timed_flush: TimedFlush::new(FLUSH_TIMEOUT),
        }
    }

    fn flush_with_incomplete(&mut self) -> Vec<AgentEvent> {
        if let Some(mut event) = self.tool_accumulator.finalize() {
            event.metadata.incomplete = Some(true);
            vec![event]
        } else {
            vec![]
        }
    }
}

impl StateMachine for QwenStateMachine {
    fn process(&mut self, event: AgentEvent) -> Vec<AgentEvent> {
        // Check timeout first
        if self.tool_accumulator.is_active() && self.timed_flush.is_expired() {
            let mut flushed = self.flush_with_incomplete();
            flushed.push(event);
            return flushed;
        }
        self.timed_flush.touch();

        match event.event_type {
            AgentEventType::ToolUse => {
                if self.tool_accumulator.is_active() {
                    // Input delta — accumulate the fragment
                    if let EventContent::Text { ref value } = event.content {
                        self.tool_accumulator.push_input(value);
                    } else if let EventContent::Json { ref value } = event.content {
                        self.tool_accumulator.push_input(&value.to_string());
                    }
                    vec![]
                } else {
                    // New tool_use start — begin accumulation
                    let tool_name = event.metadata.tool_name.clone().unwrap_or_default();
                    let parent_id = event.parent_id.clone();
                    let flushed =
                        self.tool_accumulator
                            .start(event, &tool_name, parent_id.as_deref());
                    flushed.into_iter().collect()
                }
            }
            AgentEventType::Status => {
                // content_block_stop: finalize the active tool if any, then pass status through
                if self.tool_accumulator.is_active() {
                    if let Some(tool_event) = self.tool_accumulator.finalize() {
                        return vec![tool_event];
                    }
                }
                vec![event]
            }
            // Text, Error, Usage, Done — pass through unchanged
            _ => vec![event],
        }
    }

    fn reset(&mut self) {
        self.tool_accumulator.reset();
        self.timed_flush = TimedFlush::new(FLUSH_TIMEOUT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_passes_through() {
        let mut sm = QwenStateMachine::new();
        let event = AgentEvent::text("hello", "qwen");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Text);
    }

    #[test]
    fn test_tool_accumulates_until_status() {
        let mut sm = QwenStateMachine::new();

        // tool_use start — begin accumulation
        let start = AgentEvent::tool_use("read_file", "{}", "qwen");
        let result = sm.process(start);
        assert_eq!(
            result.len(),
            0,
            "tool start should be accumulated, not emitted"
        );

        // input delta — still accumulating
        let mut chunk = AgentEvent::text(r#"{"path":"main.rs"}"#, "qwen");
        chunk.event_type = AgentEventType::ToolUse;
        let result = sm.process(chunk);
        assert_eq!(result.len(), 0, "input delta should be accumulated");

        // content_block_stop — flush assembled ToolUse
        let stop = AgentEvent::status("content_block_stop", "qwen");
        let result = sm.process(stop);
        assert_eq!(
            result.len(),
            1,
            "status should flush one assembled tool event"
        );
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
    }

    #[test]
    fn test_assistant_text_passes_through() {
        let mut sm = QwenStateMachine::new();
        let event = AgentEvent::text("assistant response text", "qwen");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Text);
    }

    #[test]
    fn test_reset_clears() {
        let mut sm = QwenStateMachine::new();

        // Start accumulating a tool call
        let start = AgentEvent::tool_use("list_files", "{}", "qwen");
        sm.process(start);
        assert!(sm.tool_accumulator.is_active());

        sm.reset();
        assert!(!sm.tool_accumulator.is_active());

        // After reset, text should pass through normally
        let text = AgentEvent::text("hello", "qwen");
        let result = sm.process(text);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_timeout_flush_emits_incomplete() {
        let mut sm = QwenStateMachine::new();
        sm.timed_flush = TimedFlush::new(Duration::from_millis(1));

        // Start a tool call
        let start = AgentEvent::tool_use("read_file", "{}", "qwen");
        sm.process(start);

        // Wait for timeout to expire
        std::thread::sleep(Duration::from_millis(5));

        // Next event triggers timeout flush with incomplete: true
        let text = AgentEvent::text("next", "qwen");
        let result = sm.process(text);

        assert!(
            result.len() >= 2,
            "should have flushed incomplete tool + next event"
        );
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
        assert_eq!(result[0].metadata.incomplete, Some(true));
    }
}
