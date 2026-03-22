use crate::agent_event::AgentEvent;
use crate::transport::event_bus::SessionHistoryRecorder;
use crate::transport::session_handle::{ForkInfo, SessionHandle, SessionSource, SessionSummary, ViewMode};
use crate::transport::session_store::SessionStore;
use crate::transport::request::SessionStatus;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Orchestrates session lifecycle: create, restore, fork, list, delete.
/// Uses interior mutability (Arc<Mutex<...>>) following codebase conventions.
pub struct SessionManager {
    store: Arc<SessionStore>,
    index: Arc<Mutex<Vec<SessionSummary>>>,
    recorder: Arc<SessionHistoryRecorder>,
}

impl SessionManager {
    pub fn new(sessions_dir: &Path) -> Result<Self, String> {
        let store = Arc::new(SessionStore::new(sessions_dir)?);
        let index = store.read_index().unwrap_or_default();
        let recorder = Arc::new(SessionHistoryRecorder::new(store.clone()));

        Ok(Self {
            store,
            index: Arc::new(Mutex::new(index)),
            recorder,
        })
    }

    /// Create a new session. Returns the session ID.
    pub fn create_session(&self, provider: &str, model: &str) -> Result<String, String> {
        let handle = SessionHandle::new(provider, model);
        let session_id = handle.id.clone();

        self.store.create_session(&handle)?;
        self.recorder.track_session(handle.clone());

        // Update index
        let mut index = self.index.lock().unwrap();
        index.push(handle.to_summary());
        self.store.write_index(&index)?;

        Ok(session_id)
    }

    /// Restore a session from disk. Returns the handle and its events.
    pub fn restore_session(&self, session_id: &str) -> Result<(SessionHandle, Vec<AgentEvent>), String> {
        if !self.store.session_exists(session_id) {
            return Err(format!("Session {} not found", session_id));
        }

        let handle = self.store.read_metadata(session_id)?;
        let events = self.store.read_events(session_id)?;

        // Track restored session in recorder
        self.recorder.track_session(handle.clone());

        Ok((handle, events))
    }

    /// Fork a session at a given event index. Returns the new session ID.
    pub fn fork_session(&self, parent_session_id: &str, fork_event_index: u32) -> Result<String, String> {
        let parent = self.store.read_metadata(parent_session_id)?;
        let parent_events = self.store.read_events(parent_session_id)?;

        let idx = fork_event_index as usize;
        if idx > parent_events.len() {
            return Err(format!(
                "Fork index {} exceeds event count {}",
                fork_event_index, parent_events.len()
            ));
        }

        // Create new session based on parent
        let mut forked = SessionHandle::new(&parent.provider, &parent.model);
        forked.title = format!("Fork of {}", if parent.title.is_empty() { &parent.id } else { &parent.title });
        forked.forked_from = Some(ForkInfo {
            parent_session_id: parent_session_id.to_string(),
            fork_event_index,
        });

        let forked_id = forked.id.clone();
        self.store.create_session(&forked)?;

        // Copy events up to fork point
        for event in &parent_events[..idx] {
            self.store.append_event(&forked_id, event)?;
        }

        forked.event_count = fork_event_index;
        self.store.write_metadata(&forked)?;
        self.recorder.track_session(forked.clone());

        // Update index
        let mut index = self.index.lock().unwrap();
        index.push(forked.to_summary());
        self.store.write_index(&index)?;

        Ok(forked_id)
    }

    /// List all sessions.
    pub fn list_sessions(&self) -> Vec<SessionSummary> {
        self.index.lock().unwrap().clone()
    }

    /// Delete a session.
    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        self.store.delete_session(session_id)?;

        let mut index = self.index.lock().unwrap();
        index.retain(|s| s.id != session_id);
        self.store.write_index(&index)?;

        Ok(())
    }

    /// Rename a session.
    pub fn rename_session(&self, session_id: &str, title: &str) -> Result<(), String> {
        let mut handle = self.store.read_metadata(session_id)?;
        handle.title = title.to_string();
        self.store.write_metadata(&handle)?;

        // Update index
        let mut index = self.index.lock().unwrap();
        if let Some(entry) = index.iter_mut().find(|s| s.id == session_id) {
            entry.title = title.to_string();
        }
        self.store.write_index(&index)?;

        Ok(())
    }

    /// Finalize a session — flush metadata with final status and update index.
    /// Called when the transport's CLI process ends.
    pub fn finalize_session(&self, session_id: &str, final_status: SessionStatus) -> Result<(), String> {
        // Flush handle from recorder's cache
        let recorder_handles = self.recorder.handles_ref();
        let mut handles = recorder_handles.lock().unwrap();
        if let Some(handle) = handles.get_mut(session_id) {
            handle.status = final_status.clone();
            handle.touch();
            self.store.write_metadata(handle)?;

            // Update index
            let mut index = self.index.lock().unwrap();
            if let Some(entry) = index.iter_mut().find(|s| s.id == session_id) {
                entry.status = final_status;
                entry.event_count = handle.event_count;
            }
            self.store.write_index(&index)?;
        } else {
            // Session not in recorder cache — read from disk and update
            let mut handle = self.store.read_metadata(session_id)?;
            handle.status = final_status.clone();
            handle.touch();
            self.store.write_metadata(&handle)?;

            let mut index = self.index.lock().unwrap();
            if let Some(entry) = index.iter_mut().find(|s| s.id == session_id) {
                entry.status = final_status;
            }
            self.store.write_index(&index)?;
        }

        // Remove from recorder tracking
        handles.remove(session_id);

        Ok(())
    }

    /// Set view mode for a session.
    pub fn set_view_mode(&self, session_id: &str, mode: ViewMode) -> Result<(), String> {
        let mut handle = self.store.read_metadata(session_id)?;
        handle.view_mode = mode;
        self.store.write_metadata(&handle)?;
        Ok(())
    }

    /// Get the recorder for wiring into the event bus.
    pub fn recorder(&self) -> Arc<SessionHistoryRecorder> {
        self.recorder.clone()
    }

    /// Get the store reference.
    pub fn store(&self) -> Arc<SessionStore> {
        self.store.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::event_bus::AgentEventSubscriber;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SessionManager) {
        let dir = TempDir::new().unwrap();
        let mgr = SessionManager::new(dir.path()).unwrap();
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

        // Simulate events via recorder
        let recorder = mgr.recorder();
        recorder.on_event(&id, &crate::agent_event::AgentEvent::text("hello", "claude"));

        // Finalize
        mgr.finalize_session(&id, SessionStatus::Terminated).unwrap();

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
        store.append_event(&parent_id, &crate::agent_event::AgentEvent::text("hello", "claude")).unwrap();
        store.append_event(&parent_id, &crate::agent_event::AgentEvent::text("world", "claude")).unwrap();
        store.append_event(&parent_id, &crate::agent_event::AgentEvent::text("three", "claude")).unwrap();

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
