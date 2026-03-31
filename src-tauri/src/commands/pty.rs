use crate::config;
use crate::error::ReasonanceError;
use crate::pty_manager::PtyManager;
use log::{debug, error, info};
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

    // Delegate LLM command validation to the config module (GEC-6)
    config::is_allowed_llm_command(command)
}

#[tauri::command]
pub fn spawn_process(
    command: String,
    args: Vec<String>,
    cwd: String,
    app: AppHandle,
    pty_manager: State<'_, PtyManager>,
) -> Result<String, ReasonanceError> {
    info!("cmd::spawn_process(command={}, cwd={})", command, cwd);
    if !is_allowed_command(&command) {
        error!(
            "cmd::spawn_process rejected disallowed command: {}",
            command
        );
        return Err(ReasonanceError::Security {
            message: format!(
                "Command '{}' is not allowed. Only configured LLM commands and known shells are permitted.",
                command
            ),
            code: crate::error::SecurityErrorCode::DisallowedCommand,
        });
    }
    let result = pty_manager.spawn(&command, &args, &cwd, app);
    match &result {
        Ok(id) => debug!("cmd::spawn_process spawned pty_id={}", id),
        Err(e) => error!("cmd::spawn_process failed: {}", e),
    }
    result
}

#[tauri::command]
pub fn write_pty(
    id: String,
    data: String,
    pty_manager: State<'_, PtyManager>,
) -> Result<(), ReasonanceError> {
    debug!("cmd::write_pty(id={}, len={})", id, data.len());
    pty_manager.write(&id, &data)
}

#[tauri::command]
pub fn resize_pty(
    id: String,
    cols: u16,
    rows: u16,
    pty_manager: State<'_, PtyManager>,
) -> Result<(), ReasonanceError> {
    debug!("cmd::resize_pty(id={}, cols={}, rows={})", id, cols, rows);
    pty_manager.resize(&id, cols, rows)
}

#[tauri::command]
pub fn kill_process(id: String, pty_manager: State<'_, PtyManager>) -> Result<(), ReasonanceError> {
    info!("cmd::kill_process(id={})", id);
    pty_manager.kill(&id).map_err(|e| {
        error!("cmd::kill_process failed for id={}: {}", id, e);
        e
    })
}

#[tauri::command]
pub fn kill_project_ptys(
    project_id: String,
    pty_manager: State<'_, PtyManager>,
) -> Result<Vec<String>, ReasonanceError> {
    info!("cmd::kill_project_ptys(project_id={})", project_id);
    Ok(pty_manager.kill_project_ptys(&project_id))
}

/// Reconnect a dead PTY by killing the old one (if still around) and spawning a
/// fresh process with the same shell and working directory.
///
/// Returns the new PTY ID. The frontend is responsible for calling this command
/// with exponential backoff delays between attempts (see `ReconnectConfig`).
#[tauri::command]
pub fn reconnect_pty(
    pty_id: String,
    command: String,
    args: Vec<String>,
    cwd: String,
    app: AppHandle,
    pty_manager: State<'_, PtyManager>,
) -> Result<String, ReasonanceError> {
    info!(
        "cmd::reconnect_pty(pty_id={}, command={}, cwd={})",
        pty_id, command, cwd
    );

    if !is_allowed_command(&command) {
        error!(
            "cmd::reconnect_pty rejected disallowed command: {}",
            command
        );
        return Err(ReasonanceError::Security {
            message: format!(
                "Command '{}' is not allowed. Only configured LLM commands and known shells are permitted.",
                command
            ),
            code: crate::error::SecurityErrorCode::DisallowedCommand,
        });
    }

    // Best-effort cleanup of the old PTY — ignore not-found errors.
    if let Err(e) = pty_manager.kill_process(&pty_id) {
        debug!("cmd::reconnect_pty: kill old pty_id={} ({})", pty_id, e);
    }

    let new_id = pty_manager.spawn(&command, &args, &cwd, app)?;
    info!("cmd::reconnect_pty: new pty_id={}", new_id);
    Ok(new_id)
}

/// Remove PTY entries for processes that have already exited.
///
/// Called periodically by the frontend (every 60 s) to keep the PTY table
/// clean without requiring the terminal component to explicitly kill each PTY
/// on unmount. Returns the list of swept PTY IDs.
#[tauri::command]
pub fn sweep_ptys(pty_manager: State<'_, PtyManager>) -> Vec<String> {
    debug!("cmd::sweep_ptys");
    pty_manager.sweep_dead_ptys()
}

/// Kill all active PTY processes. Intended for app shutdown only.
///
/// Returns the number of PTYs that were killed.
#[tauri::command]
pub fn kill_all_ptys(pty_manager: State<'_, PtyManager>) -> usize {
    info!("cmd::kill_all_ptys");
    pty_manager.kill_all()
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
