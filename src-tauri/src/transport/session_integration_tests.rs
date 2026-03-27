#[cfg(test)]
mod tests {
    use crate::agent_event::AgentEvent;
    use crate::event_bus::{Event, EventBus};
    use crate::storage::InMemoryBackend;
    use crate::subscribers::session_writer::SessionHistoryWriter;
    use crate::transport::session_handle::SessionHandle;
    use crate::transport::session_manager::SessionManager;
    use crate::transport::session_store::SessionStore;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_full_session_lifecycle() {
        let backend = Arc::new(InMemoryBackend::new());
        let mgr = SessionManager::new(backend).await.unwrap();
        let writer = Arc::new(SessionHistoryWriter::new(mgr.store()));
        mgr.set_writer(writer.clone());

        // Create session
        let id = mgr.create_session("claude", "sonnet").await.unwrap();

        // Write events directly via store
        let store = mgr.store();
        store
            .append_event(&id, &AgentEvent::text("Hello", "claude"))
            .await
            .unwrap();
        store
            .append_event(&id, &AgentEvent::text("World", "claude"))
            .await
            .unwrap();

        // Restore and verify
        let (handle, events) = mgr.restore_session(&id).await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(handle.provider, "claude");

        // Fork at event 1
        let fork_id = mgr.fork_session(&id, 1).await.unwrap();
        let (fork_handle, fork_events) = mgr.restore_session(&fork_id).await.unwrap();
        assert_eq!(fork_events.len(), 1);
        assert!(fork_handle.forked_from.is_some());

        // Delete original
        mgr.delete_session(&id).await.unwrap();
        assert!(mgr.restore_session(&id).await.is_err());

        // Fork still exists
        assert!(mgr.restore_session(&fork_id).await.is_ok());
    }

    #[tokio::test]
    async fn test_event_bus_to_storage_pipeline() {
        let backend = Arc::new(InMemoryBackend::new());
        let store = Arc::new(SessionStore::new(backend));
        let handle = SessionHandle::new("claude", "sonnet");
        let session_id = handle.id.clone();
        store.create_session(&handle).await.unwrap();

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

        // Verify events persisted to storage
        let events = store.read_events(&session_id).await.unwrap();
        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn test_session_survives_reload() {
        // Shared backend simulates data surviving across "app launches"
        let backend = Arc::new(InMemoryBackend::new());
        let session_id;

        // First "app launch" -- create session and write events
        {
            let mgr = SessionManager::new(backend.clone()).await.unwrap();
            let writer = Arc::new(SessionHistoryWriter::new(mgr.store()));
            mgr.set_writer(writer);

            session_id = mgr.create_session("claude", "opus").await.unwrap();
            mgr.rename_session(&session_id, "Important Chat")
                .await
                .unwrap();

            // Write event directly via store
            mgr.store()
                .append_event(&session_id, &AgentEvent::text("persisted", "claude"))
                .await
                .unwrap();
        }

        // Second "app launch" -- restore from storage
        {
            let mgr2 = SessionManager::new(backend.clone()).await.unwrap();
            let sessions = mgr2.list_sessions();
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].title, "Important Chat");

            let (handle, events) = mgr2.restore_session(&session_id).await.unwrap();
            assert_eq!(handle.title, "Important Chat");
            assert_eq!(events.len(), 1);
        }
    }
}
