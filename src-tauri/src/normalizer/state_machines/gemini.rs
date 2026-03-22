use super::StateMachine;
use super::accumulator::{ToolInputAccumulator, TimedFlush};
use crate::agent_event::{AgentEvent, AgentEventType, EventContent};
use std::time::Duration;

const FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

pub struct GeminiStateMachine {
    tool_accumulator: ToolInputAccumulator,
    timed_flush: TimedFlush,
}

impl GeminiStateMachine {
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

impl StateMachine for GeminiStateMachine {
    fn process(&mut self, event: AgentEvent) -> Vec<AgentEvent> {
        if self.tool_accumulator.is_active() && self.timed_flush.is_expired() {
            let mut flushed = self.flush_with_incomplete();
            flushed.push(event);
            return flushed;
        }
        self.timed_flush.touch();

        match event.event_type {
            AgentEventType::ToolUse => {
                if self.tool_accumulator.is_active() {
                    if let EventContent::Text { ref value } = event.content {
                        self.tool_accumulator.push_input(value);
                    } else if let EventContent::Json { ref value } = event.content {
                        self.tool_accumulator.push_input(&value.to_string());
                    }
                    vec![]
                } else {
                    let tool_name = event.metadata.tool_name.clone().unwrap_or_default();
                    let parent_id = event.parent_id.clone();
                    let flushed = self.tool_accumulator.start(event, &tool_name, parent_id.as_deref());
                    flushed.into_iter().collect()
                }
            }
            _ => {
                let mut result = Vec::new();
                if self.tool_accumulator.is_active() {
                    if let Some(tool_event) = self.tool_accumulator.finalize() {
                        result.push(tool_event);
                    }
                }
                result.push(event);
                result
            }
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
        let mut sm = GeminiStateMachine::new();
        let event = AgentEvent::text("hello", "gemini");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Text);
    }

    #[test]
    fn test_tool_use_accumulates_and_flushes() {
        let mut sm = GeminiStateMachine::new();
        let start = AgentEvent::tool_use("read_file", r#"{"path":"test.rs"}"#, "gemini");
        let result = sm.process(start);
        assert_eq!(result.len(), 0);
        let text = AgentEvent::text("done", "gemini");
        let result = sm.process(text);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
        assert_eq!(result[1].event_type, AgentEventType::Text);
    }

    #[test]
    fn test_multiple_tool_input_accumulates() {
        let mut sm = GeminiStateMachine::new();
        let start = AgentEvent::tool_use("read_file", "{}", "gemini");
        sm.process(start);
        let mut chunk = AgentEvent::text(r#"{"path":"test.rs"}"#, "gemini");
        chunk.event_type = AgentEventType::ToolUse;
        sm.process(chunk);
        let done = AgentEvent::done("sess", "gemini");
        let result = sm.process(done);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
    }

    #[test]
    fn test_reset_clears() {
        let mut sm = GeminiStateMachine::new();
        let start = AgentEvent::tool_use("read_file", "{}", "gemini");
        sm.process(start);
        sm.reset();
        let text = AgentEvent::text("hello", "gemini");
        let result = sm.process(text);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_timeout_flush_emits_incomplete() {
        let mut sm = GeminiStateMachine::new();
        sm.timed_flush = TimedFlush::new(Duration::from_millis(1));
        let start = AgentEvent::tool_use("read_file", "{}", "gemini");
        sm.process(start);
        std::thread::sleep(Duration::from_millis(5));
        let text = AgentEvent::text("next", "gemini");
        let result = sm.process(text);
        assert!(result.len() >= 2);
        assert_eq!(result[0].metadata.incomplete, Some(true));
    }
}
