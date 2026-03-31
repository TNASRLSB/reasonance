use crate::error::ReasonanceError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub run_id: String,
    pub timestamp: String,
    pub input_summary: String,
    pub output_summary: String,
    pub outcome: String,
    #[serde(default)]
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemoryStore {
    pub node_id: String,
    pub entries: Vec<MemoryEntry>,
}

// v1 module retained for migration (AgentMemoryV2::migrate_from_json) and the
// get_agent_memory Tauri command.  Methods only exercised by tests are allowed
// to be "dead" from the library's perspective.
impl AgentMemoryStore {
    #[cfg(test)]
    pub fn new(node_id: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            entries: Vec::new(),
        }
    }

    pub fn load(path: &str) -> Result<Self, ReasonanceError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ReasonanceError::io("agent memory", e))?;
        let store = serde_json::from_str(&content)?;
        Ok(store)
    }

    #[cfg(test)]
    pub fn save(&self, path: &str) -> Result<(), ReasonanceError> {
        let p = std::path::Path::new(path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ReasonanceError::io("agent memory", e))?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json).map_err(|e| ReasonanceError::io("agent memory", e))
    }

    #[cfg(test)]
    pub fn add_entry(&mut self, entry: MemoryEntry, max_entries: u32) {
        self.entries.push(entry);
        let max = max_entries as usize;
        while self.entries.len() > max {
            self.entries.remove(0);
        }
    }

    pub fn workflow_memory_path(workflow_path: &str, node_id: &str) -> PathBuf {
        let wf = std::path::Path::new(workflow_path);
        let dir = wf.parent().unwrap_or_else(|| std::path::Path::new("."));
        dir.join("agent-memory").join(format!("{}.json", node_id))
    }

    pub fn global_memory_path(node_id: &str) -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir
            .join("reasonance")
            .join("agent-memory")
            .join(format!("{}.json", node_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_entry_fifo_eviction() {
        let mut store = AgentMemoryStore::new("test-node");
        for i in 0..5 {
            store.add_entry(
                MemoryEntry {
                    run_id: format!("run-{}", i),
                    timestamp: format!("2026-03-24T00:0{}:00Z", i),
                    input_summary: String::new(),
                    output_summary: String::new(),
                    outcome: "success".to_string(),
                    context: serde_json::Value::Null,
                },
                3,
            );
        }
        assert_eq!(store.entries.len(), 3);
        assert_eq!(store.entries[0].run_id, "run-2");
        assert_eq!(store.entries[1].run_id, "run-3");
        assert_eq!(store.entries[2].run_id, "run-4");
    }

    #[test]
    fn test_save_and_load() {
        let mut store = AgentMemoryStore::new("save-load-node");
        store.add_entry(
            MemoryEntry {
                run_id: "run-1".to_string(),
                timestamp: "2026-03-24T00:00:00Z".to_string(),
                input_summary: "test input".to_string(),
                output_summary: "test output".to_string(),
                outcome: "success".to_string(),
                context: serde_json::json!({"key": "value"}),
            },
            50,
        );

        let dir = std::env::temp_dir().join("reasonance_test_agent_memory");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("save-load-node.json");
        let path_str = path.to_str().unwrap();

        store.save(path_str).unwrap();
        let loaded = AgentMemoryStore::load(path_str).unwrap();
        assert_eq!(loaded.node_id, "save-load-node");
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].run_id, "run-1");
        assert_eq!(loaded.entries[0].input_summary, "test input");
        assert_eq!(loaded.entries[0].context["key"], "value");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_missing_file() {
        let result = AgentMemoryStore::load("/nonexistent/path/memory.json");
        assert!(result.is_err());
    }
}
