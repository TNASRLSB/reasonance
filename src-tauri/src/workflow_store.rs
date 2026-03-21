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
