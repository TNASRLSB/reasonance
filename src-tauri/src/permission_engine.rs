use log::debug;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PermissionDecision {
    Allow,
    Deny { reason: String },
    Confirm,
}

/// Context for permission evaluation
#[derive(Debug)]
pub struct PermissionContext {
    pub tool_name: String,
    pub tool_args: Option<serde_json::Value>,
    pub provider: String,
    pub permission_level: String, // "yolo", "ask", "locked"
    pub trust_level: String,     // "untrusted", "trusted", "full"
    pub project_root: Option<String>,
}

/// Read-only tools that are safe even in untrusted workspaces
const READ_ONLY_TOOLS: &[&str] = &[
    "Read", "Grep", "Glob", "WebSearch", "WebFetch", "ListDir", "Bash",
];

/// Hardcoded destructive patterns that are ALWAYS denied
const DESTRUCTIVE_PATTERNS: &[&str] = &["rm -rf /", "rm -rf ~", "rm -rf .", "chmod 777"];

/// Git branches that must never receive a force-push
const PROTECTED_BRANCHES: &[&str] = &["main", "master"];

pub struct PermissionEngine;

impl PermissionEngine {
    /// Evaluate the 6-layer decision tree. First match wins.
    ///
    /// Layer 1: Hardcoded security rules (non-overridable)
    /// Layer 2: Workspace trust level
    /// Layer 3: Policy file (.reasonance/permissions.toml) — placeholder
    /// Layer 4: Model-level permission setting
    /// Layer 5: Session memory — placeholder, checked externally
    /// Layer 6: Default → Confirm
    pub fn evaluate(ctx: &PermissionContext) -> PermissionDecision {
        // Layer 1: Hardcoded security rules (non-overridable)
        if let Some(decision) = Self::check_hardcoded_rules(ctx) {
            debug!("Permission: hardcoded rule -> {:?}", decision);
            return decision;
        }

        // Layer 2: Workspace trust level
        if let Some(decision) = Self::check_trust_level(ctx) {
            debug!("Permission: trust level -> {:?}", decision);
            return decision;
        }

        // Layer 3: Policy file (.reasonance/permissions.toml)
        // Placeholder — will check settings for allowed/denied tool lists

        // Layer 4: Model-level permission setting
        if let Some(decision) = Self::check_model_permission(ctx) {
            debug!("Permission: model config -> {:?}", decision);
            return decision;
        }

        // Layer 5: Session memory (handled externally by PermissionMemory in 1.3)

        // Layer 6: Default
        debug!("Permission: default -> Confirm");
        PermissionDecision::Confirm
    }

    /// Layer 1: Hardcoded security rules — ALWAYS enforced, even in yolo mode
    fn check_hardcoded_rules(ctx: &PermissionContext) -> Option<PermissionDecision> {
        // Check tool args for destructive patterns
        if let Some(ref args) = ctx.tool_args {
            let args_str = args.to_string().to_lowercase();

            for pattern in DESTRUCTIVE_PATTERNS {
                if args_str.contains(pattern) {
                    return Some(PermissionDecision::Deny {
                        reason: format!(
                            "Hardcoded safety rule: '{}' is always denied",
                            pattern
                        ),
                    });
                }
            }

            // Check for force-push to protected branches
            if ctx.tool_name == "Bash" || ctx.tool_name == "bash" {
                if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                    if cmd.contains("git push") && cmd.contains("--force") {
                        for branch in PROTECTED_BRANCHES {
                            if cmd.contains(branch) {
                                return Some(PermissionDecision::Deny {
                                    reason: format!(
                                        "Force-push to {} is always denied",
                                        branch
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Check for writes outside project root
        if let (Some(ref root), Some(ref args)) = (&ctx.project_root, &ctx.tool_args) {
            if ctx.tool_name == "Write" || ctx.tool_name == "Edit" {
                if let Some(path) = args.get("file_path").and_then(|v| v.as_str()) {
                    if !path.starts_with(root) {
                        return Some(PermissionDecision::Deny {
                            reason: format!(
                                "Write outside project root denied: {}",
                                path
                            ),
                        });
                    }
                }
            }
        }

        None
    }

    /// Layer 2: Workspace trust level
    fn check_trust_level(ctx: &PermissionContext) -> Option<PermissionDecision> {
        match ctx.trust_level.as_str() {
            "untrusted" => {
                if !READ_ONLY_TOOLS.contains(&ctx.tool_name.as_str()) {
                    return Some(PermissionDecision::Deny {
                        reason: format!(
                            "Tool '{}' denied in untrusted workspace (only read-only tools allowed)",
                            ctx.tool_name
                        ),
                    });
                }
                None
            }
            "full" => {
                // Full trust = allow everything that passes hardcoded rules
                Some(PermissionDecision::Allow)
            }
            _ => None, // "trusted" or unknown — continue evaluation
        }
    }

    /// Layer 4: Model-level permission setting
    fn check_model_permission(ctx: &PermissionContext) -> Option<PermissionDecision> {
        match ctx.permission_level.as_str() {
            "yolo" => Some(PermissionDecision::Allow),
            "locked" => Some(PermissionDecision::Deny {
                reason: "Permission mode is 'locked' -- all tools denied".to_string(),
            }),
            "ask" => None, // Continue to default (Confirm)
            _ => None,
        }
    }

    /// Check if a tool is in the read-only set
    pub fn is_read_only_tool(tool_name: &str) -> bool {
        READ_ONLY_TOOLS.contains(&tool_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base_context() -> PermissionContext {
        PermissionContext {
            tool_name: "Write".to_string(),
            tool_args: None,
            provider: "claude".to_string(),
            permission_level: "ask".to_string(),
            trust_level: "trusted".to_string(),
            project_root: Some("/home/user/project".to_string()),
        }
    }

    // ── Layer 1: Hardcoded security rules ──────────────────────────────

    #[test]
    fn test_rm_rf_root_always_denied() {
        let mut ctx = base_context();
        ctx.tool_name = "Bash".to_string();
        ctx.tool_args = Some(json!({"command": "rm -rf /"}));

        let decision = PermissionEngine::evaluate(&ctx);
        assert!(
            matches!(decision, PermissionDecision::Deny { ref reason } if reason.contains("rm -rf /")),
            "Expected deny for rm -rf /, got {:?}",
            decision
        );
    }

    #[test]
    fn test_rm_rf_home_always_denied() {
        let mut ctx = base_context();
        ctx.tool_name = "Bash".to_string();
        ctx.tool_args = Some(json!({"command": "rm -rf ~"}));

        let decision = PermissionEngine::evaluate(&ctx);
        assert!(
            matches!(decision, PermissionDecision::Deny { ref reason } if reason.contains("rm -rf ~")),
            "Expected deny for rm -rf ~, got {:?}",
            decision
        );
    }

    #[test]
    fn test_force_push_main_denied() {
        let mut ctx = base_context();
        ctx.tool_name = "Bash".to_string();
        ctx.tool_args = Some(json!({"command": "git push --force origin main"}));

        let decision = PermissionEngine::evaluate(&ctx);
        assert!(
            matches!(decision, PermissionDecision::Deny { ref reason } if reason.contains("main")),
            "Expected deny for force-push to main, got {:?}",
            decision
        );
    }

    #[test]
    fn test_write_outside_project_denied() {
        let mut ctx = base_context();
        ctx.tool_args = Some(json!({"file_path": "/etc/passwd"}));

        let decision = PermissionEngine::evaluate(&ctx);
        assert!(
            matches!(decision, PermissionDecision::Deny { ref reason } if reason.contains("outside project root")),
            "Expected deny for write outside project, got {:?}",
            decision
        );
    }

    #[test]
    fn test_hardcoded_rules_even_in_yolo() {
        let mut ctx = base_context();
        ctx.permission_level = "yolo".to_string();
        ctx.tool_name = "Bash".to_string();
        ctx.tool_args = Some(json!({"command": "rm -rf /"}));

        let decision = PermissionEngine::evaluate(&ctx);
        assert!(
            matches!(decision, PermissionDecision::Deny { .. }),
            "Hardcoded rules must override yolo mode, got {:?}",
            decision
        );
    }

    // ── Layer 2: Workspace trust level ─────────────────────────────────

    #[test]
    fn test_untrusted_blocks_write_tools() {
        let mut ctx = base_context();
        ctx.trust_level = "untrusted".to_string();
        ctx.tool_name = "Write".to_string();

        let decision = PermissionEngine::evaluate(&ctx);
        assert!(
            matches!(decision, PermissionDecision::Deny { ref reason } if reason.contains("untrusted")),
            "Expected deny for Write in untrusted workspace, got {:?}",
            decision
        );
    }

    #[test]
    fn test_untrusted_allows_read_tools() {
        let mut ctx = base_context();
        ctx.trust_level = "untrusted".to_string();
        ctx.tool_name = "Read".to_string();

        let decision = PermissionEngine::evaluate(&ctx);
        // Should not be denied by trust level — falls through to model permission (ask → Confirm)
        assert!(
            !matches!(decision, PermissionDecision::Deny { ref reason } if reason.contains("untrusted")),
            "Read should not be blocked in untrusted workspace, got {:?}",
            decision
        );
    }

    #[test]
    fn test_full_trust_allows_all() {
        let mut ctx = base_context();
        ctx.trust_level = "full".to_string();
        ctx.tool_name = "Write".to_string();

        let decision = PermissionEngine::evaluate(&ctx);
        assert_eq!(decision, PermissionDecision::Allow);
    }

    // ── Layer 4: Model-level permission ────────────────────────────────

    #[test]
    fn test_yolo_allows_non_destructive() {
        let mut ctx = base_context();
        ctx.permission_level = "yolo".to_string();
        ctx.tool_name = "Write".to_string();
        ctx.tool_args = Some(json!({"file_path": "/home/user/project/test.txt"}));

        let decision = PermissionEngine::evaluate(&ctx);
        assert_eq!(decision, PermissionDecision::Allow);
    }

    #[test]
    fn test_locked_denies_all() {
        let mut ctx = base_context();
        ctx.permission_level = "locked".to_string();

        let decision = PermissionEngine::evaluate(&ctx);
        assert!(
            matches!(decision, PermissionDecision::Deny { ref reason } if reason.contains("locked")),
            "Expected deny for locked mode, got {:?}",
            decision
        );
    }

    #[test]
    fn test_ask_defaults_to_confirm() {
        let ctx = base_context(); // permission_level = "ask", trust = "trusted"

        let decision = PermissionEngine::evaluate(&ctx);
        assert_eq!(decision, PermissionDecision::Confirm);
    }

    // ── Integration / priority tests ───────────────────────────────────

    #[test]
    fn test_layer_priority_hardcoded_over_yolo() {
        let mut ctx = base_context();
        ctx.permission_level = "yolo".to_string();
        ctx.trust_level = "full".to_string();
        ctx.tool_name = "Bash".to_string();
        ctx.tool_args = Some(json!({"command": "rm -rf /"}));

        let decision = PermissionEngine::evaluate(&ctx);
        assert!(
            matches!(decision, PermissionDecision::Deny { .. }),
            "Hardcoded deny must beat yolo + full trust, got {:?}",
            decision
        );
    }

    #[test]
    fn test_read_only_tool_check() {
        assert!(PermissionEngine::is_read_only_tool("Read"));
        assert!(PermissionEngine::is_read_only_tool("Grep"));
        assert!(PermissionEngine::is_read_only_tool("Glob"));
        assert!(!PermissionEngine::is_read_only_tool("Write"));
        assert!(!PermissionEngine::is_read_only_tool("Edit"));
    }
}
