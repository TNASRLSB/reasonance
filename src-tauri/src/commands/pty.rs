use crate::pty_manager::PtyManager;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn spawn_process(
    command: String,
    args: Vec<String>,
    cwd: String,
    app: AppHandle,
    pty_manager: State<'_, PtyManager>,
) -> Result<String, String> {
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
