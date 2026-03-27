use crate::error::ReasonanceError;
use serde::Serialize;
use std::collections::HashMap;

/// Describes what a node type can do (for frontend palette).
#[derive(Debug, Clone, Serialize)]
pub struct NodeDescriptor {
    pub type_id: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub config_schema: serde_json::Value,
}

/// What to do when a node fails.
#[derive(Debug, Clone)]
pub enum ErrorAction {
    Retry,
    Skip,
    Fail,
    Fallback(String),
}

/// Every HIVE node type must implement this trait.
pub trait HiveNodeHandler: Send + Sync {
    /// Unique type identifier (matches `NodeType` lowercase string).
    fn type_id(&self) -> &str;

    /// Validate node configuration against the handler's schema.
    fn validate_config(&self, config: &serde_json::Value) -> Result<(), ReasonanceError>;

    /// Describe node capabilities for the frontend palette.
    fn describe(&self) -> NodeDescriptor;

    /// Handle an execution failure — return the recovery action.
    fn on_error(&self, error: &ReasonanceError) -> ErrorAction;
}

// ── Built-in handlers ────────────────────────────────────────────────────────

pub struct AgentNodeHandler;

impl HiveNodeHandler for AgentNodeHandler {
    fn type_id(&self) -> &str {
        "agent"
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<(), ReasonanceError> {
        // Agent nodes need at minimum a "provider" or "llm" field.
        // The existing AgentNodeConfig uses "llm"; we accept either for forward compat.
        if config.get("provider").is_none() && config.get("llm").is_none() {
            return Err(ReasonanceError::validation(
                "config.provider",
                "Agent node requires a provider (\"provider\" or \"llm\" field)",
            ));
        }
        Ok(())
    }

    fn describe(&self) -> NodeDescriptor {
        NodeDescriptor {
            type_id: "agent".to_string(),
            display_name: "Agent".to_string(),
            description: "LLM-powered agent node".to_string(),
            category: "agent".to_string(),
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "provider": { "type": "string" },
                    "llm": { "type": "string" },
                    "model": { "type": "string" },
                    "system_prompt": { "type": "string" },
                    "capabilities": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "retry": { "type": "integer", "minimum": 0 },
                    "fallback": { "type": "string" },
                    "timeout": { "type": "integer", "minimum": 1 }
                },
                "anyOf": [
                    { "required": ["provider"] },
                    { "required": ["llm"] }
                ]
            }),
        }
    }

    fn on_error(&self, _error: &ReasonanceError) -> ErrorAction {
        ErrorAction::Retry
    }
}

pub struct ResourceNodeHandler;

impl HiveNodeHandler for ResourceNodeHandler {
    fn type_id(&self) -> &str {
        "resource"
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<(), ReasonanceError> {
        if config.get("kind").is_none() {
            return Err(ReasonanceError::validation(
                "config.kind",
                "Resource node requires a kind (e.g. \"file\", \"folder\", \"api\")",
            ));
        }
        Ok(())
    }

    fn describe(&self) -> NodeDescriptor {
        NodeDescriptor {
            type_id: "resource".to_string(),
            display_name: "Resource".to_string(),
            description: "File, folder, API, or database resource node".to_string(),
            category: "resource".to_string(),
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["file", "folder", "api", "database"] },
                    "path": { "type": "string" },
                    "access": { "type": "string", "enum": ["read", "write", "read-write"] }
                },
                "required": ["kind"]
            }),
        }
    }

    fn on_error(&self, _error: &ReasonanceError) -> ErrorAction {
        ErrorAction::Skip
    }
}

pub struct LogicNodeHandler;

impl HiveNodeHandler for LogicNodeHandler {
    fn type_id(&self) -> &str {
        "logic"
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<(), ReasonanceError> {
        if config.get("kind").is_none() {
            return Err(ReasonanceError::validation(
                "config.kind",
                "Logic node requires a kind (e.g. \"condition\", \"router\")",
            ));
        }
        if config.get("rule").is_none() {
            return Err(ReasonanceError::validation(
                "config.rule",
                "Logic node requires a rule expression",
            ));
        }
        Ok(())
    }

    fn describe(&self) -> NodeDescriptor {
        NodeDescriptor {
            type_id: "logic".to_string(),
            display_name: "Logic".to_string(),
            description: "Conditional branching and routing node".to_string(),
            category: "logic".to_string(),
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["condition", "router", "filter"] },
                    "rule": { "type": "string" },
                    "onTrue": { "type": "string" },
                    "onFalse": { "type": "string" }
                },
                "required": ["kind", "rule"]
            }),
        }
    }

    fn on_error(&self, _error: &ReasonanceError) -> ErrorAction {
        ErrorAction::Fail
    }
}

// ── Registry ─────────────────────────────────────────────────────────────────

/// Type-safe registry of all HIVE node handlers.
///
/// Implements `Send + Sync` so it can be used as Tauri managed state.
/// Populated at startup with the three built-in handlers; custom handlers can be
/// added via [`HiveNodeRegistry::register`].
pub struct HiveNodeRegistry {
    handlers: HashMap<String, Box<dyn HiveNodeHandler>>,
}

impl HiveNodeRegistry {
    /// Create a new registry pre-populated with Agent, Resource, and Logic handlers.
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
        };
        registry.register(Box::new(AgentNodeHandler));
        registry.register(Box::new(ResourceNodeHandler));
        registry.register(Box::new(LogicNodeHandler));
        registry
    }

    /// Register a new handler. Replaces any existing handler with the same `type_id`.
    pub fn register(&mut self, handler: Box<dyn HiveNodeHandler>) {
        self.handlers.insert(handler.type_id().to_string(), handler);
    }

    /// Look up a handler by type ID.
    pub fn get(&self, type_id: &str) -> Option<&dyn HiveNodeHandler> {
        self.handlers.get(type_id).map(|h| h.as_ref())
    }

    /// Validate a node config, returning a typed error if the type is unknown or invalid.
    pub fn validate_config(
        &self,
        type_id: &str,
        config: &serde_json::Value,
    ) -> Result<(), ReasonanceError> {
        self.get(type_id)
            .ok_or_else(|| ReasonanceError::not_found("node_type", type_id))?
            .validate_config(config)
    }

    /// Return descriptors for all registered node types (order not guaranteed).
    pub fn describe_all(&self) -> Vec<NodeDescriptor> {
        self.handlers.values().map(|h| h.describe()).collect()
    }

    /// Return all registered type IDs.
    pub fn type_ids(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
}

impl Default for HiveNodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tauri command ─────────────────────────────────────────────────────────────

/// Return descriptors for all registered HIVE node types.
///
/// The frontend uses this to populate the node palette and for config validation.
#[tauri::command]
pub async fn get_node_types(
    registry: tauri::State<'_, HiveNodeRegistry>,
) -> Result<Vec<NodeDescriptor>, ReasonanceError> {
    Ok(registry.describe_all())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 1. Registry has exactly 3 built-in handlers
    #[test]
    fn test_registry_has_three_built_in_handlers() {
        let registry = HiveNodeRegistry::new();
        assert_eq!(registry.handlers.len(), 3);
    }

    // 2. get() returns the correct handler for each built-in type
    #[test]
    fn test_get_returns_correct_handler() {
        let registry = HiveNodeRegistry::new();
        assert!(registry.get("agent").is_some());
        assert_eq!(registry.get("agent").unwrap().type_id(), "agent");

        assert!(registry.get("resource").is_some());
        assert_eq!(registry.get("resource").unwrap().type_id(), "resource");

        assert!(registry.get("logic").is_some());
        assert_eq!(registry.get("logic").unwrap().type_id(), "logic");
    }

    // 3. get() returns None for an unknown type
    #[test]
    fn test_get_returns_none_for_unknown_type() {
        let registry = HiveNodeRegistry::new();
        assert!(registry.get("nonexistent").is_none());
        assert!(registry.get("").is_none());
    }

    // 4. validate_config for agent with provider field → Ok
    #[test]
    fn test_agent_validate_config_with_provider_ok() {
        let registry = HiveNodeRegistry::new();
        let config = serde_json::json!({ "provider": "claude" });
        assert!(registry.validate_config("agent", &config).is_ok());
    }

    // 4b. validate_config for agent with legacy "llm" field → Ok
    #[test]
    fn test_agent_validate_config_with_llm_field_ok() {
        let registry = HiveNodeRegistry::new();
        let config = serde_json::json!({ "llm": "claude" });
        assert!(registry.validate_config("agent", &config).is_ok());
    }

    // 5. validate_config for agent without provider → Err
    #[test]
    fn test_agent_validate_config_without_provider_err() {
        let registry = HiveNodeRegistry::new();
        let config = serde_json::json!({ "model": "claude-3-5-sonnet" });
        let result = registry.validate_config("agent", &config);
        assert!(result.is_err());
        if let Err(ReasonanceError::Validation { field, .. }) = result {
            assert!(field.contains("provider"));
        } else {
            panic!("Expected Validation error");
        }
    }

    // 6. validate_config for resource with kind → Ok
    #[test]
    fn test_resource_validate_config_ok() {
        let registry = HiveNodeRegistry::new();
        let config = serde_json::json!({ "kind": "folder", "path": "/src" });
        assert!(registry.validate_config("resource", &config).is_ok());
    }

    // 6b. validate_config for resource without kind → Err
    #[test]
    fn test_resource_validate_config_without_kind_err() {
        let registry = HiveNodeRegistry::new();
        let config = serde_json::json!({ "path": "/src" });
        let result = registry.validate_config("resource", &config);
        assert!(result.is_err());
    }

    // 6c. validate_config for logic with kind and rule → Ok
    #[test]
    fn test_logic_validate_config_ok() {
        let registry = HiveNodeRegistry::new();
        let config = serde_json::json!({ "kind": "condition", "rule": "output.score > 0.5" });
        assert!(registry.validate_config("logic", &config).is_ok());
    }

    // 6d. validate_config for logic missing rule → Err
    #[test]
    fn test_logic_validate_config_without_rule_err() {
        let registry = HiveNodeRegistry::new();
        let config = serde_json::json!({ "kind": "condition" });
        let result = registry.validate_config("logic", &config);
        assert!(result.is_err());
        if let Err(ReasonanceError::Validation { field, .. }) = result {
            assert!(field.contains("rule"));
        } else {
            panic!("Expected Validation error for missing rule");
        }
    }

    // 7. describe_all returns 3 descriptors
    #[test]
    fn test_describe_all_returns_three_descriptors() {
        let registry = HiveNodeRegistry::new();
        let descriptors = registry.describe_all();
        assert_eq!(descriptors.len(), 3);
    }

    // 8. type_ids returns the 3 built-in IDs
    #[test]
    fn test_type_ids_returns_three_ids() {
        let registry = HiveNodeRegistry::new();
        let mut ids = registry.type_ids();
        ids.sort();
        assert_eq!(ids, vec!["agent", "logic", "resource"]);
    }

    // 9. on_error returns correct action per type
    #[test]
    fn test_on_error_returns_correct_action_per_type() {
        let registry = HiveNodeRegistry::new();
        let dummy_error = ReasonanceError::internal("test error");

        let agent_action = registry.get("agent").unwrap().on_error(&dummy_error);
        assert!(matches!(agent_action, ErrorAction::Retry));

        let resource_action = registry.get("resource").unwrap().on_error(&dummy_error);
        assert!(matches!(resource_action, ErrorAction::Skip));

        let logic_action = registry.get("logic").unwrap().on_error(&dummy_error);
        assert!(matches!(logic_action, ErrorAction::Fail));
    }

    // 10. validate_config for unknown type_id → NotFound error
    #[test]
    fn test_validate_config_unknown_type_returns_not_found() {
        let registry = HiveNodeRegistry::new();
        let config = serde_json::json!({});
        let result = registry.validate_config("unknown_type", &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReasonanceError::NotFound { .. }
        ));
    }

    // 11. Custom handler can be registered and retrieved
    #[test]
    fn test_custom_handler_registration() {
        struct CustomHandler;
        impl HiveNodeHandler for CustomHandler {
            fn type_id(&self) -> &str {
                "custom"
            }
            fn validate_config(&self, _config: &serde_json::Value) -> Result<(), ReasonanceError> {
                Ok(())
            }
            fn describe(&self) -> NodeDescriptor {
                NodeDescriptor {
                    type_id: "custom".to_string(),
                    display_name: "Custom".to_string(),
                    description: "Test custom handler".to_string(),
                    category: "custom".to_string(),
                    config_schema: serde_json::json!({}),
                }
            }
            fn on_error(&self, _error: &ReasonanceError) -> ErrorAction {
                ErrorAction::Fallback("fallback-node-id".to_string())
            }
        }

        let mut registry = HiveNodeRegistry::new();
        registry.register(Box::new(CustomHandler));
        assert_eq!(registry.handlers.len(), 4);
        assert!(registry.get("custom").is_some());
    }

    // 12. NodeDescriptors have correct categories
    #[test]
    fn test_descriptors_have_correct_categories() {
        let registry = HiveNodeRegistry::new();
        let descriptors = registry.describe_all();
        let categories: Vec<&str> = descriptors.iter().map(|d| d.category.as_str()).collect();
        assert!(categories.contains(&"agent"));
        assert!(categories.contains(&"resource"));
        assert!(categories.contains(&"logic"));
    }

    // 13. NodeDescriptor serializes to JSON cleanly
    #[test]
    fn test_descriptor_serializes_to_json() {
        let handler = AgentNodeHandler;
        let descriptor = handler.describe();
        let json = serde_json::to_string(&descriptor).unwrap();
        assert!(json.contains("\"type_id\":\"agent\""));
        assert!(json.contains("\"category\":\"agent\""));
        assert!(json.contains("config_schema"));
    }
}
