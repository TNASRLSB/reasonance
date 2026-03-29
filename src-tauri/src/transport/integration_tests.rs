#[cfg(test)]
mod tests {
    use crate::agent_event::AgentEventType;
    use crate::event_bus::{Event, EventBus};
    use crate::subscribers::history::HistoryRecorder;
    use crate::transport::request::AgentRequest;
    use crate::transport::StructuredAgentTransport;
    use std::path::Path;
    use std::sync::Arc;

    #[test]
    fn test_transport_lifecycle() {
        let transport = StructuredAgentTransport::new(Path::new("normalizers")).unwrap();

        // Verify no active sessions at start
        assert!(transport.active_sessions().is_empty());
    }

    #[test]
    fn test_transport_rejects_unknown_provider() {
        use crate::permission_engine::PermissionMemory;
        use crate::policy_file::PolicyFile;
        use crate::workspace_trust::TrustStore;
        use std::sync::Mutex;
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let trust_store = TrustStore::new(tmp.path().join("trust.json"));
        let memory = PermissionMemory::new();
        let policy = PolicyFile::new();
        let slot_registry = Mutex::new(crate::model_slots::ModelSlotRegistry::new());
        let settings = Mutex::new(crate::settings::LayeredSettings::new());

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
            yolo: true, // yolo so engine doesn't block on untrusted
        };

        assert!(transport
            .send(
                req,
                &trust_store,
                &memory,
                &policy,
                &slot_registry,
                &settings
            )
            .is_err());
    }

    #[tokio::test]
    async fn test_event_bus_integration() {
        let bus = Arc::new(EventBus::new(tokio::runtime::Handle::current()));
        bus.register_channel("transport:event", true);
        bus.register_channel("transport:complete", true);

        let recorder = Arc::new(HistoryRecorder::new());
        bus.subscribe("transport:event", recorder.clone());
        bus.subscribe("transport:complete", recorder.clone());

        let event1 = crate::agent_event::AgentEvent::text("hello", "claude");
        let event2 = crate::agent_event::AgentEvent::usage(100, 200, "claude");
        let event3 = crate::agent_event::AgentEvent::done("sess-1", "claude");

        bus.publish(Event::from_agent_event(
            "transport:event",
            "sess-1",
            &event1,
        ));
        bus.publish(Event::from_agent_event(
            "transport:event",
            "sess-1",
            &event2,
        ));
        bus.publish(Event::from_agent_event(
            "transport:complete",
            "sess-1",
            &event3,
        ));

        let events = recorder.get_events("sess-1");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, AgentEventType::Text);
        assert_eq!(events[1].event_type, AgentEventType::Usage);
        assert_eq!(events[2].event_type, AgentEventType::Done);
    }
}
