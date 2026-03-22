use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliVersionInfo {
    pub provider: String,
    pub current_version: Option<String>,
    pub last_checked: Option<u64>,
    pub auto_update: bool,
    pub version_command: Vec<String>,
    pub update_command: Vec<String>,
}

pub struct CliUpdater {
    providers: Mutex<HashMap<String, CliVersionInfo>>,
}

impl CliUpdater {
    pub fn new() -> Self {
        Self {
            providers: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self, name: &str, info: CliVersionInfo) {
        self.providers.lock().unwrap().insert(name.to_string(), info);
    }

    pub fn providers(&self) -> Vec<String> {
        self.providers.lock().unwrap().keys().cloned().collect()
    }

    pub fn get_info(&self, provider: &str) -> Option<CliVersionInfo> {
        self.providers.lock().unwrap().get(provider).cloned()
    }

    pub fn set_version(&self, provider: &str, version: &str) {
        let mut providers = self.providers.lock().unwrap();
        if let Some(info) = providers.get_mut(provider) {
            info.current_version = Some(version.to_string());
            info.last_checked = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
        }
    }

    pub fn version_changed(&self, provider: &str, new_version: &str) -> bool {
        let providers = self.providers.lock().unwrap();
        match providers.get(provider) {
            Some(info) => match &info.current_version {
                Some(current) => current != new_version,
                None => true,
            },
            None => false,
        }
    }

    pub fn auto_update_providers(&self) -> Vec<String> {
        self.providers
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, info)| info.auto_update)
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn register_from_configs(
        &self,
        configs: &HashMap<String, crate::normalizer::TomlConfig>,
    ) {
        for (name, config) in configs {
            self.register(
                name,
                CliVersionInfo {
                    provider: name.clone(),
                    current_version: None,
                    last_checked: None,
                    auto_update: true,
                    version_command: config.cli.version_command.clone(),
                    update_command: config.cli.update_command.clone(),
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_updater_creation() {
        let updater = CliUpdater::new();
        assert!(updater.providers().is_empty());
    }

    #[test]
    fn test_register_provider() {
        let updater = CliUpdater::new();
        updater.register("claude", CliVersionInfo {
            provider: "claude".to_string(),
            current_version: None,
            last_checked: None,
            auto_update: true,
            version_command: vec!["claude".into(), "--version".into()],
            update_command: vec!["claude".into(), "update".into()],
        });
        assert_eq!(updater.providers().len(), 1);
        assert!(updater.get_info("claude").is_some());
    }

    #[test]
    fn test_update_version() {
        let updater = CliUpdater::new();
        updater.register("claude", CliVersionInfo {
            provider: "claude".to_string(),
            current_version: None,
            last_checked: None,
            auto_update: true,
            version_command: vec!["claude".into(), "--version".into()],
            update_command: vec!["claude".into(), "update".into()],
        });
        updater.set_version("claude", "1.0.5");
        let info = updater.get_info("claude").unwrap();
        assert_eq!(info.current_version, Some("1.0.5".to_string()));
        assert!(info.last_checked.is_some());
    }

    #[test]
    fn test_version_changed() {
        let updater = CliUpdater::new();
        updater.register("claude", CliVersionInfo {
            provider: "claude".to_string(),
            current_version: Some("1.0.4".to_string()),
            last_checked: None,
            auto_update: true,
            version_command: vec!["claude".into(), "--version".into()],
            update_command: vec!["claude".into(), "update".into()],
        });
        assert!(updater.version_changed("claude", "1.0.5"));
        assert!(!updater.version_changed("claude", "1.0.4"));
        assert!(!updater.version_changed("unknown", "1.0.0"));
    }

    #[test]
    fn test_auto_update_providers() {
        let updater = CliUpdater::new();
        updater.register("claude", CliVersionInfo {
            provider: "claude".to_string(),
            current_version: None,
            last_checked: None,
            auto_update: true,
            version_command: vec![],
            update_command: vec![],
        });
        updater.register("gemini", CliVersionInfo {
            provider: "gemini".to_string(),
            current_version: None,
            last_checked: None,
            auto_update: false,
            version_command: vec![],
            update_command: vec![],
        });
        let auto = updater.auto_update_providers();
        assert_eq!(auto.len(), 1);
        assert_eq!(auto[0], "claude");
    }
}
