#[cfg(test)]
mod tests {
    use crate::agent_event::{AgentEventType, EventContent};
    use crate::normalizer::NormalizerRegistry;

    /// Simulates a real Claude CLI stream-json session:
    /// system events → assistant (response) → result (usage + done)
    #[test]
    fn test_full_claude_stream_session() {
        let mut registry =
            NormalizerRegistry::load_from_dir(std::path::Path::new("normalizers")).unwrap();

        assert!(registry.has_provider("claude"));

        // Real Claude CLI stream-json events
        let stream_lines = vec![
            r#"{"type":"system","subtype":"init","session_id":"sess-1","model":"claude-opus-4-6"}"#,
            r#"{"type":"assistant","message":{"id":"msg_123","model":"claude-opus-4-6","type":"message","role":"assistant","content":[{"type":"text","text":"Hello world"}],"usage":{"input_tokens":10,"output_tokens":5}},"session_id":"sess-1"}"#,
            r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":2100,"duration_api_ms":2050,"num_turns":1,"result":"Hello world","stop_reason":"end_turn","session_id":"sess-1","total_cost_usd":0.05,"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":100,"cache_read_input_tokens":200}}"#,
        ];

        let mut all_events = vec![];
        for line in stream_lines {
            let events = registry.process("claude", line);
            all_events.extend(events);
        }

        // Should have: 1 text event, 1 usage event
        // (system/init events are ignored — no rules match)
        // Note: done events are emitted by stream_reader when stdout closes, not by TOML rules
        let text_events: Vec<_> = all_events
            .iter()
            .filter(|e| e.event_type == AgentEventType::Text)
            .collect();
        let usage_events: Vec<_> = all_events
            .iter()
            .filter(|e| e.event_type == AgentEventType::Usage)
            .collect();

        assert_eq!(text_events.len(), 1, "Expected 1 text event");
        assert_eq!(usage_events.len(), 1, "Expected 1 usage event");

        // Verify text content
        if let EventContent::Text { ref value } = text_events[0].content {
            assert_eq!(value, "Hello world");
        }

        // Verify usage from result event
        assert_eq!(usage_events[0].metadata.input_tokens, Some(10));
        assert_eq!(usage_events[0].metadata.output_tokens, Some(5));
        assert_eq!(usage_events[0].metadata.cache_creation_tokens, Some(100));
        assert_eq!(usage_events[0].metadata.cache_read_tokens, Some(200));
        assert_eq!(usage_events[0].metadata.duration_ms, Some(2100));
        assert_eq!(usage_events[0].metadata.total_cost_usd, Some(0.05));

        // All events have provider set
        for event in &all_events {
            assert_eq!(event.metadata.provider, "claude");
        }
    }

    #[test]
    fn test_claude_error_handling() {
        let mut registry =
            NormalizerRegistry::load_from_dir(std::path::Path::new("normalizers")).unwrap();

        // CLI error format
        let error_line = r#"{"type":"error","message":"Server is overloaded","code":"overloaded"}"#;
        let events = registry.process("claude", error_line);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AgentEventType::Error);
        assert_eq!(
            events[0].metadata.error_code,
            Some("overloaded".to_string())
        );
    }

    #[test]
    fn test_claude_system_events_ignored() {
        let mut registry =
            NormalizerRegistry::load_from_dir(std::path::Path::new("normalizers")).unwrap();

        // System/hook events should not match any rules
        let system_line =
            r#"{"type":"system","subtype":"init","cwd":"/tmp","session_id":"sess-1"}"#;
        let events = registry.process("claude", system_line);
        assert!(events.is_empty(), "System events should be ignored");

        let hook_line = r#"{"type":"system","subtype":"hook_started","hook_id":"h1"}"#;
        let events = registry.process("claude", hook_line);
        assert!(events.is_empty(), "Hook events should be ignored");
    }
}
