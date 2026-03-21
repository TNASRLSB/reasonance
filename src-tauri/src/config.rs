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
    config_dir.join("reasonance").join("llms.toml")
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
        assert_eq!(llm.yolo_flag.as_deref(), Some("--dangerously-skip-permissions"));
        assert_eq!(llm.image_mode.as_deref(), Some("auto"));
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
}
