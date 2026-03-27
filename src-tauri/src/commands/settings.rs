use crate::error::ReasonanceError;
use crate::settings::LayeredSettings;
use log::info;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn get_setting(
    settings: State<'_, Mutex<LayeredSettings>>,
    key: String,
) -> Result<Option<serde_json::Value>, ReasonanceError> {
    info!("cmd::get_setting key={}", key);
    let s = settings.lock().unwrap_or_else(|e| e.into_inner());
    match s.get_value(&key) {
        Some(toml_val) => {
            let json =
                serde_json::to_value(toml_val).map_err(|e| ReasonanceError::Serialization {
                    context: format!("setting key={}", key),
                    message: e.to_string(),
                })?;
            Ok(Some(json))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub fn set_setting(
    settings: State<'_, Mutex<LayeredSettings>>,
    key: String,
    value: serde_json::Value,
    layer: Option<String>,
) -> Result<(), ReasonanceError> {
    let layer = layer.as_deref().unwrap_or("user");
    info!("cmd::set_setting key={} layer={}", key, layer);
    let toml_val: toml::Value =
        serde_json::from_value(value).map_err(|e| ReasonanceError::Serialization {
            context: format!("setting key={}", key),
            message: format!("Failed to convert JSON to TOML: {}", e),
        })?;
    let mut s = settings.lock().unwrap_or_else(|e| e.into_inner());
    s.set(&key, toml_val, layer)
        .map_err(ReasonanceError::internal)
}

#[tauri::command]
pub fn get_all_settings(
    settings: State<'_, Mutex<LayeredSettings>>,
) -> Result<serde_json::Value, ReasonanceError> {
    info!("cmd::get_all_settings");
    let s = settings.lock().unwrap_or_else(|e| e.into_inner());
    serde_json::to_value(s.resolved()).map_err(|e| ReasonanceError::Serialization {
        context: "all settings".to_string(),
        message: e.to_string(),
    })
}

#[tauri::command]
pub fn reload_settings(settings: State<'_, Mutex<LayeredSettings>>) -> Result<(), ReasonanceError> {
    info!("cmd::reload_settings");
    let mut s = settings.lock().unwrap_or_else(|e| e.into_inner());
    s.reload();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_settings() -> Mutex<LayeredSettings> {
        Mutex::new(LayeredSettings::new())
    }

    #[test]
    fn get_setting_returns_default() {
        let settings = make_settings();
        let s = settings.lock().unwrap();
        let val = s.get_value("editor.tab_size");
        assert!(val.is_some());
    }

    #[test]
    fn set_and_get_roundtrip() {
        let settings = make_settings();
        let mut s = settings.lock().unwrap();
        s.set("editor.tab_size", toml::Value::Integer(8), "user")
            .unwrap();
        let val = s.get::<i64>("editor.tab_size");
        assert_eq!(val, Some(8));
    }

    #[test]
    fn set_creates_nested_path() {
        let settings = make_settings();
        let mut s = settings.lock().unwrap();
        s.set(
            "custom.deeply.nested.value",
            toml::Value::Boolean(true),
            "user",
        )
        .unwrap();
        let val = s.get::<bool>("custom.deeply.nested.value");
        assert_eq!(val, Some(true));
    }

    #[test]
    fn set_invalid_layer_returns_error() {
        let settings = make_settings();
        let mut s = settings.lock().unwrap();
        let result = s.set("key", toml::Value::Integer(1), "invalid");
        assert!(result.is_err());
    }
}
