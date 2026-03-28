# W2.1 Permission Engine Wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire PermissionEngine as sole decision point in transport, implement Layer 3 (policy file with regex), Layer 5 (session memory), redesign frontend for per-tool approval with 4 scopes.

**Architecture:** Pure `evaluate()` returns `EvaluationResult` (decision + deciding_layer). Transport calls engine instead of inline trust checks, publishes audit events. Policy file pre-loaded at startup with compiled regexes. PermissionMemory handles Once/Session scope; permissions.toml is single source of truth for Project scope. Frontend PermissionRequestBlock shows per-tool approval with 4 scopes, calls adapter permission commands.

**Tech Stack:** Rust (regex, toml, serde), TypeScript/Svelte (adapter commands, Zod)

**Spec:** `.claude/docs/specs/2026-03-28-w2-1-permission-engine-wiring.md`

---

## File Structure

### New files
| File | Responsibility |
|------|---------------|
| `src-tauri/src/policy_file.rs` | PolicyFile struct: TOML parsing, regex compilation, caching, add/save rules |

### Modified files
| File | Changes |
|------|---------|
| `src-tauri/Cargo.toml` | Add `regex` crate |
| `src-tauri/src/lib.rs` | Declare `policy_file` module, manage `PermissionEngine` as state, pre-load policy at startup |
| `src-tauri/src/permission_engine.rs` | Add `EvaluationResult`, refactor `evaluate()` to take `&PermissionMemory` + `&PolicyFile`, implement Layer 3 + Layer 5 |
| `src-tauri/src/commands/permission.rs` | Route Project scope to PolicyFile instead of PermissionMemory |
| `src-tauri/src/transport/mod.rs` | Replace inline trust checks with `engine.evaluate()`, publish audit events, remove old `build_permission_args*` methods |
| `src-tauri/src/commands/batch.rs` | Add dispatch entries for 5 permission commands |
| `src/lib/adapter/batch-schemas.ts` | Add Zod schemas for permission commands |
| `src/lib/adapter/index.ts` | Add 5 permission methods to Adapter interface |
| `src/lib/adapter/tauri.ts` | Add 5 permission adapter methods via enqueue() |
| `src/lib/components/chat/PermissionRequestBlock.svelte` | Redesign for per-tool approval with 4 scopes |
| `src/lib/components/chat/ChatView.svelte` | Remove `sessionApprovedTools`, use adapter permission commands |
| `src/lib/components/chat/ChatMessages.svelte` | Pass adapter + sessionId to PermissionRequestBlock |
| `tests/mocks/adapter.ts` | Add permission method stubs |

---

## Task 1: Add regex crate + create PolicyFile module

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/policy_file.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `regex` to Cargo.toml**

In `src-tauri/Cargo.toml` `[dependencies]`, add:

```toml
regex = "1"
```

- [ ] **Step 2: Create policy_file.rs with PolicyFile struct**

Create `src-tauri/src/policy_file.rs`:

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use log::{debug, error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::ReasonanceError;

/// A compiled tool rule from permissions.toml
#[derive(Debug)]
struct CompiledToolRule {
    decision: String, // "allow" | "deny" | "confirm"
    patterns_deny: Vec<Regex>,
    patterns_allow: Vec<Regex>,
}

/// Raw deserialized tool rule from TOML
#[derive(Debug, Deserialize, Serialize)]
struct RawToolRule {
    decision: String,
    #[serde(default)]
    patterns_deny: Vec<String>,
    #[serde(default)]
    patterns_allow: Vec<String>,
}

/// Raw permissions file structure
#[derive(Debug, Deserialize, Serialize, Default)]
struct RawPolicyFile {
    #[serde(default)]
    tools: HashMap<String, RawToolRule>,
}

/// Cached, compiled policy file with pre-compiled regex patterns.
/// Thread-safe via Mutex — suitable as Tauri managed state.
pub struct PolicyFile {
    inner: Mutex<PolicyFileInner>,
}

struct PolicyFileInner {
    project_path: Option<PathBuf>,
    global_path: Option<PathBuf>,
    /// Merged rules: project overrides global. Key = tool name.
    rules: HashMap<String, CompiledToolRule>,
}

impl PolicyFile {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PolicyFileInner {
                project_path: None,
                global_path: None,
                rules: HashMap::new(),
            }),
        }
    }

    /// Load policy from project and/or global paths. Project takes priority.
    pub fn load(&self, project_root: Option<&Path>, global_config_dir: Option<&Path>) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());

        let project_path = project_root.map(|r| r.join(".reasonance").join("permissions.toml"));
        let global_path = global_config_dir.map(|d| d.join("permissions.toml"));

        inner.project_path = project_path.clone();
        inner.global_path = global_path.clone();

        let mut merged = HashMap::new();

        // Load global first (lower priority)
        if let Some(ref path) = global_path {
            if path.exists() {
                match Self::parse_file(path) {
                    Ok(rules) => {
                        info!("PolicyFile: loaded {} global rules from {:?}", rules.len(), path);
                        merged.extend(rules);
                    }
                    Err(e) => warn!("PolicyFile: failed to parse global {:?}: {}", path, e),
                }
            }
        }

        // Load project (overrides global)
        if let Some(ref path) = project_path {
            if path.exists() {
                match Self::parse_file(path) {
                    Ok(rules) => {
                        info!("PolicyFile: loaded {} project rules from {:?}", rules.len(), path);
                        merged.extend(rules);
                    }
                    Err(e) => warn!("PolicyFile: failed to parse project {:?}: {}", path, e),
                }
            }
        }

        inner.rules = merged;
    }

    /// Reload policy files (called on fs change event).
    pub fn reload(&self) {
        let (project_path, global_path) = {
            let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            (
                inner.project_path.as_ref().and_then(|p| p.parent().and_then(|pp| pp.parent()).map(|r| r.to_path_buf())),
                inner.global_path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf())),
            )
        };
        self.load(project_path.as_deref(), global_path.as_deref());
    }

    /// Look up a tool in the policy. Returns None if tool not in policy.
    pub fn evaluate(&self, tool_name: &str, tool_args: &str) -> Option<PolicyDecision> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let rule = inner.rules.get(tool_name)?;

        // Check deny patterns first
        for pattern in &rule.patterns_deny {
            if pattern.is_match(tool_args) {
                debug!("PolicyFile: tool={} args matched deny pattern '{}'", tool_name, pattern);
                return Some(PolicyDecision::Deny {
                    reason: format!("Policy deny pattern matched: {}", pattern),
                });
            }
        }

        // Check allow patterns
        for pattern in &rule.patterns_allow {
            if pattern.is_match(tool_args) {
                debug!("PolicyFile: tool={} args matched allow pattern '{}'", tool_name, pattern);
                return Some(PolicyDecision::Allow);
            }
        }

        // No pattern matched — use the base decision
        match rule.decision.as_str() {
            "allow" => Some(PolicyDecision::Allow),
            "deny" => Some(PolicyDecision::Deny {
                reason: format!("Policy file denies tool '{}'", tool_name),
            }),
            "confirm" => Some(PolicyDecision::Confirm),
            _ => {
                warn!("PolicyFile: unknown decision '{}' for tool '{}'", rule.decision, tool_name);
                None
            }
        }
    }

    /// Add a rule (for Project scope persistence). Writes to project-level file.
    pub fn add_policy_rule(&self, tool_name: &str, decision: &str) -> Result<(), ReasonanceError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let path = inner.project_path.as_ref().ok_or_else(|| {
            ReasonanceError::config("No project path set for policy file")
        })?;

        // Read existing file or create new
        let mut raw: RawPolicyFile = if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| ReasonanceError::io("read permissions.toml", e))?;
            toml::from_str(&content).unwrap_or_default()
        } else {
            RawPolicyFile::default()
        };

        // Add or update the rule
        raw.tools.insert(tool_name.to_string(), RawToolRule {
            decision: decision.to_string(),
            patterns_deny: Vec::new(),
            patterns_allow: Vec::new(),
        });

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ReasonanceError::io("create .reasonance dir", e))?;
        }

        // Write back
        let content = toml::to_string_pretty(&raw)
            .map_err(|e| ReasonanceError::serialization("permissions.toml", e.to_string()))?;
        std::fs::write(path, content)
            .map_err(|e| ReasonanceError::io("write permissions.toml", e))?;

        info!("PolicyFile: added rule tool={} decision={}", tool_name, decision);
        drop(inner);
        self.reload();
        Ok(())
    }

    fn parse_file(path: &Path) -> Result<HashMap<String, CompiledToolRule>, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("read error: {}", e))?;
        let raw: RawPolicyFile = toml::from_str(&content)
            .map_err(|e| format!("TOML parse error: {}", e))?;

        let mut rules = HashMap::new();
        for (tool_name, raw_rule) in raw.tools {
            let patterns_deny = raw_rule.patterns_deny.iter()
                .filter_map(|p| match Regex::new(p) {
                    Ok(r) => Some(r),
                    Err(e) => {
                        warn!("PolicyFile: invalid deny regex '{}': {}", p, e);
                        None
                    }
                })
                .collect();
            let patterns_allow = raw_rule.patterns_allow.iter()
                .filter_map(|p| match Regex::new(p) {
                    Ok(r) => Some(r),
                    Err(e) => {
                        warn!("PolicyFile: invalid allow regex '{}': {}", p, e);
                        None
                    }
                })
                .collect();

            rules.insert(tool_name, CompiledToolRule {
                decision: raw_rule.decision,
                patterns_deny,
                patterns_allow,
            });
        }
        Ok(rules)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
    Confirm,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_toml(dir: &Path, content: &str) -> PathBuf {
        let reasonance_dir = dir.join(".reasonance");
        std::fs::create_dir_all(&reasonance_dir).unwrap();
        let path = reasonance_dir.join("permissions.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        dir.to_path_buf()
    }

    #[test]
    fn test_basic_allow_deny() {
        let dir = tempfile::tempdir().unwrap();
        let root = write_toml(dir.path(), r#"
[tools.Write]
decision = "allow"

[tools.WebSearch]
decision = "deny"
"#);
        let pf = PolicyFile::new();
        pf.load(Some(&root), None);

        assert_eq!(pf.evaluate("Write", ""), Some(PolicyDecision::Allow));
        assert!(matches!(pf.evaluate("WebSearch", ""), Some(PolicyDecision::Deny { .. })));
        assert_eq!(pf.evaluate("Read", ""), None); // not in file
    }

    #[test]
    fn test_regex_deny_pattern() {
        let dir = tempfile::tempdir().unwrap();
        let root = write_toml(dir.path(), r#"
[tools.Bash]
decision = "confirm"
patterns_deny = ["^rm\\s+-rf", "^DROP\\s+TABLE"]
"#);
        let pf = PolicyFile::new();
        pf.load(Some(&root), None);

        assert!(matches!(pf.evaluate("Bash", "rm -rf /tmp"), Some(PolicyDecision::Deny { .. })));
        assert!(matches!(pf.evaluate("Bash", "DROP TABLE users"), Some(PolicyDecision::Deny { .. })));
        // "inform" should NOT match "rm" pattern
        assert_eq!(pf.evaluate("Bash", "echo inform user"), Some(PolicyDecision::Confirm));
    }

    #[test]
    fn test_regex_allow_pattern() {
        let dir = tempfile::tempdir().unwrap();
        let root = write_toml(dir.path(), r#"
[tools.Bash]
decision = "deny"
patterns_allow = ["^ls\\b", "^npm\\s+test"]
"#);
        let pf = PolicyFile::new();
        pf.load(Some(&root), None);

        assert_eq!(pf.evaluate("Bash", "ls -la"), Some(PolicyDecision::Allow));
        assert_eq!(pf.evaluate("Bash", "npm test"), Some(PolicyDecision::Allow));
        assert!(matches!(pf.evaluate("Bash", "npm install malware"), Some(PolicyDecision::Deny { .. })));
    }

    #[test]
    fn test_deny_pattern_takes_priority_over_allow() {
        let dir = tempfile::tempdir().unwrap();
        let root = write_toml(dir.path(), r#"
[tools.Bash]
decision = "confirm"
patterns_deny = ["^rm\\s+-rf"]
patterns_allow = ["^rm"]
"#);
        let pf = PolicyFile::new();
        pf.load(Some(&root), None);

        // "rm -rf" matches deny pattern (checked first) → Deny
        assert!(matches!(pf.evaluate("Bash", "rm -rf /"), Some(PolicyDecision::Deny { .. })));
        // "rm file.txt" matches allow pattern → Allow
        assert_eq!(pf.evaluate("Bash", "rm file.txt"), Some(PolicyDecision::Allow));
    }

    #[test]
    fn test_absent_file_returns_none() {
        let pf = PolicyFile::new();
        pf.load(Some(Path::new("/nonexistent")), None);
        assert_eq!(pf.evaluate("Write", ""), None);
    }

    #[test]
    fn test_add_policy_rule() {
        let dir = tempfile::tempdir().unwrap();
        let pf = PolicyFile::new();
        pf.load(Some(dir.path()), None);

        pf.add_policy_rule("Write", "allow").unwrap();

        assert_eq!(pf.evaluate("Write", ""), Some(PolicyDecision::Allow));
        // Verify file was written
        let path = dir.path().join(".reasonance").join("permissions.toml");
        assert!(path.exists());
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("[tools.Write]"));
    }

    #[test]
    fn test_project_overrides_global() {
        let project_dir = tempfile::tempdir().unwrap();
        let global_dir = tempfile::tempdir().unwrap();

        // Global says deny Write
        let global_reasonance = global_dir.path().to_path_buf();
        std::fs::create_dir_all(&global_reasonance).unwrap();
        std::fs::write(
            global_reasonance.join("permissions.toml"),
            "[tools.Write]\ndecision = \"deny\"\n",
        ).unwrap();

        // Project says allow Write
        write_toml(project_dir.path(), "[tools.Write]\ndecision = \"allow\"\n");

        let pf = PolicyFile::new();
        pf.load(Some(project_dir.path()), Some(&global_reasonance));

        // Project wins
        assert_eq!(pf.evaluate("Write", ""), Some(PolicyDecision::Allow));
    }
}
```

- [ ] **Step 3: Register module in lib.rs**

In `src-tauri/src/lib.rs`, add module declaration:

```rust
pub mod policy_file;
```

And in the `.manage()` chain, add:

```rust
.manage(policy_file::PolicyFile::new())
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: compiles with no errors

- [ ] **Step 5: Run tests**

Run: `cd src-tauri && cargo test -- policy_file`
Expected: all 7 tests pass

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/policy_file.rs src-tauri/src/lib.rs
git commit -m "feat(permissions): add PolicyFile with TOML parsing, regex patterns, caching"
```

---

## Task 2: Refactor evaluate() — EvaluationResult + Layer 3 + Layer 5

**Files:**
- Modify: `src-tauri/src/permission_engine.rs`

This task changes `evaluate()` from a static method returning `PermissionDecision` to a method returning `EvaluationResult`, integrating Layer 3 (PolicyFile) and Layer 5 (PermissionMemory).

- [ ] **Step 1: Write failing tests for new evaluate()**

Add these tests at the end of the existing test module in `src-tauri/src/permission_engine.rs`:

```rust
    #[test]
    fn test_evaluation_result_includes_deciding_layer() {
        let policy = crate::policy_file::PolicyFile::new();
        let memory = PermissionMemory::new();
        let ctx = PermissionContext {
            tool_name: "Bash".to_string(),
            tool_args: Some(serde_json::json!({"command": "rm -rf /"})),
            provider: "claude".to_string(),
            permission_level: "yolo".to_string(),
            trust_level: "trusted".to_string(),
            project_root: Some("/project".to_string()),
        };
        let result = PermissionEngine::evaluate(&ctx, &memory, &policy);
        assert_eq!(result.deciding_layer, 1); // hardcoded
        assert!(matches!(result.decision, PermissionDecision::Deny { .. }));
    }

    #[test]
    fn test_layer5_session_memory_allow() {
        let policy = crate::policy_file::PolicyFile::new();
        let memory = PermissionMemory::new();
        memory.record("sess1", "Write", PermissionDecision::Allow, DecisionScope::Session);

        let ctx = PermissionContext {
            tool_name: "Write".to_string(),
            tool_args: None,
            provider: "claude".to_string(),
            permission_level: "ask".to_string(),
            trust_level: "trusted".to_string(),
            project_root: Some("/project".to_string()),
        };
        let result = PermissionEngine::evaluate_with_session(&ctx, &memory, &policy, "sess1");
        assert_eq!(result.deciding_layer, 5);
        assert_eq!(result.decision, PermissionDecision::Allow);
    }

    #[test]
    fn test_layer5_once_consumed() {
        let policy = crate::policy_file::PolicyFile::new();
        let memory = PermissionMemory::new();
        memory.record("sess1", "Write", PermissionDecision::Allow, DecisionScope::Once);

        let ctx = PermissionContext {
            tool_name: "Write".to_string(),
            tool_args: None,
            provider: "claude".to_string(),
            permission_level: "ask".to_string(),
            trust_level: "trusted".to_string(),
            project_root: Some("/project".to_string()),
        };

        // First call: consumed
        let r1 = PermissionEngine::evaluate_with_session(&ctx, &memory, &policy, "sess1");
        assert_eq!(r1.deciding_layer, 5);
        assert_eq!(r1.decision, PermissionDecision::Allow);

        // Second call: Once is consumed, falls through to Layer 6 (Confirm)
        let r2 = PermissionEngine::evaluate_with_session(&ctx, &memory, &policy, "sess1");
        assert_eq!(r2.deciding_layer, 6);
        assert_eq!(r2.decision, PermissionDecision::Confirm);
    }

    #[test]
    fn test_layer3_policy_applies() {
        let policy = crate::policy_file::PolicyFile::new();
        // Manually add a rule to simulate loaded policy
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".reasonance")).unwrap();
        std::fs::write(
            dir.path().join(".reasonance").join("permissions.toml"),
            "[tools.WebSearch]\ndecision = \"deny\"\n",
        ).unwrap();
        policy.load(Some(dir.path()), None);

        let memory = PermissionMemory::new();
        let ctx = PermissionContext {
            tool_name: "WebSearch".to_string(),
            tool_args: None,
            provider: "claude".to_string(),
            permission_level: "ask".to_string(),
            trust_level: "trusted".to_string(),
            project_root: Some(dir.path().to_string_lossy().to_string()),
        };
        let result = PermissionEngine::evaluate_with_session(&ctx, &memory, &policy, "sess1");
        assert_eq!(result.deciding_layer, 3);
        assert!(matches!(result.decision, PermissionDecision::Deny { .. }));
    }

    #[test]
    fn test_yolo_skips_layer5() {
        let policy = crate::policy_file::PolicyFile::new();
        let memory = PermissionMemory::new();
        // Even with a deny in memory, yolo mode should Allow (Layer 4)
        memory.record("sess1", "Write", PermissionDecision::Deny { reason: "user denied".into() }, DecisionScope::Session);

        let ctx = PermissionContext {
            tool_name: "Write".to_string(),
            tool_args: None,
            provider: "claude".to_string(),
            permission_level: "yolo".to_string(),
            trust_level: "trusted".to_string(),
            project_root: Some("/project".to_string()),
        };
        let result = PermissionEngine::evaluate_with_session(&ctx, &memory, &policy, "sess1");
        assert_eq!(result.deciding_layer, 4); // yolo allows at Layer 4
        assert_eq!(result.decision, PermissionDecision::Allow);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test -- permission_engine 2>&1 | tail -20`
Expected: FAIL — `EvaluationResult` and `evaluate_with_session` don't exist yet

- [ ] **Step 3: Implement EvaluationResult and refactored evaluate()**

In `src-tauri/src/permission_engine.rs`, add the `EvaluationResult` struct and refactor `evaluate()`:

```rust
/// Result of a permission evaluation, including which layer made the decision.
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub decision: PermissionDecision,
    pub deciding_layer: u8,
    pub tool_name: String,
    pub permission_level: String,
    pub trust_level: String,
}
```

Add a new `evaluate_with_session()` method to `PermissionEngine` that integrates Layer 3 and Layer 5:

```rust
    /// Full 6-layer evaluation with session memory and policy file.
    /// Pure function — no side effects, no I/O.
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
            debug!("Permission: L1 hardcoded -> {:?}", decision);
            return make_result(decision, 1);
        }

        // Layer 2: Workspace trust level
        if let Some(decision) = Self::check_trust_level(ctx) {
            debug!("Permission: L2 trust -> {:?}", decision);
            return make_result(decision, 2);
        }

        // Layer 3: Policy file
        let tool_args_str = ctx.tool_args
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default();
        if let Some(policy_decision) = policy.evaluate(&ctx.tool_name, &tool_args_str) {
            let decision = match policy_decision {
                crate::policy_file::PolicyDecision::Allow => PermissionDecision::Allow,
                crate::policy_file::PolicyDecision::Deny { reason } => PermissionDecision::Deny { reason },
                crate::policy_file::PolicyDecision::Confirm => PermissionDecision::Confirm,
            };
            debug!("Permission: L3 policy -> {:?}", decision);
            return make_result(decision, 3);
        }

        // Layer 4: Model-level permission setting
        if let Some(decision) = Self::check_model_permission(ctx) {
            debug!("Permission: L4 config -> {:?}", decision);
            return make_result(decision, 4);
        }

        // Layer 5: Session memory (Once + Session scope only, NOT Project)
        if let Some(decision) = memory.lookup(session_id, &ctx.tool_name) {
            debug!("Permission: L5 memory -> {:?}", decision);
            return make_result(decision, 5);
        }

        // Layer 6: Default
        debug!("Permission: L6 default -> Confirm");
        make_result(PermissionDecision::Confirm, 6)
    }

    /// Backward-compatible evaluate without session context (for existing callers).
    /// Wraps evaluate_with_session with empty memory and policy.
    pub fn evaluate(
        ctx: &PermissionContext,
        memory: &PermissionMemory,
        policy: &crate::policy_file::PolicyFile,
    ) -> EvaluationResult {
        Self::evaluate_with_session(ctx, memory, policy, "")
    }
```

Update the original `evaluate()` call sites — the old static `evaluate(ctx)` signature is replaced by the new one that takes memory and policy. Existing tests that call `PermissionEngine::evaluate(ctx)` need to be updated to pass `&PermissionMemory::new()` and `&PolicyFile::new()`.

- [ ] **Step 4: Update existing tests to use new signature**

All existing tests that call `PermissionEngine::evaluate(&ctx)` need to be updated to `PermissionEngine::evaluate(&ctx, &PermissionMemory::new(), &PolicyFile::new())` and check `result.decision` instead of the raw `PermissionDecision`.

For each existing test, change:
```rust
// Before:
let decision = PermissionEngine::evaluate(&ctx);
assert_eq!(decision, PermissionDecision::Allow);

// After:
let result = PermissionEngine::evaluate(&ctx, &PermissionMemory::new(), &crate::policy_file::PolicyFile::new());
assert_eq!(result.decision, PermissionDecision::Allow);
```

- [ ] **Step 5: Run all tests**

Run: `cd src-tauri && cargo test -- permission_engine`
Expected: all tests pass (existing + new)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/permission_engine.rs
git commit -m "feat(permissions): add EvaluationResult, integrate Layer 3 + Layer 5 in evaluate()"
```

---

## Task 3: Wire engine into transport

**Files:**
- Modify: `src-tauri/src/transport/mod.rs`
- Modify: `src-tauri/src/lib.rs` (manage PermissionEngine state, pre-load policy at startup)

- [ ] **Step 1: Add PermissionEngine and PolicyFile to transport send()**

In `src-tauri/src/transport/mod.rs`, the `send()` method currently has inline trust checks (around lines 100-135). Replace them with a call to `PermissionEngine::evaluate_with_session()`.

The transport needs access to `PermissionEngine`, `PermissionMemory`, and `PolicyFile` — these are available via `app.state::<T>()` in the Tauri command that calls `send()`, or passed as parameters.

Read the current `send()` signature and the Tauri command that calls it (`agent_send` in `commands/transport.rs`). Add `memory: &PermissionMemory` and `policy: &PolicyFile` parameters to `send()`, or access them via `AppHandle`.

Replace the inline trust checks with:

```rust
use crate::permission_engine::{PermissionEngine, PermissionContext, EvaluationResult, PermissionDecision};

// Replace lines 100-135 with:
let trust_level_str = match trust_level {
    Some(crate::workspace_trust::TrustLevel::Trusted) => "trusted",
    Some(crate::workspace_trust::TrustLevel::ReadOnly) => "read_only",
    Some(crate::workspace_trust::TrustLevel::Blocked) => "blocked",
    None => "untrusted",
};

let ctx = PermissionContext {
    tool_name: "*".to_string(),
    tool_args: None,
    provider: provider.clone(),
    permission_level: config.cli.permission_level.clone().unwrap_or_else(|| {
        if request.yolo { "yolo".to_string() } else { "ask".to_string() }
    }),
    trust_level: trust_level_str.to_string(),
    project_root: request.cwd.clone(),
};

let eval_result = PermissionEngine::evaluate(&ctx, memory, policy);

// Publish audit event
if let Ok(event_bus) = /* get EventBus */ {
    event_bus.publish(crate::event_bus::Event::new(
        "permission:decision",
        serde_json::json!({
            "tool": eval_result.tool_name,
            "decision": format!("{:?}", eval_result.decision),
            "layer": eval_result.deciding_layer,
            "session_id": &session_id,
            "trust_level": eval_result.trust_level,
            "permission_level": eval_result.permission_level,
        }),
        "permission_engine",
    ));
}

match eval_result.decision {
    PermissionDecision::Deny { reason } => {
        return Err(crate::error::ReasonanceError::PermissionDenied {
            action: reason,
            tool: None,
        });
    }
    _ => { /* Allow or Confirm — proceed with appropriate CLI args */ }
}
```

- [ ] **Step 2: Build --allowedTools and --permission args from engine decision**

Based on `eval_result.decision`:
- `Allow` (yolo or trusted) → pass `--dangerously-skip-permissions` (from normalizer config)
- `Confirm` (ask mode) → do NOT pass permission flags, let CLI ask interactively
- For ReadOnly trust level → pass `--allowedTools` with read_only_tools from normalizer config

This replaces `build_permission_args_with_trust()`, `build_allowed_tools_args()`, and `build_read_only_tools_args()`.

- [ ] **Step 3: Remove old inline methods**

Mark or remove:
- `build_permission_args()` (already `#[allow(dead_code)]`)
- `build_permission_args_with_trust()`
- `build_read_only_tools_args()`
- `build_allowed_tools_args()` (keep if still useful for building the tool list)

- [ ] **Step 4: Pre-load policy at startup**

In `src-tauri/src/lib.rs` setup, after project root is known, load the policy:

```rust
// In setup() after project root detection:
if let Ok(policy) = app.try_state::<crate::policy_file::PolicyFile>() {
    let config_dir = dirs::config_dir().map(|d| d.join("reasonance"));
    policy.load(project_root.as_deref(), config_dir.as_deref());
}
```

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: compiles

- [ ] **Step 6: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: all pass

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/transport/mod.rs src-tauri/src/lib.rs
git commit -m "feat(permissions): wire engine into transport, replace inline trust checks, add audit events"
```

---

## Task 4: Update commands/permission.rs for Project scope

**Files:**
- Modify: `src-tauri/src/commands/permission.rs`

- [ ] **Step 1: Route Project scope to PolicyFile**

When `record_permission_decision` is called with `scope = "project"`, write to PolicyFile instead of PermissionMemory:

```rust
#[tauri::command]
pub async fn record_permission_decision(
    session_id: String,
    tool_name: String,
    action: String,
    scope: String,
    memory: State<'_, PermissionMemory>,
    policy: State<'_, crate::policy_file::PolicyFile>,
) -> Result<(), crate::error::ReasonanceError> {
    let decision = match action.as_str() {
        "allow" => PermissionDecision::Allow,
        "deny" => PermissionDecision::Deny { reason: "User denied".to_string() },
        _ => return Err(crate::error::ReasonanceError::validation("action", "must be 'allow' or 'deny'")),
    };

    match scope.as_str() {
        "once" => memory.record(&session_id, &tool_name, decision, DecisionScope::Once),
        "session" => memory.record(&session_id, &tool_name, decision, DecisionScope::Session),
        "project" => {
            // Project scope → write to permissions.toml (single source of truth)
            let decision_str = match action.as_str() {
                "allow" => "allow",
                "deny" => "deny",
                _ => unreachable!(),
            };
            policy.add_policy_rule(&tool_name, decision_str)?;
        }
        _ => return Err(crate::error::ReasonanceError::validation("scope", "must be 'once', 'session', or 'project'")),
    }
    Ok(())
}
```

- [ ] **Step 2: Verify compilation and tests**

Run: `cd src-tauri && cargo check && cargo test -- commands::permission`
Expected: pass

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/permission.rs
git commit -m "feat(permissions): route Project scope to PolicyFile instead of PermissionMemory"
```

---

## Task 5: Frontend — adapter permission methods + batch dispatch

**Files:**
- Modify: `src/lib/adapter/index.ts`
- Modify: `src/lib/adapter/tauri.ts`
- Modify: `src/lib/adapter/batch-schemas.ts`
- Modify: `src-tauri/src/commands/batch.rs`
- Modify: `tests/mocks/adapter.ts`

- [ ] **Step 1: Add permission methods to Adapter interface**

In `src/lib/adapter/index.ts`, add to the `Adapter` interface:

```typescript
  // Permissions
  recordPermissionDecision(sessionId: string, toolName: string, action: string, scope: string): Promise<void>;
  lookupPermissionDecision(sessionId: string, toolName: string): Promise<PermissionDecision | null>;
  clearPermissionSession(sessionId: string): Promise<void>;
```

Add the `PermissionDecision` type near the top:

```typescript
export type PermissionDecision = 'Allow' | { Deny: { reason: string } } | 'Confirm';
```

- [ ] **Step 2: Add adapter implementations**

In `src/lib/adapter/tauri.ts`, add the methods (routed through `enqueue` for batching):

```typescript
  async recordPermissionDecision(sessionId: string, toolName: string, action: string, scope: string): Promise<void> {
    return this.enqueue('record_permission_decision', { sessionId, toolName, action, scope }) as Promise<void>;
  }
  async lookupPermissionDecision(sessionId: string, toolName: string): Promise<PermissionDecision | null> {
    return this.enqueue('lookup_permission_decision', { sessionId, toolName }) as Promise<PermissionDecision | null>;
  }
  async clearPermissionSession(sessionId: string): Promise<void> {
    return this.enqueue('clear_permission_session', { sessionId }) as Promise<void>;
  }
```

- [ ] **Step 3: Add Rust batch dispatch entries**

In `src-tauri/src/commands/batch.rs`, add dispatch arms for the permission commands. These need `_inner` extraction first, or call the command functions directly since they take `State<PermissionMemory>` — use `app.state::<PermissionMemory>()`.

Add to the `dispatch()` match:

```rust
"record_permission_decision" => {
    let session_id: String = extract(&args, "sessionId")?;
    let tool_name: String = extract(&args, "toolName")?;
    let action: String = extract(&args, "action")?;
    let scope: String = extract(&args, "scope")?;
    let memory = app.state::<crate::permission_engine::PermissionMemory>();
    let policy = app.state::<crate::policy_file::PolicyFile>();
    // Inline the logic since we need both states
    let decision = match action.as_str() {
        "allow" => crate::permission_engine::PermissionDecision::Allow,
        "deny" => crate::permission_engine::PermissionDecision::Deny { reason: "User denied".into() },
        other => return Err(ReasonanceError::validation("action", format!("must be 'allow' or 'deny', got '{}'", other))),
    };
    match scope.as_str() {
        "once" => memory.record(&session_id, &tool_name, decision, crate::permission_engine::DecisionScope::Once),
        "session" => memory.record(&session_id, &tool_name, decision, crate::permission_engine::DecisionScope::Session),
        "project" => {
            let d = if action == "allow" { "allow" } else { "deny" };
            policy.add_policy_rule(&tool_name, d)?;
        }
        other => return Err(ReasonanceError::validation("scope", format!("must be 'once', 'session', or 'project', got '{}'", other))),
    }
    Ok(Value::Null)
}
"lookup_permission_decision" => {
    let session_id: String = extract(&args, "sessionId")?;
    let tool_name: String = extract(&args, "toolName")?;
    let memory = app.state::<crate::permission_engine::PermissionMemory>();
    let result = memory.lookup(&session_id, &tool_name);
    Ok(serde_json::to_value(result).unwrap())
}
"clear_permission_session" => {
    let session_id: String = extract(&args, "sessionId")?;
    let memory = app.state::<crate::permission_engine::PermissionMemory>();
    memory.clear_session(&session_id);
    Ok(Value::Null)
}
```

- [ ] **Step 4: Add Zod schemas**

In `src/lib/adapter/batch-schemas.ts`, add:

```typescript
  record_permission_decision: z.null(),
  lookup_permission_decision: z.union([
    z.literal('Allow'),
    z.object({ Deny: z.object({ reason: z.string() }) }),
    z.literal('Confirm'),
  ]).nullable(),
  clear_permission_session: z.null(),
```

- [ ] **Step 5: Update mock adapter**

In `tests/mocks/adapter.ts`, add stubs:

```typescript
  async recordPermissionDecision() {},
  async lookupPermissionDecision() { return null; },
  async clearPermissionSession() {},
```

- [ ] **Step 6: Verify**

Run: `cd src-tauri && cargo check && npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5`
Expected: clean

- [ ] **Step 7: Commit**

```bash
git add src/lib/adapter/index.ts src/lib/adapter/tauri.ts src/lib/adapter/batch-schemas.ts src-tauri/src/commands/batch.rs tests/mocks/adapter.ts
git commit -m "feat(permissions): add permission adapter methods with batch dispatch and Zod schemas"
```

---

## Task 6: Redesign PermissionRequestBlock for per-tool approval

**Files:**
- Modify: `src/lib/components/chat/PermissionRequestBlock.svelte`
- Modify: `src/lib/components/chat/ChatMessages.svelte`

- [ ] **Step 1: Redesign PermissionRequestBlock**

Replace the entire `PermissionRequestBlock.svelte` with per-tool approval UI. Each denied tool gets its own row with 4 action buttons:

```svelte
<script lang="ts">
  import type { Adapter } from '$lib/adapter/index';

  let { denials, sessionId, adapter, onAllDecided }: {
    denials: Array<{ tool_name?: string; name?: string; args?: unknown }>;
    sessionId: string;
    adapter: Adapter;
    onAllDecided: () => void;
  } = $props();

  let decisions = $state<Map<string, string>>(new Map());

  let toolEntries = $derived.by(() => {
    if (!Array.isArray(denials)) return [];
    return denials.map((d) => ({
      name: d.tool_name ?? d.name ?? 'unknown',
      args: d.args ? JSON.stringify(d.args, null, 0) : '',
    }));
  });

  let allDecided = $derived(toolEntries.length > 0 && decisions.size >= toolEntries.length);

  $effect(() => {
    if (allDecided) onAllDecided();
  });

  async function handleAction(toolName: string, action: string, scope: string) {
    decisions.set(toolName, `${action}:${scope}`);
    decisions = new Map(decisions);
    await adapter.recordPermissionDecision(sessionId, toolName, action, scope);
  }
</script>

<div class="permission-request" role="alert">
  <div class="header">PERMISSION REQUIRED</div>

  {#each toolEntries as tool}
    {@const decided = decisions.has(tool.name)}
    <div class="tool-row" class:decided>
      <div class="tool-info">
        <span class="tool-name">{tool.name}</span>
        {#if tool.args}
          <span class="tool-args">{tool.args}</span>
        {/if}
      </div>
      {#if !decided}
        <div class="actions">
          <button class="btn allow" onclick={() => handleAction(tool.name, 'allow', 'once')}>Allow once</button>
          <button class="btn allow-session" onclick={() => handleAction(tool.name, 'allow', 'session')}>Allow session</button>
          <button class="btn allow-project" onclick={() => handleAction(tool.name, 'allow', 'project')}>Allow project</button>
          <button class="btn deny" onclick={() => handleAction(tool.name, 'deny', 'once')}>Deny</button>
        </div>
      {:else}
        <span class="decided-label">{decisions.get(tool.name)?.startsWith('allow') ? 'Allowed' : 'Denied'}</span>
      {/if}
    </div>
  {/each}
</div>

<style>
  .permission-request {
    border: 2px solid var(--warning);
    background: var(--bg-secondary);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .header {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--warning);
  }

  .tool-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2);
    border: 1px solid var(--border);
    background: var(--bg-primary);
  }

  .tool-row.decided {
    opacity: 0.6;
  }

  .tool-info {
    display: flex;
    gap: var(--space-2);
    align-items: baseline;
  }

  .tool-name {
    font-family: var(--font-mono);
    font-size: var(--font-size-small);
    font-weight: 700;
    color: var(--text-primary);
  }

  .tool-args {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 300px;
  }

  .actions {
    display: flex;
    gap: var(--space-1);
    flex-wrap: wrap;
  }

  .btn {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: var(--stack-tight) var(--space-2);
    border: var(--border-width) solid var(--border);
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .btn.allow {
    background: var(--accent-btn);
    color: var(--text-on-accent);
    border-color: var(--accent);
  }
  .btn.allow:hover { opacity: 0.85; }

  .btn.allow-session, .btn.allow-project {
    background: transparent;
    color: var(--accent-text);
    border-color: var(--accent);
  }
  .btn.allow-session:hover, .btn.allow-project:hover {
    background: var(--bg-hover);
  }

  .btn.deny {
    background: transparent;
    color: var(--text-muted);
  }
  .btn.deny:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .decided-label {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    color: var(--text-muted);
  }
</style>
```

- [ ] **Step 2: Update ChatMessages to pass adapter and sessionId**

In `src/lib/components/chat/ChatMessages.svelte`, update the component props to include `adapter` and `sessionId`:

Add `sessionId` to the props interface:

```svelte
let { events = [], streaming = false, adapter, onFork, permissionLevel = 'ask', onApproveTools, sessionId = '' }: {
    events: AgentEvent[];
    streaming: boolean;
    adapter?: Adapter;
    onFork?: (eventIndex: number) => void;
    permissionLevel?: 'yolo' | 'ask' | 'locked';
    onApproveTools?: (tools: string[], remember: boolean) => void;
    sessionId?: string;
} = $props();
```

Update the PermissionRequestBlock rendering to pass adapter and sessionId:

```svelte
{:else if event.event_type === 'permission_denial'}
  {#if permissionLevel === 'ask' && adapter}
    <PermissionRequestBlock
      denials={event.content.type === 'json' ? event.content.value : []}
      {sessionId}
      {adapter}
      onAllDecided={() => {
        if (onApproveTools) {
          const tools = (event.content.type === 'json' && Array.isArray(event.content.value))
            ? event.content.value.map((d: { tool_name?: string; name?: string }) => d.tool_name ?? d.name ?? '')
            : [];
          onApproveTools(tools, false);
        }
      }}
    />
  {:else}
    <PermissionDenialBlock
      denials={event.content.type === 'json' ? event.content.value : []}
      locked={permissionLevel === 'locked'}
    />
  {/if}
```

- [ ] **Step 3: Pass sessionId from ChatView to ChatMessages**

In `src/lib/components/chat/ChatView.svelte`, add `sessionId` to the ChatMessages invocation:

```svelte
<ChatMessages {events} {streaming} {adapter} onFork={handleFork} {permissionLevel} onApproveTools={handleApproveTools} {sessionId} />
```

- [ ] **Step 4: Verify**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5`
Expected: clean

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/chat/PermissionRequestBlock.svelte src/lib/components/chat/ChatMessages.svelte src/lib/components/chat/ChatView.svelte
git commit -m "feat(permissions): redesign PermissionRequestBlock for per-tool approval with 4 scopes"
```

---

## Task 7: Remove sessionApprovedTools from ChatView

**Files:**
- Modify: `src/lib/components/chat/ChatView.svelte`

- [ ] **Step 1: Remove sessionApprovedTools state and related code**

In `src/lib/components/chat/ChatView.svelte`:

1. Remove the `sessionApprovedTools` state declaration (line 25)
2. In `handleSend()`, remove the `mergedTools` construction that uses `sessionApprovedTools`. The allowed tools now come from `configAllowedTools` only — the permission memory handles the rest backend-side.
3. Simplify `handleApproveTools()`: since PermissionRequestBlock now calls `recordPermissionDecision` directly, `handleApproveTools` only needs to trigger the re-send:

```typescript
  async function handleApproveTools(tools: string[]) {
    // PermissionRequestBlock already recorded decisions via adapter.
    // Re-send the last user message — backend now knows about approved tools via PermissionMemory.
    const lastUserEvent = [...events].reverse().find(
      (e) => e.metadata.provider === 'user' && e.event_type === 'text'
    );
    if (!lastUserEvent || lastUserEvent.content.type !== 'text') return;

    try {
      setStreaming(sessionId, true);
      const cwd = get(projectRoot) || undefined;
      const isYolo = permissionLevel === 'yolo';
      const tools = configAllowedTools.length > 0 ? configAllowedTools : undefined;
      await adapter.agentSend(lastUserEvent.content.value, provider, model, sessionId, cwd, isYolo, tools);
    } catch (e) {
      console.error('Replay failed:', e);
      setStreaming(sessionId, false);
    }
  }
```

4. Update `handleSend()` to not reference `sessionApprovedTools`:

```typescript
  async function handleSend(text: string) {
    // ... user event creation stays the same ...
    const cwd = get(projectRoot) || undefined;
    const isYolo = permissionLevel === 'yolo';
    const tools = configAllowedTools.length > 0 ? configAllowedTools : undefined;
    await adapter.agentSend(text, provider, model, sessionId, cwd, isYolo, tools);
  }
```

5. Update the `onApproveTools` callback passed to ChatMessages — it now takes only `tools: string[]`, not `(tools, remember)`:

```svelte
<ChatMessages {events} {streaming} {adapter} onFork={handleFork} {permissionLevel} onApproveTools={handleApproveTools} {sessionId} />
```

- [ ] **Step 2: Update ChatMessages onApproveTools type**

In `ChatMessages.svelte`, change the `onApproveTools` prop type:

```typescript
onApproveTools?: (tools: string[]) => void;
```

- [ ] **Step 3: Verify**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5`
Expected: clean

Run: `npx vitest run`
Expected: all pass

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/chat/ChatView.svelte src/lib/components/chat/ChatMessages.svelte
git commit -m "refactor(permissions): remove sessionApprovedTools, use PermissionMemory via adapter"
```

---

## Task 8: Full validation

- [ ] **Step 1: Rust tests**

Run: `cd src-tauri && cargo test`
Expected: all pass

- [ ] **Step 2: Clippy**

Run: `cd src-tauri && cargo clippy -- -D warnings`
Expected: clean

- [ ] **Step 3: Frontend tests**

Run: `npx svelte-kit sync && npx vitest run`
Expected: all pass

- [ ] **Step 4: Svelte check**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors

- [ ] **Step 5: Build**

Run: `npx vite build`
Expected: success

- [ ] **Step 6: Benchmark evaluate() < 1ms**

Add a Criterion benchmark for `evaluate()` in `src-tauri/benches/` or use a simple timing test:

```rust
#[test]
fn test_evaluate_performance() {
    let policy = crate::policy_file::PolicyFile::new();
    let memory = PermissionMemory::new();
    let ctx = PermissionContext {
        tool_name: "Bash".to_string(),
        tool_args: Some(serde_json::json!({"command": "ls -la"})),
        provider: "claude".to_string(),
        permission_level: "ask".to_string(),
        trust_level: "trusted".to_string(),
        project_root: Some("/project".to_string()),
    };

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = PermissionEngine::evaluate_with_session(&ctx, &memory, &policy, "sess1");
    }
    let elapsed = start.elapsed();
    let per_call = elapsed / 1000;
    assert!(per_call.as_micros() < 1000, "evaluate() took {}us, expected < 1000us", per_call.as_micros());
}
```

- [ ] **Step 7: Final commit if any changes**

```bash
git status
# If changes remain: stage and commit
```
