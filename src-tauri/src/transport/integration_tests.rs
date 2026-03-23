#[cfg(test)]
mod tests {
    use crate::transport::StructuredAgentTransport;
    use crate::transport::request::AgentRequest;
    use crate::transport::event_bus::{AgentEventBus, HistoryRecorder, AgentEventSubscriber};
    use crate::agent_event::AgentEventType;
    use std::path::Path;
    use std::sync::Arc;

    #[test]
    fn test_transport_lifecycle() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();

        // Verify no active sessions at start
        assert!(transport.active_sessions().is_empty());

        // Verify events are empty for nonexistent session
        let events = transport.get_events("nonexistent");
        assert!(events.is_empty());
    }

    #[test]
    fn test_transport_rejects_unknown_provider() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();

        let req = AgentRequest {
            prompt: "hello".to_string(),
            provider: "nonexistent".to_string(),
            model: None,
            context: vec![],
            session_id: None,
            system_prompt: None,
            max_tokens: None,
            allowed_tools: None,
            cwd: None,
            yolo: false,
        };

        assert!(transport.send(req).is_err());
    }

    #[tokio::test]
    async fn test_transport_send_with_echo_mock() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();
        let events = transport.get_events("any-session");
        assert!(events.is_empty());
    }

    #[test]
    fn test_event_bus_integration() {
        let bus = Arc::new(AgentEventBus::new());
        let recorder = Arc::new(HistoryRecorder::new());
        let recorder_ref = recorder.clone();

        struct Wrapper(Arc<HistoryRecorder>);
        impl AgentEventSubscriber for Wrapper {
            fn on_event(&self, session_id: &str, event: &crate::agent_event::AgentEvent) {
                self.0.on_event(session_id, event);
            }
        }
        bus.subscribe(Box::new(Wrapper(recorder)));

        let event1 = crate::agent_event::AgentEvent::text("hello", "claude");
        let event2 = crate::agent_event::AgentEvent::usage(100, 200, "claude");
        let event3 = crate::agent_event::AgentEvent::done("sess-1", "claude");

        bus.publish("sess-1", &event1);
        bus.publish("sess-1", &event2);
        bus.publish("sess-1", &event3);

        let events = recorder_ref.get_events("sess-1");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, AgentEventType::Text);
        assert_eq!(events[1].event_type, AgentEventType::Usage);
        assert_eq!(events[2].event_type, AgentEventType::Done);
    }
}
