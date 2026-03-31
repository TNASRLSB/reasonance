use crate::agent_event::{AgentEvent, EventContent};
use std::time::{Duration, Instant};

// ─── TextAccumulator ─────────────────────────────────────────────────────────

pub struct TextAccumulator {
    buffer: String,
}

impl TextAccumulator {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    pub fn push(&mut self, text: &str) {
        self.buffer.push_str(text);
    }

    pub fn take(&mut self) -> String {
        std::mem::take(&mut self.buffer)
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    #[cfg(test)]
    pub fn peek(&self) -> &str {
        &self.buffer
    }
}

// ─── ToolInputAccumulator ────────────────────────────────────────────────────

pub struct ToolInputAccumulator {
    tool_name: Option<String>,
    tool_id: Option<String>,
    input_buffer: String,
    start_event: Option<AgentEvent>,
}

impl ToolInputAccumulator {
    pub fn new() -> Self {
        Self {
            tool_name: None,
            tool_id: None,
            input_buffer: String::new(),
            start_event: None,
        }
    }

    /// Begin accumulating for a new tool call.
    /// If a tool was already being accumulated, auto-flushes and returns it.
    pub fn start(
        &mut self,
        event: AgentEvent,
        tool_name: &str,
        tool_id: Option<&str>,
    ) -> Option<AgentEvent> {
        let flushed = if self.is_active() {
            self.finalize()
        } else {
            None
        };
        self.tool_name = Some(tool_name.to_string());
        self.tool_id = tool_id.map(|s| s.to_string());
        self.input_buffer.clear();
        self.start_event = Some(event);
        flushed
    }

    pub fn push_input(&mut self, fragment: &str) {
        self.input_buffer.push_str(fragment);
    }

    pub fn is_active(&self) -> bool {
        self.start_event.is_some()
    }

    /// Consume the accumulated tool call and return it as a finalised AgentEvent.
    pub fn finalize(&mut self) -> Option<AgentEvent> {
        let mut event = self.start_event.take()?;
        if !self.input_buffer.is_empty() {
            let parsed = serde_json::from_str(&self.input_buffer)
                .unwrap_or(serde_json::Value::String(self.input_buffer.clone()));
            event.content = EventContent::Json { value: parsed };
        }
        event.metadata.tool_name = self.tool_name.take();
        self.tool_id = None;
        self.input_buffer.clear();
        Some(event)
    }

    pub fn reset(&mut self) {
        self.tool_name = None;
        self.tool_id = None;
        self.input_buffer.clear();
        self.start_event = None;
    }
}

// ─── TimedFlush ──────────────────────────────────────────────────────────────

pub struct TimedFlush {
    last_event_at: Instant,
    timeout: Duration,
}

impl TimedFlush {
    pub fn new(timeout: Duration) -> Self {
        Self {
            last_event_at: Instant::now(),
            timeout,
        }
    }

    pub fn touch(&mut self) {
        self.last_event_at = Instant::now();
    }

    pub fn is_expired(&self) -> bool {
        self.last_event_at.elapsed() >= self.timeout
    }

    #[cfg(test)]
    pub fn elapsed(&self) -> Duration {
        self.last_event_at.elapsed()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::{AgentEvent, AgentEventType, EventContent};

    // ── TextAccumulator tests ─────────────────────────────────────────────

    #[test]
    fn test_text_accumulator_push_and_take() {
        let mut acc = TextAccumulator::new();
        acc.push("hello ");
        acc.push("world");
        let result = acc.take();
        assert_eq!(result, "hello world");
        assert!(acc.is_empty());
    }

    #[test]
    fn test_text_accumulator_empty() {
        let acc = TextAccumulator::new();
        assert!(acc.is_empty());
        assert_eq!(acc.peek(), "");
    }

    #[test]
    fn test_text_accumulator_peek() {
        let mut acc = TextAccumulator::new();
        acc.push("hello");
        assert_eq!(acc.peek(), "hello");
        assert!(!acc.is_empty());
        // peek must not consume
        assert_eq!(acc.peek(), "hello");
    }

    #[test]
    fn test_text_accumulator_take_clears() {
        let mut acc = TextAccumulator::new();
        acc.push("data");
        let first = acc.take();
        assert_eq!(first, "data");
        let second = acc.take();
        assert_eq!(second, "");
    }

    // ── ToolInputAccumulator tests ────────────────────────────────────────

    fn make_tool_event(tool_name: &str, provider: &str) -> AgentEvent {
        AgentEvent::tool_use(tool_name, "{}", provider)
    }

    #[test]
    fn test_tool_input_start_and_finalize() {
        let mut acc = ToolInputAccumulator::new();
        let event = make_tool_event("read_file", "gemini");

        let flushed = acc.start(event, "read_file", Some("tool-1"));
        assert!(
            flushed.is_none(),
            "start on fresh accumulator should return None"
        );
        assert!(acc.is_active());

        acc.push_input(r#"{"path":"#);
        acc.push_input(r#""src/main.rs"}"#);

        let result = acc.finalize().expect("finalize should return an event");
        assert_eq!(result.event_type, AgentEventType::ToolUse);
        assert_eq!(result.metadata.tool_name, Some("read_file".to_string()));
        assert!(matches!(result.content, EventContent::Json { .. }));
        assert!(!acc.is_active());
    }

    #[test]
    fn test_tool_input_start_while_active_flushes() {
        let mut acc = ToolInputAccumulator::new();

        // Start first tool
        let event1 = make_tool_event("read_file", "gemini");
        let _ = acc.start(event1, "read_file", Some("tool-1"));
        acc.push_input(r#"{"path":"foo.rs"}"#);

        // Start second tool — should flush first
        let event2 = make_tool_event("write_file", "gemini");
        let flushed = acc.start(event2, "write_file", Some("tool-2"));

        let flushed_event = flushed.expect("starting while active should flush pending tool");
        assert_eq!(
            flushed_event.metadata.tool_name,
            Some("read_file".to_string())
        );

        // Accumulator should now be active for tool-2
        assert!(acc.is_active());
    }

    #[test]
    fn test_tool_input_finalize_when_inactive() {
        let mut acc = ToolInputAccumulator::new();
        let result = acc.finalize();
        assert!(result.is_none());
    }

    #[test]
    fn test_tool_input_reset() {
        let mut acc = ToolInputAccumulator::new();
        let event = make_tool_event("list_files", "gemini");
        acc.start(event, "list_files", None);
        assert!(acc.is_active());

        acc.reset();
        assert!(!acc.is_active());
        assert!(acc.finalize().is_none());
    }

    // ── TimedFlush tests ──────────────────────────────────────────────────

    #[test]
    fn test_timed_flush_fresh_not_expired() {
        let tf = TimedFlush::new(Duration::from_secs(10));
        assert!(!tf.is_expired());
    }

    #[test]
    fn test_timed_flush_expired() {
        let tf = TimedFlush::new(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        assert!(tf.is_expired());
    }

    #[test]
    fn test_timed_flush_touch_resets() {
        let mut tf = TimedFlush::new(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        assert!(tf.is_expired(), "should be expired before touch");
        tf.touch();
        assert!(!tf.is_expired(), "should not be expired right after touch");
    }

    #[test]
    fn test_timed_flush_elapsed() {
        let tf = TimedFlush::new(Duration::from_secs(60));
        std::thread::sleep(Duration::from_millis(10));
        assert!(
            tf.elapsed() >= Duration::from_millis(5),
            "elapsed should be at least 5ms after sleeping 10ms"
        );
    }
}
