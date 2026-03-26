use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use crate::transport::request::CliMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiatedCapabilities {
    pub provider: String,
    pub cli_version: String,
    pub cli_mode: CliMode,
    pub features: HashMap<String, FeatureSupport>,
    pub negotiated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "level")]
pub enum FeatureSupport {
    Full,
    Partial {
        limitations: Vec<String>,
        workaround: Option<Workaround>,
    },
    Unsupported {
        alternative: Option<Workaround>,
    },
}

impl FeatureSupport {
    #[allow(dead_code)] // Public API for capability queries
    pub fn is_supported(&self) -> bool {
        matches!(self, FeatureSupport::Full | FeatureSupport::Partial { .. })
    }

    #[allow(dead_code)] // Public API for capability queries
    pub fn needs_workaround(&self) -> bool {
        matches!(self, FeatureSupport::Partial { workaround: Some(_), .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workaround {
    pub description: String,
    pub method: WorkaroundMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkaroundMethod {
    InlineInPrompt,
    SimulateFromBatch,
    FallbackFlag(String),
    SkipSilently,
}

pub struct CapabilityNegotiator {
    results: Mutex<HashMap<String, NegotiatedCapabilities>>,
}

impl CapabilityNegotiator {
    pub fn new() -> Self {
        Self {
            results: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_capabilities(&self, provider: &str) -> Option<NegotiatedCapabilities> {
        self.results.lock().unwrap_or_else(|e| e.into_inner()).get(provider).cloned()
    }

    pub fn set_capabilities(&self, provider: &str, caps: NegotiatedCapabilities) {
        info!("Capabilities negotiated for provider='{}', mode={:?}, features={}", provider, caps.cli_mode, caps.features.len());
        self.results.lock().unwrap_or_else(|e| e.into_inner()).insert(provider.to_string(), caps);
    }

    pub fn all_capabilities(&self) -> HashMap<String, NegotiatedCapabilities> {
        self.results.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    #[allow(dead_code)] // Roadmap: used for capability cache invalidation
    pub fn is_cache_valid(&self, provider: &str, current_cli_version: &str) -> bool {
        let results = self.results.lock().unwrap_or_else(|e| e.into_inner());
        match results.get(provider) {
            Some(caps) => {
                if caps.cli_version != current_cli_version {
                    debug!("Capability cache invalid for '{}': version mismatch ({} != {})", provider, caps.cli_version, current_cli_version);
                    return false;
                }
                // Also check age — 30 days max
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let age_days = (now - caps.negotiated_at) / 86400;
                age_days < 30
            }
            None => false,
        }
    }

    #[allow(dead_code)] // Roadmap: persist capability cache to disk
    pub fn save_cache(&self, cache_dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
        let results = self.results.lock().unwrap_or_else(|e| e.into_inner());
        debug!("Saving capability cache to {}: {} providers", cache_dir.display(), results.len());
        for (provider, caps) in results.iter() {
            let path = cache_dir.join(format!("{}.json", provider));
            let json = serde_json::to_string_pretty(caps).map_err(|e| e.to_string())?;
            std::fs::write(&path, json).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn load_cache(&self, cache_dir: &Path) -> Result<(), String> {
        if !cache_dir.exists() {
            debug!("Capability cache dir {} does not exist, skipping load", cache_dir.display());
            return Ok(());
        }
        debug!("Loading capability cache from {}", cache_dir.display());
        let mut results = self.results.lock().unwrap_or_else(|e| e.into_inner());
        let mut loaded = 0u32;
        for entry in std::fs::read_dir(cache_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(caps) = serde_json::from_str::<NegotiatedCapabilities>(&content) {
                        results.insert(caps.provider.clone(), caps);
                        loaded += 1;
                    } else {
                        warn!("Failed to parse capability cache file: {}", path.display());
                    }
                }
            }
        }
        info!("Loaded {} capability cache entries from {}", loaded, cache_dir.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_negotiated_capabilities_creation() {
        let caps = NegotiatedCapabilities {
            provider: "claude".to_string(),
            cli_version: "1.0.0".to_string(),
            cli_mode: CliMode::Structured,
            features: HashMap::new(),
            negotiated_at: 0,
        };
        assert_eq!(caps.provider, "claude");
        assert!(caps.features.is_empty());
    }

    #[test]
    fn test_feature_support_full() {
        let feature = FeatureSupport::Full;
        assert!(feature.is_supported());
        assert!(!feature.needs_workaround());
    }

    #[test]
    fn test_feature_support_partial() {
        let feature = FeatureSupport::Partial {
            limitations: vec!["no streaming".into()],
            workaround: Some(Workaround {
                description: "Simulate streaming".into(),
                method: WorkaroundMethod::SimulateFromBatch,
            }),
        };
        assert!(feature.is_supported());
        assert!(feature.needs_workaround());
    }

    #[test]
    fn test_feature_support_unsupported() {
        let feature = FeatureSupport::Unsupported { alternative: None };
        assert!(!feature.is_supported());
    }

    #[test]
    fn test_negotiator_creation() {
        let negotiator = CapabilityNegotiator::new();
        assert!(negotiator.get_capabilities("claude").is_none());
        assert!(negotiator.all_capabilities().is_empty());
    }

    #[test]
    fn test_set_and_get_capabilities() {
        let negotiator = CapabilityNegotiator::new();
        let mut features = HashMap::new();
        features.insert("streaming".to_string(), FeatureSupport::Full);
        features.insert("thinking".to_string(), FeatureSupport::Unsupported { alternative: None });

        let caps = NegotiatedCapabilities {
            provider: "claude".to_string(),
            cli_version: "1.0.0".to_string(),
            cli_mode: CliMode::Structured,
            features,
            negotiated_at: 12345,
        };

        negotiator.set_capabilities("claude", caps);
        let result = negotiator.get_capabilities("claude");
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.features.len(), 2);
        assert!(r.features.get("streaming").unwrap().is_supported());
    }

    #[test]
    fn test_cache_save_and_load() {
        let dir = TempDir::new().unwrap();
        let negotiator = CapabilityNegotiator::new();

        let mut features = HashMap::new();
        features.insert("streaming".to_string(), FeatureSupport::Full);

        let caps = NegotiatedCapabilities {
            provider: "claude".to_string(),
            cli_version: "1.0.0".to_string(),
            cli_mode: CliMode::Structured,
            features,
            negotiated_at: 12345,
        };

        negotiator.set_capabilities("claude", caps.clone());
        negotiator.save_cache(dir.path()).unwrap();

        let negotiator2 = CapabilityNegotiator::new();
        negotiator2.load_cache(dir.path()).unwrap();
        let loaded = negotiator2.get_capabilities("claude");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().cli_version, "1.0.0");
    }

    #[test]
    fn test_cache_invalidation_on_version_change() {
        let dir = TempDir::new().unwrap();
        let negotiator = CapabilityNegotiator::new();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let caps = NegotiatedCapabilities {
            provider: "claude".to_string(),
            cli_version: "1.0.0".to_string(),
            cli_mode: CliMode::Structured,
            features: HashMap::new(),
            negotiated_at: now,
        };

        negotiator.set_capabilities("claude", caps);
        negotiator.save_cache(dir.path()).unwrap();

        // Should be invalid for different version
        assert!(!negotiator.is_cache_valid("claude", "2.0.0"));
        // Should be valid for same version
        assert!(negotiator.is_cache_valid("claude", "1.0.0"));
    }
}
