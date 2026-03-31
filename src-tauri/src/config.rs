use log::debug;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct LlmConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub llm_type: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub yolo_flag: Option<String>,
    #[serde(default)]
    pub image_mode: Option<String>,
    #[serde(default)]
    pub permission_level: Option<String>,
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub context_menu_llm: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub settings: Option<Settings>,
    #[serde(default)]
    pub llm: Option<Vec<LlmConfig>>,
}

pub fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let path = config_dir.join("reasonance").join("llms.toml");
    debug!("Config path resolved to {}", path.display());
    path
}

// SEC-07: The config file (llms.toml) is stored as plain-text TOML on disk.
// It contains LLM configuration such as command names, model names, and env var
// names (e.g. "ANTHROPIC_API_KEY") — but NOT the actual secret values.
// Actual secrets are read at runtime from the process environment via get_env_var.
// This is acceptable: the config file is user-owned and stored in the user's
// XDG config directory (~/.config/reasonance/). No encryption is applied.

/// Check whether `command` is a configured LLM command or LLM name in llms.toml.
///
/// Reads and parses the config file each call so that runtime config changes are
/// picked up without restart. Returns `false` on any I/O or parse error.
pub fn is_allowed_llm_command(command: &str) -> bool {
    let binary = std::path::Path::new(command)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(command);

    let path = config_path();
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let app_config: AppConfig = match toml::from_str(&contents) {
        Ok(c) => c,
        Err(_) => return false,
    };

    if let Some(llms) = app_config.llm {
        for llm in &llms {
            // Match against the explicit command field
            if let Some(cmd) = &llm.command {
                let llm_binary = std::path::Path::new(cmd)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(cmd.as_str());
                if cmd == command || llm_binary == binary {
                    return true;
                }
            }
            // Also match against the LLM name itself (used as command by convention)
            let name_binary = std::path::Path::new(&llm.name)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&llm.name);
            if llm.name == command || name_binary == binary {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_is_nonempty() {
        let path = config_path();
        assert!(!path.as_os_str().is_empty());
        assert!(path.to_string_lossy().contains("llms.toml"));
    }

    #[test]
    fn parse_minimal_toml() {
        let toml_str = "";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(config.settings.is_none());
        assert!(config.llm.is_none());
    }

    #[test]
    fn parse_empty_config() {
        let toml_str = "[settings]\n";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        let settings = config.settings.unwrap();
        assert!(settings.default.is_none());
        assert!(settings.context_menu_llm.is_none());
    }

    #[test]
    fn parse_full_config_with_llm_entry() {
        let toml_str = r#"
[settings]
default = "claude"
context_menu_llm = "claude"

[[llm]]
name = "claude"
type = "cli"
command = "claude"
args = ["--dangerously-skip-permissions"]
yolo_flag = "--dangerously-skip-permissions"
image_mode = "auto"
permission_level = "ask"
allowed_tools = ["Read", "Edit"]
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        let settings = config.settings.unwrap();
        assert_eq!(settings.default.as_deref(), Some("claude"));
        assert_eq!(settings.context_menu_llm.as_deref(), Some("claude"));

        let llms = config.llm.unwrap();
        assert_eq!(llms.len(), 1);
        let llm = &llms[0];
        assert_eq!(llm.name, "claude");
        assert_eq!(llm.llm_type, "cli");
        assert_eq!(llm.command.as_deref(), Some("claude"));
        assert_eq!(
            llm.yolo_flag.as_deref(),
            Some("--dangerously-skip-permissions")
        );
        assert_eq!(llm.image_mode.as_deref(), Some("auto"));
        assert_eq!(llm.permission_level.as_deref(), Some("ask"));
        assert_eq!(
            llm.allowed_tools,
            Some(vec!["Read".to_string(), "Edit".to_string()])
        );
        assert!(llm.provider.is_none());
        assert!(llm.api_key_env.is_none());
        assert!(llm.model.is_none());
        assert!(llm.endpoint.is_none());
    }

    #[test]
    fn parse_api_llm_entry() {
        let toml_str = r#"
[[llm]]
name = "openai"
type = "api"
provider = "openai"
api_key_env = "OPENAI_API_KEY"
model = "gpt-4o"
endpoint = "https://api.openai.com/v1"
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        let llms = config.llm.unwrap();
        let llm = &llms[0];
        assert_eq!(llm.provider.as_deref(), Some("openai"));
        assert_eq!(llm.api_key_env.as_deref(), Some("OPENAI_API_KEY"));
        assert_eq!(llm.model.as_deref(), Some("gpt-4o"));
        assert!(llm.command.is_none());
        assert!(llm.yolo_flag.is_none());
    }

    #[test]
    fn is_allowed_llm_command_rejects_unknown() {
        // Commands that would never appear in any real llms.toml
        assert!(!is_allowed_llm_command("rm"));
        assert!(!is_allowed_llm_command("curl"));
        assert!(!is_allowed_llm_command("/usr/bin/python3"));
    }
}
