#[cfg(test)]
mod tests {
    use crate::normalizer::NormalizerRegistry;
    use crate::agent_event::{AgentEventType, EventContent};

    /// Simulates a real Claude CLI stream-json session:
    /// message_start → content_block_start → N deltas → content_block_stop → message_delta (usage) → message_stop
    #[test]
    fn test_full_claude_stream_session() {
        let mut registry = NormalizerRegistry::load_from_dir(
            std::path::Path::new("normalizers")
        ).unwrap();

        assert!(registry.has_provider("claude"));

        // These are real Claude stream-json events (simplified)
        let stream_lines = vec![
            r#"{"type":"message_start","message":{"id":"msg_123","model":"claude-sonnet-4-6","role":"assistant"}}"#,
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world"}}"#,
            r#"{"type":"content_block_stop","index":0}"#,
            r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"input_tokens":10,"output_tokens":5}}"#,
            r#"{"type":"message_stop"}"#,
        ];

        let mut all_events = vec![];
        for line in stream_lines {
            let events = registry.process("claude", line);
            all_events.extend(events);
        }

        // Should have: 2 text events, 1 usage event, 1 done event
        // (message_start, content_block_start, content_block_stop are internal — no rules match)
        let text_events: Vec<_> = all_events.iter().filter(|e| e.event_type == AgentEventType::Text).collect();
        let usage_events: Vec<_> = all_events.iter().filter(|e| e.event_type == AgentEventType::Usage).collect();
        let done_events: Vec<_> = all_events.iter().filter(|e| e.event_type == AgentEventType::Done).collect();

        assert_eq!(text_events.len(), 2, "Expected 2 text events");
        assert_eq!(usage_events.len(), 1, "Expected 1 usage event");
        assert_eq!(done_events.len(), 1, "Expected 1 done event");

        // Verify text content
        if let EventContent::Text { ref value } = text_events[0].content {
            assert_eq!(value, "Hello");
        }
        if let EventContent::Text { ref value } = text_events[1].content {
            assert_eq!(value, " world");
        }

        // Verify usage
        assert_eq!(usage_events[0].metadata.input_tokens, Some(10));
        assert_eq!(usage_events[0].metadata.output_tokens, Some(5));

        // All events have provider set
        for event in &all_events {
            assert_eq!(event.metadata.provider, "claude");
        }
    }

    #[test]
    fn test_claude_error_handling() {
        let mut registry = NormalizerRegistry::load_from_dir(
            std::path::Path::new("normalizers")
        ).unwrap();

        let error_line = r#"{"type":"error","error":{"type":"overloaded","message":"Server is overloaded"}}"#;
        let events = registry.process("claude", error_line);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AgentEventType::Error);
        assert_eq!(events[0].metadata.error_code, Some("overloaded".to_string()));
        assert_eq!(
            events[0].metadata.error_severity,
            Some(crate::agent_event::ErrorSeverity::Recoverable)
        );
    }

    #[test]
    fn test_claude_thinking_events() {
        let mut registry = NormalizerRegistry::load_from_dir(
            std::path::Path::new("normalizers")
        ).unwrap();

        let thinking_line = r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Let me consider..."}}"#;
        let events = registry.process("claude", thinking_line);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AgentEventType::Thinking);
        if let EventContent::Text { ref value } = events[0].content {
            assert_eq!(value, "Let me consider...");
        }
    }
}
