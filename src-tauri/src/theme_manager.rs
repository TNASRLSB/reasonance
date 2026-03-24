use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct ThemePreferences {
    #[serde(rename = "activeTheme")]
    pub active_theme: String,
    #[serde(rename = "activeModifiers")]
    pub active_modifiers: Vec<String>,
}

impl Default for ThemePreferences {
    fn default() -> Self {
        Self {
            active_theme: "reasonance-dark".to_string(),
            active_modifiers: vec![],
        }
    }
}

fn themes_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("reasonance")
        .join("themes")
}

fn preferences_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("reasonance")
        .join("theme-preferences.json")
}

#[tauri::command]
pub fn list_user_themes() -> Result<Vec<String>, String> {
    let dir = themes_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;
    let mut names = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    Ok(names)
}

#[tauri::command]
pub fn load_user_theme(name: String) -> Result<String, String> {
    let path = themes_dir().join(format!("{}.json", name));
    fs::read_to_string(&path).map_err(|e| format!("Failed to read theme {}: {}", name, e))
}

#[tauri::command]
pub fn save_user_theme(name: String, content: String) -> Result<(), String> {
    let dir = themes_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", name));
    fs::write(&path, content).map_err(|e| format!("Failed to write theme {}: {}", name, e))
}

#[tauri::command]
pub fn delete_user_theme(name: String) -> Result<(), String> {
    let path = themes_dir().join(format!("{}.json", name));
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())
    } else {
        Err(format!("Theme not found: {}", name))
    }
}

#[tauri::command]
pub fn load_theme_preferences() -> Result<ThemePreferences, String> {
    let path = preferences_path();
    if !path.exists() {
        return Ok(ThemePreferences::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| format!("Invalid preferences: {}", e))
}

#[tauri::command]
pub fn save_theme_preferences(prefs: ThemePreferences) -> Result<(), String> {
    let path = preferences_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&prefs).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}
