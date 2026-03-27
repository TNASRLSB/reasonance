#[cfg(test)]
mod tests {
    use crate::agent_event::AgentEvent;
    use crate::event_bus::{Event, EventBus};
    use crate::subscribers::history::HistoryRecorder;
    use crate::subscribers::session_writer::SessionHistoryWriter;
    use crate::transport::session_handle::SessionHandle;
    use crate::transport::session_manager::SessionManager;
    use crate::transport::session_store::SessionStore;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn test_full_session_lifecycle() {
        let dir = TempDir::new().unwrap();
        let mgr = SessionManager::new(dir.path()).unwrap();
        let writer = Arc::new(SessionHistoryWriter::new(mgr.store()));
        mgr.set_writer(writer.clone());

        // Create session
        let id = mgr.create_session("claude", "sonnet").unwrap();

        // Write events directly via store
        let store = mgr.store();
        store
            .append_event(&id, &AgentEvent::text("Hello", "claude"))
            .unwrap();
        store
            .append_event(&id, &AgentEvent::text("World", "claude"))
            .unwrap();

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

    #[tokio::test]
    async fn test_event_bus_to_disk_pipeline() {
        let dir = TempDir::new().unwrap();
        let store = Arc::new(SessionStore::new(dir.path()).unwrap());
        let handle = SessionHandle::new("claude", "sonnet");
        let session_id = handle.id.clone();
        store.create_session(&handle).unwrap();

        // Set up v2 event bus with session writer
        let bus = Arc::new(EventBus::new(tokio::runtime::Handle::current()));
        bus.register_channel("transport:event", true);
        bus.register_channel("transport:complete", true);

        let writer = Arc::new(SessionHistoryWriter::new(store.clone()));
        writer.track_session(handle);
        bus.subscribe_async("transport:event", writer.clone());
        bus.subscribe_async("transport:complete", writer.clone());

        // Publish events through the bus
        bus.publish(Event::from_agent_event(
            "transport:event",
            &session_id,
            &AgentEvent::text("hello", "claude"),
        ));
        bus.publish(Event::from_agent_event(
            "transport:event",
            &session_id,
            &AgentEvent::usage(100, 200, "claude"),
        ));
        bus.publish(Event::from_agent_event(
            "transport:complete",
            &session_id,
            &AgentEvent::done("sess", "claude"),
        ));

        // Yield to let async handlers complete
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify events persisted to disk
        let events = store.read_events(&session_id).unwrap();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_session_survives_reload() {
        let dir = TempDir::new().unwrap();
        let session_id;

        // First "app launch" -- create session and write events
        {
            let mgr = SessionManager::new(dir.path()).unwrap();
            let writer = Arc::new(SessionHistoryWriter::new(mgr.store()));
            mgr.set_writer(writer);

            session_id = mgr.create_session("claude", "opus").unwrap();
            mgr.rename_session(&session_id, "Important Chat").unwrap();

            // Write event directly via store
            mgr.store()
                .append_event(&session_id, &AgentEvent::text("persisted", "claude"))
                .unwrap();
        }

        // Second "app launch" -- restore from disk
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
