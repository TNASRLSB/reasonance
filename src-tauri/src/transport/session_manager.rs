use crate::agent_event::AgentEvent;
use crate::transport::request::SessionStatus;
use crate::transport::session_handle::{ForkInfo, SessionHandle, SessionSummary, ViewMode};
use crate::transport::session_store::SessionStore;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::path::Path;
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
    pub fn new(sessions_dir: &Path) -> Result<Self, crate::error::ReasonanceError> {
        info!(
            "SessionManager: initializing with sessions_dir={}",
            sessions_dir.display()
        );
        let store = Arc::new(SessionStore::new(sessions_dir)?);
        let index = store.read_index().unwrap_or_default();
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
    pub fn create_session(
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

        self.store.create_session(&handle)?;
        if let Some(ref writer) = *self.writer.lock().unwrap_or_else(|e| e.into_inner()) {
            writer.track_session(handle.clone());
        }

        // Update index
        let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
        index.push(handle.to_summary());
        self.store.write_index(&index)?;

        info!("SessionManager: session created session_id={}", session_id);
        Ok(session_id)
    }

    /// Restore a session from disk. Returns the handle and its events.
    pub fn restore_session(
        &self,
        session_id: &str,
    ) -> Result<(SessionHandle, Vec<AgentEvent>), crate::error::ReasonanceError> {
        info!("SessionManager: restoring session={}", session_id);
        if !self.store.session_exists(session_id) {
            warn!("SessionManager: session={} not found on disk", session_id);
            return Err(crate::error::ReasonanceError::not_found(
                "session", session_id,
            ));
        }

        let mut handle = self.store.read_metadata(session_id)?;
        let events = self.store.read_events(session_id)?;

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
    pub fn fork_session(
        &self,
        parent_session_id: &str,
        fork_event_index: u32,
    ) -> Result<String, crate::error::ReasonanceError> {
        info!(
            "SessionManager: forking session={} at event_index={}",
            parent_session_id, fork_event_index
        );
        let parent = self.store.read_metadata(parent_session_id)?;
        let parent_events = self.store.read_events(parent_session_id)?;

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
        self.store.create_session(&forked)?;

        // Copy events up to fork point
        debug!(
            "SessionManager: copying {} events to forked session={}",
            idx, forked_id
        );
        for event in &parent_events[..idx] {
            self.store.append_event(&forked_id, event)?;
        }

        forked.event_count = fork_event_index;
        self.store.write_metadata(&forked)?;
        if let Some(ref writer) = *self.writer.lock().unwrap_or_else(|e| e.into_inner()) {
            writer.track_session(forked.clone());
        }

        // Update index
        let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
        index.push(forked.to_summary());
        self.store.write_index(&index)?;

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
    pub fn delete_session(&self, session_id: &str) -> Result<(), crate::error::ReasonanceError> {
        info!("SessionManager: deleting session={}", session_id);
        self.store.delete_session(session_id)?;

        let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
        index.retain(|s| s.id != session_id);
        self.store.write_index(&index)?;

        info!("SessionManager: session={} deleted", session_id);
        Ok(())
    }

    /// Rename a session.
    pub fn rename_session(
        &self,
        session_id: &str,
        title: &str,
    ) -> Result<(), crate::error::ReasonanceError> {
        info!(
            "SessionManager: renaming session={} to {:?}",
            session_id, title
        );
        let mut handle = self.store.read_metadata(session_id)?;
        handle.title = title.to_string();
        self.store.write_metadata(&handle)?;

        // Update index
        let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = index.iter_mut().find(|s| s.id == session_id) {
            entry.title = title.to_string();
        }
        self.store.write_index(&index)?;

        info!("SessionManager: session={} renamed", session_id);
        Ok(())
    }

    /// Finalize a session — flush metadata with final status and update index.
    /// Called when the transport's CLI process ends.
    #[allow(dead_code)] // Roadmap: wired when transport completion triggers finalization
    pub fn finalize_session(
        &self,
        session_id: &str,
        final_status: SessionStatus,
    ) -> Result<(), crate::error::ReasonanceError> {
        info!(
            "SessionManager: finalizing session={} with status={:?}",
            session_id, final_status
        );
        // Collect data under writer lock, then release before acquiring index lock
        let event_count = {
            let writer_guard = self.writer.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref writer) = *writer_guard {
                let writer_handles = writer.handles_ref();
                let mut handles = writer_handles.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(handle) = handles.get_mut(session_id) {
                    handle.status = final_status.clone();
                    handle.touch();
                    self.store.write_metadata(handle)?;
                    let count = handle.event_count;
                    handles.remove(session_id);
                    Some(count)
                } else {
                    None
                }
            } else {
                None
            }
        }; // writer lock released here

        if let Some(count) = event_count {
            // Handle was in recorder cache
            let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = index.iter_mut().find(|s| s.id == session_id) {
                entry.status = final_status;
                entry.event_count = count;
            }
            self.store.write_index(&index)?;
        } else {
            // Session not in recorder cache — read from disk and update
            debug!(
                "SessionManager: session={} not in recorder cache, reading from disk",
                session_id
            );
            let mut handle = self.store.read_metadata(session_id)?;
            handle.status = final_status.clone();
            handle.touch();
            self.store.write_metadata(&handle)?;

            let mut index = self.index.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = index.iter_mut().find(|s| s.id == session_id) {
                entry.status = final_status;
            }
            self.store.write_index(&index)?;
        }

        info!("SessionManager: session={} finalized", session_id);
        Ok(())
    }

    /// Set view mode for a session.
    pub fn set_view_mode(
        &self,
        session_id: &str,
        mode: ViewMode,
    ) -> Result<(), crate::error::ReasonanceError> {
        debug!(
            "SessionManager: setting view_mode={:?} for session={}",
            mode, session_id
        );
        let mut handle = self.store.read_metadata(session_id)?;
        handle.view_mode = mode;
        self.store.write_metadata(&handle)?;
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
    use crate::subscribers::session_writer::SessionHistoryWriter;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SessionManager) {
        let dir = TempDir::new().unwrap();
        let mgr = SessionManager::new(dir.path()).unwrap();
        // Wire writer so track_session calls succeed
        let writer = std::sync::Arc::new(SessionHistoryWriter::new(mgr.store()));
        mgr.set_writer(writer);
        (dir, mgr)
    }

    #[test]
    fn test_create_session() {
        let (_dir, mgr) = setup();
        let id = mgr.create_session("claude", "sonnet").unwrap();
        assert!(!id.is_empty());

        let sessions = mgr.list_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider, "claude");
    }

    #[test]
    fn test_restore_session() {
        let (_dir, mgr) = setup();
        let id = mgr.create_session("claude", "sonnet").unwrap();

        let (handle, events) = mgr.restore_session(&id).unwrap();
        assert_eq!(handle.id, id);
        assert!(events.is_empty());
    }

    #[test]
    fn test_restore_nonexistent() {
        let (_dir, mgr) = setup();
        let result = mgr.restore_session("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_session() {
        let (_dir, mgr) = setup();
        let id = mgr.create_session("claude", "sonnet").unwrap();
        assert_eq!(mgr.list_sessions().len(), 1);

        mgr.delete_session(&id).unwrap();
        assert_eq!(mgr.list_sessions().len(), 0);
    }

    #[test]
    fn test_rename_session() {
        let (_dir, mgr) = setup();
        let id = mgr.create_session("claude", "sonnet").unwrap();

        mgr.rename_session(&id, "My Chat").unwrap();

        let sessions = mgr.list_sessions();
        assert_eq!(sessions[0].title, "My Chat");
    }

    #[test]
    fn test_finalize_session() {
        let (_dir, mgr) = setup();
        let id = mgr.create_session("claude", "sonnet").unwrap();

        // Simulate events via store directly (writer tracks but doesn't write)
        mgr.store()
            .append_event(
                &id,
                &crate::agent_event::AgentEvent::text("hello", "claude"),
            )
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
            .unwrap();

        // Check index reflects final status
        let sessions = mgr.list_sessions();
        assert_eq!(sessions[0].status, SessionStatus::Terminated);
        assert_eq!(sessions[0].event_count, 1);

        // Check metadata on disk
        let (handle, _) = mgr.restore_session(&id).unwrap();
        assert_eq!(handle.status, SessionStatus::Terminated);
    }

    #[test]
    fn test_fork_session() {
        let (_dir, mgr) = setup();
        let parent_id = mgr.create_session("claude", "sonnet").unwrap();

        // Manually append some events via store
        let store = mgr.store();
        store
            .append_event(
                &parent_id,
                &crate::agent_event::AgentEvent::text("hello", "claude"),
            )
            .unwrap();
        store
            .append_event(
                &parent_id,
                &crate::agent_event::AgentEvent::text("world", "claude"),
            )
            .unwrap();
        store
            .append_event(
                &parent_id,
                &crate::agent_event::AgentEvent::text("three", "claude"),
            )
            .unwrap();

        // Update metadata to reflect events
        let mut handle = store.read_metadata(&parent_id).unwrap();
        handle.event_count = 3u32;
        store.write_metadata(&handle).unwrap();

        // Fork at event 2
        let fork_id = mgr.fork_session(&parent_id, 2).unwrap();

        let (forked, events) = mgr.restore_session(&fork_id).unwrap();
        assert_eq!(events.len(), 2);
        assert!(forked.title.contains("Fork"));
        assert!(forked.forked_from.is_some());
        assert_eq!(forked.forked_from.unwrap().fork_event_index, 2);
    }

    #[test]
    fn test_fork_invalid_index() {
        let (_dir, mgr) = setup();
        let parent_id = mgr.create_session("claude", "sonnet").unwrap();

        let result = mgr.fork_session(&parent_id, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_view_mode() {
        let (_dir, mgr) = setup();
        let id = mgr.create_session("claude", "sonnet").unwrap();

        mgr.set_view_mode(&id, ViewMode::Terminal).unwrap();

        let (handle, _) = mgr.restore_session(&id).unwrap();
        assert_eq!(handle.view_mode, ViewMode::Terminal);
    }
}
