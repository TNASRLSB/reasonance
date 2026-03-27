use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model slot types — each slot targets a different usage context.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelSlot {
    Chat,     // main conversation (the ultimate fallback)
    Workflow, // HIVE node default
    Summary,  // session title, analytics insights
    Quick,    // fast ops (inline edit, commit msg, etc.)
}

/// Per-provider model slot configuration.
///
/// Only `chat` is semantically "required" — if it is `None`, all resolution
/// returns `None`. No enforcement at this layer; callers decide what to do.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelSlotConfig {
    pub chat: Option<String>,
    pub workflow: Option<String>,
    pub summary: Option<String>,
    pub quick: Option<String>,
}

impl ModelSlotConfig {
    /// Resolve which model to use for a given slot.
    ///
    /// Fallback chain:
    /// - `Chat`     → chat (no fallback — it is the root)
    /// - `Workflow` → workflow → chat
    /// - `Summary`  → summary → chat
    /// - `Quick`    → quick → summary → chat
    pub fn resolve(&self, slot: &ModelSlot) -> Option<&str> {
        match slot {
            ModelSlot::Chat => self.chat.as_deref(),
            ModelSlot::Workflow => self.workflow.as_deref().or(self.chat.as_deref()),
            ModelSlot::Summary => self.summary.as_deref().or(self.chat.as_deref()),
            ModelSlot::Quick => self
                .quick
                .as_deref()
                .or(self.summary.as_deref())
                .or(self.chat.as_deref()),
        }
    }

    /// List all four slots with their resolved (fallback-applied) models.
    pub fn list_resolved(&self) -> Vec<(ModelSlot, Option<String>)> {
        vec![
            (
                ModelSlot::Chat,
                self.resolve(&ModelSlot::Chat).map(|s| s.to_string()),
            ),
            (
                ModelSlot::Workflow,
                self.resolve(&ModelSlot::Workflow).map(|s| s.to_string()),
            ),
            (
                ModelSlot::Summary,
                self.resolve(&ModelSlot::Summary).map(|s| s.to_string()),
            ),
            (
                ModelSlot::Quick,
                self.resolve(&ModelSlot::Quick).map(|s| s.to_string()),
            ),
        ]
    }
}

/// Registry of slot configs keyed by provider name.
///
/// Wrapped in a `Mutex` when managed by Tauri state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelSlotRegistry {
    pub providers: HashMap<String, ModelSlotConfig>,
}

impl ModelSlotRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve the model for a slot within a provider.
    ///
    /// Returns `None` if the provider is not registered or the slot (and its
    /// fallbacks) have no model configured.
    pub fn resolve_model(&self, provider: &str, slot: &ModelSlot) -> Option<String> {
        self.providers
            .get(provider)
            .and_then(|config| config.resolve(slot))
            .map(|s| s.to_string())
    }

    /// Set a specific slot for a provider, creating the config entry if needed.
    pub fn set_slot(&mut self, provider: &str, slot: ModelSlot, model: String) {
        let config = self.providers.entry(provider.to_string()).or_default();
        match slot {
            ModelSlot::Chat => config.chat = Some(model),
            ModelSlot::Workflow => config.workflow = Some(model),
            ModelSlot::Summary => config.summary = Some(model),
            ModelSlot::Quick => config.quick = Some(model),
        }
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

use std::sync::Mutex;
use tauri::State;

use crate::error::ReasonanceError;

/// Parse a slot name string into a `ModelSlot` variant.
fn parse_slot(slot: &str) -> Result<ModelSlot, ReasonanceError> {
    match slot {
        "chat" => Ok(ModelSlot::Chat),
        "workflow" => Ok(ModelSlot::Workflow),
        "summary" => Ok(ModelSlot::Summary),
        "quick" => Ok(ModelSlot::Quick),
        other => Err(ReasonanceError::validation(
            "slot",
            format!(
                "unknown slot '{}'; expected chat | workflow | summary | quick",
                other
            ),
        )),
    }
}

/// Resolve the model for a given provider + slot (respects fallback chain).
#[tauri::command]
pub async fn get_model_for_slot(
    provider: String,
    slot: String,
    registry: State<'_, Mutex<ModelSlotRegistry>>,
) -> Result<Option<String>, ReasonanceError> {
    let slot = parse_slot(&slot)?;
    let reg = registry.lock().unwrap();
    Ok(reg.resolve_model(&provider, &slot))
}

/// Set a specific slot for a provider.
#[tauri::command]
pub async fn set_model_slot(
    provider: String,
    slot: String,
    model: String,
    registry: State<'_, Mutex<ModelSlotRegistry>>,
) -> Result<(), ReasonanceError> {
    let slot = parse_slot(&slot)?;
    registry.lock().unwrap().set_slot(&provider, slot, model);
    Ok(())
}

/// List all four slots with resolved models for a provider.
///
/// Returns an empty vec if the provider has no configuration.
#[tauri::command]
pub async fn list_model_slots(
    provider: String,
    registry: State<'_, Mutex<ModelSlotRegistry>>,
) -> Result<Vec<(ModelSlot, Option<String>)>, ReasonanceError> {
    let reg = registry.lock().unwrap();
    Ok(reg
        .providers
        .get(&provider)
        .map(|c| c.list_resolved())
        .unwrap_or_default())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_chat(chat: &str) -> ModelSlotConfig {
        ModelSlotConfig {
            chat: Some(chat.to_string()),
            ..Default::default()
        }
    }

    // 1. Chat slot resolves directly
    #[test]
    fn chat_slot_resolves_directly() {
        let cfg = config_with_chat("claude-opus-4-5");
        assert_eq!(cfg.resolve(&ModelSlot::Chat), Some("claude-opus-4-5"));
    }

    // 2. Workflow falls back to chat when not explicitly set
    #[test]
    fn workflow_falls_back_to_chat() {
        let cfg = config_with_chat("claude-sonnet-4-6");
        assert_eq!(cfg.resolve(&ModelSlot::Workflow), Some("claude-sonnet-4-6"));
    }

    // 3. Summary falls back to chat when not explicitly set
    #[test]
    fn summary_falls_back_to_chat() {
        let cfg = config_with_chat("claude-sonnet-4-6");
        assert_eq!(cfg.resolve(&ModelSlot::Summary), Some("claude-sonnet-4-6"));
    }

    // 4. Quick falls back to summary then to chat
    #[test]
    fn quick_falls_back_through_summary_to_chat() {
        // No summary set: quick → chat
        let cfg = config_with_chat("claude-haiku-3-5");
        assert_eq!(cfg.resolve(&ModelSlot::Quick), Some("claude-haiku-3-5"));

        // Summary set: quick → summary
        let cfg2 = ModelSlotConfig {
            chat: Some("claude-opus-4-5".to_string()),
            summary: Some("claude-haiku-3-5".to_string()),
            ..Default::default()
        };
        assert_eq!(cfg2.resolve(&ModelSlot::Quick), Some("claude-haiku-3-5"));
    }

    // 5. All slots return None when nothing is configured
    #[test]
    fn all_slots_return_none_when_empty() {
        let cfg = ModelSlotConfig::default();
        assert_eq!(cfg.resolve(&ModelSlot::Chat), None);
        assert_eq!(cfg.resolve(&ModelSlot::Workflow), None);
        assert_eq!(cfg.resolve(&ModelSlot::Summary), None);
        assert_eq!(cfg.resolve(&ModelSlot::Quick), None);
    }

    // 6. set_slot / resolve_model roundtrip via registry
    #[test]
    fn set_and_get_slot_roundtrip() {
        let mut reg = ModelSlotRegistry::new();
        reg.set_slot("anthropic", ModelSlot::Chat, "claude-opus-4-5".to_string());
        assert_eq!(
            reg.resolve_model("anthropic", &ModelSlot::Chat),
            Some("claude-opus-4-5".to_string())
        );
    }

    // 7. Per-provider isolation — one provider does not bleed into another
    #[test]
    fn per_provider_isolation() {
        let mut reg = ModelSlotRegistry::new();
        reg.set_slot("anthropic", ModelSlot::Chat, "claude-opus-4-5".to_string());
        reg.set_slot("openai", ModelSlot::Chat, "gpt-4o".to_string());

        assert_eq!(
            reg.resolve_model("anthropic", &ModelSlot::Chat),
            Some("claude-opus-4-5".to_string())
        );
        assert_eq!(
            reg.resolve_model("openai", &ModelSlot::Chat),
            Some("gpt-4o".to_string())
        );
        // Unknown provider returns None
        assert_eq!(reg.resolve_model("unknown", &ModelSlot::Chat), None);
    }

    // 8. list_resolved returns all four slots with correct fallback values
    #[test]
    fn list_resolved_returns_all_four_slots() {
        let cfg = ModelSlotConfig {
            chat: Some("claude-opus-4-5".to_string()),
            workflow: Some("claude-sonnet-4-6".to_string()),
            summary: None,
            quick: None,
        };
        let resolved = cfg.list_resolved();
        assert_eq!(resolved.len(), 4);

        let map: HashMap<String, Option<String>> = resolved
            .into_iter()
            .map(|(slot, model)| {
                let key = format!("{:?}", slot).to_lowercase();
                (key, model)
            })
            .collect();

        assert_eq!(map["chat"], Some("claude-opus-4-5".to_string()));
        // workflow is set explicitly
        assert_eq!(map["workflow"], Some("claude-sonnet-4-6".to_string()));
        // summary falls back to chat
        assert_eq!(map["summary"], Some("claude-opus-4-5".to_string()));
        // quick falls back via summary → chat
        assert_eq!(map["quick"], Some("claude-opus-4-5".to_string()));
    }

    // 9. parse_slot rejects unknown slot names
    #[test]
    fn parse_slot_rejects_unknown() {
        assert!(parse_slot("invalid").is_err());
        assert!(parse_slot("").is_err());
        assert!(parse_slot("Chat").is_err()); // case-sensitive
    }

    // 10. parse_slot accepts all valid names
    #[test]
    fn parse_slot_accepts_all_valid_names() {
        assert!(parse_slot("chat").is_ok());
        assert!(parse_slot("workflow").is_ok());
        assert!(parse_slot("summary").is_ok());
        assert!(parse_slot("quick").is_ok());
    }
}
