use crate::agent_event::AgentEvent;
use crate::storage::StorageBackend;
use crate::transport::session_handle::{SessionHandle, SessionSummary};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::sync::Arc;

/// Namespace used for all session data in the storage backend.
const NAMESPACE: &str = "sessions";

/// Key for the session index (list of summaries).
const INDEX_KEY: &str = "_index";

/// Handles all persistence for sessions via a `StorageBackend`.
///
/// Key conventions within the `"sessions"` namespace:
///   `"{session_id}:meta"`   -- SessionHandle as JSON blob (put/get)
///   `"{session_id}:events"` -- AgentEvent JSONL stream (append/read_stream)
///   `"_index"`              -- Vec<SessionSummary> as JSON blob (put/get)
pub struct SessionStore {
    backend: Arc<dyn StorageBackend>,
}

impl SessionStore {
    pub fn new(backend: Arc<dyn StorageBackend>) -> Self {
        debug!("SessionStore: initializing with storage backend");
        Self { backend }
    }

    /// Create a new session by writing its metadata inside a transaction.
    pub async fn create_session(
        &self,
        handle: &SessionHandle,
    ) -> Result<(), crate::error::ReasonanceError> {
        debug!("SessionStore: creating session={}", handle.id);
        let tx = self.backend.begin_transaction(NAMESPACE).await?;
        let meta_key = format!("{}:meta", handle.id);
        let meta_bytes = serde_json::to_vec_pretty(handle).map_err(|e| {
            crate::error::ReasonanceError::serialization("session metadata", e.to_string())
        })?;
        self.backend.tx_put(&tx, &meta_key, &meta_bytes).await?;
        self.backend.commit(tx).await?;
        debug!("SessionStore: session={} created", handle.id);
        Ok(())
    }

    /// Append a single event to the session's JSONL stream.
    pub async fn append_event(
        &self,
        session_id: &str,
        event: &AgentEvent,
    ) -> Result<(), crate::error::ReasonanceError> {
        let key = format!("{}:events", session_id);
        let line = serde_json::to_vec(event)?;
        trace!("SessionStore: appending event to session={}", session_id);
        self.backend.append(NAMESPACE, &key, &line).await
    }

    /// Read all events from a session's JSONL stream.
    pub async fn read_events(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentEvent>, crate::error::ReasonanceError> {
        let key = format!("{}:events", session_id);
        debug!("SessionStore: reading events for session={}", session_id);

        let lines = self.backend.read_stream(NAMESPACE, &key).await?;
        if lines.is_empty() {
            debug!(
                "SessionStore: no events for session={}, returning empty",
                session_id
            );
            return Ok(vec![]);
        }

        let mut events = Vec::new();
        for line_bytes in &lines {
            match serde_json::from_slice::<AgentEvent>(line_bytes) {
                Ok(event) => events.push(event),
                Err(e) => {
                    warn!(
                        "SessionStore: skipping corrupted event line for session={}: {}",
                        session_id, e
                    );
                }
            }
        }

        debug!(
            "SessionStore: read {} events for session={}",
            events.len(),
            session_id
        );
        Ok(events)
    }

    /// Write session metadata (atomic via backend).
    pub async fn write_metadata(
        &self,
        handle: &SessionHandle,
    ) -> Result<(), crate::error::ReasonanceError> {
        let key = format!("{}:meta", handle.id);
        debug!("SessionStore: writing metadata for session={}", handle.id);
        let json = serde_json::to_string_pretty(handle)?;
        self.backend.put(NAMESPACE, &key, json.as_bytes()).await
    }

    /// Read session metadata.
    pub async fn read_metadata(
        &self,
        session_id: &str,
    ) -> Result<SessionHandle, crate::error::ReasonanceError> {
        let key = format!("{}:meta", session_id);
        debug!("SessionStore: reading metadata for session={}", session_id);
        match self.backend.get(NAMESPACE, &key).await? {
            Some(bytes) => Ok(serde_json::from_slice(&bytes)?),
            None => {
                error!(
                    "SessionStore: metadata not found for session={}",
                    session_id
                );
                Err(crate::error::ReasonanceError::not_found(
                    "session metadata",
                    session_id,
                ))
            }
        }
    }

    /// Write the session index (list of summaries).
    pub async fn write_index(
        &self,
        summaries: &[SessionSummary],
    ) -> Result<(), crate::error::ReasonanceError> {
        debug!(
            "SessionStore: writing index with {} entries",
            summaries.len()
        );
        let json = serde_json::to_string_pretty(summaries)?;
        self.backend
            .put(NAMESPACE, INDEX_KEY, json.as_bytes())
            .await
    }

    /// Read the session index.
    pub async fn read_index(&self) -> Result<Vec<SessionSummary>, crate::error::ReasonanceError> {
        match self.backend.get(NAMESPACE, INDEX_KEY).await? {
            Some(bytes) => {
                debug!("SessionStore: reading index");
                Ok(serde_json::from_slice(&bytes)?)
            }
            None => {
                debug!("SessionStore: no index found, returning empty");
                Ok(vec![])
            }
        }
    }

    /// Delete a session (metadata + events).
    pub async fn delete_session(
        &self,
        session_id: &str,
    ) -> Result<(), crate::error::ReasonanceError> {
        let meta_key = format!("{}:meta", session_id);
        let events_key = format!("{}:events", session_id);
        debug!("SessionStore: deleting session={}", session_id);

        let meta_existed = self.backend.delete(NAMESPACE, &meta_key).await?;
        // Also delete the events stream; ignore whether it existed.
        let _ = self.backend.delete(NAMESPACE, &events_key).await?;

        if !meta_existed {
            warn!(
                "SessionStore: attempted to delete non-existent session={}",
                session_id
            );
        }

        Ok(())
    }

    /// Check if a session exists.
    pub async fn session_exists(&self, session_id: &str) -> bool {
        let key = format!("{}:meta", session_id);
        self.backend.exists(NAMESPACE, &key).await.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::AgentEvent;
    use crate::storage::InMemoryBackend;
    use crate::transport::session_handle::SessionHandle;

    fn setup() -> SessionStore {
        let backend = Arc::new(InMemoryBackend::new());
        SessionStore::new(backend)
    }

    #[tokio::test]
    async fn test_create_and_read_metadata() {
        let store = setup();
        let handle = SessionHandle::new("claude", "claude-sonnet-4-6");
        let id = handle.id.clone();

        store.create_session(&handle).await.unwrap();
        assert!(store.session_exists(&id).await);

        let loaded = store.read_metadata(&id).await.unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.provider, "claude");
    }

    #[tokio::test]
    async fn test_append_and_read_events() {
        let store = setup();
        let handle = SessionHandle::new("claude", "sonnet");
        store.create_session(&handle).await.unwrap();

        let event1 = AgentEvent::text("hello", "claude");
        let event2 = AgentEvent::text("world", "claude");

        store.append_event(&handle.id, &event1).await.unwrap();
        store.append_event(&handle.id, &event2).await.unwrap();

        let events = store.read_events(&handle.id).await.unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_read_events_empty() {
        let store = setup();
        let handle = SessionHandle::new("claude", "sonnet");
        store.create_session(&handle).await.unwrap();

        let events = store.read_events(&handle.id).await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_read_events_nonexistent() {
        let store = setup();
        let events = store.read_events("nonexistent").await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_write_and_read_index() {
        let store = setup();
        let h1 = SessionHandle::new("claude", "sonnet");
        let h2 = SessionHandle::new("claude", "opus");

        let summaries = vec![h1.to_summary(), h2.to_summary()];
        store.write_index(&summaries).await.unwrap();

        let loaded = store.read_index().await.unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[tokio::test]
    async fn test_read_index_empty() {
        let store = setup();
        let loaded = store.read_index().await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn test_delete_session() {
        let store = setup();
        let handle = SessionHandle::new("claude", "sonnet");
        store.create_session(&handle).await.unwrap();
        assert!(store.session_exists(&handle.id).await);

        store.delete_session(&handle.id).await.unwrap();
        assert!(!store.session_exists(&handle.id).await);
    }

    #[tokio::test]
    async fn test_update_metadata() {
        let store = setup();
        let mut handle = SessionHandle::new("claude", "sonnet");
        store.create_session(&handle).await.unwrap();

        handle.title = "My chat".to_string();
        handle.event_count = 5u32;
        store.write_metadata(&handle).await.unwrap();

        let loaded = store.read_metadata(&handle.id).await.unwrap();
        assert_eq!(loaded.title, "My chat");
        assert_eq!(loaded.event_count, 5u32);
    }
}
