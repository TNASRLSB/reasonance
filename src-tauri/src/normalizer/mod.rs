pub mod content_parser;
pub mod pipeline;
pub mod rules_engine;
pub mod state_machines;

#[cfg(test)]
use crate::agent_event::AgentEvent;
use crate::error::ReasonanceError;
use pipeline::NormalizerPipeline;
use rules_engine::Rule;
use serde::Deserialize;
use state_machines::{claude::ClaudeStateMachine, generic::GenericStateMachine, StateMachine};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields populated by serde deserialization from TOML configs
pub struct TomlConfig {
    pub cli: CliConfig,
    #[serde(default)]
    pub capabilities: HashMap<String, toml::Value>,
    #[serde(default)]
    pub rules: Vec<TomlRule>,
    #[serde(default)]
    pub context: Option<ContextConfig>,
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub session: Option<SessionConfig>,
    #[serde(default)]
    pub commands: Option<CommandsConfig>,
    #[serde(default)]
    pub tests: Vec<TomlTest>,
    #[serde(default)]
    pub direct_api: Option<DirectApiConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CliConfig {
    pub name: String,
    pub binary: String,
    #[serde(default)]
    pub programmatic_args: Vec<String>,
    #[serde(default)]
    pub resume_args: Vec<String>,
    #[serde(default)]
    pub version_command: Vec<String>,
    #[serde(default)]
    pub update_command: Vec<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Extra args appended to grant the CLI full access to the project folder.
    /// Supports `{project_root}` placeholder.
    #[serde(default)]
    pub permission_args: Vec<String>,
    /// The CLI flag for passing allowed tool names (e.g. "--allowedTools").
    /// Absent for providers that don't support selective tool approval.
    #[serde(default)]
    pub allowed_tools_arg: Option<String>,
    /// Tool names allowed in read-only workspace mode (e.g. ["Read", "Grep", "Glob"]).
    #[serde(default)]
    pub read_only_tools: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TomlRule {
    pub name: String,
    pub when: String,
    pub emit: String,
    #[serde(default)]
    pub mappings: HashMap<String, String>,
    /// Optional JSON path to an array of content blocks (e.g. "message.content").
    /// When set, the pipeline iterates over each element and emits one event per block,
    /// mapping each block's `type` field to the appropriate AgentEventType.
    #[serde(default)]
    pub content_blocks: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields populated by serde from TOML
pub struct ContextConfig {
    pub mode: Option<String>,
    pub flag: Option<String>,
    pub file_format: Option<String>,
    pub selection_format: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetryConfig {
    pub max_retries: Option<u32>,
    pub backoff: Option<toml::Value>,
    pub retryable_codes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields populated by serde from TOML
pub struct SessionConfig {
    pub session_id_path: Option<String>,
    pub model_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields populated by serde from TOML
pub struct CommandsConfig {
    pub cancel: Option<toml::Value>,
    pub pause: Option<toml::Value>,
    pub interrupt: Option<toml::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields populated by serde from TOML
pub struct TomlTest {
    pub name: String,
    pub prompt: String,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub expected: Vec<toml::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields populated by serde from TOML
pub struct DirectApiConfig {
    pub endpoint: Option<String>,
    pub auth_header: Option<String>,
    pub auth_env: Option<String>,
    pub stream: Option<bool>,
    pub request_template: Option<toml::Value>,
}

impl TomlConfig {
    pub fn parse(toml_str: &str) -> Result<Self, ReasonanceError> {
        toml::from_str(toml_str)
            .map_err(|e| ReasonanceError::config(format!("TOML parse error: {}", e)))
    }

    pub fn to_rules(&self) -> Vec<Rule> {
        self.rules
            .iter()
            .map(|r| Rule {
                name: r.name.clone(),
                when: r.when.clone(),
                emit: r.emit.clone(),
                mappings: r.mappings.clone(),
                content_blocks: r.content_blocks.clone(),
            })
            .collect()
    }

    /// Returns the JSON path for extracting CLI session IDs from stream output.
    pub fn session_id_path(&self) -> Option<&str> {
        self.session
            .as_ref()
            .and_then(|s| s.session_id_path.as_deref())
    }
}

#[derive(Default)]
pub struct NormalizerRegistry {
    pipelines: HashMap<String, NormalizerPipeline>,
    configs: HashMap<String, TomlConfig>,
    toml_sources: HashMap<String, String>,
}

impl NormalizerRegistry {
    pub fn load_from_dir(dir: &Path) -> Result<Self, ReasonanceError> {
        let mut pipelines = HashMap::new();
        let mut configs = HashMap::new();
        let mut toml_sources = HashMap::new();

        if !dir.exists() {
            return Ok(Self {
                pipelines,
                configs,
                toml_sources,
            });
        }

        for entry in std::fs::read_dir(dir).map_err(|e| {
            ReasonanceError::io(format!("read normalizers dir {}", dir.display()), e)
        })? {
            let entry = entry.map_err(|e| ReasonanceError::io("read normalizers dir entry", e))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                let content = std::fs::read_to_string(&path).map_err(|e| {
                    ReasonanceError::io(format!("read normalizer file {}", path.display()), e)
                })?;
                let config = TomlConfig::parse(&content)?;
                let name = config.cli.name.clone();

                let state_machine: Box<dyn StateMachine> = match name.as_str() {
                    "claude" => Box::new(ClaudeStateMachine::new()),
                    "gemini" => Box::new(state_machines::gemini::GeminiStateMachine::new()),
                    "kimi" => Box::new(state_machines::kimi::KimiStateMachine::new()),
                    "qwen" => Box::new(state_machines::qwen::QwenStateMachine::new()),
                    "codex" => Box::new(state_machines::codex::CodexStateMachine::new()),
                    _ => Box::new(GenericStateMachine::new()),
                };

                let pipeline =
                    NormalizerPipeline::new(config.to_rules(), state_machine, name.clone());

                pipelines.insert(name.clone(), pipeline);
                toml_sources.insert(name.clone(), content);
                configs.insert(name, config);
            }
        }

        Ok(Self {
            pipelines,
            configs,
            toml_sources,
        })
    }

    pub fn has_provider(&self, name: &str) -> bool {
        self.pipelines.contains_key(name)
    }

    #[cfg(test)]
    pub fn process(&mut self, provider: &str, raw_json: &str) -> Vec<AgentEvent> {
        match self.pipelines.get_mut(provider) {
            Some(pipeline) => pipeline.process(raw_json),
            None => vec![],
        }
    }

    pub fn get_config(&self, provider: &str) -> Option<&TomlConfig> {
        self.configs.get(provider)
    }

    pub fn providers(&self) -> Vec<String> {
        self.pipelines.keys().cloned().collect()
    }

    pub fn get_toml_source(&self, provider: &str) -> Option<String> {
        self.toml_sources.get(provider).cloned()
    }

    pub fn reload_provider(
        &mut self,
        provider: &str,
        toml_str: &str,
    ) -> Result<(), ReasonanceError> {
        let config = TomlConfig::parse(toml_str)?;

        let state_machine: Box<dyn StateMachine> = match provider {
            "claude" => Box::new(ClaudeStateMachine::new()),
            "gemini" => Box::new(state_machines::gemini::GeminiStateMachine::new()),
            "kimi" => Box::new(state_machines::kimi::KimiStateMachine::new()),
            "qwen" => Box::new(state_machines::qwen::QwenStateMachine::new()),
            "codex" => Box::new(state_machines::codex::CodexStateMachine::new()),
            _ => Box::new(GenericStateMachine::new()),
        };

        let pipeline =
            NormalizerPipeline::new(config.to_rules(), state_machine, provider.to_string());

        self.pipelines.insert(provider.to_string(), pipeline);
        self.configs.insert(provider.to_string(), config);
        self.toml_sources
            .insert(provider.to_string(), toml_str.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod fixture_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn sample_toml() -> &'static str {
        r#"
[cli]
name = "testprovider"
binary = "test"
programmatic_args = ["-p", "{prompt}"]
version_command = ["test", "--version"]
update_command = ["test", "update"]

[capabilities]
streaming = true
session_resume = false

[[rules]]
name = "text"
when = 'type == "text_delta"'
emit = "text"

[rules.mappings]
content = "text"

[[rules]]
name = "done"
when = 'type == "done"'
emit = "done"
"#
    }

    #[test]
    fn test_parse_toml_config() {
        let config = TomlConfig::parse(sample_toml()).unwrap();
        assert_eq!(config.cli.name, "testprovider");
        assert_eq!(config.cli.binary, "test");
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].name, "text");
        assert_eq!(config.rules[0].emit, "text");
    }

    #[test]
    fn test_parse_toml_rules_have_mappings() {
        let config = TomlConfig::parse(sample_toml()).unwrap();
        assert_eq!(
            config.rules[0].mappings.get("content"),
            Some(&"text".to_string())
        );
    }

    #[test]
    fn test_registry_load_from_dir() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("test.toml");
        fs::write(&toml_path, sample_toml()).unwrap();

        let registry = NormalizerRegistry::load_from_dir(dir.path()).unwrap();
        assert!(registry.has_provider("testprovider"));
    }

    #[test]
    fn test_registry_process_line() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("test.toml");
        fs::write(&toml_path, sample_toml()).unwrap();

        let mut registry = NormalizerRegistry::load_from_dir(dir.path()).unwrap();
        let events = registry.process("testprovider", r#"{"type":"text_delta","text":"Hello"}"#);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_registry_unknown_provider() {
        let dir = TempDir::new().unwrap();
        let mut registry = NormalizerRegistry::load_from_dir(dir.path()).unwrap();
        let events = registry.process("unknown", r#"{"type":"text"}"#);
        assert!(events.is_empty());
    }

    #[test]
    fn test_get_toml_source() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("test.toml");
        fs::write(&toml_path, sample_toml()).unwrap();

        let registry = NormalizerRegistry::load_from_dir(dir.path()).unwrap();
        let source = registry.get_toml_source("testprovider");
        assert!(source.is_some());
        assert!(source.unwrap().contains("testprovider"));
    }

    #[test]
    fn test_get_toml_source_unknown() {
        let dir = TempDir::new().unwrap();
        let registry = NormalizerRegistry::load_from_dir(dir.path()).unwrap();
        assert!(registry.get_toml_source("unknown").is_none());
    }

    #[test]
    fn test_reload_provider() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("test.toml");
        fs::write(&toml_path, sample_toml()).unwrap();

        let mut registry = NormalizerRegistry::load_from_dir(dir.path()).unwrap();
        assert!(registry.has_provider("testprovider"));

        let modified_toml = sample_toml().replace(
            r#"when = 'type == "text_delta"'"#,
            r#"when = 'type == "content"'"#,
        );
        let result = registry.reload_provider("testprovider", &modified_toml);
        assert!(result.is_ok());
        assert!(registry.has_provider("testprovider"));
    }

    #[test]
    fn test_reload_provider_invalid_toml() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("test.toml");
        fs::write(&toml_path, sample_toml()).unwrap();

        let mut registry = NormalizerRegistry::load_from_dir(dir.path()).unwrap();
        let result = registry.reload_provider("testprovider", "invalid { toml");
        assert!(result.is_err());
        assert!(registry.has_provider("testprovider"));
    }

    #[test]
    fn test_session_id_path() {
        let toml_str = r#"
[cli]
name = "testprov"
binary = "test"

[session]
session_id_path = "message.id"
model_path = "message.model"
"#;
        let config = TomlConfig::parse(toml_str).unwrap();
        assert_eq!(config.session_id_path(), Some("message.id"));
    }

    #[test]
    fn test_session_id_path_missing() {
        let config = TomlConfig::parse(sample_toml()).unwrap();
        assert_eq!(config.session_id_path(), None);
    }
}
