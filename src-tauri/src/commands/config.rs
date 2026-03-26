use crate::config;
use crate::error::ReasonanceError;
use log::{info, error, debug};

#[tauri::command]
pub fn read_config() -> Result<String, ReasonanceError> {
    info!("cmd::read_config called");
    let path = config::config_path();
    if !path.exists() {
        debug!("cmd::read_config config file not found at {:?}", path);
        return Ok(String::new());
    }
    std::fs::read_to_string(&path).map_err(|e| {
        error!("cmd::read_config failed to read {:?}: {}", path, e);
        ReasonanceError::io(format!("read config {:?}", path), e)
    })
}

#[tauri::command]
pub fn write_config(content: String) -> Result<(), ReasonanceError> {
    info!("cmd::write_config called");
    // Validate TOML parses correctly
    let parsed: config::AppConfig = toml::from_str(&content)
        .map_err(|e| {
            error!("cmd::write_config invalid TOML format: {}", e);
            ReasonanceError::Serialization {
                context: "config TOML".to_string(),
                message: format!("Invalid config format: {}", e),
            }
        })?;

    // Validate command fields in LLM entries
    if let Some(llms) = &parsed.llm {
        const KNOWN_LLM_BINARIES: &[&str] = &[
            "claude", "aider", "codex", "copilot", "continue",
            "ollama", "llm", "sgpt", "tgpt", "mods",
            "fabric", "cursor", "windsurf", "cline",
            "gemini", "kimi", "qwen", "interpreter",
            "github-copilot-cli",
        ];

        for llm in llms {
            if let Some(cmd) = &llm.command {
                let binary = std::path::Path::new(cmd.as_str())
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(cmd.as_str());

                if !KNOWN_LLM_BINARIES.contains(&binary) {
                    error!("cmd::write_config unrecognized LLM command: {}", cmd);
                    return Err(ReasonanceError::validation(
                        "command",
                        format!(
                            "Unrecognized LLM command '{}'. Allowed: {}",
                            cmd,
                            KNOWN_LLM_BINARIES.join(", ")
                        ),
                    ));
                }
            }
        }
    }

    let path = config::config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ReasonanceError::io("create config dir", e))?;
    }
    std::fs::write(&path, content)
        .map_err(|e| ReasonanceError::io("write config", e))
}
