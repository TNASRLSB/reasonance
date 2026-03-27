use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

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
pub struct MemoryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_max_entries", rename = "maxEntries")]
    pub max_entries: u32,
    #[serde(default = "default_persist")]
    pub persist: String,
}
fn default_max_entries() -> u32 {
    50
}
fn default_persist() -> String {
    "none".to_string()
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
    #[serde(default)]
    pub memory: Option<MemoryConfig>,
    /// Per-node timeout in seconds. Overrides `WorkflowSettings.timeout` when set.
    #[serde(default)]
    pub timeout: Option<u64>,
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
    #[serde(default, rename = "onTrue")]
    pub on_true: Option<String>,
    #[serde(default, rename = "onFalse")]
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
    #[serde(default)]
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
    #[serde(default = "default_permission_level", rename = "permissionLevel")]
    pub permission_level: String,
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
fn default_permission_level() -> String {
    "supervised".to_string()
}
impl Default for WorkflowSettings {
    fn default() -> Self {
        Self {
            max_concurrent_agents: default_max_concurrent(),
            default_retry: default_retry(),
            timeout: default_timeout(),
            permission_level: default_permission_level(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default, rename = "schemaVersion")]
    pub schema_version: u32,
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

pub fn migrate(wf: &mut Workflow) {
    if wf.schema_version < 1 {
        // Add IDs to edges missing them
        for edge in &mut wf.edges {
            if edge.id.is_empty() {
                edge.id = Uuid::new_v4().to_string();
            }
        }
        // Ensure permission_level default
        if wf.settings.permission_level.is_empty() {
            wf.settings.permission_level = default_permission_level();
        }
        wf.schema_version = 1;
    }
}

pub struct WorkflowStore {
    /// Workflow definitions loaded from disk. Plain HashMap: bounded (loaded from
    /// finite config files), no dynamic lifecycle — entries persist until app exit.
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

    #[allow(dead_code)] // Used for project-scoped workflow storage
    pub fn project_dir(project_root: &str) -> PathBuf {
        PathBuf::from(project_root)
            .join(".reasonance")
            .join("workflows")
    }

    pub fn load(&self, file_path: &str) -> Result<Workflow, crate::error::ReasonanceError> {
        let content = std::fs::read_to_string(file_path).map_err(|e| {
            crate::error::ReasonanceError::io(format!("read workflow {}", file_path), e)
        })?;
        let mut workflow: Workflow = serde_json::from_str(&content)?;
        migrate(&mut workflow);
        self.workflows
            .lock()
            .unwrap()
            .insert(file_path.to_string(), workflow.clone());
        Ok(workflow)
    }

    pub fn save(
        &self,
        file_path: &str,
        workflow: &Workflow,
    ) -> Result<(), crate::error::ReasonanceError> {
        let parent = std::path::Path::new(file_path).parent().ok_or_else(|| {
            crate::error::ReasonanceError::validation("file_path", "Invalid file path")
        })?;
        std::fs::create_dir_all(parent)
            .map_err(|e| crate::error::ReasonanceError::io("create workflow directory", e))?;
        let json = serde_json::to_string_pretty(workflow)?;
        std::fs::write(file_path, json).map_err(|e| {
            crate::error::ReasonanceError::io(format!("write workflow {}", file_path), e)
        })?;
        self.workflows
            .lock()
            .unwrap()
            .insert(file_path.to_string(), workflow.clone());
        Ok(())
    }

    pub fn list_workflows(dir: &str) -> Result<Vec<String>, crate::error::ReasonanceError> {
        let path = std::path::Path::new(dir);
        if !path.exists() {
            return Ok(vec![]);
        }
        let entries = std::fs::read_dir(path).map_err(|e| {
            crate::error::ReasonanceError::io(format!("read workflow dir {}", dir), e)
        })?;
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

    pub fn delete(&self, file_path: &str) -> Result<(), crate::error::ReasonanceError> {
        if std::path::Path::new(file_path).exists() {
            std::fs::remove_file(file_path).map_err(|e| {
                crate::error::ReasonanceError::io(format!("delete workflow {}", file_path), e)
            })?;
        }
        self.workflows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(file_path);
        Ok(())
    }

    pub fn get(&self, file_path: &str) -> Option<Workflow> {
        self.workflows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(file_path)
            .cloned()
    }

    pub fn create_empty(name: &str) -> Workflow {
        let now = chrono::Utc::now().format("%Y-%m-%d").to_string();
        Workflow {
            name: name.to_string(),
            version: "1.0".to_string(),
            schema_version: 1,
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
            schema_version: 1,
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
            edges: vec![WorkflowEdge {
                id: "e1".to_string(),
                from: "resource-1".to_string(),
                to: "agent-1".to_string(),
                label: None,
            }],
            settings: WorkflowSettings::default(),
        }
    }

    #[test]
    fn test_create_empty() {
        let wf = WorkflowStore::create_empty("My Hive");
        assert_eq!(wf.name, "My Hive");
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
        assert_eq!(
            dir,
            std::path::PathBuf::from("/my/project/.reasonance/workflows")
        );
    }

    #[test]
    fn test_schema_version_default() {
        let wf = WorkflowStore::create_empty("Versioned");
        assert_eq!(wf.schema_version, 1);
    }

    #[test]
    fn test_schema_version_missing_defaults_to_zero() {
        let json = r#"{"name":"Old","nodes":[],"edges":[]}"#;
        let wf: Workflow = serde_json::from_str(json).unwrap();
        assert_eq!(wf.schema_version, 0);
    }

    #[test]
    fn test_permission_level_default() {
        let settings = WorkflowSettings::default();
        assert_eq!(settings.permission_level, "supervised");
    }

    #[test]
    fn test_permission_level_deserialization() {
        let json = r#"{"permissionLevel":"trusted"}"#;
        let settings: WorkflowSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.permission_level, "trusted");
    }

    #[test]
    fn test_agent_memory_config() {
        let json =
            r#"{"llm":"claude","memory":{"enabled":true,"maxEntries":100,"persist":"workflow"}}"#;
        let config: AgentNodeConfig = serde_json::from_str(json).unwrap();
        assert!(config.memory.is_some());
        let mem = config.memory.unwrap();
        assert!(mem.enabled);
        assert_eq!(mem.max_entries, 100);
        assert_eq!(mem.persist, "workflow");
    }

    #[test]
    fn test_agent_memory_config_default_none() {
        let json = r#"{"llm":"claude"}"#;
        let config: AgentNodeConfig = serde_json::from_str(json).unwrap();
        assert!(config.memory.is_none());
    }

    #[test]
    fn test_migrate_v0_to_v1() {
        let json = r#"{
            "name": "Legacy",
            "nodes": [{"id":"a1","type":"agent","label":"X","config":{"llm":"claude"},"position":{"x":0,"y":0}}],
            "edges": [{"from":"a1","to":"a1"}],
            "settings": {}
        }"#;
        let mut wf: Workflow = serde_json::from_str(json).unwrap();
        migrate(&mut wf);
        assert_eq!(wf.schema_version, 1);
        assert_eq!(wf.settings.permission_level, "supervised");
        assert!(!wf.edges[0].id.is_empty());
    }
}
