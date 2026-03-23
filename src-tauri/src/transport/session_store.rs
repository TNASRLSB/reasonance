use crate::agent_event::AgentEvent;
use crate::transport::session_handle::{SessionHandle, SessionSummary};
#[allow(unused_imports)]
use log::{info, warn, error, debug, trace};
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// Handles all disk I/O for session persistence.
///
/// Directory structure:
///   {base_dir}/
///     index.json              — Vec<SessionSummary>
///     {session-id}/
///       metadata.json         — SessionHandle (without events)
///       events.jsonl          — one AgentEvent per line, append-only
pub struct SessionStore {
    base_dir: PathBuf,
}

impl SessionStore {
    pub fn new(base_dir: &Path) -> Result<Self, String> {
        debug!("SessionStore: initializing at {}", base_dir.display());
        fs::create_dir_all(base_dir).map_err(|e| {
            error!("SessionStore: failed to create sessions dir {}: {}", base_dir.display(), e);
            format!("Failed to create sessions dir: {}", e)
        })?;
        Ok(Self {
            base_dir: base_dir.to_path_buf(),
        })
    }

    /// Create the directory structure for a new session and write initial metadata.
    pub fn create_session(&self, handle: &SessionHandle) -> Result<(), String> {
        let session_dir = self.session_dir(&handle.id);
        debug!("SessionStore: creating session dir at {}", session_dir.display());
        fs::create_dir_all(&session_dir)
            .map_err(|e| {
                error!("SessionStore: failed to create session dir {}: {}", session_dir.display(), e);
                format!("Failed to create session dir: {}", e)
            })?;

        self.write_metadata(handle)?;

        // Create empty events.jsonl
        let events_path = session_dir.join("events.jsonl");
        fs::File::create(&events_path)
            .map_err(|e| {
                error!("SessionStore: failed to create events file {}: {}", events_path.display(), e);
                format!("Failed to create events file: {}", e)
            })?;

        debug!("SessionStore: session={} created on disk", handle.id);
        Ok(())
    }

    /// Append a single event to the session's JSONL file.
    pub fn append_event(&self, session_id: &str, event: &AgentEvent) -> Result<(), String> {
        let events_path = self.session_dir(session_id).join("events.jsonl");
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)
            .map_err(|e| {
                error!("SessionStore: failed to open events file for session={}: {}", session_id, e);
                format!("Failed to open events file: {}", e)
            })?;

        let mut line = serde_json::to_string(event)
            .map_err(|e| {
                error!("SessionStore: failed to serialize event for session={}: {}", session_id, e);
                format!("Failed to serialize event: {}", e)
            })?;
        line.push('\n');
        trace!("SessionStore: appending event to session={}: {}", session_id, line.trim());
        file.write_all(line.as_bytes())
            .map_err(|e| {
                error!("SessionStore: failed to write event for session={}: {}", session_id, e);
                format!("Failed to write event: {}", e)
            })?;

        Ok(())
    }

    /// Read all events from a session's JSONL file.
    pub fn read_events(&self, session_id: &str) -> Result<Vec<AgentEvent>, String> {
        let events_path = self.session_dir(session_id).join("events.jsonl");
        if !events_path.exists() {
            debug!("SessionStore: no events file for session={}, returning empty", session_id);
            return Ok(vec![]);
        }

        debug!("SessionStore: reading events for session={}", session_id);
        let file = fs::File::open(&events_path)
            .map_err(|e| {
                error!("SessionStore: failed to open events file for session={}: {}", session_id, e);
                format!("Failed to open events file: {}", e)
            })?;
        let reader = std::io::BufReader::new(file);

        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| {
                error!("SessionStore: failed to read line from events file for session={}: {}", session_id, e);
                format!("Failed to read line: {}", e)
            })?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let event: AgentEvent = serde_json::from_str(trimmed)
                .map_err(|e| {
                    error!("SessionStore: failed to parse event for session={}: {}", session_id, e);
                    format!("Failed to parse event: {}", e)
                })?;
            events.push(event);
        }

        debug!("SessionStore: read {} events for session={}", events.len(), session_id);
        Ok(events)
    }

    /// Atomic write: write to .tmp then rename, so a crash mid-write doesn't corrupt the file.
    fn write_atomic(&self, path: &Path, content: &[u8]) -> Result<(), String> {
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, content)
            .map_err(|e| format!("Failed to write tmp file: {}", e))?;
        fs::rename(&tmp_path, path)
            .map_err(|e| format!("Failed to rename tmp file: {}", e))?;
        Ok(())
    }

    /// Write session metadata to disk (atomic write).
    pub fn write_metadata(&self, handle: &SessionHandle) -> Result<(), String> {
        let metadata_path = self.session_dir(&handle.id).join("metadata.json");
        debug!("SessionStore: writing metadata for session={}", handle.id);
        let json = serde_json::to_string_pretty(handle)
            .map_err(|e| {
                error!("SessionStore: failed to serialize metadata for session={}: {}", handle.id, e);
                format!("Failed to serialize metadata: {}", e)
            })?;
        self.write_atomic(&metadata_path, json.as_bytes())
    }

    /// Read session metadata from disk.
    pub fn read_metadata(&self, session_id: &str) -> Result<SessionHandle, String> {
        let metadata_path = self.session_dir(session_id).join("metadata.json");
        debug!("SessionStore: reading metadata for session={}", session_id);
        let json = fs::read_to_string(&metadata_path)
            .map_err(|e| {
                error!("SessionStore: failed to read metadata for session={}: {}", session_id, e);
                format!("Failed to read metadata: {}", e)
            })?;
        serde_json::from_str(&json)
            .map_err(|e| {
                error!("SessionStore: failed to parse metadata for session={}: {}", session_id, e);
                format!("Failed to parse metadata: {}", e)
            })
    }

    /// Write the session index (list of summaries) to disk (atomic write).
    pub fn write_index(&self, summaries: &[SessionSummary]) -> Result<(), String> {
        let index_path = self.base_dir.join("index.json");
        debug!("SessionStore: writing index with {} entries", summaries.len());
        let json = serde_json::to_string_pretty(summaries)
            .map_err(|e| {
                error!("SessionStore: failed to serialize index: {}", e);
                format!("Failed to serialize index: {}", e)
            })?;
        self.write_atomic(&index_path, json.as_bytes())
    }

    /// Read the session index from disk.
    pub fn read_index(&self) -> Result<Vec<SessionSummary>, String> {
        let index_path = self.base_dir.join("index.json");
        if !index_path.exists() {
            debug!("SessionStore: no index file found, returning empty");
            return Ok(vec![]);
        }
        debug!("SessionStore: reading index from {}", index_path.display());
        let json = fs::read_to_string(&index_path)
            .map_err(|e| {
                error!("SessionStore: failed to read index: {}", e);
                format!("Failed to read index: {}", e)
            })?;
        serde_json::from_str(&json)
            .map_err(|e| {
                error!("SessionStore: failed to parse index: {}", e);
                format!("Failed to parse index: {}", e)
            })
    }

    /// Delete a session directory.
    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let session_dir = self.session_dir(session_id);
        if session_dir.exists() {
            debug!("SessionStore: deleting session dir {}", session_dir.display());
            fs::remove_dir_all(&session_dir)
                .map_err(|e| {
                    error!("SessionStore: failed to delete session dir for session={}: {}", session_id, e);
                    format!("Failed to delete session: {}", e)
                })?;
        } else {
            warn!("SessionStore: attempted to delete non-existent session={}", session_id);
        }
        Ok(())
    }

    /// Check if a session exists on disk.
    pub fn session_exists(&self, session_id: &str) -> bool {
        self.session_dir(session_id).join("metadata.json").exists()
    }

    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::AgentEvent;
    use crate::transport::session_handle::SessionHandle;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SessionStore) {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::new(dir.path()).unwrap();
        (dir, store)
    }

    #[test]
    fn test_create_and_read_metadata() {
        let (_dir, store) = setup();
        let handle = SessionHandle::new("claude", "claude-sonnet-4-6");
        let id = handle.id.clone();

        store.create_session(&handle).unwrap();
        assert!(store.session_exists(&id));

        let loaded = store.read_metadata(&id).unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.provider, "claude");
    }

    #[test]
    fn test_append_and_read_events() {
        let (_dir, store) = setup();
        let handle = SessionHandle::new("claude", "sonnet");
        store.create_session(&handle).unwrap();

        let event1 = AgentEvent::text("hello", "claude");
        let event2 = AgentEvent::text("world", "claude");

        store.append_event(&handle.id, &event1).unwrap();
        store.append_event(&handle.id, &event2).unwrap();

        let events = store.read_events(&handle.id).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_read_events_empty() {
        let (_dir, store) = setup();
        let handle = SessionHandle::new("claude", "sonnet");
        store.create_session(&handle).unwrap();

        let events = store.read_events(&handle.id).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_read_events_nonexistent() {
        let (_dir, store) = setup();
        let events = store.read_events("nonexistent").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_write_and_read_index() {
        let (_dir, store) = setup();
        let h1 = SessionHandle::new("claude", "sonnet");
        let h2 = SessionHandle::new("claude", "opus");

        let summaries = vec![h1.to_summary(), h2.to_summary()];
        store.write_index(&summaries).unwrap();

        let loaded = store.read_index().unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_read_index_empty() {
        let (_dir, store) = setup();
        let loaded = store.read_index().unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_delete_session() {
        let (_dir, store) = setup();
        let handle = SessionHandle::new("claude", "sonnet");
        store.create_session(&handle).unwrap();
        assert!(store.session_exists(&handle.id));

        store.delete_session(&handle.id).unwrap();
        assert!(!store.session_exists(&handle.id));
    }

    #[test]
    fn test_update_metadata() {
        let (_dir, store) = setup();
        let mut handle = SessionHandle::new("claude", "sonnet");
        store.create_session(&handle).unwrap();

        handle.title = "My chat".to_string();
        handle.event_count = 5u32;
        store.write_metadata(&handle).unwrap();

        let loaded = store.read_metadata(&handle.id).unwrap();
        assert_eq!(loaded.title, "My chat");
        assert_eq!(loaded.event_count, 5u32);
    }
}
