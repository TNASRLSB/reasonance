use crate::error::ReasonanceError;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub struct ResourceLockManager {
    /// Reader lock sets — resource_id → agent_ids holding read locks.
    /// Plain HashMap: mutex-set semantics (not lifecycle-tracked resources).
    readers: Arc<Mutex<HashMap<String, HashSet<String>>>>,
    /// Writer lock — resource_id → single agent_id holding write lock.
    /// Plain HashMap: mutex-set semantics (not lifecycle-tracked resources).
    writers: Arc<Mutex<HashMap<String, String>>>,
}

impl ResourceLockManager {
    pub fn new() -> Self {
        Self {
            readers: Arc::new(Mutex::new(HashMap::new())),
            writers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Acquire a read or write lock on a resource for an agent.
    /// Write lock: fails if any readers or another writer exists.
    /// Read lock: fails if a writer exists.
    pub fn acquire(
        &self,
        resource_id: &str,
        agent_id: &str,
        write: bool,
    ) -> Result<(), ReasonanceError> {
        let mut readers = self.readers.lock().unwrap_or_else(|e| e.into_inner());
        let mut writers = self.writers.lock().unwrap_or_else(|e| e.into_inner());

        if write {
            // Write lock: no readers and no other writer
            if let Some(reader_set) = readers.get(resource_id) {
                if !reader_set.is_empty() {
                    return Err(ReasonanceError::workflow(
                        "",
                        resource_id,
                        format!(
                            "Resource {} has active readers: {:?}",
                            resource_id, reader_set
                        ),
                    ));
                }
            }
            if let Some(existing_writer) = writers.get(resource_id) {
                return Err(ReasonanceError::workflow(
                    "",
                    resource_id,
                    format!(
                        "Resource {} already has writer: {}",
                        resource_id, existing_writer
                    ),
                ));
            }
            writers.insert(resource_id.to_string(), agent_id.to_string());
        } else {
            // Read lock: no writer
            if let Some(existing_writer) = writers.get(resource_id) {
                return Err(ReasonanceError::workflow(
                    "",
                    resource_id,
                    format!(
                        "Resource {} has active writer: {}",
                        resource_id, existing_writer
                    ),
                ));
            }
            readers
                .entry(resource_id.to_string())
                .or_default()
                .insert(agent_id.to_string());
        }

        Ok(())
    }

    /// Release whatever lock this agent holds on a resource.
    pub fn release(&self, resource_id: &str, agent_id: &str) {
        let mut readers = self.readers.lock().unwrap_or_else(|e| e.into_inner());
        let mut writers = self.writers.lock().unwrap_or_else(|e| e.into_inner());

        // Remove from writers if this agent holds the write lock
        if let Some(writer) = writers.get(resource_id) {
            if writer == agent_id {
                writers.remove(resource_id);
            }
        }

        // Remove from readers
        if let Some(reader_set) = readers.get_mut(resource_id) {
            reader_set.remove(agent_id);
            if reader_set.is_empty() {
                readers.remove(resource_id);
            }
        }
    }

    /// Release all locks held by an agent (for stop/error cleanup).
    pub fn release_all(&self, agent_id: &str) {
        let mut readers = self.readers.lock().unwrap_or_else(|e| e.into_inner());
        let mut writers = self.writers.lock().unwrap_or_else(|e| e.into_inner());

        // Remove from all writer entries
        let writer_resources: Vec<String> = writers
            .iter()
            .filter(|(_, aid)| *aid == agent_id)
            .map(|(rid, _)| rid.clone())
            .collect();
        for rid in writer_resources {
            writers.remove(&rid);
        }

        // Remove from all reader entries
        let mut empty_reader_keys = Vec::new();
        for (rid, reader_set) in readers.iter_mut() {
            reader_set.remove(agent_id);
            if reader_set.is_empty() {
                empty_reader_keys.push(rid.clone());
            }
        }
        for rid in empty_reader_keys {
            readers.remove(&rid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_lock_allows_multiple_readers() {
        let mgr = ResourceLockManager::new();
        mgr.acquire("res1", "agent_a", false).unwrap();
        mgr.acquire("res1", "agent_b", false).unwrap();
        mgr.acquire("res1", "agent_c", false).unwrap();

        let readers = mgr.readers.lock().unwrap();
        let set = readers.get("res1").unwrap();
        assert_eq!(set.len(), 3);
        assert!(set.contains("agent_a"));
        assert!(set.contains("agent_b"));
        assert!(set.contains("agent_c"));
    }

    #[test]
    fn test_write_lock_exclusive() {
        let mgr = ResourceLockManager::new();
        mgr.acquire("res1", "agent_a", true).unwrap();

        // Second write lock should fail
        let result = mgr.acquire("res1", "agent_b", true);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already has writer"));
    }

    #[test]
    fn test_write_blocked_by_readers() {
        let mgr = ResourceLockManager::new();
        mgr.acquire("res1", "agent_a", false).unwrap();

        // Write lock should fail because there are readers
        let result = mgr.acquire("res1", "agent_b", true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("active readers"));
    }

    #[test]
    fn test_read_blocked_by_writer() {
        let mgr = ResourceLockManager::new();
        mgr.acquire("res1", "agent_a", true).unwrap();

        // Read lock should fail because there is a writer
        let result = mgr.acquire("res1", "agent_b", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("active writer"));
    }

    #[test]
    fn test_release_allows_reacquire() {
        let mgr = ResourceLockManager::new();
        mgr.acquire("res1", "agent_a", true).unwrap();

        // Cannot acquire while locked
        assert!(mgr.acquire("res1", "agent_b", true).is_err());

        // Release and try again
        mgr.release("res1", "agent_a");
        mgr.acquire("res1", "agent_b", true).unwrap();

        let writers = mgr.writers.lock().unwrap();
        assert_eq!(writers.get("res1").unwrap(), "agent_b");
    }

    #[test]
    fn test_release_all() {
        let mgr = ResourceLockManager::new();
        // agent_a holds write on res1 and read on res2
        mgr.acquire("res1", "agent_a", true).unwrap();
        mgr.acquire("res2", "agent_a", false).unwrap();
        // agent_b holds read on res2
        mgr.acquire("res2", "agent_b", false).unwrap();

        mgr.release_all("agent_a");

        // res1 write lock should be free
        mgr.acquire("res1", "agent_c", true).unwrap();

        // res2 should still have agent_b as reader
        let readers = mgr.readers.lock().unwrap();
        let set = readers.get("res2").unwrap();
        assert_eq!(set.len(), 1);
        assert!(set.contains("agent_b"));
    }
}
