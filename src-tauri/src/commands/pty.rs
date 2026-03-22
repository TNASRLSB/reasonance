use crate::config;
use crate::pty_manager::PtyManager;
use tauri::{AppHandle, State};

/// Shells that are always allowed regardless of LLM config.
const ALLOWED_SHELLS: &[&str] = &["bash", "zsh", "sh", "fish", "powershell", "cmd"];

/// Validate that `command` is either a configured LLM command or a known shell.
fn is_allowed_command(command: &str) -> bool {
    // Extract the binary name from a path (e.g. "/usr/bin/bash" → "bash")
    let binary = std::path::Path::new(command)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(command);

    // Always allow known shells
    if ALLOWED_SHELLS.contains(&binary) || ALLOWED_SHELLS.contains(&command) {
        return true;
    }

    // Allow commands defined in the LLM config
    let config_path = config::config_path();
    if let Ok(contents) = std::fs::read_to_string(&config_path) {
        if let Ok(app_config) = toml::from_str::<config::AppConfig>(&contents) {
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
        }
    }

    false
}

#[tauri::command]
pub fn spawn_process(
    command: String,
    args: Vec<String>,
    cwd: String,
    app: AppHandle,
    pty_manager: State<'_, PtyManager>,
) -> Result<String, String> {
    if !is_allowed_command(&command) {
        return Err(format!(
            "Command '{}' is not allowed. Only configured LLM commands and known shells are permitted.",
            command
        ));
    }
    pty_manager.spawn(&command, &args, &cwd, app)
}

#[tauri::command]
pub fn write_pty(
    id: String,
    data: String,
    pty_manager: State<'_, PtyManager>,
) -> Result<(), String> {
    pty_manager.write(&id, &data)
}

#[tauri::command]
pub fn resize_pty(
    id: String,
    cols: u16,
    rows: u16,
    pty_manager: State<'_, PtyManager>,
) -> Result<(), String> {
    pty_manager.resize(&id, cols, rows)
}

#[tauri::command]
pub fn kill_process(
    id: String,
    pty_manager: State<'_, PtyManager>,
) -> Result<(), String> {
    pty_manager.kill(&id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_shells_are_allowed() {
        for shell in ALLOWED_SHELLS {
            assert!(
                is_allowed_command(shell),
                "Shell '{}' should be allowed",
                shell
            );
        }
    }

    #[test]
    fn arbitrary_command_is_rejected() {
        // A command unlikely to be in any real config
        assert!(!is_allowed_command("rm"));
        assert!(!is_allowed_command("curl"));
        assert!(!is_allowed_command("/usr/bin/python3"));
    }

    #[test]
    fn shell_via_full_path_is_allowed() {
        assert!(is_allowed_command("/bin/bash"));
        assert!(is_allowed_command("/usr/bin/zsh"));
    }
}
