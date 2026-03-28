use log::warn;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ── Public types ─────────────────────────────────────────────────────────────

/// The outcome of evaluating a tool invocation against the policy rules.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
    Confirm,
}

// ── TOML schema ──────────────────────────────────────────────────────────────

/// Root of the `permissions.toml` file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PolicyToml {
    #[serde(default)]
    tools: HashMap<String, ToolPolicyToml>,
}

/// Per-tool entry in the TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolPolicyToml {
    decision: String,
    #[serde(default)]
    patterns_deny: Vec<String>,
    #[serde(default)]
    patterns_allow: Vec<String>,
}

// ── Compiled rule ────────────────────────────────────────────────────────────

/// A single tool's compiled policy: base decision + pre-compiled regexes.
#[derive(Debug, Clone)]
struct CompiledToolPolicy {
    decision: PolicyDecision,
    patterns_deny: Vec<Regex>,
    patterns_allow: Vec<Regex>,
}

// ── Inner state (behind Mutex) ───────────────────────────────────────────────

#[derive(Debug)]
struct PolicyFileInner {
    /// Merged rules: project overrides global.
    rules: HashMap<String, CompiledToolPolicy>,
    project_path: Option<PathBuf>,
    global_path: Option<PathBuf>,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Thread-safe policy file manager.
///
/// Parses `.reasonance/permissions.toml` at project and global level, compiles
/// regex patterns once, and caches everything behind a `Mutex`.
pub struct PolicyFile {
    inner: Mutex<PolicyFileInner>,
}

impl Default for PolicyFile {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyFile {
    /// Create an empty PolicyFile with no rules loaded.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PolicyFileInner {
                rules: HashMap::new(),
                project_path: None,
                global_path: None,
            }),
        }
    }

    /// Load policy rules from the project and global config directories.
    ///
    /// `project_root` — e.g. `/home/user/myproject`; we read
    ///     `<project_root>/.reasonance/permissions.toml`.
    /// `global_config_dir` — e.g. `~/.config/reasonance`; we read
    ///     `<global_config_dir>/permissions.toml`.
    ///
    /// Project-level rules override global-level rules (per tool).
    pub fn load(&self, project_root: &Path, global_config_dir: &Path) {
        let project_file = project_root.join(".reasonance").join("permissions.toml");
        let global_file = global_config_dir.join("permissions.toml");

        let global_rules = Self::parse_file(&global_file);
        let project_rules = Self::parse_file(&project_file);

        // Merge: start with global, then overwrite with project.
        let mut merged = global_rules;
        merged.extend(project_rules);

        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.rules = merged;
        inner.project_path = Some(project_file);
        inner.global_path = Some(global_file);
    }

    /// Reload from the previously configured paths.
    pub fn reload(&self) {
        let (project_path, global_path) = {
            let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            (inner.project_path.clone(), inner.global_path.clone())
        };

        let global_rules = global_path
            .as_deref()
            .map(Self::parse_file)
            .unwrap_or_default();
        let project_rules = project_path
            .as_deref()
            .map(Self::parse_file)
            .unwrap_or_default();

        let mut merged = global_rules;
        merged.extend(project_rules);

        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.rules = merged;
    }

    /// Evaluate a tool invocation against the loaded policy rules.
    ///
    /// Returns `None` if the tool has no entry — the caller should fall through
    /// to the next permission layer.
    pub fn evaluate(&self, tool_name: &str, tool_args: &str) -> Option<PolicyDecision> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let policy = inner.rules.get(tool_name)?;

        // Deny patterns checked first — deny wins over allow.
        for re in &policy.patterns_deny {
            if re.is_match(tool_args) {
                return Some(PolicyDecision::Deny {
                    reason: format!(
                        "Policy deny pattern '{}' matched for tool '{}'",
                        re.as_str(),
                        tool_name
                    ),
                });
            }
        }

        // Allow patterns checked second.
        for re in &policy.patterns_allow {
            if re.is_match(tool_args) {
                return Some(PolicyDecision::Allow);
            }
        }

        // No pattern matched — use the base decision.
        Some(policy.decision.clone())
    }

    /// Add (or overwrite) a tool rule at runtime and persist it to the project
    /// `permissions.toml` file. Creates `.reasonance/` if it does not exist.
    pub fn add_policy_rule(&self, tool_name: &str, decision: &str) -> Result<(), String> {
        let project_path = {
            let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner
                .project_path
                .clone()
                .ok_or_else(|| "No project path configured — call load() first".to_string())?
        };

        // Read existing file (or start empty).
        let mut toml_data: PolicyToml = if project_path.exists() {
            let content = std::fs::read_to_string(&project_path)
                .map_err(|e| format!("Failed to read {}: {}", project_path.display(), e))?;
            toml::from_str(&content)
                .map_err(|e| format!("Failed to parse {}: {}", project_path.display(), e))?
        } else {
            PolicyToml::default()
        };

        // Upsert the tool entry (keep existing patterns if re-setting the decision).
        let entry = toml_data
            .tools
            .entry(tool_name.to_string())
            .or_insert_with(|| ToolPolicyToml {
                decision: decision.to_string(),
                patterns_deny: Vec::new(),
                patterns_allow: Vec::new(),
            });
        entry.decision = decision.to_string();

        // Ensure parent dir exists.
        if let Some(parent) = project_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir {}: {}", parent.display(), e))?;
        }

        // Write back.
        let serialized = toml::to_string_pretty(&toml_data)
            .map_err(|e| format!("Failed to serialize TOML: {}", e))?;
        std::fs::write(&project_path, serialized)
            .map_err(|e| format!("Failed to write {}: {}", project_path.display(), e))?;

        // Reload so the in-memory cache reflects the change.
        self.reload();

        Ok(())
    }

    // ── Private helpers ──────────────────────────────────────────────────

    /// Parse a single TOML file and compile its regex patterns.
    /// Returns an empty map if the file does not exist or is invalid.
    fn parse_file(path: &Path) -> HashMap<String, CompiledToolPolicy> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return HashMap::new(),
        };

        let toml_data: PolicyToml = match toml::from_str(&content) {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to parse policy file {}: {}", path.display(), e);
                return HashMap::new();
            }
        };

        Self::compile_rules(&toml_data)
    }

    /// Convert parsed TOML into compiled rules.
    fn compile_rules(toml_data: &PolicyToml) -> HashMap<String, CompiledToolPolicy> {
        let mut rules = HashMap::new();

        for (tool_name, entry) in &toml_data.tools {
            let decision = match entry.decision.as_str() {
                "allow" => PolicyDecision::Allow,
                "deny" => PolicyDecision::Deny {
                    reason: format!("Policy file denies tool '{}'", tool_name),
                },
                "confirm" => PolicyDecision::Confirm,
                other => {
                    warn!(
                        "Unknown decision '{}' for tool '{}', defaulting to confirm",
                        other, tool_name
                    );
                    PolicyDecision::Confirm
                }
            };

            let patterns_deny = Self::compile_patterns(&entry.patterns_deny, tool_name, "deny");
            let patterns_allow = Self::compile_patterns(&entry.patterns_allow, tool_name, "allow");

            rules.insert(
                tool_name.clone(),
                CompiledToolPolicy {
                    decision,
                    patterns_deny,
                    patterns_allow,
                },
            );
        }

        rules
    }

    /// Compile a list of regex pattern strings, skipping invalid ones with a
    /// warning instead of panicking.
    fn compile_patterns(patterns: &[String], tool_name: &str, kind: &str) -> Vec<Regex> {
        patterns
            .iter()
            .filter_map(|pat| match Regex::new(pat) {
                Ok(re) => Some(re),
                Err(e) => {
                    warn!(
                        "Invalid regex in {} pattern for tool '{}': '{}' — {}",
                        kind, tool_name, pat, e
                    );
                    None
                }
            })
            .collect()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: write a permissions.toml into `<dir>/.reasonance/permissions.toml`.
    fn write_project_policy(dir: &Path, content: &str) {
        let policy_dir = dir.join(".reasonance");
        fs::create_dir_all(&policy_dir).unwrap();
        fs::write(policy_dir.join("permissions.toml"), content).unwrap();
    }

    /// Helper: write a global permissions.toml into `<dir>/permissions.toml`.
    fn write_global_policy(dir: &Path, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("permissions.toml"), content).unwrap();
    }

    // ── 1. Basic allow / deny / absent ───────────────────────────────────

    #[test]
    fn test_basic_allow_deny() {
        let project = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();

        write_project_policy(
            project.path(),
            r#"
[tools.Write]
decision = "allow"

[tools.WebSearch]
decision = "deny"
"#,
        );

        let pf = PolicyFile::new();
        pf.load(project.path(), global.path());

        // Write → Allow
        assert_eq!(pf.evaluate("Write", ""), Some(PolicyDecision::Allow));

        // WebSearch → Deny
        assert!(matches!(
            pf.evaluate("WebSearch", ""),
            Some(PolicyDecision::Deny { .. })
        ));

        // Read → not in file → None
        assert_eq!(pf.evaluate("Read", ""), None);
    }

    // ── 2. Regex deny patterns ──────────────────────────────────────────

    #[test]
    fn test_regex_deny_pattern() {
        let project = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();

        write_project_policy(
            project.path(),
            r#"
[tools.Bash]
decision = "confirm"
patterns_deny = ["^rm\\s+-rf"]
"#,
        );

        let pf = PolicyFile::new();
        pf.load(project.path(), global.path());

        // "rm -rf /tmp" matches ^rm\s+-rf → Deny
        assert!(matches!(
            pf.evaluate("Bash", "rm -rf /tmp"),
            Some(PolicyDecision::Deny { .. })
        ));

        // "echo inform user" does NOT start with rm → falls through to base decision (confirm)
        assert_eq!(
            pf.evaluate("Bash", "echo inform user"),
            Some(PolicyDecision::Confirm)
        );
    }

    // ── 3. Regex allow patterns ─────────────────────────────────────────

    #[test]
    fn test_regex_allow_pattern() {
        let project = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();

        write_project_policy(
            project.path(),
            r#"
[tools.Bash]
decision = "confirm"
patterns_allow = ["^ls\\b", "^npm\\s+test"]
"#,
        );

        let pf = PolicyFile::new();
        pf.load(project.path(), global.path());

        // "ls -la" matches ^ls\b → Allow
        assert_eq!(pf.evaluate("Bash", "ls -la"), Some(PolicyDecision::Allow));

        // "npm test" matches ^npm\s+test → Allow
        assert_eq!(pf.evaluate("Bash", "npm test"), Some(PolicyDecision::Allow));

        // "cargo build" matches nothing → base decision (confirm)
        assert_eq!(
            pf.evaluate("Bash", "cargo build"),
            Some(PolicyDecision::Confirm)
        );
    }

    // ── 4. Deny patterns take priority over allow patterns ──────────────

    #[test]
    fn test_deny_pattern_takes_priority_over_allow() {
        let project = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();

        // Both patterns match "rm -rf /tmp && ls" but deny should win.
        write_project_policy(
            project.path(),
            r#"
[tools.Bash]
decision = "allow"
patterns_deny = ["rm\\s+-rf"]
patterns_allow = [".*"]
"#,
        );

        let pf = PolicyFile::new();
        pf.load(project.path(), global.path());

        // The deny pattern fires first.
        assert!(matches!(
            pf.evaluate("Bash", "rm -rf /tmp"),
            Some(PolicyDecision::Deny { .. })
        ));
    }

    // ── 5. Absent file returns None for all tools ───────────────────────

    #[test]
    fn test_absent_file_returns_none() {
        let project = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();
        // No files written — both paths are empty directories.

        let pf = PolicyFile::new();
        pf.load(project.path(), global.path());

        assert_eq!(pf.evaluate("Write", ""), None);
        assert_eq!(pf.evaluate("Bash", "ls"), None);
        assert_eq!(pf.evaluate("Read", ""), None);
    }

    // ── 6. add_policy_rule persists and reloads ─────────────────────────

    #[test]
    fn test_add_policy_rule() {
        let project = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();

        let pf = PolicyFile::new();
        pf.load(project.path(), global.path());

        // Initially no rule for Edit.
        assert_eq!(pf.evaluate("Edit", ""), None);

        // Add a rule at runtime.
        pf.add_policy_rule("Edit", "allow").unwrap();

        // Now it should evaluate to Allow.
        assert_eq!(pf.evaluate("Edit", ""), Some(PolicyDecision::Allow));

        // The file should exist on disk.
        let policy_path = project.path().join(".reasonance").join("permissions.toml");
        assert!(policy_path.exists());

        // Verify the file content round-trips correctly.
        let content = fs::read_to_string(&policy_path).unwrap();
        assert!(content.contains("[tools.Edit]"));
        assert!(content.contains("decision = \"allow\""));
    }

    // ── 7. Project-level rules override global ──────────────────────────

    #[test]
    fn test_project_overrides_global() {
        let project = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();

        write_global_policy(
            global.path(),
            r#"
[tools.Write]
decision = "deny"

[tools.Bash]
decision = "confirm"
"#,
        );

        write_project_policy(
            project.path(),
            r#"
[tools.Write]
decision = "allow"
"#,
        );

        let pf = PolicyFile::new();
        pf.load(project.path(), global.path());

        // Project says allow → overrides global deny.
        assert_eq!(pf.evaluate("Write", ""), Some(PolicyDecision::Allow));

        // Bash only in global → still visible.
        assert_eq!(
            pf.evaluate("Bash", "cargo build"),
            Some(PolicyDecision::Confirm)
        );
    }

    // ── 8. Invalid regex is skipped, not a panic ────────────────────────

    #[test]
    fn test_invalid_regex_skipped() {
        let project = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();

        write_project_policy(
            project.path(),
            r#"
[tools.Bash]
decision = "confirm"
patterns_deny = ["(unclosed"]
patterns_allow = ["^ls\\b"]
"#,
        );

        let pf = PolicyFile::new();
        pf.load(project.path(), global.path());

        // The invalid deny pattern is skipped. Allow pattern should still work.
        assert_eq!(pf.evaluate("Bash", "ls -la"), Some(PolicyDecision::Allow));

        // Non-matching input falls through to base decision.
        assert_eq!(
            pf.evaluate("Bash", "cargo build"),
            Some(PolicyDecision::Confirm)
        );
    }

    // ── 9. Reload picks up file changes ─────────────────────────────────

    #[test]
    fn test_reload_picks_up_changes() {
        let project = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();

        write_project_policy(
            project.path(),
            r#"
[tools.Write]
decision = "deny"
"#,
        );

        let pf = PolicyFile::new();
        pf.load(project.path(), global.path());

        assert!(matches!(
            pf.evaluate("Write", ""),
            Some(PolicyDecision::Deny { .. })
        ));

        // Overwrite the file.
        write_project_policy(
            project.path(),
            r#"
[tools.Write]
decision = "allow"
"#,
        );

        pf.reload();

        assert_eq!(pf.evaluate("Write", ""), Some(PolicyDecision::Allow));
    }
}
