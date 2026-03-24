use log::{debug, info};
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
    profiles.insert(
        "kimi".to_string(),
        (
            CapabilityProfile {
                read_file: true,
                write_file: true,
                execute_command: true,
                web_search: false,
                image_input: false,
                long_context: true,
            },
            vec![],
            Some(128_000),
        ),
    );
    profiles.insert(
        "qwen".to_string(),
        (
            CapabilityProfile {
                read_file: true,
                write_file: true,
                execute_command: true,
                web_search: false,
                image_input: false,
                long_context: true,
            },
            vec![],
            Some(128_000),
        ),
    );
    profiles.insert(
        "codex".to_string(),
        (
            CapabilityProfile {
                read_file: true,
                write_file: true,
                execute_command: true,
                web_search: false,
                image_input: false,
                long_context: true,
            },
            vec![],
            Some(200_000),
        ),
    );
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
        info!("Scanning for CLI agents");
        let candidates = vec![
            ("Claude", "claude"),
            ("Gemini", "gemini"),
            ("Aider", "aider"),
            ("GitHub Copilot", "github-copilot-cli"),
            ("Ollama", "ollama"),
            ("Open Interpreter", "interpreter"),
            ("Kimi", "kimi"),
            ("Qwen Code", "qwen"),
            ("Codex", "codex"),
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
        info!("CLI scan complete: discovered {} agents", discovered.len());
        for agent in &discovered {
            debug!("Discovered CLI agent: name='{}', command={:?}", agent.name, agent.command);
        }
        let mut agents = self.agents.lock().unwrap_or_else(|e| e.into_inner());
        agents.retain(|a| a.source != DiscoverySource::Cli);
        agents.extend(discovered.clone());
        discovered
    }

    pub async fn probe_apis(&self) -> Vec<DiscoveredAgent> {
        info!("Probing API endpoints for agents");
        let mut discovered = Vec::new();
        if let Ok(agents) = Self::probe_ollama().await {
            discovered.extend(agents);
        }

        // OpenAI-compatible on common ports
        for port in [1234, 8080, 5000] {
            let url = format!("http://localhost:{}", port);
            if let Ok(oai_agents) = Self::probe_openai_compatible(&url).await {
                discovered.extend(oai_agents);
            }
        }

        let mut agents = self.agents.lock().unwrap_or_else(|e| e.into_inner());
        agents.retain(|a| a.source != DiscoverySource::Api);
        agents.extend(discovered.clone());
        discovered
    }

    async fn probe_openai_compatible(base_url: &str) -> Result<Vec<DiscoveredAgent>, String> {
        debug!("Probing OpenAI-compatible API at {}", base_url);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .map_err(|e| e.to_string())?;

        let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API returned {}", resp.status()));
        }

        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let models: Vec<String> = body["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| m["id"].as_str().map(String::from))
            .collect();

        if models.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![DiscoveredAgent {
            name: format!("openai-compatible@{}", base_url),
            source: DiscoverySource::Api,
            command: None,
            endpoint: Some(base_url.to_string()),
            models,
            capabilities: CapabilityProfile {
                read_file: false,
                write_file: false,
                execute_command: false,
                web_search: false,
                image_input: false,
                long_context: false,
            },
            max_context: None,
            available: true,
        }])
    }

    async fn probe_ollama() -> Result<Vec<DiscoveredAgent>, String> {
        debug!("Probing Ollama API at localhost:11434");
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
        info!("Starting full agent discovery (CLI + API)");
        self.scan_cli();
        self.probe_apis().await;
        let all = self.agents.lock().unwrap_or_else(|e| e.into_inner()).clone();
        info!("Full discovery complete: {} total agents", all.len());
        all
    }

    pub fn register_custom_agent(&self, agent: DiscoveredAgent) {
        let mut agents = self.agents.lock().unwrap_or_else(|e| e.into_inner());
        agents.retain(|a| a.name != agent.name);
        agents.push(agent);
    }

    pub fn get_agents(&self) -> Vec<DiscoveredAgent> {
        self.agents.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_engine_new_starts_empty() {
        let engine = DiscoveryEngine::new();
        assert!(engine.get_agents().is_empty());
    }

    #[test]
    fn capability_profile_default_is_all_false() {
        let profile = CapabilityProfile::default();
        assert!(!profile.read_file);
        assert!(!profile.write_file);
        assert!(!profile.execute_command);
        assert!(!profile.web_search);
        assert!(!profile.image_input);
        assert!(!profile.long_context);
    }

    #[test]
    fn scan_cli_runs_without_panic() {
        let engine = DiscoveryEngine::new();
        // Should not panic regardless of what tools are installed
        let _agents = engine.scan_cli();
    }

    #[test]
    fn discovered_agent_serialization_roundtrip() {
        let agent = DiscoveredAgent {
            name: "TestAgent".to_string(),
            source: DiscoverySource::Cli,
            command: Some("test-agent".to_string()),
            endpoint: None,
            models: vec!["model-a".to_string(), "model-b".to_string()],
            capabilities: CapabilityProfile {
                read_file: true,
                write_file: false,
                execute_command: true,
                web_search: false,
                image_input: true,
                long_context: false,
            },
            max_context: Some(128_000),
            available: true,
        };

        let json = serde_json::to_string(&agent).unwrap();
        let deserialized: DiscoveredAgent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "TestAgent");
        assert_eq!(deserialized.source, DiscoverySource::Cli);
        assert_eq!(deserialized.command.as_deref(), Some("test-agent"));
        assert!(deserialized.endpoint.is_none());
        assert_eq!(deserialized.models.len(), 2);
        assert!(deserialized.capabilities.read_file);
        assert!(!deserialized.capabilities.write_file);
        assert_eq!(deserialized.max_context, Some(128_000));
        assert!(deserialized.available);
    }

    #[tokio::test]
    async fn test_probe_openai_compatible_timeout() {
        // Should not crash on unreachable endpoint
        let result = DiscoveryEngine::probe_openai_compatible("http://localhost:19999").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_register_custom_agent() {
        let engine = DiscoveryEngine::new();
        let agent = DiscoveredAgent {
            name: "custom-tool".to_string(),
            source: DiscoverySource::Manual,
            command: Some("my-tool".to_string()),
            endpoint: None,
            models: vec!["default".to_string()],
            capabilities: CapabilityProfile {
                read_file: true,
                write_file: true,
                execute_command: false,
                web_search: false,
                image_input: false,
                long_context: false,
            },
            max_context: None,
            available: true,
        };
        engine.register_custom_agent(agent);
        let agents = engine.get_agents();
        assert!(agents.iter().any(|a| a.name == "custom-tool"));
    }

    #[test]
    fn test_register_custom_agent_replaces_existing() {
        let engine = DiscoveryEngine::new();
        let agent1 = DiscoveredAgent {
            name: "custom-tool".to_string(),
            source: DiscoverySource::Manual,
            command: Some("my-tool-v1".to_string()),
            endpoint: None,
            models: vec!["v1".to_string()],
            capabilities: CapabilityProfile::default(),
            max_context: None,
            available: true,
        };
        let agent2 = DiscoveredAgent {
            name: "custom-tool".to_string(),
            source: DiscoverySource::Manual,
            command: Some("my-tool-v2".to_string()),
            endpoint: None,
            models: vec!["v2".to_string()],
            capabilities: CapabilityProfile::default(),
            max_context: None,
            available: true,
        };
        engine.register_custom_agent(agent1);
        engine.register_custom_agent(agent2);
        let agents = engine.get_agents();
        let matching: Vec<_> = agents.iter().filter(|a| a.name == "custom-tool").collect();
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].command.as_deref(), Some("my-tool-v2"));
    }

    #[test]
    fn discovery_source_serialization() {
        let cli = serde_json::to_string(&DiscoverySource::Cli).unwrap();
        let api = serde_json::to_string(&DiscoverySource::Api).unwrap();
        let manual = serde_json::to_string(&DiscoverySource::Manual).unwrap();
        assert_eq!(cli, "\"cli\"");
        assert_eq!(api, "\"api\"");
        assert_eq!(manual, "\"manual\"");
    }
}
