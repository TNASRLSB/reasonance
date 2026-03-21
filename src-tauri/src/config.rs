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
