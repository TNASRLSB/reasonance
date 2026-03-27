use crate::agent_event::AgentEvent;
use crate::storage::StorageBackend;
use crate::transport::request::SessionStatus;
use crate::transport::session_handle::{ForkInfo, SessionHandle, SessionSummary, ViewMode};
use crate::transport::session_store::SessionStore;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::sync::{Arc, Mutex};

/// Orchestrates session lifecycle: create, restore, fork, list, delete.
/// Uses interior mutability (Arc<Mutex<...>>) following codebase conventions.
pub struct SessionManager {
    store: Arc<SessionStore>,
    index: Arc<Mutex<Vec<SessionSummary>>>,
    /// Handle to the v2 SessionHistoryWriter (set after EventBus wiring).
    /// Used by `create_session`, `restore_session`, `fork_session` to
    /// register sessions for event tracking.
    writer: Mutex<Option<Arc<crate::subscribers::session_writer::SessionHistoryWriter>>>,
}

impl SessionManager {
    pub async fn new(
        backend: Arc<dyn StorageBackend>,
    ) -> Result<Self, crate::error::ReasonanceError> {
        info!("SessionManager: initializing with storage backend");
        let store = Arc::new(SessionStore::new(backend));
        let index = store.read_index().await.unwrap_or_default();
        info!(
            "SessionManager: loaded {} existing sessions from index",
            index.len()
        );

        Ok(Self {
            store,
            index: Arc::new(Mutex::new(index)),
            writer: Mutex::new(None),
        })
    }

    /// Set the session history writer. Called from `setup()` after EventBus wiring.
    pub fn set_writer(
        &self,
        writer: Arc<crate::subscribers::session_writer::SessionHistoryWriter>,
    ) {
        *self.writer.lock().unwrap_or_else(|e| e.into_inner()) = Some(writer);
    }

    /// Create a new session. Returns the session ID.
    pub async fn create_session(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<String, crate::error::ReasonanceError> {
        info!(
            "SessionManager: creating session provider={} model={}",
            provider, model
        );
        let handle = SessionHandle::new(provider, model);
        let session_id = handle.id.clone();

        self.store.create_session(&handle).await?;
        if let Some(ref writer) = *self.writer.lock().unwrap_or_else(|e| e.into_inner()) {
            writer.track_session(handle.clone());
        }

        // Update index: clone data out before .await
        let index_snapshot = {
            let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
            index.push(handle.to_summary());
            index.clone()
        };
        self.store.write_index(&index_snapshot).await?;

        info!("SessionManager: session created session_id={}", session_id);
        Ok(session_id)
    }

    /// Restore a session from storage. Returns the handle and its events.
    pub async fn restore_session(
        &self,
        session_id: &str,
    ) -> Result<(SessionHandle, Vec<AgentEvent>), crate::error::ReasonanceError> {
        info!("SessionManager: restoring session={}", session_id);
        if !self.store.session_exists(session_id).await {
            warn!("SessionManager: session={} not found", session_id);
            return Err(crate::error::ReasonanceError::not_found(
                "session", session_id,
            ));
        }

        let mut handle = self.store.read_metadata(session_id).await?;
        let events = self.store.read_events(session_id).await?;

        // Reconcile event_count from JSONL (source of truth) in case metadata was stale
        if handle.event_count != events.len() as u32 {
            debug!(
                "SessionManager: reconciled event_count for session={}: metadata={} jsonl={}",
                session_id,
                handle.event_count,
                events.len()
            );
        }
        handle.event_count = events.len() as u32;

        // Track restored session in recorder
        if let Some(ref writer) = *self.writer.lock().unwrap_or_else(|e| e.into_inner()) {
            writer.track_session(handle.clone());
        }

        info!(
            "SessionManager: session={} restored with {} events",
            session_id,
            events.len()
        );
        Ok((handle, events))
    }

    /// Fork a session at a given event index. Returns the new session ID.
    pub async fn fork_session(
        &self,
        parent_session_id: &str,
        fork_event_index: u32,
    ) -> Result<String, crate::error::ReasonanceError> {
        info!(
            "SessionManager: forking session={} at event_index={}",
            parent_session_id, fork_event_index
        );
        let parent = self.store.read_metadata(parent_session_id).await?;
        let parent_events = self.store.read_events(parent_session_id).await?;

        let idx = fork_event_index as usize;
        if idx > parent_events.len() {
            warn!(
                "SessionManager: fork index {} exceeds event count {} for session={}",
                fork_event_index,
                parent_events.len(),
                parent_session_id
            );
            return Err(crate::error::ReasonanceError::validation(
                "fork_event_index",
                format!(
                    "Fork index {} exceeds event count {}",
                    fork_event_index,
                    parent_events.len()
                ),
            ));
        }

        // Create new session based on parent
        let mut forked = SessionHandle::new(&parent.provider, &parent.model);
        forked.title = format!(
            "Fork of {}",
            if parent.title.is_empty() {
                &parent.id
            } else {
                &parent.title
            }
        );
        forked.forked_from = Some(ForkInfo {
            parent_session_id: parent_session_id.to_string(),
            fork_event_index,
        });

        let forked_id = forked.id.clone();
        self.store.create_session(&forked).await?;

        // Copy events up to fork point
        debug!(
            "SessionManager: copying {} events to forked session={}",
            idx, forked_id
        );
        for event in &parent_events[..idx] {
            self.store.append_event(&forked_id, event).await?;
        }

        forked.event_count = fork_event_index;
        self.store.write_metadata(&forked).await?;
        if let Some(ref writer) = *self.writer.lock().unwrap_or_else(|e| e.into_inner()) {
            writer.track_session(forked.clone());
        }

        // Update index: clone data out before .await
        let index_snapshot = {
            let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
            index.push(forked.to_summary());
            index.clone()
        };
        self.store.write_index(&index_snapshot).await?;

        info!(
            "SessionManager: forked session={} -> new session={} with {} events",
            parent_session_id, forked_id, idx
        );
        Ok(forked_id)
    }

    /// List all sessions.
    pub fn list_sessions(&self) -> Vec<SessionSummary> {
        let sessions = self.index.lock().unwrap_or_else(|e| e.into_inner()).clone();
        debug!("SessionManager: listing {} sessions", sessions.len());
        sessions
    }

    /// Delete a session.
    pub async fn delete_session(
        &self,
        session_id: &str,
    ) -> Result<(), crate::error::ReasonanceError> {
        info!("SessionManager: deleting session={}", session_id);
        self.store.delete_session(session_id).await?;

        let index_snapshot = {
            let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
            index.retain(|s| s.id != session_id);
            index.clone()
        };
        self.store.write_index(&index_snapshot).await?;

        info!("SessionManager: session={} deleted", session_id);
        Ok(())
    }

    /// Rename a session.
    pub async fn rename_session(
        &self,
        session_id: &str,
        title: &str,
    ) -> Result<(), crate::error::ReasonanceError> {
        info!(
            "SessionManager: renaming session={} to {:?}",
            session_id, title
        );
        let mut handle = self.store.read_metadata(session_id).await?;
        handle.title = title.to_string();
        self.store.write_metadata(&handle).await?;

        // Update index: clone data out before .await
        let index_snapshot = {
            let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = index.iter_mut().find(|s| s.id == session_id) {
                entry.title = title.to_string();
            }
            index.clone()
        };
        self.store.write_index(&index_snapshot).await?;

        info!("SessionManager: session={} renamed", session_id);
        Ok(())
    }

    /// Finalize a session -- flush metadata with final status and update index.
    /// Called when the transport's CLI process ends.
    #[allow(dead_code)] // Roadmap: wired when transport completion triggers finalization
    pub async fn finalize_session(
        &self,
        session_id: &str,
        final_status: SessionStatus,
    ) -> Result<(), crate::error::ReasonanceError> {
        info!(
            "SessionManager: finalizing session={} with status={:?}",
            session_id, final_status
        );
        // Collect data under writer lock, then release before any async work
        let writer_result: Option<(SessionHandle, u32)> = {
            let writer_guard = self.writer.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref writer) = *writer_guard {
                let writer_handles = writer.handles_ref();
                let mut handles = writer_handles.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(handle) = handles.get_mut(session_id) {
                    handle.status = final_status.clone();
                    handle.touch();
                    let handle_clone = handle.clone();
                    let count = handle.event_count;
                    handles.remove(session_id);
                    Some((handle_clone, count))
                } else {
                    None
                }
            } else {
                None
            }
        }; // all locks released here

        if let Some((handle_clone, count)) = writer_result {
            // Handle was in recorder cache -- persist metadata, then update index
            self.store.write_metadata(&handle_clone).await?;

            let index_snapshot = {
                let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(entry) = index.iter_mut().find(|s| s.id == session_id) {
                    entry.status = final_status;
                    entry.event_count = count;
                }
                index.clone()
            };
            self.store.write_index(&index_snapshot).await?;
        } else {
            // Session not in recorder cache -- read from storage and update
            debug!(
                "SessionManager: session={} not in recorder cache, reading from storage",
                session_id
            );
            let mut handle = self.store.read_metadata(session_id).await?;
            handle.status = final_status.clone();
            handle.touch();
            self.store.write_metadata(&handle).await?;

            let index_snapshot = {
                let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(entry) = index.iter_mut().find(|s| s.id == session_id) {
                    entry.status = final_status;
                }
                index.clone()
            };
            self.store.write_index(&index_snapshot).await?;
        }

        info!("SessionManager: session={} finalized", session_id);
        Ok(())
    }

    /// Set view mode for a session.
    pub async fn set_view_mode(
        &self,
        session_id: &str,
        mode: ViewMode,
    ) -> Result<(), crate::error::ReasonanceError> {
        debug!(
            "SessionManager: setting view_mode={:?} for session={}",
            mode, session_id
        );
        let mut handle = self.store.read_metadata(session_id).await?;
        handle.view_mode = mode;
        self.store.write_metadata(&handle).await?;
        Ok(())
    }

    /// Get the store reference.
    pub fn store(&self) -> Arc<SessionStore> {
        self.store.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryBackend;
    use crate::subscribers::session_writer::SessionHistoryWriter;

    async fn setup() -> SessionManager {
        let backend = Arc::new(InMemoryBackend::new());
        let mgr = SessionManager::new(backend).await.unwrap();
        // Wire writer so track_session calls succeed
        let writer = Arc::new(SessionHistoryWriter::new(mgr.store()));
        mgr.set_writer(writer);
        mgr
    }

    #[tokio::test]
    async fn test_create_session() {
        let mgr = setup().await;
        let id = mgr.create_session("claude", "sonnet").await.unwrap();
        assert!(!id.is_empty());

        let sessions = mgr.list_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider, "claude");
    }

    #[tokio::test]
    async fn test_restore_session() {
        let mgr = setup().await;
        let id = mgr.create_session("claude", "sonnet").await.unwrap();

        let (handle, events) = mgr.restore_session(&id).await.unwrap();
        assert_eq!(handle.id, id);
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_restore_nonexistent() {
        let mgr = setup().await;
        let result = mgr.restore_session("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_session() {
        let mgr = setup().await;
        let id = mgr.create_session("claude", "sonnet").await.unwrap();
        assert_eq!(mgr.list_sessions().len(), 1);

        mgr.delete_session(&id).await.unwrap();
        assert_eq!(mgr.list_sessions().len(), 0);
    }

    #[tokio::test]
    async fn test_rename_session() {
        let mgr = setup().await;
        let id = mgr.create_session("claude", "sonnet").await.unwrap();

        mgr.rename_session(&id, "My Chat").await.unwrap();

        let sessions = mgr.list_sessions();
        assert_eq!(sessions[0].title, "My Chat");
    }

    #[tokio::test]
    async fn test_finalize_session() {
        let mgr = setup().await;
        let id = mgr.create_session("claude", "sonnet").await.unwrap();

        // Simulate events via store directly (writer tracks but doesn't write)
        mgr.store()
            .append_event(
                &id,
                &crate::agent_event::AgentEvent::text("hello", "claude"),
            )
            .await
            .unwrap();

        // Update the writer handle's event_count to match
        let writer_guard = mgr.writer.lock().unwrap();
        if let Some(ref writer) = *writer_guard {
            let handles = writer.handles_ref();
            let mut h = handles.lock().unwrap();
            if let Some(handle) = h.get_mut(&id) {
                handle.event_count = 1;
            }
        }
        drop(writer_guard);

        // Finalize
        mgr.finalize_session(&id, SessionStatus::Terminated)
            .await
            .unwrap();

        // Check index reflects final status
        let sessions = mgr.list_sessions();
        assert_eq!(sessions[0].status, SessionStatus::Terminated);
        assert_eq!(sessions[0].event_count, 1);

        // Check metadata in storage
        let (handle, _) = mgr.restore_session(&id).await.unwrap();
        assert_eq!(handle.status, SessionStatus::Terminated);
    }

    #[tokio::test]
    async fn test_fork_session() {
        let mgr = setup().await;
        let parent_id = mgr.create_session("claude", "sonnet").await.unwrap();

        // Manually append some events via store
        let store = mgr.store();
        store
            .append_event(
                &parent_id,
                &crate::agent_event::AgentEvent::text("hello", "claude"),
            )
            .await
            .unwrap();
        store
            .append_event(
                &parent_id,
                &crate::agent_event::AgentEvent::text("world", "claude"),
            )
            .await
            .unwrap();
        store
            .append_event(
                &parent_id,
                &crate::agent_event::AgentEvent::text("three", "claude"),
            )
            .await
            .unwrap();

        // Update metadata to reflect events
        let mut handle = store.read_metadata(&parent_id).await.unwrap();
        handle.event_count = 3u32;
        store.write_metadata(&handle).await.unwrap();

        // Fork at event 2
        let fork_id = mgr.fork_session(&parent_id, 2).await.unwrap();

        let (forked, events) = mgr.restore_session(&fork_id).await.unwrap();
        assert_eq!(events.len(), 2);
        assert!(forked.title.contains("Fork"));
        assert!(forked.forked_from.is_some());
        assert_eq!(forked.forked_from.unwrap().fork_event_index, 2);
    }

    #[tokio::test]
    async fn test_fork_invalid_index() {
        let mgr = setup().await;
        let parent_id = mgr.create_session("claude", "sonnet").await.unwrap();

        let result = mgr.fork_session(&parent_id, 100).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_view_mode() {
        let mgr = setup().await;
        let id = mgr.create_session("claude", "sonnet").await.unwrap();

        mgr.set_view_mode(&id, ViewMode::Terminal).await.unwrap();

        let (handle, _) = mgr.restore_session(&id).await.unwrap();
        assert_eq!(handle.view_mode, ViewMode::Terminal);
    }
}
