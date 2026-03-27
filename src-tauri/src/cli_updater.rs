use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

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
        info!("CLI updater: registered provider '{}'", name);
        self.providers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(name.to_string(), info);
    }

    pub fn providers(&self) -> Vec<String> {
        self.providers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }

    pub fn get_info(&self, provider: &str) -> Option<CliVersionInfo> {
        self.providers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(provider)
            .cloned()
    }

    pub fn set_version(&self, provider: &str, version: &str) {
        debug!(
            "CLI updater: setting version for provider='{}' to '{}'",
            provider, version
        );
        let mut providers = self.providers.lock().unwrap_or_else(|e| e.into_inner());
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

    #[allow(dead_code)] // Roadmap: used by background CLI update task
    pub fn version_changed(&self, provider: &str, new_version: &str) -> bool {
        let providers = self.providers.lock().unwrap_or_else(|e| e.into_inner());
        let changed = match providers.get(provider) {
            Some(info) => match &info.current_version {
                Some(current) => current != new_version,
                None => true,
            },
            None => false,
        };
        if changed {
            info!(
                "CLI version change detected for '{}': new version '{}'",
                provider, new_version
            );
        }
        changed
    }

    #[allow(dead_code)] // Roadmap: used by background CLI update task
    pub fn auto_update_providers(&self) -> Vec<String> {
        self.providers
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, info)| info.auto_update)
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn register_from_configs(&self, configs: &HashMap<String, crate::normalizer::TomlConfig>) {
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

    /// Collect a snapshot of providers that have auto_update enabled and a non-empty update_command.
    fn auto_update_snapshot(&self) -> Vec<CliVersionInfo> {
        self.providers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|info| info.auto_update && !info.update_command.is_empty())
            .cloned()
            .collect()
    }
}

/// Payload emitted on the `cli://update-result` event channel.
#[derive(Clone, Serialize)]
struct CliUpdateEvent {
    provider: String,
    success: bool,
    old_version: Option<String>,
    new_version: Option<String>,
    error: Option<String>,
}

/// Run version-check + update for every auto-update provider.
/// Designed to be spawned as a fire-and-forget background task at app startup.
pub async fn run_background_updates(app: AppHandle, updater: Arc<CliUpdater>) {
    // Let the app settle before hitting the network / spawning subprocesses.
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let providers = updater.auto_update_snapshot();
    if providers.is_empty() {
        info!("CLI updater: no auto-update providers registered — skipping");
        return;
    }

    info!(
        "CLI updater: starting background updates for {} provider(s)",
        providers.len()
    );

    for info in &providers {
        let provider = &info.provider;

        // 1. Detect current version (best-effort).
        let old_version = if !info.version_command.is_empty() {
            match tokio::process::Command::new(&info.version_command[0])
                .args(&info.version_command[1..])
                .output()
                .await
            {
                Ok(out) if out.status.success() => {
                    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !v.is_empty() {
                        updater.set_version(provider, &v);
                        debug!("CLI updater: {provider} current version = {v}");
                        Some(v)
                    } else {
                        None
                    }
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    warn!(
                        "CLI updater: version command for '{provider}' exited with {}: {stderr}",
                        out.status
                    );
                    None
                }
                Err(e) => {
                    // Binary not found → provider not installed, skip update entirely.
                    debug!("CLI updater: '{provider}' not installed ({e}), skipping");
                    continue;
                }
            }
        } else {
            None
        };

        // 2. Run the update command.
        info!("CLI updater: updating '{provider}' …");
        let result = tokio::process::Command::new(&info.update_command[0])
            .args(&info.update_command[1..])
            .output()
            .await;

        let (success, err_msg) = match &result {
            Ok(out) if out.status.success() => (true, None),
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                warn!(
                    "CLI updater: update for '{provider}' failed ({}): {stderr}",
                    out.status
                );
                (false, Some(stderr))
            }
            Err(e) => {
                error!("CLI updater: failed to spawn update for '{provider}': {e}");
                (false, Some(e.to_string()))
            }
        };

        // 3. Re-check version after update to detect changes.
        let new_version = if success && !info.version_command.is_empty() {
            match tokio::process::Command::new(&info.version_command[0])
                .args(&info.version_command[1..])
                .output()
                .await
            {
                Ok(out) if out.status.success() => {
                    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !v.is_empty() {
                        updater.set_version(provider, &v);
                        Some(v)
                    } else {
                        old_version.clone()
                    }
                }
                _ => old_version.clone(),
            }
        } else {
            old_version.clone()
        };

        if success {
            let changed =
                matches!((&old_version, &new_version), (Some(old), Some(new)) if old != new);
            if changed {
                info!(
                    "CLI updater: '{provider}' updated {} → {}",
                    old_version.as_deref().unwrap_or("?"),
                    new_version.as_deref().unwrap_or("?")
                );
            } else {
                info!("CLI updater: '{provider}' already up to date");
            }
        }

        // 4. Notify frontend.
        let _ = app.emit(
            "cli://update-result",
            CliUpdateEvent {
                provider: provider.clone(),
                success,
                old_version,
                new_version,
                error: err_msg,
            },
        );
    }

    info!("CLI updater: background update cycle complete");
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
        updater.register(
            "claude",
            CliVersionInfo {
                provider: "claude".to_string(),
                current_version: None,
                last_checked: None,
                auto_update: true,
                version_command: vec!["claude".into(), "--version".into()],
                update_command: vec!["claude".into(), "update".into()],
            },
        );
        assert_eq!(updater.providers().len(), 1);
        assert!(updater.get_info("claude").is_some());
    }

    #[test]
    fn test_update_version() {
        let updater = CliUpdater::new();
        updater.register(
            "claude",
            CliVersionInfo {
                provider: "claude".to_string(),
                current_version: None,
                last_checked: None,
                auto_update: true,
                version_command: vec!["claude".into(), "--version".into()],
                update_command: vec!["claude".into(), "update".into()],
            },
        );
        updater.set_version("claude", "1.0.5");
        let info = updater.get_info("claude").unwrap();
        assert_eq!(info.current_version, Some("1.0.5".to_string()));
        assert!(info.last_checked.is_some());
    }

    #[test]
    fn test_version_changed() {
        let updater = CliUpdater::new();
        updater.register(
            "claude",
            CliVersionInfo {
                provider: "claude".to_string(),
                current_version: Some("1.0.4".to_string()),
                last_checked: None,
                auto_update: true,
                version_command: vec!["claude".into(), "--version".into()],
                update_command: vec!["claude".into(), "update".into()],
            },
        );
        assert!(updater.version_changed("claude", "1.0.5"));
        assert!(!updater.version_changed("claude", "1.0.4"));
        assert!(!updater.version_changed("unknown", "1.0.0"));
    }

    #[test]
    fn test_auto_update_providers() {
        let updater = CliUpdater::new();
        updater.register(
            "claude",
            CliVersionInfo {
                provider: "claude".to_string(),
                current_version: None,
                last_checked: None,
                auto_update: true,
                version_command: vec![],
                update_command: vec![],
            },
        );
        updater.register(
            "gemini",
            CliVersionInfo {
                provider: "gemini".to_string(),
                current_version: None,
                last_checked: None,
                auto_update: false,
                version_command: vec![],
                update_command: vec![],
            },
        );
        let auto = updater.auto_update_providers();
        assert_eq!(auto.len(), 1);
        assert_eq!(auto[0], "claude");
    }
}
