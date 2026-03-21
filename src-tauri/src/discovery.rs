use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProfile {
    pub read_file: bool,
    pub write_file: bool,
    pub execute_command: bool,
    pub web_search: bool,
    pub image_input: bool,
    pub long_context: bool,
}

impl Default for CapabilityProfile {
    fn default() -> Self {
        Self {
            read_file: false,
            write_file: false,
            execute_command: false,
            web_search: false,
            image_input: false,
            long_context: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiscoverySource {
    Cli,
    Api,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredAgent {
    pub name: String,
    pub source: DiscoverySource,
    pub command: Option<String>,
    pub endpoint: Option<String>,
    pub models: Vec<String>,
    pub capabilities: CapabilityProfile,
    pub max_context: Option<u64>,
    pub available: bool,
}

fn builtin_profiles() -> HashMap<String, (CapabilityProfile, Vec<String>, Option<u64>)> {
    let mut profiles = HashMap::new();
    profiles.insert(
        "claude".to_string(),
        (
            CapabilityProfile {
                read_file: true,
                write_file: true,
                execute_command: true,
                web_search: false,
                image_input: true,
                long_context: true,
            },
            vec!["opus".into(), "sonnet".into(), "haiku".into()],
            Some(200_000),
        ),
    );
    profiles.insert(
        "gemini".to_string(),
        (
            CapabilityProfile {
                read_file: true,
                write_file: true,
                execute_command: true,
                web_search: true,
                image_input: true,
                long_context: true,
            },
            vec!["pro".into(), "flash".into()],
            Some(1_000_000),
        ),
    );
    profiles.insert(
        "aider".to_string(),
        (
            CapabilityProfile {
                read_file: true,
                write_file: true,
                execute_command: true,
                web_search: false,
                image_input: false,
                long_context: false,
            },
            vec![],
            None,
        ),
    );
    profiles.insert("ollama".to_string(), (CapabilityProfile::default(), vec![], None));
    profiles
}

pub struct DiscoveryEngine {
    pub agents: Arc<Mutex<Vec<DiscoveredAgent>>>,
}

impl DiscoveryEngine {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn scan_cli(&self) -> Vec<DiscoveredAgent> {
        let candidates = vec![
            ("Claude", "claude"),
            ("Gemini", "gemini"),
            ("Aider", "aider"),
            ("GitHub Copilot", "github-copilot-cli"),
            ("Ollama", "ollama"),
            ("Open Interpreter", "interpreter"),
        ];
        let profiles = builtin_profiles();
        let discovered: Vec<DiscoveredAgent> = candidates
            .into_iter()
            .filter_map(|(name, cmd)| {
                let found = if cfg!(target_os = "windows") {
                    std::process::Command::new("where").arg(cmd).output()
                } else {
                    std::process::Command::new("which").arg(cmd).output()
                }
                .map(|o| o.status.success())
                .unwrap_or(false);
                if !found {
                    return None;
                }
                let (capabilities, models, max_context) = profiles
                    .get(cmd)
                    .cloned()
                    .unwrap_or((CapabilityProfile::default(), vec![], None));
                Some(DiscoveredAgent {
                    name: name.to_string(),
                    source: DiscoverySource::Cli,
                    command: Some(cmd.to_string()),
                    endpoint: None,
                    models,
                    capabilities,
                    max_context,
                    available: true,
                })
            })
            .collect();
        let mut agents = self.agents.lock().unwrap();
        agents.retain(|a| a.source != DiscoverySource::Cli);
        agents.extend(discovered.clone());
        discovered
    }

    pub async fn probe_apis(&self) -> Vec<DiscoveredAgent> {
        let mut discovered = Vec::new();
        if let Ok(agents) = Self::probe_ollama().await {
            discovered.extend(agents);
        }
        let mut agents = self.agents.lock().unwrap();
        agents.retain(|a| a.source != DiscoverySource::Api);
        agents.extend(discovered.clone());
        discovered
    }

    async fn probe_ollama() -> Result<Vec<DiscoveredAgent>, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .get("http://localhost:11434/api/tags")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err("Ollama API returned non-200".to_string());
        }
        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let models: Vec<String> = body
            .get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        if models.is_empty() {
            return Ok(vec![]);
        }
        Ok(vec![DiscoveredAgent {
            name: "Ollama (API)".to_string(),
            source: DiscoverySource::Api,
            command: None,
            endpoint: Some("http://localhost:11434".to_string()),
            models,
            capabilities: CapabilityProfile::default(),
            max_context: None,
            available: true,
        }])
    }

    pub async fn discover_all(&self) -> Vec<DiscoveredAgent> {
        self.scan_cli();
        self.probe_apis().await;
        self.agents.lock().unwrap().clone()
    }

    pub fn get_agents(&self) -> Vec<DiscoveredAgent> {
        self.agents.lock().unwrap().clone()
    }
}
