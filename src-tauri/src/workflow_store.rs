use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Agent,
    Resource,
    Logic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNodeConfig {
    pub llm: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub retry: Option<u32>,
    #[serde(default)]
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNodeConfig {
    pub kind: String,
    pub path: Option<String>,
    #[serde(default = "default_access")]
    pub access: String,
}
fn default_access() -> String {
    "read".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicNodeConfig {
    pub kind: String,
    pub rule: String,
    #[serde(default)]
    pub on_true: Option<String>,
    #[serde(default)]
    pub on_false: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub label: String,
    pub config: serde_json::Value,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSettings {
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_agents: u32,
    #[serde(default = "default_retry")]
    pub default_retry: u32,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}
fn default_max_concurrent() -> u32 {
    5
}
fn default_retry() -> u32 {
    2
}
fn default_timeout() -> u64 {
    300
}
impl Default for WorkflowSettings {
    fn default() -> Self {
        Self {
            max_concurrent_agents: default_max_concurrent(),
            default_retry: default_retry(),
            timeout: default_timeout(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub modified: Option<String>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    #[serde(default)]
    pub settings: WorkflowSettings,
}
fn default_version() -> String {
    "1.0".to_string()
}

pub struct WorkflowStore {
    pub workflows: Arc<Mutex<HashMap<String, Workflow>>>,
}

impl WorkflowStore {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn global_dir() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.join("reasonance").join("workflows")
    }

    pub fn project_dir(project_root: &str) -> PathBuf {
        PathBuf::from(project_root).join(".reasonance").join("workflows")
    }

    pub fn load(&self, file_path: &str) -> Result<Workflow, String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read {}: {}", file_path, e))?;
        let workflow: Workflow = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid workflow JSON: {}", e))?;
        self.workflows
            .lock()
            .unwrap()
            .insert(file_path.to_string(), workflow.clone());
        Ok(workflow)
    }

    pub fn save(&self, file_path: &str, workflow: &Workflow) -> Result<(), String> {
        let parent = std::path::Path::new(file_path)
            .parent()
            .ok_or("Invalid file path")?;
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
        let json = serde_json::to_string_pretty(workflow)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        std::fs::write(file_path, json)
            .map_err(|e| format!("Failed to write {}: {}", file_path, e))?;
        self.workflows
            .lock()
            .unwrap()
            .insert(file_path.to_string(), workflow.clone());
        Ok(())
    }

    pub fn list_workflows(dir: &str) -> Result<Vec<String>, String> {
        let path = std::path::Path::new(dir);
        if !path.exists() {
            return Ok(vec![]);
        }
        let entries =
            std::fs::read_dir(path).map_err(|e| format!("Failed to read dir: {}", e))?;
        let mut workflows = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(s) = p.to_str() {
                    workflows.push(s.to_string());
                }
            }
        }
        workflows.sort();
        Ok(workflows)
    }

    pub fn delete(&self, file_path: &str) -> Result<(), String> {
        if std::path::Path::new(file_path).exists() {
            std::fs::remove_file(file_path)
                .map_err(|e| format!("Failed to delete: {}", e))?;
        }
        self.workflows.lock().unwrap().remove(file_path);
        Ok(())
    }

    pub fn get(&self, file_path: &str) -> Option<Workflow> {
        self.workflows.lock().unwrap().get(file_path).cloned()
    }

    pub fn create_empty(name: &str) -> Workflow {
        let now = chrono::Utc::now().format("%Y-%m-%d").to_string();
        Workflow {
            name: name.to_string(),
            version: "1.0".to_string(),
            description: None,
            created: Some(now.clone()),
            modified: Some(now),
            nodes: vec![],
            edges: vec![],
            settings: WorkflowSettings::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_workflow() -> Workflow {
        Workflow {
            name: "Test Workflow".to_string(),
            version: "1.0".to_string(),
            description: Some("A test workflow".to_string()),
            created: Some("2026-03-21".to_string()),
            modified: Some("2026-03-21".to_string()),
            nodes: vec![
                WorkflowNode {
                    id: "agent-1".to_string(),
                    node_type: NodeType::Agent,
                    label: "Writer".to_string(),
                    config: serde_json::json!({"llm": "claude"}),
                    position: Position { x: 100.0, y: 100.0 },
                },
                WorkflowNode {
                    id: "resource-1".to_string(),
                    node_type: NodeType::Resource,
                    label: "Codebase".to_string(),
                    config: serde_json::json!({"kind": "folder", "path": "/src"}),
                    position: Position { x: 300.0, y: 100.0 },
                },
            ],
            edges: vec![
                WorkflowEdge {
                    id: "e1".to_string(),
                    from: "resource-1".to_string(),
                    to: "agent-1".to_string(),
                    label: None,
                },
            ],
            settings: WorkflowSettings::default(),
        }
    }

    #[test]
    fn test_create_empty() {
        let wf = WorkflowStore::create_empty("My Swarm");
        assert_eq!(wf.name, "My Swarm");
        assert_eq!(wf.version, "1.0");
        assert!(wf.nodes.is_empty());
        assert!(wf.edges.is_empty());
        assert!(wf.created.is_some());
        assert_eq!(wf.settings.max_concurrent_agents, 5);
    }

    #[test]
    fn test_save_and_load() {
        let store = WorkflowStore::new();
        let wf = sample_workflow();
        let dir = std::env::temp_dir().join("reasonance_test_wf");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.json");
        let path_str = path.to_str().unwrap();

        store.save(path_str, &wf).unwrap();
        let loaded = store.load(path_str).unwrap();
        assert_eq!(loaded.name, "Test Workflow");
        assert_eq!(loaded.nodes.len(), 2);
        assert_eq!(loaded.edges.len(), 1);

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_delete() {
        let store = WorkflowStore::new();
        let wf = WorkflowStore::create_empty("Delete Me");
        let dir = std::env::temp_dir().join("reasonance_test_del");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("delete_me.json");
        let path_str = path.to_str().unwrap();

        store.save(path_str, &wf).unwrap();
        assert!(path.exists());
        store.delete(path_str).unwrap();
        assert!(!path.exists());

        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_list_workflows() {
        let dir = std::env::temp_dir().join("reasonance_test_list");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.json"), "{}").unwrap();
        std::fs::write(dir.join("b.json"), "{}").unwrap();
        std::fs::write(dir.join("c.txt"), "not json").unwrap();

        let list = WorkflowStore::list_workflows(dir.to_str().unwrap()).unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|p| p.ends_with("a.json")));
        assert!(list.iter().any(|p| p.ends_with("b.json")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_get_cached() {
        let store = WorkflowStore::new();
        let wf = sample_workflow();
        let dir = std::env::temp_dir().join("reasonance_test_cache");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("cached.json");
        let path_str = path.to_str().unwrap();

        store.save(path_str, &wf).unwrap();
        let cached = store.get(path_str);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().name, "Test Workflow");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_load_invalid_json() {
        let store = WorkflowStore::new();
        let dir = std::env::temp_dir().join("reasonance_test_invalid");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("invalid.json");
        std::fs::write(&path, "not valid json").unwrap();

        let result = store.load(path.to_str().unwrap());
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let wf = sample_workflow();
        let json = serde_json::to_string(&wf).unwrap();
        let deserialized: Workflow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, wf.name);
        assert_eq!(deserialized.nodes.len(), wf.nodes.len());
        assert_eq!(deserialized.edges.len(), wf.edges.len());
    }

    #[test]
    fn test_default_settings() {
        let settings = WorkflowSettings::default();
        assert_eq!(settings.max_concurrent_agents, 5);
        assert_eq!(settings.default_retry, 2);
        assert_eq!(settings.timeout, 300);
    }

    #[test]
    fn test_global_dir() {
        let dir = WorkflowStore::global_dir();
        assert!(dir.to_str().unwrap().contains("reasonance"));
        assert!(dir.to_str().unwrap().contains("workflows"));
    }

    #[test]
    fn test_project_dir() {
        let dir = WorkflowStore::project_dir("/my/project");
        assert_eq!(dir, std::path::PathBuf::from("/my/project/.reasonance/workflows"));
    }
}
