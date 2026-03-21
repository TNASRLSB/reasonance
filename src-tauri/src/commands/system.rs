#[tauri::command]
pub fn open_external(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_env_var(name: String) -> Result<Option<String>, String> {
    Ok(std::env::var(&name).ok())
}
