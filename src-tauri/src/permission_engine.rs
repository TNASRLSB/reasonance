use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Default permission request timeout in seconds (5 minutes).
pub const DEFAULT_PERMISSION_TIMEOUT_SECS: u64 = 300;

/// Configuration for permission request timeouts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionTimeoutConfig {
    pub timeout_secs: u64,
    pub auto_deny_on_timeout: bool,
}

impl Default for PermissionTimeoutConfig {
    fn default() -> Self {
        Self {
            timeout_secs: DEFAULT_PERMISSION_TIMEOUT_SECS,
            auto_deny_on_timeout: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub trust_level: String,      // "untrusted", "trusted", "full"
    pub project_root: Option<String>,
}

/// Read-only tools that are safe even in untrusted workspaces
const READ_ONLY_TOOLS: &[&str] = &["Read", "Grep", "Glob", "WebSearch", "WebFetch", "ListDir"];

/// Hardcoded destructive patterns that are ALWAYS denied
const DESTRUCTIVE_PATTERNS: &[&str] = &["rm -rf /", "rm -rf ~", "rm -rf .", "chmod 777"];

/// Git branches that must never receive a force-push
const PROTECTED_BRANCHES: &[&str] = &["main", "master"];

/// The result of evaluating a permission request through the 6-layer engine.
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub decision: PermissionDecision,
    pub deciding_layer: u8,
    pub tool_name: String,
    pub permission_level: String,
    pub trust_level: String,
}

pub struct PermissionEngine;

impl PermissionEngine {
    /// Convenience wrapper: evaluate without a session context.
    ///
    /// Calls `evaluate_with_session` with an empty session ID so that Layer 5
    /// (session memory) will never match.
    pub fn evaluate(
        ctx: &PermissionContext,
        memory: &PermissionMemory,
        policy: &crate::policy_file::PolicyFile,
    ) -> EvaluationResult {
        Self::evaluate_with_session(ctx, memory, policy, "")
    }

    /// Full 6-layer permission evaluation.
    ///
    /// Layer 1: Hardcoded security rules (non-overridable)
    /// Layer 2: Workspace trust level
    /// Layer 3: Policy file (.reasonance/permissions.toml)
    /// Layer 4: Model-level permission setting
    /// Layer 5: Session memory (skipped when Layer 4 returns Allow / yolo)
    /// Layer 6: Default → Confirm
    pub fn evaluate_with_session(
        ctx: &PermissionContext,
        memory: &PermissionMemory,
        policy: &crate::policy_file::PolicyFile,
        session_id: &str,
    ) -> EvaluationResult {
        let make_result = |decision: PermissionDecision, layer: u8| EvaluationResult {
            decision,
            deciding_layer: layer,
            tool_name: ctx.tool_name.clone(),
            permission_level: ctx.permission_level.clone(),
            trust_level: ctx.trust_level.clone(),
        };

        // Layer 1: Hardcoded security rules (non-overridable)
        if let Some(decision) = Self::check_hardcoded_rules(ctx) {
            debug!("Permission: hardcoded rule -> {:?}", decision);
            return make_result(decision, 1);
        }

        // Layer 2: Workspace trust level
        if let Some(decision) = Self::check_trust_level(ctx) {
            debug!("Permission: trust level -> {:?}", decision);
            return make_result(decision, 2);
        }

        // Layer 3: Policy file (.reasonance/permissions.toml)
        let tool_args_str = ctx
            .tool_args
            .as_ref()
            .map(|v| {
                // For Bash, extract the "command" field for cleaner matching
                if ctx.tool_name == "Bash" || ctx.tool_name == "bash" {
                    if let Some(cmd) = v.get("command").and_then(|c| c.as_str()) {
                        return cmd.to_string();
                    }
                }
                v.to_string()
            })
            .unwrap_or_default();

        if let Some(policy_decision) = policy.evaluate(&ctx.tool_name, &tool_args_str) {
            let decision = match policy_decision {
                crate::policy_file::PolicyDecision::Allow => PermissionDecision::Allow,
                crate::policy_file::PolicyDecision::Deny { reason } => {
                    PermissionDecision::Deny { reason }
                }
                crate::policy_file::PolicyDecision::Confirm => PermissionDecision::Confirm,
            };
            debug!("Permission: policy file -> {:?}", decision);
            return make_result(decision, 3);
        }

        // Layer 4: Model-level permission setting
        if let Some(decision) = Self::check_model_permission(ctx) {
            debug!("Permission: model config -> {:?}", decision);
            // In yolo mode (Allow), skip Layer 5 — return immediately
            return make_result(decision, 4);
        }

        // Layer 5: Session memory
        if let Some(decision) = memory.lookup(session_id, &ctx.tool_name) {
            debug!("Permission: session memory -> {:?}", decision);
            return make_result(decision, 5);
        }

        // Layer 6: Default
        debug!("Permission: default -> Confirm");
        make_result(PermissionDecision::Confirm, 6)
    }

    /// Layer 1: Hardcoded security rules — ALWAYS enforced, even in yolo mode
    fn check_hardcoded_rules(ctx: &PermissionContext) -> Option<PermissionDecision> {
        // Check tool args for destructive patterns
        if let Some(ref args) = ctx.tool_args {
            let args_str = args.to_string().to_lowercase();

            for pattern in DESTRUCTIVE_PATTERNS {
                if args_str.contains(pattern) {
                    return Some(PermissionDecision::Deny {
                        reason: format!("Hardcoded safety rule: '{}' is always denied", pattern),
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
                                    reason: format!("Force-push to {} is always denied", branch),
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
                            reason: format!("Write outside project root denied: {}", path),
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

// ── Per-Tool Approval Memory ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DecisionScope {
    /// Expires after single use
    Once,
    /// Persists for session duration (in-memory only)
    Session,
    /// Persists to disk (future: .reasonance/permissions.toml) — for now, in-memory
    Project,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDecision {
    pub action: PermissionDecision,
    pub scope: DecisionScope,
    pub timestamp: String,
}

/// Stateful memory for per-tool permission decisions.
///
/// Thread-safe (Send + Sync via Mutex) — suitable as Tauri managed state.
/// Key: (session_id, tool_name).
pub struct PermissionMemory {
    decisions: Mutex<HashMap<(String, String), StoredDecision>>,
}

impl Default for PermissionMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionMemory {
    pub fn new() -> Self {
        Self {
            decisions: Mutex::new(HashMap::new()),
        }
    }

    /// Record a decision for a (session, tool) pair.
    pub fn record(
        &self,
        session_id: &str,
        tool_name: &str,
        action: PermissionDecision,
        scope: DecisionScope,
    ) {
        let key = (session_id.to_string(), tool_name.to_string());
        let decision = StoredDecision {
            action,
            scope,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.decisions.lock().unwrap().insert(key, decision);
    }

    /// Look up a prior decision. Returns None if no decision exists.
    /// If scope is `Once`, the decision is consumed (removed) atomically.
    pub fn lookup(&self, session_id: &str, tool_name: &str) -> Option<PermissionDecision> {
        let key = (session_id.to_string(), tool_name.to_string());
        let mut decisions = self.decisions.lock().unwrap();

        match decisions.get(&key) {
            Some(stored) => {
                let result = stored.action.clone();
                if stored.scope == DecisionScope::Once {
                    decisions.remove(&key);
                }
                Some(result)
            }
            None => None,
        }
    }

    /// Clear all decisions for a given session.
    pub fn clear_session(&self, session_id: &str) {
        let mut decisions = self.decisions.lock().unwrap();
        decisions.retain(|(sid, _), _| sid != session_id);
    }

    /// Clear all decisions across all sessions.
    pub fn clear_all(&self) {
        self.decisions.lock().unwrap().clear();
    }

    /// List all decisions for a session (for debugging / UI).
    pub fn list_decisions(&self, session_id: &str) -> Vec<(String, StoredDecision)> {
        self.decisions
            .lock()
            .unwrap()
            .iter()
            .filter(|((sid, _), _)| sid == session_id)
            .map(|((_, tool), decision)| (tool.clone(), decision.clone()))
            .collect()
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

    /// Helper: default empty memory + policy for tests that don't need them.
    fn empty_deps() -> (PermissionMemory, crate::policy_file::PolicyFile) {
        (
            PermissionMemory::new(),
            crate::policy_file::PolicyFile::new(),
        )
    }

    // ── Layer 1: Hardcoded security rules ──────────────────────────────

    #[test]
    fn test_rm_rf_root_always_denied() {
        let mut ctx = base_context();
        ctx.tool_name = "Bash".to_string();
        ctx.tool_args = Some(json!({"command": "rm -rf /"}));

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert!(
            matches!(result.decision, PermissionDecision::Deny { ref reason } if reason.contains("rm -rf /")),
            "Expected deny for rm -rf /, got {:?}",
            result.decision
        );
    }

    #[test]
    fn test_rm_rf_home_always_denied() {
        let mut ctx = base_context();
        ctx.tool_name = "Bash".to_string();
        ctx.tool_args = Some(json!({"command": "rm -rf ~"}));

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert!(
            matches!(result.decision, PermissionDecision::Deny { ref reason } if reason.contains("rm -rf ~")),
            "Expected deny for rm -rf ~, got {:?}",
            result.decision
        );
    }

    #[test]
    fn test_force_push_main_denied() {
        let mut ctx = base_context();
        ctx.tool_name = "Bash".to_string();
        ctx.tool_args = Some(json!({"command": "git push --force origin main"}));

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert!(
            matches!(result.decision, PermissionDecision::Deny { ref reason } if reason.contains("main")),
            "Expected deny for force-push to main, got {:?}",
            result.decision
        );
    }

    #[test]
    fn test_write_outside_project_denied() {
        let mut ctx = base_context();
        ctx.tool_args = Some(json!({"file_path": "/etc/passwd"}));

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert!(
            matches!(result.decision, PermissionDecision::Deny { ref reason } if reason.contains("outside project root")),
            "Expected deny for write outside project, got {:?}",
            result.decision
        );
    }

    #[test]
    fn test_hardcoded_rules_even_in_yolo() {
        let mut ctx = base_context();
        ctx.permission_level = "yolo".to_string();
        ctx.tool_name = "Bash".to_string();
        ctx.tool_args = Some(json!({"command": "rm -rf /"}));

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert!(
            matches!(result.decision, PermissionDecision::Deny { .. }),
            "Hardcoded rules must override yolo mode, got {:?}",
            result.decision
        );
    }

    // ── Layer 2: Workspace trust level ─────────────────────────────────

    #[test]
    fn test_untrusted_blocks_write_tools() {
        let mut ctx = base_context();
        ctx.trust_level = "untrusted".to_string();
        ctx.tool_name = "Write".to_string();

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert!(
            matches!(result.decision, PermissionDecision::Deny { ref reason } if reason.contains("untrusted")),
            "Expected deny for Write in untrusted workspace, got {:?}",
            result.decision
        );
    }

    #[test]
    fn test_untrusted_allows_read_tools() {
        let mut ctx = base_context();
        ctx.trust_level = "untrusted".to_string();
        ctx.tool_name = "Read".to_string();

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        // Should not be denied by trust level — falls through to model permission (ask -> Confirm)
        assert!(
            !matches!(result.decision, PermissionDecision::Deny { ref reason } if reason.contains("untrusted")),
            "Read should not be blocked in untrusted workspace, got {:?}",
            result.decision
        );
    }

    #[test]
    fn test_full_trust_allows_all() {
        let mut ctx = base_context();
        ctx.trust_level = "full".to_string();
        ctx.tool_name = "Write".to_string();

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert_eq!(result.decision, PermissionDecision::Allow);
    }

    // ── Layer 4: Model-level permission ────────────────────────────────

    #[test]
    fn test_yolo_allows_non_destructive() {
        let mut ctx = base_context();
        ctx.permission_level = "yolo".to_string();
        ctx.tool_name = "Write".to_string();
        ctx.tool_args = Some(json!({"file_path": "/home/user/project/test.txt"}));

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert_eq!(result.decision, PermissionDecision::Allow);
    }

    #[test]
    fn test_locked_denies_all() {
        let mut ctx = base_context();
        ctx.permission_level = "locked".to_string();

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert!(
            matches!(result.decision, PermissionDecision::Deny { ref reason } if reason.contains("locked")),
            "Expected deny for locked mode, got {:?}",
            result.decision
        );
    }

    #[test]
    fn test_ask_defaults_to_confirm() {
        let ctx = base_context(); // permission_level = "ask", trust = "trusted"

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert_eq!(result.decision, PermissionDecision::Confirm);
    }

    // ── Integration / priority tests ───────────────────────────────────

    #[test]
    fn test_layer_priority_hardcoded_over_yolo() {
        let mut ctx = base_context();
        ctx.permission_level = "yolo".to_string();
        ctx.trust_level = "full".to_string();
        ctx.tool_name = "Bash".to_string();
        ctx.tool_args = Some(json!({"command": "rm -rf /"}));

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert!(
            matches!(result.decision, PermissionDecision::Deny { .. }),
            "Hardcoded deny must beat yolo + full trust, got {:?}",
            result.decision
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

    // ── PermissionMemory tests ────────────────────────────────────────

    #[test]
    fn test_memory_record_and_lookup_allow() {
        let mem = PermissionMemory::new();
        mem.record(
            "s1",
            "Write",
            PermissionDecision::Allow,
            DecisionScope::Session,
        );
        assert_eq!(mem.lookup("s1", "Write"), Some(PermissionDecision::Allow));
    }

    #[test]
    fn test_memory_record_and_lookup_deny() {
        let mem = PermissionMemory::new();
        mem.record(
            "s1",
            "Bash",
            PermissionDecision::Deny {
                reason: "User denied".to_string(),
            },
            DecisionScope::Session,
        );
        let result = mem.lookup("s1", "Bash");
        assert!(matches!(result, Some(PermissionDecision::Deny { .. })));
    }

    #[test]
    fn test_memory_once_scope_consumed_after_use() {
        let mem = PermissionMemory::new();
        mem.record("s1", "Edit", PermissionDecision::Allow, DecisionScope::Once);

        // First lookup returns the decision
        assert_eq!(mem.lookup("s1", "Edit"), Some(PermissionDecision::Allow));
        // Second lookup returns None — consumed
        assert_eq!(mem.lookup("s1", "Edit"), None);
    }

    #[test]
    fn test_memory_session_scope_persists() {
        let mem = PermissionMemory::new();
        mem.record(
            "s1",
            "Write",
            PermissionDecision::Allow,
            DecisionScope::Session,
        );

        assert_eq!(mem.lookup("s1", "Write"), Some(PermissionDecision::Allow));
        assert_eq!(mem.lookup("s1", "Write"), Some(PermissionDecision::Allow));
        assert_eq!(mem.lookup("s1", "Write"), Some(PermissionDecision::Allow));
    }

    #[test]
    fn test_memory_lookup_nonexistent_returns_none() {
        let mem = PermissionMemory::new();
        assert_eq!(mem.lookup("s1", "Write"), None);
        assert_eq!(mem.lookup("nonexistent", "anything"), None);
    }

    #[test]
    fn test_memory_clear_session() {
        let mem = PermissionMemory::new();
        mem.record(
            "s1",
            "Write",
            PermissionDecision::Allow,
            DecisionScope::Session,
        );
        mem.record(
            "s1",
            "Edit",
            PermissionDecision::Allow,
            DecisionScope::Session,
        );
        mem.record(
            "s2",
            "Write",
            PermissionDecision::Allow,
            DecisionScope::Session,
        );

        mem.clear_session("s1");

        assert_eq!(mem.lookup("s1", "Write"), None);
        assert_eq!(mem.lookup("s1", "Edit"), None);
        // s2 untouched
        assert_eq!(mem.lookup("s2", "Write"), Some(PermissionDecision::Allow));
    }

    #[test]
    fn test_memory_clear_all() {
        let mem = PermissionMemory::new();
        mem.record(
            "s1",
            "Write",
            PermissionDecision::Allow,
            DecisionScope::Session,
        );
        mem.record(
            "s2",
            "Edit",
            PermissionDecision::Allow,
            DecisionScope::Project,
        );

        mem.clear_all();

        assert_eq!(mem.lookup("s1", "Write"), None);
        assert_eq!(mem.lookup("s2", "Edit"), None);
    }

    #[test]
    fn test_memory_list_decisions() {
        let mem = PermissionMemory::new();
        mem.record(
            "s1",
            "Write",
            PermissionDecision::Allow,
            DecisionScope::Session,
        );
        mem.record(
            "s1",
            "Edit",
            PermissionDecision::Deny {
                reason: "no".to_string(),
            },
            DecisionScope::Once,
        );
        mem.record(
            "s2",
            "Bash",
            PermissionDecision::Allow,
            DecisionScope::Session,
        );

        let list = mem.list_decisions("s1");
        assert_eq!(list.len(), 2);

        let tool_names: Vec<&str> = list.iter().map(|(t, _)| t.as_str()).collect();
        assert!(tool_names.contains(&"Write"));
        assert!(tool_names.contains(&"Edit"));
    }

    #[test]
    fn test_memory_project_scope_persists_in_memory() {
        let mem = PermissionMemory::new();
        mem.record(
            "s1",
            "Bash",
            PermissionDecision::Allow,
            DecisionScope::Project,
        );

        // Project scope persists like Session (in-memory for now)
        assert_eq!(mem.lookup("s1", "Bash"), Some(PermissionDecision::Allow));
        assert_eq!(mem.lookup("s1", "Bash"), Some(PermissionDecision::Allow));
    }

    #[test]
    fn test_memory_overwrite_decision() {
        let mem = PermissionMemory::new();
        mem.record(
            "s1",
            "Write",
            PermissionDecision::Allow,
            DecisionScope::Session,
        );
        mem.record(
            "s1",
            "Write",
            PermissionDecision::Deny {
                reason: "changed mind".to_string(),
            },
            DecisionScope::Session,
        );

        let result = mem.lookup("s1", "Write");
        assert!(matches!(result, Some(PermissionDecision::Deny { .. })));
    }

    // ── EvaluationResult & layer integration tests ────────────────────

    #[test]
    fn test_evaluation_result_includes_deciding_layer() {
        // Hardcoded deny (rm -rf) should report layer=1
        let mut ctx = base_context();
        ctx.tool_name = "Bash".to_string();
        ctx.tool_args = Some(json!({"command": "rm -rf /"}));

        let (mem, policy) = empty_deps();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert!(matches!(result.decision, PermissionDecision::Deny { .. }));
        assert_eq!(result.deciding_layer, 1, "Hardcoded deny should be layer 1");
        assert_eq!(result.tool_name, "Bash");
    }

    #[test]
    fn test_layer5_session_memory_allow() {
        // Record Allow in memory for (sess1, Write), evaluate should return layer=5
        let mut ctx = base_context();
        ctx.tool_name = "Write".to_string();
        ctx.tool_args = Some(json!({"file_path": "/home/user/project/foo.txt"}));

        let mem = PermissionMemory::new();
        mem.record(
            "sess1",
            "Write",
            PermissionDecision::Allow,
            DecisionScope::Session,
        );
        let policy = crate::policy_file::PolicyFile::new();

        let result = PermissionEngine::evaluate_with_session(&ctx, &mem, &policy, "sess1");
        assert_eq!(result.decision, PermissionDecision::Allow);
        assert_eq!(result.deciding_layer, 5, "Session memory should be layer 5");
    }

    #[test]
    fn test_layer5_once_consumed() {
        // Once scope: first call -> Allow layer=5, second call -> Confirm layer=6
        let mut ctx = base_context();
        ctx.tool_name = "Write".to_string();
        ctx.tool_args = Some(json!({"file_path": "/home/user/project/foo.txt"}));

        let mem = PermissionMemory::new();
        mem.record(
            "sess1",
            "Write",
            PermissionDecision::Allow,
            DecisionScope::Once,
        );
        let policy = crate::policy_file::PolicyFile::new();

        let r1 = PermissionEngine::evaluate_with_session(&ctx, &mem, &policy, "sess1");
        assert_eq!(r1.decision, PermissionDecision::Allow);
        assert_eq!(r1.deciding_layer, 5, "First call should hit layer 5");

        let r2 = PermissionEngine::evaluate_with_session(&ctx, &mem, &policy, "sess1");
        assert_eq!(r2.decision, PermissionDecision::Confirm);
        assert_eq!(
            r2.deciding_layer, 6,
            "Second call should fall through to layer 6"
        );
    }

    #[test]
    fn test_layer3_policy_applies() {
        // Create a temp permissions.toml denying WebSearch, evaluate -> layer=3, Deny
        let project = tempfile::TempDir::new().unwrap();
        let global = tempfile::TempDir::new().unwrap();

        // Write policy file that denies WebSearch
        let policy_dir = project.path().join(".reasonance");
        std::fs::create_dir_all(&policy_dir).unwrap();
        std::fs::write(
            policy_dir.join("permissions.toml"),
            r#"
[tools.WebSearch]
decision = "deny"
"#,
        )
        .unwrap();

        let policy = crate::policy_file::PolicyFile::new();
        policy.load(project.path(), global.path());

        let mut ctx = base_context();
        ctx.tool_name = "WebSearch".to_string();
        ctx.tool_args = None;

        let mem = PermissionMemory::new();
        let result = PermissionEngine::evaluate(&ctx, &mem, &policy);
        assert!(matches!(result.decision, PermissionDecision::Deny { .. }));
        assert_eq!(result.deciding_layer, 3, "Policy file should be layer 3");
    }

    #[test]
    fn test_yolo_skips_layer5() {
        // Even with Deny in memory, yolo mode -> layer=4, Allow
        // Layer 4 returns Allow before reaching Layer 5.
        let mut ctx = base_context();
        ctx.permission_level = "yolo".to_string();
        ctx.tool_name = "Write".to_string();
        ctx.tool_args = Some(json!({"file_path": "/home/user/project/foo.txt"}));

        let mem = PermissionMemory::new();
        mem.record(
            "sess1",
            "Write",
            PermissionDecision::Deny {
                reason: "user denied".to_string(),
            },
            DecisionScope::Session,
        );
        let policy = crate::policy_file::PolicyFile::new();

        let result = PermissionEngine::evaluate_with_session(&ctx, &mem, &policy, "sess1");
        assert_eq!(result.decision, PermissionDecision::Allow);
        assert_eq!(
            result.deciding_layer, 4,
            "Yolo should return at layer 4, skipping layer 5"
        );
    }
}
