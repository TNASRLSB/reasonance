# Phase 6: Intelligence — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add runtime intelligence — capability negotiation, CLI version management, normalizer health testing, self-heal flow, and normalizer versioning with rollback — so REASONANCE can dynamically adapt to each provider's actual capabilities.

**Architecture:** The CapabilityNegotiator discovers what each provider actually supports by running TOML-defined test commands in parallel at startup, caching results to disk. The CLI Updater checks/updates provider binaries in the background. Normalizer Health validates that normalizers correctly parse provider output via test suites defined in TOML. When health degrades after a CLI update, the Self-Heal flow uses an LLM to iteratively fix the TOML normalizer. All normalizer changes are versioned with rollback support.

**Tech Stack:** Rust (Tauri 2 backend), TOML normalizer configs, serde/tokio for async, filesystem caching (`~/.reasonance/`), Svelte 5 frontend for status display

**Spec reference:** `.claude/docs/specs/2026-03-22-structured-agent-transport-design.md` sections 6 (Normalizer Health, CLI Updater, Self-Heal) and 7 (Capability Negotiation)

---

## File Structure

### New Files (Rust backend)

| File | Responsibility |
|------|---------------|
| `src-tauri/src/capability.rs` | `CapabilityNegotiator`, `NegotiatedCapabilities`, `FeatureSupport`, `Workaround` types + parallel negotiation logic |
| `src-tauri/src/cli_updater.rs` | `CliUpdater`, `CliVersionInfo` — background version check/update per provider |
| `src-tauri/src/normalizer_health.rs` | `NormalizerHealth`, `HealthStatus`, `HealthReport` — runs TOML test suites against live CLIs |
| `src-tauri/src/self_heal.rs` | `SelfHealConfig`, `SelfHealResult` — iterative LLM-driven normalizer repair |
| `src-tauri/src/normalizer_version.rs` | `NormalizerVersionStore` — version tracking, backup, rollback for TOML files |
| `src-tauri/src/commands/capability.rs` | Tauri commands: `get_capabilities`, `get_provider_capabilities`, `get_cli_versions`, `get_normalizer_versions`, `rollback_normalizer`, `get_health_report`, `get_all_health_reports` |

### New Files (Frontend)

| File | Responsibility |
|------|---------------|
| `src/lib/types/capability.ts` | TypeScript mirrors: `NegotiatedCapabilities`, `FeatureSupport`, `HealthReport`, `HealthStatus`, `CliVersionInfo` |
| `src/lib/stores/capabilities.ts` | Writable store for negotiated capabilities per provider |

### Modified Files

| File | Changes |
|------|---------|
| `src-tauri/src/lib.rs` | Register new managed state (`CapabilityNegotiator`, `CliUpdater`, `NormalizerHealth`, `NormalizerVersionStore`) + new commands + setup hook for startup negotiation; add `mod` declarations for all new modules |
| `src-tauri/src/normalizer/mod.rs` | Add `Clone` derives to all config structs, add `toml_sources` field, `get_toml_source()` and `reload_provider()` methods |
| `src-tauri/src/commands/mod.rs` | Add `pub mod capability;` |
| `src-tauri/src/normalizer/mod.rs` | Add `pub fn get_toml_source(&self, provider: &str) -> Option<String>` to `NormalizerRegistry` for self-heal to read current TOML; add `pub fn reload_provider(&mut self, provider: &str, toml_str: &str) -> Result<(), String>` to hot-reload a normalizer |
| `src/lib/adapter/index.ts` | Add 7 new Adapter methods for capability/health/version commands |
| `src/lib/adapter/tauri.ts` | Implement the 7 new methods via `invoke()` |
| `tests/mocks/adapter.ts` | Stub implementations for the 7 new methods |

**Note:** The file structure table for `src-tauri/src/normalizer/mod.rs` is listed under Modified Files above. Changes in Tasks 2 and 7 both touch this file.

---

## Task Dependency Graph

```
Task 1: NormalizerVersionStore (no deps)
Task 2: NormalizerRegistry extensions (no deps)
Task 3: NormalizerHealth (depends on Task 2)
Task 4: CliUpdater (no deps)
Task 5: CapabilityNegotiator (depends on Task 3)
Task 6: SelfHeal (depends on Tasks 2, 3)
Task 7: Tauri commands + wiring (depends on Tasks 1-6)
Task 8: Frontend types, adapter, store (depends on Task 7)
```

Tasks 1, 2, 4 can run in parallel. Task 3 after 2. Tasks 5, 6 after 3. Task 7 after all backend. Task 8 after 7.

---

### Task 1: Normalizer Version Store

**Files:**
- Create: `src-tauri/src/normalizer_version.rs`
- Modify: `src-tauri/src/lib.rs:1` (add `mod normalizer_version;`)

**Context:** Normalizer TOML files live in `src-tauri/normalizers/`. This module tracks versions and allows rollback. Versions are stored in `~/.reasonance/normalizer-versions/` as timestamped backups.

- [ ] **Step 1: Write the failing tests**

In `src-tauri/src/normalizer_version.rs`, add a `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_toml() -> &'static str {
        r#"[cli]
name = "testprovider"
binary = "test"
programmatic_args = ["-p", "{prompt}"]

[[rules]]
name = "text"
when = 'type == "text"'
emit = "text"
"#
    }

    #[test]
    fn test_version_store_creation() {
        let dir = TempDir::new().unwrap();
        let store = NormalizerVersionStore::new(dir.path());
        assert!(store.list_versions("testprovider").is_empty());
    }

    #[test]
    fn test_backup_and_list() {
        let dir = TempDir::new().unwrap();
        let store = NormalizerVersionStore::new(dir.path());
        let version_id = store.backup("testprovider", sample_toml()).unwrap();
        assert!(!version_id.is_empty());
        let versions = store.list_versions("testprovider");
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].id, version_id);
    }

    #[test]
    fn test_restore_version() {
        let dir = TempDir::new().unwrap();
        let store = NormalizerVersionStore::new(dir.path());
        let v1 = store.backup("testprovider", sample_toml()).unwrap();
        let modified = sample_toml().replace("testprovider", "modified");
        let _v2 = store.backup("testprovider", &modified).unwrap();

        let restored = store.restore("testprovider", &v1).unwrap();
        assert!(restored.contains("testprovider"));
        assert!(!restored.contains("modified"));
    }

    #[test]
    fn test_restore_nonexistent_fails() {
        let dir = TempDir::new().unwrap();
        let store = NormalizerVersionStore::new(dir.path());
        let result = store.restore("testprovider", "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_current_returns_latest() {
        let dir = TempDir::new().unwrap();
        let store = NormalizerVersionStore::new(dir.path());
        let _v1 = store.backup("testprovider", "v1 content").unwrap();
        let v2 = store.backup("testprovider", "v2 content").unwrap();
        let current = store.current("testprovider").unwrap();
        assert_eq!(current.id, v2);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test normalizer_version --lib 2>&1 | head -30`
Expected: FAIL — `NormalizerVersionStore` not defined

- [ ] **Step 3: Write minimal implementation**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub id: String,
    pub provider: String,
    pub timestamp: u64,
    pub checksum: String,
}

pub struct NormalizerVersionStore {
    base_dir: PathBuf,
    index: Mutex<HashMap<String, Vec<VersionEntry>>>,
}

impl NormalizerVersionStore {
    pub fn new(base_dir: &Path) -> Self {
        let _ = std::fs::create_dir_all(base_dir);
        let index = Self::load_index(base_dir);
        Self {
            base_dir: base_dir.to_path_buf(),
            index: Mutex::new(index),
        }
    }

    pub fn backup(&self, provider: &str, toml_content: &str) -> Result<String, String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let checksum = format!("{:x}", md5_hash(toml_content));
        let id = format!("{}-{}", timestamp, &checksum[..8]);

        let provider_dir = self.base_dir.join(provider);
        std::fs::create_dir_all(&provider_dir).map_err(|e| e.to_string())?;

        let file_path = provider_dir.join(format!("{}.toml", id));
        std::fs::write(&file_path, toml_content).map_err(|e| e.to_string())?;

        let entry = VersionEntry {
            id: id.clone(),
            provider: provider.to_string(),
            timestamp,
            checksum,
        };

        let mut index = self.index.lock().unwrap();
        index.entry(provider.to_string()).or_default().push(entry);
        self.save_index(&index)?;

        Ok(id)
    }

    pub fn restore(&self, provider: &str, version_id: &str) -> Result<String, String> {
        let file_path = self.base_dir.join(provider).join(format!("{}.toml", version_id));
        std::fs::read_to_string(&file_path)
            .map_err(|_| format!("Version {} not found for {}", version_id, provider))
    }

    pub fn list_versions(&self, provider: &str) -> Vec<VersionEntry> {
        let index = self.index.lock().unwrap();
        index.get(provider).cloned().unwrap_or_default()
    }

    pub fn current(&self, provider: &str) -> Option<VersionEntry> {
        let index = self.index.lock().unwrap();
        index.get(provider).and_then(|v| v.last().cloned())
    }

    fn load_index(base_dir: &Path) -> HashMap<String, Vec<VersionEntry>> {
        let index_path = base_dir.join("index.json");
        if let Ok(content) = std::fs::read_to_string(&index_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        }
    }

    fn save_index(&self, index: &HashMap<String, Vec<VersionEntry>>) -> Result<(), String> {
        let index_path = self.base_dir.join("index.json");
        let json = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
        std::fs::write(&index_path, json).map_err(|e| e.to_string())
    }
}

/// Simple hash for checksums (not cryptographic — just for dedup)
fn md5_hash(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}
```

- [ ] **Step 4: Add module declaration to lib.rs**

In `src-tauri/src/lib.rs`, add after `mod workflow_engine;`:

```rust
mod normalizer_version;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test normalizer_version --lib`
Expected: 5 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/normalizer_version.rs src-tauri/src/lib.rs
git commit -m "feat: add NormalizerVersionStore — backup, restore, rollback for TOML normalizers"
```

---

### Task 2: NormalizerRegistry Extensions

**Files:**
- Modify: `src-tauri/src/normalizer/mod.rs:119-178` (NormalizerRegistry impl)

**Context:** Self-heal and health check need to read the raw TOML source for a provider and hot-reload a modified normalizer without restarting. Add two methods to `NormalizerRegistry`. Also add `Clone` derives to all TOML config structs (needed by Task 7 setup hook).

- [ ] **Step 1: Write the failing tests**

Add to the existing `tests` module in `src-tauri/src/normalizer/mod.rs`:

```rust
    #[test]
    fn test_get_toml_source() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("test.toml");
        fs::write(&toml_path, sample_toml()).unwrap();

        let registry = NormalizerRegistry::load_from_dir(dir.path()).unwrap();
        let source = registry.get_toml_source("testprovider");
        assert!(source.is_some());
        assert!(source.unwrap().contains("testprovider"));
    }

    #[test]
    fn test_get_toml_source_unknown() {
        let dir = TempDir::new().unwrap();
        let registry = NormalizerRegistry::load_from_dir(dir.path()).unwrap();
        assert!(registry.get_toml_source("unknown").is_none());
    }

    #[test]
    fn test_reload_provider() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("test.toml");
        fs::write(&toml_path, sample_toml()).unwrap();

        let mut registry = NormalizerRegistry::load_from_dir(dir.path()).unwrap();
        assert!(registry.has_provider("testprovider"));

        // Reload with modified TOML (different provider name won't change key — uses original key)
        let modified_toml = sample_toml().replace(r#"when = 'type == "text_delta"'"#, r#"when = 'type == "content"'"#);
        let result = registry.reload_provider("testprovider", &modified_toml);
        assert!(result.is_ok());
        assert!(registry.has_provider("testprovider"));
    }

    #[test]
    fn test_reload_provider_invalid_toml() {
        let dir = TempDir::new().unwrap();
        let toml_path = dir.path().join("test.toml");
        fs::write(&toml_path, sample_toml()).unwrap();

        let mut registry = NormalizerRegistry::load_from_dir(dir.path()).unwrap();
        let result = registry.reload_provider("testprovider", "invalid { toml");
        assert!(result.is_err());
        // Original provider should still work
        assert!(registry.has_provider("testprovider"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test normalizer::tests --lib 2>&1 | head -30`
Expected: FAIL — `get_toml_source` and `reload_provider` not defined

- [ ] **Step 3: Write minimal implementation**

First, add `Clone` to all config struct derives (needed by Task 7 setup hook). In `src-tauri/src/normalizer/mod.rs`, change every `#[derive(Debug, Deserialize)]` to `#[derive(Debug, Clone, Deserialize)]` for: `TomlConfig`, `CliConfig`, `TomlRule`, `ContextConfig`, `RetryConfig`, `SessionConfig`, `CommandsConfig`, `TomlTest`, `DirectApiConfig`.

Then add a new field `toml_sources` to `NormalizerRegistry` and the two methods. In `src-tauri/src/normalizer/mod.rs`:

Change the struct definition:

```rust
pub struct NormalizerRegistry {
    pipelines: HashMap<String, NormalizerPipeline>,
    configs: HashMap<String, TomlConfig>,
    toml_sources: HashMap<String, String>,
}
```

Update `load_from_dir` to also store the raw TOML source:

```rust
    pub fn load_from_dir(dir: &Path) -> Result<Self, String> {
        let mut pipelines = HashMap::new();
        let mut configs = HashMap::new();
        let mut toml_sources = HashMap::new();

        if !dir.exists() {
            return Ok(Self { pipelines, configs, toml_sources });
        }

        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let config = TomlConfig::parse(&content)?;
                let name = config.cli.name.clone();

                let state_machine: Box<dyn StateMachine> = match name.as_str() {
                    "claude" => Box::new(ClaudeStateMachine::new()),
                    _ => Box::new(GenericStateMachine::new()),
                };

                let pipeline = NormalizerPipeline::new(
                    config.to_rules(),
                    state_machine,
                    name.clone(),
                );

                pipelines.insert(name.clone(), pipeline);
                toml_sources.insert(name.clone(), content);
                configs.insert(name, config);
            }
        }

        Ok(Self { pipelines, configs, toml_sources })
    }
```

Add the two new methods to `impl NormalizerRegistry`:

```rust
    pub fn get_toml_source(&self, provider: &str) -> Option<String> {
        self.toml_sources.get(provider).cloned()
    }

    pub fn reload_provider(&mut self, provider: &str, toml_str: &str) -> Result<(), String> {
        let config = TomlConfig::parse(toml_str)?;

        let state_machine: Box<dyn StateMachine> = match provider {
            "claude" => Box::new(ClaudeStateMachine::new()),
            _ => Box::new(GenericStateMachine::new()),
        };

        let pipeline = NormalizerPipeline::new(
            config.to_rules(),
            state_machine,
            provider.to_string(),
        );

        self.pipelines.insert(provider.to_string(), pipeline);
        self.configs.insert(provider.to_string(), config);
        self.toml_sources.insert(provider.to_string(), toml_str.to_string());
        Ok(())
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test normalizer::tests --lib`
Expected: All tests PASS (existing + 4 new)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/normalizer/mod.rs
git commit -m "feat: add get_toml_source and reload_provider to NormalizerRegistry"
```

---

### Task 3: Normalizer Health

**Files:**
- Create: `src-tauri/src/normalizer_health.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod normalizer_health;`)

**Context:** Runs TOML-defined `[[tests]]` against real CLI output to validate normalizer correctness. Each test spawns the CLI with a short prompt, collects events, and checks `expected` assertions. Results are `Healthy`, `Degraded`, or `Broken`. This module does NOT call CLIs itself — it delegates to `StructuredAgentTransport::send()` internally. However, for unit testability, we define the health logic to work on pre-collected `AgentEvent` vectors.

**Important:** The `[[tests]]` section already exists in `claude.toml` (lines 97-112). The `TomlTest` struct already exists in `normalizer/mod.rs:87-93`. This task implements the runner and evaluator.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::{AgentEvent, AgentEventType, EventContent, AgentEventMetadata};

    fn test_metadata() -> AgentEventMetadata {
        AgentEventMetadata {
            session_id: Some("test".to_string()),
            input_tokens: None,
            output_tokens: None,
            tool_name: None,
            model: None,
            provider: "test".to_string(),
            error_severity: None,
            error_code: None,
            stream_metrics: None,
        }
    }

    fn make_text_event(text: &str) -> AgentEvent {
        AgentEvent {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::Text,
            content: EventContent::Text { value: text.to_string() },
            timestamp: 0,
            metadata: test_metadata(),
        }
    }

    fn make_done_event() -> AgentEvent {
        AgentEvent {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::Done,
            content: EventContent::Text { value: String::new() },
            timestamp: 0,
            metadata: test_metadata(),
        }
    }

    #[test]
    fn test_evaluate_basic_text_pass() {
        let test_case = TestCase {
            name: "basic_text".to_string(),
            expected: vec![
                ExpectedEvent {
                    event_type: "text".to_string(),
                    required: true,
                    validate: Validation::ContentMatches("REASONANCE_TEST_OK".to_string()),
                },
                ExpectedEvent {
                    event_type: "done".to_string(),
                    required: true,
                    validate: Validation::Exists,
                },
            ],
        };

        let events = vec![
            make_text_event("REASONANCE_TEST_OK"),
            make_done_event(),
        ];

        let result = evaluate_test_case(&test_case, &events);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_missing_required_event() {
        let test_case = TestCase {
            name: "basic_text".to_string(),
            expected: vec![
                ExpectedEvent {
                    event_type: "text".to_string(),
                    required: true,
                    validate: Validation::ContentMatches("REASONANCE_TEST_OK".to_string()),
                },
                ExpectedEvent {
                    event_type: "done".to_string(),
                    required: true,
                    validate: Validation::Exists,
                },
            ],
        };

        // Only text, no done event
        let events = vec![make_text_event("REASONANCE_TEST_OK")];

        let result = evaluate_test_case(&test_case, &events);
        assert!(!result.passed);
        assert!(result.failure_reason.is_some());
    }

    #[test]
    fn test_evaluate_content_mismatch() {
        let test_case = TestCase {
            name: "basic_text".to_string(),
            expected: vec![
                ExpectedEvent {
                    event_type: "text".to_string(),
                    required: true,
                    validate: Validation::ContentMatches("REASONANCE_TEST_OK".to_string()),
                },
            ],
        };

        let events = vec![make_text_event("wrong output")];

        let result = evaluate_test_case(&test_case, &events);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_optional_missing_still_passes() {
        let test_case = TestCase {
            name: "thinking".to_string(),
            expected: vec![
                ExpectedEvent {
                    event_type: "thinking".to_string(),
                    required: false,
                    validate: Validation::ContentNotEmpty,
                },
            ],
        };

        // No thinking events — but it's optional
        let events = vec![make_text_event("answer")];

        let result = evaluate_test_case(&test_case, &events);
        assert!(result.passed);
    }

    #[test]
    fn test_health_status_from_results() {
        let all_pass = vec![
            TestCaseResult { name: "basic_text".to_string(), passed: true, failure_reason: None },
            TestCaseResult { name: "thinking".to_string(), passed: true, failure_reason: None },
        ];
        assert!(matches!(health_status_from_results(&all_pass), HealthStatus::Healthy));

        let some_fail = vec![
            TestCaseResult { name: "basic_text".to_string(), passed: true, failure_reason: None },
            TestCaseResult { name: "thinking".to_string(), passed: false, failure_reason: Some("missing".into()) },
        ];
        assert!(matches!(health_status_from_results(&some_fail), HealthStatus::Degraded { .. }));

        let all_fail = vec![
            TestCaseResult { name: "basic_text".to_string(), passed: false, failure_reason: Some("no text".into()) },
        ];
        assert!(matches!(health_status_from_results(&all_fail), HealthStatus::Broken { .. }));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test normalizer_health --lib 2>&1 | head -30`
Expected: FAIL — module not found

- [ ] **Step 3: Write minimal implementation**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub expected: Vec<ExpectedEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedEvent {
    pub event_type: String,
    pub required: bool,
    pub validate: Validation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Validation {
    Exists,
    ContentNotEmpty,
    ContentMatches(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    pub name: String,
    pub passed: bool,
    pub failure_reason: Option<String>,
}

// Note: Spec uses `missing: Vec<String>` for Degraded; we use `failing_tests`
// which is more descriptive. This is a deliberate deviation from spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded { failing_tests: Vec<String> },
    Broken { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub provider: String,
    pub status: HealthStatus,
    pub results: Vec<TestCaseResult>,
    pub capabilities_confirmed: Vec<String>,
    pub capabilities_missing: Vec<String>,
    pub capabilities_broken: Vec<String>,
    pub tested_at: u64,
    pub cli_version: String,
}

/// Container for health reports per provider. Registered as Tauri managed state.
pub struct NormalizerHealth {
    reports: Mutex<HashMap<String, HealthReport>>,
}

impl NormalizerHealth {
    pub fn new() -> Self {
        Self {
            reports: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_report(&self, provider: &str, report: HealthReport) {
        self.reports.lock().unwrap().insert(provider.to_string(), report);
    }

    pub fn get_report(&self, provider: &str) -> Option<HealthReport> {
        self.reports.lock().unwrap().get(provider).cloned()
    }

    pub fn all_reports(&self) -> HashMap<String, HealthReport> {
        self.reports.lock().unwrap().clone()
    }
}

use crate::agent_event::{AgentEvent, AgentEventType, EventContent};

pub fn evaluate_test_case(test_case: &TestCase, events: &[AgentEvent]) -> TestCaseResult {
    for expected in &test_case.expected {
        let matching_events: Vec<&AgentEvent> = events
            .iter()
            .filter(|e| event_type_matches(&e.event_type, &expected.event_type))
            .collect();

        if matching_events.is_empty() {
            if expected.required {
                return TestCaseResult {
                    name: test_case.name.clone(),
                    passed: false,
                    failure_reason: Some(format!(
                        "Required event '{}' not found",
                        expected.event_type
                    )),
                };
            }
            continue; // optional, skip
        }

        // Validate the matching events
        let validation_passed = matching_events.iter().any(|e| validate_event(e, &expected.validate));
        if !validation_passed && expected.required {
            return TestCaseResult {
                name: test_case.name.clone(),
                passed: false,
                failure_reason: Some(format!(
                    "Event '{}' found but validation '{}' failed",
                    expected.event_type,
                    validation_label(&expected.validate),
                )),
            };
        }
    }

    TestCaseResult {
        name: test_case.name.clone(),
        passed: true,
        failure_reason: None,
    }
}

pub fn health_status_from_results(results: &[TestCaseResult]) -> HealthStatus {
    let failing: Vec<String> = results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| r.name.clone())
        .collect();

    if failing.is_empty() {
        HealthStatus::Healthy
    } else if failing.len() == results.len() {
        HealthStatus::Broken {
            error: format!("All {} tests failed", results.len()),
        }
    } else {
        HealthStatus::Degraded {
            failing_tests: failing,
        }
    }
}

fn event_type_matches(actual: &AgentEventType, expected: &str) -> bool {
    match (actual, expected) {
        (AgentEventType::Text, "text") => true,
        (AgentEventType::Thinking, "thinking") => true,
        (AgentEventType::ToolUse, "tool_use") => true,
        (AgentEventType::ToolResult, "tool_result") => true,
        (AgentEventType::Error, "error") => true,
        (AgentEventType::Usage, "usage") => true,
        (AgentEventType::Done, "done") => true,
        (AgentEventType::Status, "status") => true,
        (AgentEventType::Metrics, "metrics") => true,
        _ => false,
    }
}

fn validate_event(event: &AgentEvent, validation: &Validation) -> bool {
    match validation {
        Validation::Exists => true,
        Validation::ContentNotEmpty => {
            match &event.content {
                EventContent::Text { value } => !value.is_empty(),
                EventContent::Code { source, .. } => !source.is_empty(),
                _ => true,
            }
        }
        Validation::ContentMatches(pattern) => {
            match &event.content {
                EventContent::Text { value } => value.contains(pattern),
                EventContent::Code { source, .. } => source.contains(pattern),
                _ => false,
            }
        }
    }
}

fn validation_label(v: &Validation) -> &str {
    match v {
        Validation::Exists => "exists",
        Validation::ContentNotEmpty => "content_not_empty",
        Validation::ContentMatches(_) => "content_matches",
    }
}
```

- [ ] **Step 4: Add module declaration to lib.rs**

In `src-tauri/src/lib.rs`, add after `mod normalizer_version;`:

```rust
mod normalizer_health;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test normalizer_health --lib`
Expected: 5 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/normalizer_health.rs src-tauri/src/lib.rs
git commit -m "feat: add NormalizerHealth — test case evaluator and health status derivation"
```

---

### Task 4: CLI Updater

**Files:**
- Create: `src-tauri/src/cli_updater.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod cli_updater;`)

**Context:** Checks CLI versions for installed providers. Runs `version_command` from TOML config to detect current version. Runs `update_command` if `auto_update` is true. This task implements the data structures and version-check logic; actual CLI spawning is async and tested in integration tests only. Unit tests cover the struct/parsing logic.

- [ ] **Step 1: Write the failing tests**

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test cli_updater --lib 2>&1 | head -30`
Expected: FAIL — module not found

- [ ] **Step 3: Write minimal implementation**

```rust
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
                None => true, // no version known → counts as changed
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

    /// Populate from TOML configs. Called during app startup.
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
                    auto_update: true, // default on
                    version_command: config.cli.version_command.clone(),
                    update_command: config.cli.update_command.clone(),
                },
            );
        }
    }
}
```

- [ ] **Step 4: Add module declaration to lib.rs**

In `src-tauri/src/lib.rs`, add after `mod normalizer_health;`:

```rust
mod cli_updater;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test cli_updater --lib`
Expected: 5 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/cli_updater.rs src-tauri/src/lib.rs
git commit -m "feat: add CliUpdater — version tracking and auto-update config per provider"
```

---

### Task 5: Capability Negotiator

**Files:**
- Create: `src-tauri/src/capability.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod capability;`)

**Context:** The CapabilityNegotiator replaces the static `CapabilityProfile` in `discovery.rs` with dynamic, per-provider feature detection. It reads `[capabilities]` from TOML configs, stores `FeatureSupport` (Full/Partial/Unsupported) per feature, and caches results to `~/.reasonance/capabilities/`. Unit tests cover the data structures and cache logic. Live negotiation (spawning CLIs) is tested in integration tests only.

- [ ] **Step 1: Write the failing tests**

```rust
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

        let caps = NegotiatedCapabilities {
            provider: "claude".to_string(),
            cli_version: "1.0.0".to_string(),
            cli_mode: CliMode::Structured,
            features: HashMap::new(),
            negotiated_at: 12345,
        };

        negotiator.set_capabilities("claude", caps);
        negotiator.save_cache(dir.path()).unwrap();

        // Should be invalid for different version
        assert!(!negotiator.is_cache_valid("claude", "2.0.0"));
        // Should be valid for same version
        assert!(negotiator.is_cache_valid("claude", "1.0.0"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test capability --lib 2>&1 | head -30`
Expected: FAIL — module not found

- [ ] **Step 3: Write minimal implementation**

```rust
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
    pub fn is_supported(&self) -> bool {
        matches!(self, FeatureSupport::Full | FeatureSupport::Partial { .. })
    }

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
        self.results.lock().unwrap().get(provider).cloned()
    }

    pub fn set_capabilities(&self, provider: &str, caps: NegotiatedCapabilities) {
        self.results.lock().unwrap().insert(provider.to_string(), caps);
    }

    pub fn all_capabilities(&self) -> HashMap<String, NegotiatedCapabilities> {
        self.results.lock().unwrap().clone()
    }

    pub fn is_cache_valid(&self, provider: &str, current_cli_version: &str) -> bool {
        let results = self.results.lock().unwrap();
        match results.get(provider) {
            Some(caps) => {
                if caps.cli_version != current_cli_version {
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

    pub fn save_cache(&self, cache_dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
        let results = self.results.lock().unwrap();
        for (provider, caps) in results.iter() {
            let path = cache_dir.join(format!("{}.json", provider));
            let json = serde_json::to_string_pretty(caps).map_err(|e| e.to_string())?;
            std::fs::write(&path, json).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn load_cache(&self, cache_dir: &Path) -> Result<(), String> {
        if !cache_dir.exists() {
            return Ok(());
        }
        let mut results = self.results.lock().unwrap();
        for entry in std::fs::read_dir(cache_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(caps) = serde_json::from_str::<NegotiatedCapabilities>(&content) {
                        results.insert(caps.provider.clone(), caps);
                    }
                }
            }
        }
        Ok(())
    }
}
```

- [ ] **Step 4: Add module declaration to lib.rs**

In `src-tauri/src/lib.rs`, add after `mod cli_updater;`:

```rust
mod capability;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test capability --lib`
Expected: 8 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/capability.rs src-tauri/src/lib.rs
git commit -m "feat: add CapabilityNegotiator — feature detection with cache and workarounds"
```

---

### Task 6: Self-Heal Flow

**Files:**
- Create: `src-tauri/src/self_heal.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod self_heal;`)

**Context:** When normalizer health degrades after a CLI update, the self-heal flow generates a candidate TOML fix by sending the current TOML + test failures to an LLM. It iterates up to `max_iterations` times, testing each candidate. This task implements the data structures, prompt generation, and result evaluation. The actual LLM call is abstracted behind a trait for testability.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_heal_config_defaults() {
        let config = SelfHealConfig::default();
        assert_eq!(config.max_iterations, 3);
        assert!(config.feedback_to_llm);
    }

    #[test]
    fn test_build_heal_prompt_first_iteration() {
        let current_toml = r#"[cli]
name = "test"
binary = "test"
"#;
        let failures = vec![
            crate::normalizer_health::TestCaseResult {
                name: "basic_text".into(),
                passed: false,
                failure_reason: Some("Required event 'text' not found".into()),
            },
        ];

        let prompt = build_heal_prompt(current_toml, &failures, None);
        assert!(prompt.contains("name = \"test\""));
        assert!(prompt.contains("basic_text"));
        assert!(prompt.contains("Required event 'text' not found"));
        assert!(!prompt.contains("Previous attempt")); // no previous attempt
    }

    #[test]
    fn test_build_heal_prompt_with_previous_attempt() {
        let current_toml = "[cli]\nname = \"test\"\n";
        let failures = vec![
            crate::normalizer_health::TestCaseResult {
                name: "basic_text".into(),
                passed: false,
                failure_reason: Some("still failing".into()),
            },
        ];
        let previous = "previous toml attempt";

        let prompt = build_heal_prompt(current_toml, &failures, Some(previous));
        assert!(prompt.contains("Previous attempt"));
        assert!(prompt.contains("previous toml attempt"));
    }

    #[test]
    fn test_extract_toml_from_response() {
        let response = r#"Here is the fixed TOML:

```toml
[cli]
name = "test"
binary = "test"
```

This should fix the issue."#;

        let extracted = extract_toml_from_response(response);
        assert!(extracted.is_some());
        let toml = extracted.unwrap();
        assert!(toml.contains("[cli]"));
        assert!(toml.contains("name = \"test\""));
    }

    #[test]
    fn test_extract_toml_no_block() {
        let response = "No TOML here, just text.";
        let extracted = extract_toml_from_response(response);
        assert!(extracted.is_none());
    }

    #[test]
    fn test_self_heal_result_fixed() {
        let result = SelfHealResult::Fixed {
            new_toml: "fixed".into(),
            iterations: 2,
            tokens_used: 1500,
        };
        assert!(result.is_fixed());
    }

    #[test]
    fn test_self_heal_result_failed() {
        let result = SelfHealResult::Failed {
            best_attempt: Some("attempt".into()),
            remaining_failures: vec![],
        };
        assert!(!result.is_fixed());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test self_heal --lib 2>&1 | head -30`
Expected: FAIL — module not found

- [ ] **Step 3: Write minimal implementation**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfHealConfig {
    pub max_iterations: u32,
    pub feedback_to_llm: bool,
}

impl Default for SelfHealConfig {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            feedback_to_llm: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum SelfHealResult {
    Fixed {
        new_toml: String,
        iterations: u32,
        tokens_used: u64,
    },
    Failed {
        best_attempt: Option<String>,
        remaining_failures: Vec<crate::normalizer_health::TestCaseResult>,
    },
}

impl SelfHealResult {
    pub fn is_fixed(&self) -> bool {
        matches!(self, SelfHealResult::Fixed { .. })
    }
}

/// Build the prompt sent to the LLM to fix a broken normalizer TOML.
pub fn build_heal_prompt(
    current_toml: &str,
    failures: &[crate::normalizer_health::TestCaseResult],
    previous_attempt: Option<&str>,
) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are a REASONANCE normalizer engineer. A normalizer TOML file defines how to parse JSON output from an LLM CLI into structured AgentEvent objects.\n\n");
    prompt.push_str("The current normalizer TOML is:\n\n```toml\n");
    prompt.push_str(current_toml);
    prompt.push_str("\n```\n\n");

    prompt.push_str("The following test cases are FAILING:\n\n");
    for failure in failures {
        prompt.push_str(&format!("- **{}**: {}\n", failure.name, failure.failure_reason.as_deref().unwrap_or("unknown")));
    }

    if let Some(prev) = previous_attempt {
        prompt.push_str("\n\nPrevious attempt (also failed):\n\n```toml\n");
        prompt.push_str(prev);
        prompt.push_str("\n```\n\n");
        prompt.push_str("Analyze why the previous attempt failed and try a different approach.\n");
    }

    prompt.push_str("\nFix the TOML so all tests pass. Return ONLY the complete fixed TOML wrapped in a ```toml code block. Do not change the [cli] section (name/binary must stay the same).\n");

    prompt
}

/// Extract TOML content from an LLM response that wraps it in a code block.
pub fn extract_toml_from_response(response: &str) -> Option<String> {
    // Look for ```toml ... ``` block
    let toml_start = response.find("```toml")?;
    let content_start = response[toml_start..].find('\n')? + toml_start + 1;
    let content_end = response[content_start..].find("```")? + content_start;
    let toml = response[content_start..content_end].trim().to_string();
    if toml.is_empty() {
        None
    } else {
        Some(toml)
    }
}
```

- [ ] **Step 4: Add module declaration to lib.rs**

In `src-tauri/src/lib.rs`, add after `mod capability;`:

```rust
mod self_heal;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test self_heal --lib`
Expected: 7 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/self_heal.rs src-tauri/src/lib.rs
git commit -m "feat: add self-heal flow — prompt generation and TOML extraction for LLM-driven normalizer repair"
```

---

### Task 7: Tauri Commands + Wiring

**Files:**
- Create: `src-tauri/src/commands/capability.rs`
- Modify: `src-tauri/src/commands/mod.rs` (add `pub mod capability;`)
- Modify: `src-tauri/src/lib.rs` (register managed state + commands + setup hook)

**Context:** This task wires all Phase 6 modules into the Tauri app. It creates Tauri commands that the frontend calls, registers managed state for the new modules, and adds a setup hook that loads the capability cache at startup. The commands expose: negotiate capabilities, get capabilities, get health report, run health check, get CLI versions, trigger self-heal, and rollback normalizer.

- [ ] **Step 1: Expose `registry()` on StructuredAgentTransport**

In `src-tauri/src/transport/mod.rs`, add a public getter method to `StructuredAgentTransport` (needed by commands in Step 3):

```rust
    pub fn registry(&self) -> Arc<Mutex<NormalizerRegistry>> {
        self.registry.clone()
    }
```

- [ ] **Step 2: Add module to commands/mod.rs**

Add to `src-tauri/src/commands/mod.rs`:

```rust
pub mod capability;
```

- [ ] **Step 3: Write the Tauri commands**

Create `src-tauri/src/commands/capability.rs`:

```rust
use crate::capability::{CapabilityNegotiator, NegotiatedCapabilities};
use crate::cli_updater::{CliUpdater, CliVersionInfo};
use crate::normalizer_health::{NormalizerHealth, HealthReport};
use crate::normalizer_version::{NormalizerVersionStore, VersionEntry};
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub fn get_capabilities(
    negotiator: State<'_, CapabilityNegotiator>,
) -> HashMap<String, NegotiatedCapabilities> {
    negotiator.all_capabilities()
}

#[tauri::command]
pub fn get_provider_capabilities(
    negotiator: State<'_, CapabilityNegotiator>,
    provider: String,
) -> Result<NegotiatedCapabilities, String> {
    negotiator
        .get_capabilities(&provider)
        .ok_or_else(|| format!("No capabilities for provider: {}", provider))
}

#[tauri::command]
pub fn get_cli_versions(
    updater: State<'_, CliUpdater>,
) -> Vec<CliVersionInfo> {
    updater
        .providers()
        .iter()
        .filter_map(|p| updater.get_info(p))
        .collect()
}

#[tauri::command]
pub fn get_normalizer_versions(
    version_store: State<'_, NormalizerVersionStore>,
    provider: String,
) -> Vec<VersionEntry> {
    version_store.list_versions(&provider)
}

#[tauri::command]
pub fn rollback_normalizer(
    version_store: State<'_, NormalizerVersionStore>,
    transport: State<'_, crate::transport::StructuredAgentTransport>,
    provider: String,
    version_id: String,
) -> Result<String, String> {
    let toml_content = version_store.restore(&provider, &version_id)?;

    // Hot-reload the normalizer in the transport's registry
    let registry = transport.registry();
    let mut registry = registry.lock().unwrap();
    registry.reload_provider(&provider, &toml_content)?;

    Ok(format!("Rolled back {} to version {}", provider, version_id))
}

#[tauri::command]
pub fn get_health_report(
    health: State<'_, NormalizerHealth>,
    provider: String,
) -> Result<HealthReport, String> {
    health
        .get_report(&provider)
        .ok_or_else(|| format!("No health report for provider: {}", provider))
}

#[tauri::command]
pub fn get_all_health_reports(
    health: State<'_, NormalizerHealth>,
) -> HashMap<String, HealthReport> {
    health.all_reports()
}
```

- [ ] **Step 4: Wire managed state and commands in lib.rs**

In `src-tauri/src/lib.rs`, add to the builder chain:

After existing `.manage(...)` calls, add:

```rust
        .manage(capability::CapabilityNegotiator::new())
        .manage(cli_updater::CliUpdater::new())
        .manage(normalizer_health::NormalizerHealth::new())
        .manage({
            let versions_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance")
                .join("normalizer-versions");
            normalizer_version::NormalizerVersionStore::new(&versions_dir)
        })
```

In the `.setup(|app| { ... })` block, after existing wiring, add:

```rust
            // Load capability cache
            let negotiator: tauri::State<'_, capability::CapabilityNegotiator> = app.state();
            let cache_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance")
                .join("capabilities");
            let _ = negotiator.load_cache(&cache_dir);

            // Register CLI providers from normalizer configs
            let updater: tauri::State<'_, cli_updater::CliUpdater> = app.state();
            let transport: tauri::State<'_, transport::StructuredAgentTransport> = app.state();
            let registry = transport.registry();
            let registry_guard = registry.lock().unwrap();
            let configs: std::collections::HashMap<String, _> = registry_guard.providers()
                .into_iter()
                .filter_map(|p| registry_guard.get_config(&p).map(|c| (p, c.clone())))
                .collect();
            drop(registry_guard);
            updater.register_from_configs(&configs);
```

**Note:** `TomlConfig` must derive `Clone` — this is done in Task 2 Step 3 (Clone derives added to all config structs).

In the `.invoke_handler(tauri::generate_handler![...])` block, add:

```rust
            commands::capability::get_capabilities,
            commands::capability::get_provider_capabilities,
            commands::capability::get_cli_versions,
            commands::capability::get_normalizer_versions,
            commands::capability::rollback_normalizer,
            commands::capability::get_health_report,
            commands::capability::get_all_health_reports,
```

- [ ] **Step 6: Verify compilation and tests**

Run: `cd src-tauri && cargo test 2>&1 | tail -10`
Expected: All tests PASS, no compilation errors

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/capability.rs src-tauri/src/commands/mod.rs src-tauri/src/transport/mod.rs src-tauri/src/normalizer/mod.rs src-tauri/src/lib.rs
git commit -m "feat: wire Phase 6 modules — Tauri commands, managed state, setup hooks"
```

---

### Task 8: Frontend Types, Adapter, and Store

**Files:**
- Create: `src/lib/types/capability.ts`
- Create: `src/lib/stores/capabilities.ts`
- Modify: `src/lib/adapter/index.ts`
- Modify: `src/lib/adapter/tauri.ts`
- Modify: `tests/mocks/adapter.ts`

**Context:** Frontend TypeScript types mirroring Rust structs, adapter methods for the 5 new Tauri commands, and a writable store for caching capabilities. Follow the patterns from Phase 4 (types in `src/lib/types/`, adapter in `src/lib/adapter/`, stores use `writable` from `svelte/store` in `.ts` files).

- [ ] **Step 1: Create TypeScript types**

Create `src/lib/types/capability.ts`:

```typescript
import type { CliMode } from './agent-event';

export interface NegotiatedCapabilities {
  provider: string;
  cli_version: string;
  cli_mode: CliMode;
  features: Record<string, FeatureSupport>;
  negotiated_at: number;
}

export type FeatureSupport =
  | { level: 'full' }
  | { level: 'partial'; limitations: string[]; workaround?: Workaround }
  | { level: 'unsupported'; alternative?: Workaround };

export interface Workaround {
  description: string;
  method: WorkaroundMethod;
}

export type WorkaroundMethod =
  | 'inline_in_prompt'
  | 'simulate_from_batch'
  | { fallback_flag: string }
  | 'skip_silently';

export interface CliVersionInfo {
  provider: string;
  current_version: string | null;
  last_checked: number | null;
  auto_update: boolean;
  version_command: string[];
  update_command: string[];
}

export interface VersionEntry {
  id: string;
  provider: string;
  timestamp: number;
  checksum: string;
}

export interface HealthReport {
  provider: string;
  status: HealthStatus;
  results: TestCaseResult[];
  capabilities_confirmed: string[];
  capabilities_missing: string[];
  capabilities_broken: string[];
  tested_at: number;
  cli_version: string;
}

export type HealthStatus =
  | { type: 'healthy' }
  | { type: 'degraded'; failing_tests: string[] }
  | { type: 'broken'; error: string };

export interface TestCaseResult {
  name: string;
  passed: boolean;
  failure_reason: string | null;
}
```

- [ ] **Step 2: Add Adapter interface methods**

In `src/lib/adapter/index.ts`, add to the `Adapter` interface:

```typescript
  // Capability & health commands
  getCapabilities(): Promise<Record<string, NegotiatedCapabilities>>;
  getProviderCapabilities(provider: string): Promise<NegotiatedCapabilities>;
  getCliVersions(): Promise<CliVersionInfo[]>;
  getNormalizerVersions(provider: string): Promise<VersionEntry[]>;
  rollbackNormalizer(provider: string, versionId: string): Promise<string>;
  getHealthReport(provider: string): Promise<HealthReport>;
  getAllHealthReports(): Promise<Record<string, HealthReport>>;
```

Add the imports at the top:

```typescript
import type { NegotiatedCapabilities, CliVersionInfo, VersionEntry, HealthReport } from '$lib/types/capability';
```

- [ ] **Step 3: Implement in TauriAdapter**

In `src/lib/adapter/tauri.ts`, add implementations:

```typescript
  async getCapabilities(): Promise<Record<string, NegotiatedCapabilities>> {
    return invoke('get_capabilities');
  }

  async getProviderCapabilities(provider: string): Promise<NegotiatedCapabilities> {
    return invoke('get_provider_capabilities', { provider });
  }

  async getCliVersions(): Promise<CliVersionInfo[]> {
    return invoke('get_cli_versions');
  }

  async getNormalizerVersions(provider: string): Promise<VersionEntry[]> {
    return invoke('get_normalizer_versions', { provider });
  }

  async rollbackNormalizer(provider: string, versionId: string): Promise<string> {
    return invoke('rollback_normalizer', { provider, versionId });
  }

  async getHealthReport(provider: string): Promise<HealthReport> {
    return invoke('get_health_report', { provider });
  }

  async getAllHealthReports(): Promise<Record<string, HealthReport>> {
    return invoke('get_all_health_reports');
  }
```

Add the import:

```typescript
import type { NegotiatedCapabilities, CliVersionInfo, VersionEntry, HealthReport } from '$lib/types/capability';
```

- [ ] **Step 4: Add stubs to mock adapter**

In `tests/mocks/adapter.ts`, add stub implementations:

```typescript
  async getCapabilities() { return {}; }
  async getProviderCapabilities(_provider: string) {
    return { provider: '', cli_version: '', cli_mode: 'structured' as const, features: {}, negotiated_at: 0 };
  }
  async getCliVersions() { return []; }
  async getNormalizerVersions(_provider: string) { return []; }
  async rollbackNormalizer(_provider: string, _versionId: string) { return 'ok'; }
  async getHealthReport(_provider: string) {
    return { provider: '', status: { type: 'healthy' as const }, results: [], capabilities_confirmed: [], capabilities_missing: [], capabilities_broken: [], tested_at: 0, cli_version: '' };
  }
  async getAllHealthReports() { return {}; }
```

- [ ] **Step 5: Create capabilities store**

Create `src/lib/stores/capabilities.ts`:

```typescript
import { writable, get } from 'svelte/store';
import type { NegotiatedCapabilities, CliVersionInfo } from '$lib/types/capability';

export const providerCapabilities = writable<Record<string, NegotiatedCapabilities>>({});
export const cliVersions = writable<CliVersionInfo[]>([]);

export function setCapabilities(caps: Record<string, NegotiatedCapabilities>) {
  providerCapabilities.set(caps);
}

export function updateProviderCapabilities(provider: string, caps: NegotiatedCapabilities) {
  providerCapabilities.update((current) => ({ ...current, [provider]: caps }));
}

export function setCliVersions(versions: CliVersionInfo[]) {
  cliVersions.set(versions);
}

export function isFeatureSupported(provider: string, feature: string): boolean {
  const caps = get(providerCapabilities);
  const providerCaps = caps[provider];
  if (!providerCaps) return false;
  const featureSupport = providerCaps.features[feature];
  return featureSupport?.level === 'full' || featureSupport?.level === 'partial';
}
```

- [ ] **Step 6: Run svelte-check**

Run: `npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -20`
Expected: No errors in the new/modified files

- [ ] **Step 7: Commit**

```bash
git add src/lib/types/capability.ts src/lib/stores/capabilities.ts src/lib/adapter/index.ts src/lib/adapter/tauri.ts tests/mocks/adapter.ts
git commit -m "feat: frontend types, adapter methods, and capabilities store for Phase 6"
```

---

## Summary

| Task | Description | Files | Tests |
|------|------------|-------|-------|
| 1 | NormalizerVersionStore | 1 new + 1 mod | 5 |
| 2 | NormalizerRegistry extensions + Clone derives | 1 mod | 4 |
| 3 | NormalizerHealth evaluator + container | 1 new + 1 mod | 5 |
| 4 | CliUpdater | 1 new + 1 mod | 5 |
| 5 | CapabilityNegotiator | 1 new + 1 mod | 8 |
| 6 | Self-Heal flow | 1 new + 1 mod | 7 |
| 7 | Tauri commands + wiring | 2 new + 3 mod | compilation |
| 8 | Frontend types + adapter + store | 2 new + 3 mod | svelte-check |
| **Total** | | **8 new, 10 mod** | **34+ tests** |

**Deferred to Phase 7:**
- Settings UI for per-provider capability display, re-negotiate/test buttons, auto-update toggles, self-heal consent dialog (requires UI components not yet designed)
- TOML `[capabilities.*]` test definition parsing and live CLI execution for negotiation (data structures and caching are in place; actual test running requires async orchestration and CLI spawning logic)
- Self-heal orchestration loop (prompt generation and TOML extraction are implemented; the iteration loop calling LLM + testing candidates requires integration with transport layer)
- `trigger_self_heal` Tauri command (depends on orchestration loop above)
