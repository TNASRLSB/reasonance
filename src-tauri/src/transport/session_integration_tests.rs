#[cfg(test)]
mod tests {
    use crate::agent_event::AgentEvent;
    use crate::transport::event_bus::{AgentEventBus, AgentEventSubscriber, SessionHistoryRecorder};
    use crate::transport::session_handle::SessionHandle;
    use crate::transport::session_manager::SessionManager;
    use crate::transport::session_store::SessionStore;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn test_full_session_lifecycle() {
        let dir = TempDir::new().unwrap();
        let mgr = SessionManager::new(dir.path()).unwrap();

        // Create session
        let id = mgr.create_session("claude", "sonnet").unwrap();

        // Write events via recorder
        let recorder = mgr.recorder();
        let event1 = AgentEvent::text("Hello", "claude");
        let event2 = AgentEvent::text("World", "claude");
        recorder.on_event(&id, &event1);
        recorder.on_event(&id, &event2);

        // Flush background writer before asserting
        recorder.flush();

        // Restore and verify
        let (handle, events) = mgr.restore_session(&id).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(handle.provider, "claude");

        // Fork at event 1
        let fork_id = mgr.fork_session(&id, 1).unwrap();
        let (fork_handle, fork_events) = mgr.restore_session(&fork_id).unwrap();
        assert_eq!(fork_events.len(), 1);
        assert!(fork_handle.forked_from.is_some());

        // Delete original
        mgr.delete_session(&id).unwrap();
        assert!(mgr.restore_session(&id).is_err());

        // Fork still exists
        assert!(mgr.restore_session(&fork_id).is_ok());
    }

    #[test]
    fn test_event_bus_to_disk_pipeline() {
        let dir = TempDir::new().unwrap();
        let store = Arc::new(SessionStore::new(dir.path()).unwrap());
        let handle = SessionHandle::new("claude", "sonnet");
        let session_id = handle.id.clone();
        store.create_session(&handle).unwrap();

        // Set up event bus with session recorder
        let bus = Arc::new(AgentEventBus::new());
        let recorder = Arc::new(SessionHistoryRecorder::new(store.clone()));
        recorder.track_session(handle);

        struct Wrapper(Arc<SessionHistoryRecorder>);
        impl AgentEventSubscriber for Wrapper {
            fn on_event(&self, session_id: &str, event: &AgentEvent) {
                self.0.on_event(session_id, event);
            }
        }
        let recorder_ref = recorder.clone();
        bus.subscribe(Box::new(Wrapper(recorder)));

        // Publish events through the bus
        bus.publish(&session_id, &AgentEvent::text("hello", "claude"));
        bus.publish(&session_id, &AgentEvent::usage(100, 200, "claude"));
        bus.publish(&session_id, &AgentEvent::done("sess", "claude"));

        // Flush background writer before asserting
        recorder_ref.flush();

        // Verify events persisted to disk
        let events = store.read_events(&session_id).unwrap();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_session_survives_reload() {
        let dir = TempDir::new().unwrap();
        let session_id;

        // First "app launch" — create session and write events
        {
            let mgr = SessionManager::new(dir.path()).unwrap();
            session_id = mgr.create_session("claude", "opus").unwrap();
            mgr.rename_session(&session_id, "Important Chat").unwrap();

            let recorder = mgr.recorder();
            recorder.on_event(&session_id, &AgentEvent::text("persisted", "claude"));
            // Flush before dropping to ensure write completes
            recorder.flush();
        }

        // Second "app launch" — restore from disk
        {
            let mgr2 = SessionManager::new(dir.path()).unwrap();
            let sessions = mgr2.list_sessions();
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].title, "Important Chat");

            let (handle, events) = mgr2.restore_session(&session_id).unwrap();
            assert_eq!(handle.title, "Important Chat");
            assert_eq!(events.len(), 1);
        }
    }
}
