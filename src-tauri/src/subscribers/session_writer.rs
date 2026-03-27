use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::agent_event::AgentEvent;
use crate::error::ReasonanceError;
use crate::event_bus::{AsyncEventHandler, Event};
use crate::transport::session_handle::SessionHandle;
use crate::transport::session_store::SessionStore;

/// Async session history writer that implements the EventBus v2 `AsyncEventHandler` trait.
///
/// Replaces the old `SessionHistoryRecorder` which used a background thread + mpsc channel.
/// In v2, the bus dispatches via `tokio::spawn`, so async I/O is native — no manual thread needed.
pub struct SessionHistoryWriter {
    store: Arc<SessionStore>,
    handles: Arc<Mutex<HashMap<String, SessionHandle>>>,
}

impl SessionHistoryWriter {
    pub fn new(store: Arc<SessionStore>) -> Self {
        info!("SessionHistoryWriter(v2): created");
        Self {
            store,
            handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a session to be tracked by this writer.
    pub fn track_session(&self, handle: SessionHandle) {
        info!(
            "SessionHistoryWriter(v2): tracking session={} provider={} model={}",
            handle.id, handle.provider, handle.model
        );
        self.handles
            .lock()
            .unwrap_or_else(|e| {
                warn!(
                    "SessionHistoryWriter(v2): handles lock poisoned in track_session, recovering"
                );
                e.into_inner()
            })
            .insert(handle.id.clone(), handle);
    }

    /// Get a reference to the handles map (for SessionManager integration).
    #[allow(dead_code)]
    pub fn handles_ref(&self) -> Arc<Mutex<HashMap<String, SessionHandle>>> {
        self.handles.clone()
    }
}

#[async_trait::async_trait]
impl AsyncEventHandler for SessionHistoryWriter {
    async fn handle(&self, event: Event) -> Result<(), ReasonanceError> {
        let session_id = match event.payload.get("session_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => {
                trace!(
                    "SessionHistoryWriter(v2): ignoring event {} — no session_id in payload",
                    event.id
                );
                return Ok(());
            }
        };

        let agent_event: AgentEvent = match serde_json::from_value(
            event
                .payload
                .get("event")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        ) {
            Ok(evt) => evt,
            Err(_) => {
                trace!(
                    "SessionHistoryWriter(v2): ignoring event {} — could not parse AgentEvent",
                    event.id
                );
                return Ok(());
            }
        };

        trace!(
            "SessionHistoryWriter(v2): writing event type={:?} for session={}",
            agent_event.event_type,
            session_id
        );

        // Append event to JSONL on disk.
        if let Err(e) = self.store.append_event(&session_id, &agent_event) {
            error!(
                "SessionHistoryWriter(v2): failed to append event for session={}: {}",
                session_id, e
            );
        }

        // Update handle metadata (separate from I/O to minimize lock scope)
        let metadata_to_persist = {
            let mut handles_lock = self.handles.lock().unwrap_or_else(|e| {
                warn!("SessionHistoryWriter: handles lock poisoned, recovering");
                e.into_inner()
            });
            if let Some(handle) = handles_lock.get_mut(&session_id) {
                handle.event_count += 1;
                handle.touch();
                if handle.event_count % 10 == 0 {
                    Some(handle.clone())
                } else {
                    None
                }
            } else {
                warn!(
                    "SessionHistoryWriter: event for untracked session={}",
                    session_id
                );
                None
            }
        };
        // Persist metadata outside the lock scope (disk I/O)
        if let Some(ref handle) = metadata_to_persist {
            debug!(
                "SessionHistoryWriter: persisting metadata for session={}",
                session_id
            );
            if let Err(e) = self.store.write_metadata(handle) {
                error!(
                    "SessionHistoryWriter: failed to write metadata for session={}: {}",
                    session_id, e
                );
            }
        }

        Ok(())
    }

    fn id(&self) -> &str {
        "session-history-writer-v2"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::AgentEvent;
    use crate::event_bus::Event;
    use crate::transport::session_handle::SessionHandle;
    use crate::transport::session_store::SessionStore;

    /// Build a bus `Event` from an `AgentEvent` using the standard conversion.
    fn make_event(session_id: &str, agent_event: &AgentEvent) -> Event {
        Event::from_agent_event("agent:stream", session_id, agent_event)
    }

    #[tokio::test]
    async fn handle_appends_events_to_store() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(SessionStore::new(dir.path()).unwrap());
        let handle = SessionHandle::new("claude", "sonnet");
        let session_id = handle.id.clone();

        store.create_session(&handle).unwrap();

        let writer = SessionHistoryWriter::new(store.clone());
        writer.track_session(handle);

        let ae1 = AgentEvent::text("hello", "claude");
        let ae2 = AgentEvent::text("world", "claude");

        writer.handle(make_event(&session_id, &ae1)).await.unwrap();
        writer.handle(make_event(&session_id, &ae2)).await.unwrap();

        // Verify events were written to disk.
        let events = store.read_events(&session_id).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn handle_tracks_session_metadata() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(SessionStore::new(dir.path()).unwrap());
        let handle = SessionHandle::new("claude", "sonnet");
        let session_id = handle.id.clone();
        let original_last_active = handle.last_active_at;

        store.create_session(&handle).unwrap();

        let writer = SessionHistoryWriter::new(store.clone());
        writer.track_session(handle);

        // Send a few events so we can verify event_count and touch.
        for i in 0..3 {
            let ae = AgentEvent::text(&format!("msg-{}", i), "claude");
            writer.handle(make_event(&session_id, &ae)).await.unwrap();
        }

        let handles = writer.handles_ref();
        let handles = handles.lock().unwrap();
        let h = handles.get(&session_id).unwrap();

        assert_eq!(h.event_count, 3);
        assert!(h.last_active_at >= original_last_active);
    }

    #[tokio::test]
    async fn handle_persists_metadata_every_10_events() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(SessionStore::new(dir.path()).unwrap());
        let handle = SessionHandle::new("claude", "sonnet");
        let session_id = handle.id.clone();

        store.create_session(&handle).unwrap();

        let writer = SessionHistoryWriter::new(store.clone());
        writer.track_session(handle);

        // Send 10 events to trigger the periodic metadata persist.
        for i in 0..10 {
            let ae = AgentEvent::text(&format!("msg-{}", i), "claude");
            writer.handle(make_event(&session_id, &ae)).await.unwrap();
        }

        // Read metadata from disk — should reflect event_count=10.
        let persisted = store.read_metadata(&session_id).unwrap();
        assert_eq!(persisted.event_count, 10);
    }

    #[tokio::test]
    async fn handle_ignores_events_for_untracked_sessions() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(SessionStore::new(dir.path()).unwrap());

        let writer = SessionHistoryWriter::new(store.clone());

        // Do not call track_session — the session is unknown.
        let ae = AgentEvent::text("hello", "claude");
        let result = writer.handle(make_event("untracked-session", &ae)).await;

        // Should succeed (warning logged, no panic).
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn handle_ignores_invalid_payload() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(SessionStore::new(dir.path()).unwrap());
        let writer = SessionHistoryWriter::new(store.clone());

        let event = Event::new("agent:stream", serde_json::json!({"random": true}), "test");
        let result = writer.handle(event).await;
        assert!(result.is_ok());
    }
}
