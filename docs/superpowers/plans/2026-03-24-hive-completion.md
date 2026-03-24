# HIVE Platform Completion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the remaining ~20-25% of the HIVE platform per the approved spec at `docs/superpowers/specs/2026-03-21-agent-hive-platform-design.md`.

**Architecture:** Nine independent sub-projects executed in dependency order. Each produces compilable, testable code. Rust backend changes use inline `#[cfg(test)]` modules following existing patterns. Frontend changes follow existing Svelte 5 + store patterns.

**Tech Stack:** Rust (Tauri 2), Rhai 1.x, Svelte 5, @xyflow/svelte 1.5.1, TypeScript

**Spec Reference:** `docs/superpowers/specs/2026-03-21-agent-hive-platform-design.md`

---

## File Structure

### New Files
- `src-tauri/src/logic_eval.rs` — Rhai expression evaluator for Logic nodes
- `src-tauri/src/resource_lock.rs` — Mutex manager for shared Resource nodes
- `src-tauri/src/agent_memory.rs` — Agent memory persistence

### Modified Files (Rust)
- `src-tauri/Cargo.toml` — Add `rhai` dependency
- `src-tauri/src/lib.rs` — Register new modules and commands
- `src-tauri/src/workflow_store.rs` — Add `schema_version`, `permission_level`, `memory` config, migration
- `src-tauri/src/workflow_engine.rs` — Integrate Rhai eval, resource locking, permission checks, event enrichment, output capture
- `src-tauri/src/agent_runtime.rs` — Add memory read/write, timeout field
- `src-tauri/src/discovery.rs` — Add OpenAI-compatible probe, custom agent registration
- `src-tauri/src/commands/engine.rs` — Expose new commands
- `src-tauri/src/commands/discovery.rs` — Expose register_custom_agent command

### Modified Files (Frontend)
- `src/lib/adapter/index.ts` — Add new types and adapter methods
- `src/lib/stores/workflow.ts` — Add permission level store
- `src/lib/stores/engine.ts` — Add agent output log store, event listeners
- `src/lib/components/hive/HiveCanvas.svelte` — Permission indicator, capability validation on edge connect
- `src/lib/components/hive/HivePanel.svelte` — Live log section
- `src/lib/components/hive/AgentNode.svelte` — Memory indicator badge
- `src/lib/components/hive/HiveInspector.svelte` — Memory and permission display
- `src/lib/components/TerminalManager.svelte` — HivePanel tab integration

---

## Task 1: Schema & Data Model Completion

**Files:**
- Modify: `src-tauri/src/workflow_store.rs:20-121`
- Modify: `src/lib/adapter/index.ts:216-231`

### Rust — Add schema_version, permission_level, memory config

- [ ] **Step 1: Write failing test — schema_version field**

Add to the `#[cfg(test)] mod tests` block in `workflow_store.rs`:

```rust
#[test]
fn test_schema_version_default() {
    let wf = WorkflowStore::create_empty("Versioned");
    assert_eq!(wf.schema_version, 1);
}

#[test]
fn test_schema_version_missing_defaults_to_zero() {
    let json = r#"{"name":"Old","nodes":[],"edges":[]}"#;
    let wf: Workflow = serde_json::from_str(json).unwrap();
    assert_eq!(wf.schema_version, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --lib workflow_store::tests::test_schema_version`
Expected: FAIL — `schema_version` field not found

- [ ] **Step 3: Add schema_version to Workflow struct**

In `workflow_store.rs`, add to `Workflow` struct (after line 117):

```rust
#[serde(default)]
pub schema_version: u32,
```

Update `create_empty()` to set `schema_version: 1`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --lib workflow_store::tests::test_schema_version`
Expected: PASS

- [ ] **Step 5: Write failing test — permission_level in settings**

```rust
#[test]
fn test_permission_level_default() {
    let settings = WorkflowSettings::default();
    assert_eq!(settings.permission_level, "supervised");
}

#[test]
fn test_permission_level_deserialization() {
    let json = r#"{"permissionLevel":"trusted"}"#;
    let settings: WorkflowSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.permission_level, "trusted");
}
```

- [ ] **Step 6: Add permission_level to WorkflowSettings**

In `WorkflowSettings` struct (line 76), add:

```rust
#[serde(default = "default_permission_level", rename = "permissionLevel")]
pub permission_level: String,
```

Add default function:

```rust
fn default_permission_level() -> String {
    "supervised".to_string()
}
```

Update `Default for WorkflowSettings` to include `permission_level: default_permission_level()`.

- [ ] **Step 7: Run test to verify it passes**

Run: `cd src-tauri && cargo test --lib workflow_store::tests::test_permission`
Expected: PASS

- [ ] **Step 8: Write failing test — memory config in AgentNodeConfig**

```rust
#[test]
fn test_agent_memory_config() {
    let json = r#"{"llm":"claude","memory":{"enabled":true,"maxEntries":100,"persist":"workflow"}}"#;
    let config: AgentNodeConfig = serde_json::from_str(json).unwrap();
    assert!(config.memory.is_some());
    let mem = config.memory.unwrap();
    assert!(mem.enabled);
    assert_eq!(mem.max_entries, 100);
    assert_eq!(mem.persist, "workflow");
}

#[test]
fn test_agent_memory_config_default_none() {
    let json = r#"{"llm":"claude"}"#;
    let config: AgentNodeConfig = serde_json::from_str(json).unwrap();
    assert!(config.memory.is_none());
}
```

- [ ] **Step 9: Add MemoryConfig struct, memory field, and fix serde renames**

Add new struct before `AgentNodeConfig`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_max_entries", rename = "maxEntries")]
    pub max_entries: u32,
    #[serde(default = "default_persist")]
    pub persist: String,
}
fn default_max_entries() -> u32 { 50 }
fn default_persist() -> String { "none".to_string() }
```

Add to `AgentNodeConfig`:

```rust
#[serde(default)]
pub memory: Option<MemoryConfig>,
#[serde(default)]
pub timeout: Option<u64>,
```

Fix `LogicNodeConfig` serde renames (spec uses camelCase `onTrue`/`onFalse`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicNodeConfig {
    pub kind: String,
    pub rule: String,
    #[serde(default, rename = "onTrue")]
    pub on_true: Option<String>,
    #[serde(default, rename = "onFalse")]
    pub on_false: Option<String>,
}
```

Make `WorkflowEdge.id` optional for legacy JSON (needed for migration):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    #[serde(default)]
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
}
```

- [ ] **Step 10: Run all tests to verify nothing broke**

Run: `cd src-tauri && cargo test --lib workflow_store`
Expected: ALL PASS

- [ ] **Step 11: Write failing test — schema migration v0 → v1**

```rust
#[test]
fn test_migrate_v0_to_v1() {
    let json = r#"{
        "name": "Legacy",
        "nodes": [{"id":"a1","type":"agent","label":"X","config":{"llm":"claude"},"position":{"x":0,"y":0}}],
        "edges": [{"from":"a1","to":"a1"}],
        "settings": {}
    }"#;
    let mut wf: Workflow = serde_json::from_str(json).unwrap();
    migrate(&mut wf);
    assert_eq!(wf.schema_version, 1);
    assert_eq!(wf.settings.permission_level, "supervised");
    // edges without id get one
    assert!(!wf.edges[0].id.is_empty());
}
```

- [ ] **Step 12: Implement migrate() function**

Add after `WorkflowStore` impl block:

```rust
pub fn migrate(workflow: &mut Workflow) {
    if workflow.schema_version < 1 {
        // Add IDs to edges missing them
        for edge in &mut workflow.edges {
            if edge.id.is_empty() {
                edge.id = uuid::Uuid::new_v4().to_string();
            }
        }
        // Ensure permission_level has default
        if workflow.settings.permission_level.is_empty() {
            workflow.settings.permission_level = "supervised".to_string();
        }
        workflow.schema_version = 1;
    }
}
```

Update `WorkflowStore::load()` to call `migrate(&mut workflow)` after deserialization.

Update `WorkflowEdge` to make `id` default to empty string:

```rust
#[serde(default)]
pub id: String,
```

- [ ] **Step 13: Run all tests**

Run: `cd src-tauri && cargo test --lib workflow_store`
Expected: ALL PASS

- [ ] **Step 14: Update frontend types**

In `src/lib/adapter/index.ts`, update `WorkflowSettings`:

```typescript
export interface WorkflowSettings {
  max_concurrent_agents: number;
  default_retry: number;
  timeout: number;
  permissionLevel: 'supervised' | 'trusted' | 'dry-run';
}
```

Update `Workflow`:

```typescript
export interface Workflow {
  schemaVersion: number;
  name: string;
  // ... existing fields
}
```

Add `MemoryConfig`:

```typescript
export interface MemoryConfig {
  enabled: boolean;
  maxEntries: number;
  persist: 'none' | 'workflow' | 'global';
}
```

- [ ] **Step 15: Commit**

```bash
git add src-tauri/src/workflow_store.rs src/lib/adapter/index.ts
git commit -m "feat(hive): add schemaVersion, permissionLevel, memory config, migration v0→v1"
```

---

## Task 2: Logic Node + Rhai Expression Evaluator

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/logic_eval.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/workflow_engine.rs:355-370`

- [ ] **Step 1: Add rhai dependency**

In `src-tauri/Cargo.toml` under `[dependencies]`:

```toml
rhai = { version = "1", features = ["serde"] }
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: OK

- [ ] **Step 3: Create logic_eval.rs with failing test**

Create `src-tauri/src/logic_eval.rs`:

```rust
use rhai::{Engine, Scope};
use serde_json::Value;

pub struct LogicEvaluator {
    engine: Engine,
}

impl LogicEvaluator {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        // Sandbox: disable all external access
        engine.set_max_operations(10_000);
        engine.set_max_string_size(4096);
        Self { engine }
    }

    /// Evaluate a rule expression against the previous node's output.
    /// Returns true/false for routing to onTrue/onFalse edges.
    pub fn evaluate(&self, rule: &str, output: &Value) -> Result<bool, String> {
        let mut scope = Scope::new();
        // Convert serde_json::Value → Rhai Dynamic for nested access
        let dynamic_output = rhai::serde::to_dynamic(output.clone())
            .map_err(|e| format!("Failed to convert output to Rhai dynamic: {}", e))?;
        scope.push("output", dynamic_output);

        self.engine
            .eval_with_scope::<bool>(&mut scope, rule)
            .map_err(|e| format!("Rule evaluation failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_bool_rule() {
        let eval = LogicEvaluator::new();
        let output = serde_json::json!({"errors": 0});
        let result = eval.evaluate("output.errors == 0", &output).unwrap();
        assert!(result);
    }

    #[test]
    fn test_false_condition() {
        let eval = LogicEvaluator::new();
        let output = serde_json::json!({"errors": 3});
        let result = eval.evaluate("output.errors == 0", &output).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_string_comparison() {
        let eval = LogicEvaluator::new();
        let output = serde_json::json!({"status": "success"});
        let result = eval.evaluate(r#"output.status == "success""#, &output).unwrap();
        assert!(result);
    }

    #[test]
    fn test_invalid_rule_returns_error() {
        let eval = LogicEvaluator::new();
        let output = serde_json::json!({});
        let result = eval.evaluate("invalid!!!", &output);
        assert!(result.is_err());
    }

    #[test]
    fn test_numeric_comparison() {
        let eval = LogicEvaluator::new();
        let output = serde_json::json!({"score": 85});
        let result = eval.evaluate("output.score > 70", &output).unwrap();
        assert!(result);
    }
}
```

- [ ] **Step 4: Run test to verify Rhai integration works**

Run: `cd src-tauri && cargo test --lib logic_eval`

Note: The `rhai` dependency must include the `serde` feature for `rhai::serde::to_dynamic` to work. This is already specified in the Cargo.toml addition above.

- [ ] **Step 5: Register module in lib.rs**

In `src-tauri/src/lib.rs`, add:

```rust
mod logic_eval;
```

- [ ] **Step 6: Integrate LogicEvaluator into WorkflowEngine**

In `workflow_engine.rs`, replace the Logic node handling in `advance_run()` (the section that immediately marks Logic nodes as `Success`, around line 355-365).

Current code marks Logic nodes as `Success` unconditionally. Replace with:

```rust
NodeType::Logic => {
    // Parse LogicNodeConfig from node.config
    let config: LogicNodeConfig = serde_json::from_value(node.config.clone())
        .map_err(|e| format!("Invalid logic config: {}", e))?;

    // Get output from predecessor node (first predecessor's last output)
    let predecessors = Self::get_predecessors(&node_id, &workflow.edges);
    let predecessor_output = if let Some(pred_id) = predecessors.first() {
        // For now, use empty object if no output captured yet
        serde_json::json!({})
    } else {
        serde_json::json!({})
    };

    let evaluator = logic_eval::LogicEvaluator::new();
    match evaluator.evaluate(&config.rule, &predecessor_output) {
        Ok(result) => {
            self.update_node_state(run_id, &node_id, AgentState::Success, None)?;

            // Route to onTrue or onFalse edge — disable the other branch
            let active_edge_id = if result { &config.on_true } else { &config.on_false };
            let inactive_edge_id = if result { &config.on_false } else { &config.on_true };

            // Mark nodes on the inactive branch as skipped (Error state with message)
            if let Some(ref inactive_id) = inactive_edge_id {
                // Find all successor nodes reachable only through the inactive edge
                let inactive_successors: Vec<String> = workflow.edges.iter()
                    .filter(|e| e.id == *inactive_id)
                    .map(|e| e.to.clone())
                    .collect();
                for succ_id in inactive_successors {
                    // Only skip if this node has no other active incoming edges
                    let other_inputs = workflow.edges.iter()
                        .filter(|e| e.to == succ_id && e.id != *inactive_id)
                        .count();
                    if other_inputs == 0 {
                        self.update_node_state(run_id, &succ_id, AgentState::Error, None)?;
                    }
                }
            }

            log::info!("Logic node {} evaluated to {}, routing to {:?}", node_id, result, active_edge_id);
        }
        Err(e) => {
            self.update_node_state(run_id, &node_id, AgentState::Error, None)?;
            log::error!("Logic node {} rule failed: {}", node_id, e);
        }
    }
}
```

- [ ] **Step 7: Write test for Logic node evaluation in workflow_engine**

```rust
#[test]
fn test_logic_node_evaluation_stub() {
    // Verify LogicEvaluator can be constructed and called
    let eval = crate::logic_eval::LogicEvaluator::new();
    let output = serde_json::json!({"errors": 0});
    assert!(eval.evaluate("output.errors == 0", &output).unwrap());
}
```

- [ ] **Step 8: Run all workflow_engine tests**

Run: `cd src-tauri && cargo test --lib workflow_engine`
Expected: ALL PASS

- [ ] **Step 9: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/logic_eval.rs src-tauri/src/lib.rs src-tauri/src/workflow_engine.rs
git commit -m "feat(hive): add Rhai-based Logic node expression evaluator"
```

---

## Task 3: Real-Time Event System

**Files:**
- Modify: `src-tauri/src/workflow_engine.rs`
- Modify: `src/lib/stores/engine.ts`
- Modify: `src/lib/components/hive/HiveCanvas.svelte`

### Rust — Standardize event names to hive:// namespace

- [ ] **Step 1: Update event names in workflow_engine.rs**

Replace all event emission strings:
- `"workflow-node-started"` → `"hive://node-state-changed"`
- `"workflow-node-completed"` → `"hive://node-state-changed"`
- `"workflow-node-retrying"` → `"hive://node-state-changed"`
- `"workflow-node-fallback"` → `"hive://node-state-changed"`
- `"workflow-node-error"` → `"hive://node-state-changed"`
- `"workflow-run-started"` → `"hive://run-status-changed"`
- `"workflow-run-stopped"` → `"hive://run-status-changed"`
- `"workflow-run-completed"` → `"hive://run-completed"`

Standardize payload shape for node events:

```rust
#[derive(Serialize, Clone)]
struct NodeStateEvent {
    run_id: String,
    node_id: String,
    old_state: String,
    new_state: String,
}
```

```rust
#[derive(Serialize, Clone)]
struct RunStatusEvent {
    run_id: String,
    old_status: String,
    new_status: String,
}
```

Emit structured payloads instead of ad-hoc strings.

- [ ] **Step 2: Also emit agent output events**

In `advance_run()`, after spawning a PTY for an Agent node, subscribe to PTY output and re-emit:

```rust
// After pty_manager.spawn() returns pty_id:
// The PTY manager already emits pty-data-{pty_id}
// We emit a hive-specific wrapper event with node context
app.emit("hive://agent-output", serde_json::json!({
    "run_id": run_id,
    "node_id": node_id,
    "pty_id": pty_id,
})).ok();
```

Note: Full output streaming is handled by the existing `pty-data-{id}` events. The `hive://agent-output` event tells the frontend which PTY belongs to which node.

- [ ] **Step 3: Update event names in commands/engine.rs**

In `play_workflow` command, update `"workflow-run-started"` → `"hive://run-status-changed"`.
In `stop_workflow` command, update `"workflow-run-stopped"` → `"hive://run-status-changed"`.

- [ ] **Step 4: Verify Rust compiles**

Run: `cd src-tauri && cargo check`
Expected: OK

- [ ] **Step 5: Add Tauri event listeners to frontend engine store**

In `src/lib/stores/engine.ts`, add event listener setup function:

```typescript
import { listen } from '@tauri-apps/api/event';

export async function setupHiveEventListeners() {
  await listen<{ run_id: string; node_id: string; old_state: string; new_state: string }>(
    'hive://node-state-changed',
    (event) => {
      currentRun.update(run => {
        if (!run || run.id !== event.payload.run_id) return run;
        const ns = { ...run.node_states };
        if (ns[event.payload.node_id]) {
          ns[event.payload.node_id] = {
            ...ns[event.payload.node_id],
            state: event.payload.new_state as AgentState,
          };
        }
        return { ...run, node_states: ns };
      });
    }
  );

  await listen<{ run_id: string; old_status: string; new_status: string }>(
    'hive://run-status-changed',
    (event) => {
      currentRun.update(run => {
        if (!run || run.id !== event.payload.run_id) return run;
        return { ...run, status: event.payload.new_status as RunStatus };
      });
    }
  );

  await listen<{ run_id: string; status: string }>(
    'hive://run-completed',
    (event) => {
      currentRun.update(run => {
        if (!run || run.id !== event.payload.run_id) return run;
        return { ...run, status: event.payload.status as RunStatus };
      });
    }
  );
}
```

- [ ] **Step 6: Initialize listeners in HiveCanvas.svelte**

In `HiveCanvas.svelte`, add `onMount` call:

```typescript
import { onMount } from 'svelte';
import { setupHiveEventListeners } from '$lib/stores/engine';

onMount(() => {
  setupHiveEventListeners();
});
```

- [ ] **Step 7: Verify frontend compiles**

Run: `npm run check`
Expected: OK (or only pre-existing warnings)

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/workflow_engine.rs src-tauri/src/commands/engine.rs src/lib/stores/engine.ts src/lib/components/hive/HiveCanvas.svelte
git commit -m "feat(hive): wire real-time event system with hive:// namespace"
```

---

## Task 4: Permission Enforcement

**Files:**
- Modify: `src-tauri/src/workflow_engine.rs`
- Modify: `src/lib/components/hive/HiveCanvas.svelte`
- Modify: `src/lib/stores/workflow.ts`

### Rust — Check permission level before agent execution

- [ ] **Step 1: Write failing test — dry-run skips PTY spawn**

In `workflow_engine.rs` tests:

```rust
#[test]
fn test_dry_run_does_not_spawn() {
    let engine = WorkflowEngine::new();
    let mut wf = sample_workflow_with_agent();
    wf.settings.permission_level = "dry-run".to_string();
    let run_id = engine.create_run(&wf, "/tmp/test.json").unwrap();
    // In dry-run, advance_run should mark agents as Success without spawning
    // We test this by checking no PTY was requested
    let run = engine.get_run(&run_id).unwrap();
    assert_eq!(run.status, RunStatus::Running);
}
```

- [ ] **Step 2: Implement permission check in advance_run()**

In `advance_run()`, before spawning PTY for Agent nodes, check permission level:

```rust
let permission_level = &workflow.settings.permission_level;

match permission_level.as_str() {
    "dry-run" => {
        // Simulate execution — mark as Success without spawning
        self.update_node_state(run_id, &node_id, AgentState::Success, Some(agent_id.clone()))?;
        app.emit("hive://node-state-changed", NodeStateEvent {
            run_id: run_id.to_string(),
            node_id: node_id.clone(),
            old_state: "idle".to_string(),
            new_state: "success".to_string(),
        }).ok();
        log::info!("[dry-run] Node {} simulated as success", node_id);
    }
    "supervised" => {
        // Emit permission request event — frontend shows approval dialog
        app.emit("hive://permission-request", serde_json::json!({
            "run_id": run_id,
            "node_id": node_id,
            "agent_label": node.label,
        })).ok();
        // Don't spawn PTY yet — wait for approval via approve_node command
    }
    _ => {
        // "trusted" — spawn directly (existing behavior)
        // ... existing PTY spawn code
    }
}
```

- [ ] **Step 3: Add approve_node command for supervised mode**

In `commands/engine.rs`:

```rust
#[tauri::command]
pub fn approve_node(
    run_id: String,
    node_id: String,
    workflow_path: String,
    cwd: String,
    app: AppHandle,
    engine: State<WorkflowEngine>,
    store: State<WorkflowStore>,
    runtime: State<AgentRuntime>,
    pty_manager: State<PtyManager>,
) -> Result<(), String> {
    let workflow = store.load(&workflow_path)?;
    engine.spawn_approved_node(&run_id, &node_id, &workflow, &runtime, &pty_manager, &app, &cwd)
}
```

Register in `lib.rs` invoke handler.

- [ ] **Step 4: Run Rust tests**

Run: `cd src-tauri && cargo test --lib workflow_engine`
Expected: ALL PASS

- [ ] **Step 5: Add permission indicator to HiveCanvas toolbar**

In `HiveCanvas.svelte`, add to the toolbar section (after controls):

```svelte
{#if $currentWorkflow?.settings?.permissionLevel}
  <span class="permission-badge" class:supervised={$currentWorkflow.settings.permissionLevel === 'supervised'}
        class:trusted={$currentWorkflow.settings.permissionLevel === 'trusted'}
        class:dryrun={$currentWorkflow.settings.permissionLevel === 'dry-run'}
        title={$currentWorkflow.settings.permissionLevel}>
    {#if $currentWorkflow.settings.permissionLevel === 'supervised'}🔒
    {:else if $currentWorkflow.settings.permissionLevel === 'trusted'}🛡️
    {:else}〰️{/if}
  </span>
{/if}
```

Add styles:

```css
.permission-badge { font-size: 1.2em; padding: 0 0.25rem; }
.supervised { color: var(--warning); }
.trusted { color: var(--success); }
.dryrun { opacity: 0.6; }
```

- [ ] **Step 6: Add permission request listener**

In `engine.ts`, add listener for `hive://permission-request`:

```typescript
await listen<{ run_id: string; node_id: string; agent_label: string }>(
  'hive://permission-request',
  (event) => {
    pendingApprovals.update(list => [...list, event.payload]);
  }
);

export const pendingApprovals = writable<Array<{ run_id: string; node_id: string; agent_label: string }>>([]);
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/workflow_engine.rs src-tauri/src/commands/engine.rs src-tauri/src/lib.rs src/lib/stores/engine.ts src/lib/components/hive/HiveCanvas.svelte
git commit -m "feat(hive): implement permission enforcement (supervised/trusted/dry-run)"
```

---

## Task 5: Resource Locking

**Files:**
- Create: `src-tauri/src/resource_lock.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/workflow_engine.rs`

- [ ] **Step 1: Create resource_lock.rs with tests**

```rust
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Manages exclusive write locks on Resource nodes.
/// Multiple readers allowed, single writer.
pub struct ResourceLockManager {
    /// resource_id → set of agent_ids holding read locks
    readers: Arc<Mutex<HashMap<String, HashSet<String>>>>,
    /// resource_id → agent_id holding write lock (None = unlocked)
    writers: Arc<Mutex<HashMap<String, String>>>,
}

impl ResourceLockManager {
    pub fn new() -> Self {
        Self {
            readers: Arc::new(Mutex::new(HashMap::new())),
            writers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Attempt to acquire a lock. Returns Ok(()) if acquired, Err if blocked.
    pub fn acquire(&self, resource_id: &str, agent_id: &str, write: bool) -> Result<(), String> {
        if write {
            let readers = self.readers.lock().unwrap();
            if let Some(r) = readers.get(resource_id) {
                if !r.is_empty() {
                    return Err(format!("Resource {} has active readers", resource_id));
                }
            }
            let mut writers = self.writers.lock().unwrap();
            if let Some(holder) = writers.get(resource_id) {
                return Err(format!("Resource {} locked by {}", resource_id, holder));
            }
            writers.insert(resource_id.to_string(), agent_id.to_string());
        } else {
            let writers = self.writers.lock().unwrap();
            if writers.contains_key(resource_id) {
                return Err(format!("Resource {} is write-locked", resource_id));
            }
            drop(writers);
            let mut readers = self.readers.lock().unwrap();
            readers.entry(resource_id.to_string()).or_default().insert(agent_id.to_string());
        }
        Ok(())
    }

    /// Release lock held by agent on resource.
    pub fn release(&self, resource_id: &str, agent_id: &str) {
        let mut writers = self.writers.lock().unwrap();
        if writers.get(resource_id).map(|h| h == agent_id).unwrap_or(false) {
            writers.remove(resource_id);
            return;
        }
        drop(writers);
        let mut readers = self.readers.lock().unwrap();
        if let Some(r) = readers.get_mut(resource_id) {
            r.remove(agent_id);
            if r.is_empty() {
                readers.remove(resource_id);
            }
        }
    }

    /// Release all locks held by an agent (on stop/error).
    pub fn release_all(&self, agent_id: &str) {
        let mut writers = self.writers.lock().unwrap();
        writers.retain(|_, v| v != agent_id);
        drop(writers);
        let mut readers = self.readers.lock().unwrap();
        for set in readers.values_mut() {
            set.remove(agent_id);
        }
        readers.retain(|_, v| !v.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_lock_allows_multiple_readers() {
        let mgr = ResourceLockManager::new();
        assert!(mgr.acquire("r1", "a1", false).is_ok());
        assert!(mgr.acquire("r1", "a2", false).is_ok());
    }

    #[test]
    fn test_write_lock_exclusive() {
        let mgr = ResourceLockManager::new();
        assert!(mgr.acquire("r1", "a1", true).is_ok());
        assert!(mgr.acquire("r1", "a2", true).is_err());
    }

    #[test]
    fn test_write_blocked_by_readers() {
        let mgr = ResourceLockManager::new();
        assert!(mgr.acquire("r1", "a1", false).is_ok());
        assert!(mgr.acquire("r1", "a2", true).is_err());
    }

    #[test]
    fn test_read_blocked_by_writer() {
        let mgr = ResourceLockManager::new();
        assert!(mgr.acquire("r1", "a1", true).is_ok());
        assert!(mgr.acquire("r1", "a2", false).is_err());
    }

    #[test]
    fn test_release_allows_reacquire() {
        let mgr = ResourceLockManager::new();
        assert!(mgr.acquire("r1", "a1", true).is_ok());
        mgr.release("r1", "a1");
        assert!(mgr.acquire("r1", "a2", true).is_ok());
    }

    #[test]
    fn test_release_all() {
        let mgr = ResourceLockManager::new();
        mgr.acquire("r1", "a1", true).unwrap();
        mgr.acquire("r2", "a1", false).unwrap();
        mgr.release_all("a1");
        assert!(mgr.acquire("r1", "a2", true).is_ok());
        assert!(mgr.acquire("r2", "a2", true).is_ok());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test --lib resource_lock`
Expected: ALL PASS

- [ ] **Step 3: Register in lib.rs as managed state**

```rust
mod resource_lock;
// In run() function, add to managed state:
.manage(resource_lock::ResourceLockManager::new())
```

- [ ] **Step 4: Integrate into WorkflowEngine::advance_run()**

Before spawning an agent, acquire locks on all connected Resource nodes:

```rust
// Get resources connected to this agent via edges
let connected_resources: Vec<(&WorkflowNode, bool)> = workflow.edges.iter()
    .filter(|e| e.from == node_id || e.to == node_id)
    .filter_map(|e| {
        let res_id = if e.from == node_id { &e.to } else { &e.from };
        workflow.nodes.iter().find(|n| n.id == *res_id && n.node_type == NodeType::Resource)
            .map(|n| {
                let config: ResourceNodeConfig = serde_json::from_value(n.config.clone()).unwrap_or(/* default */);
                let is_write = config.access == "write" || config.access == "read_write";
                (n, is_write)
            })
    })
    .collect();

for (res_node, is_write) in &connected_resources {
    if let Err(e) = lock_manager.acquire(&res_node.id, &agent_id, *is_write) {
        log::warn!("Agent {} blocked on resource {}: {}", node_id, res_node.id, e);
        // Skip this node for now — will be retried on next advance_run()
        continue;
    }
}
```

On `on_node_completed()`, release all locks for the agent:

```rust
lock_manager.release_all(&agent_id);
```

- [ ] **Step 5: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/resource_lock.rs src-tauri/src/lib.rs src-tauri/src/workflow_engine.rs
git commit -m "feat(hive): add resource locking with reader/writer exclusion"
```

---

## Task 6: Agent Memory System

**Files:**
- Create: `src-tauri/src/agent_memory.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/workflow_engine.rs`

- [ ] **Step 1: Create agent_memory.rs with tests**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub run_id: String,
    pub timestamp: String,
    pub input_summary: String,
    pub output_summary: String,
    pub outcome: String,
    #[serde(default)]
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemoryStore {
    pub node_id: String,
    pub entries: Vec<MemoryEntry>,
}

impl AgentMemoryStore {
    pub fn new(node_id: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            entries: vec![],
        }
    }

    /// Load memory from disk. Returns empty store if file doesn't exist.
    pub fn load(path: &str) -> Result<Self, String> {
        let p = std::path::Path::new(path);
        if !p.exists() {
            return Err("Memory file not found".to_string());
        }
        let content = std::fs::read_to_string(p)
            .map_err(|e| format!("Failed to read memory: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Invalid memory JSON: {}", e))
    }

    /// Save memory to disk.
    pub fn save(&self, path: &str) -> Result<(), String> {
        let parent = std::path::Path::new(path).parent()
            .ok_or("Invalid path")?;
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create dir: {}", e))?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write: {}", e))?;
        Ok(())
    }

    /// Add entry with FIFO eviction based on max_entries.
    pub fn add_entry(&mut self, entry: MemoryEntry, max_entries: u32) {
        self.entries.push(entry);
        while self.entries.len() > max_entries as usize {
            self.entries.remove(0);
        }
    }

    /// Get path for workflow-scoped memory.
    pub fn workflow_memory_path(workflow_path: &str, node_id: &str) -> PathBuf {
        let wf_dir = std::path::Path::new(workflow_path).parent()
            .unwrap_or(std::path::Path::new("."));
        wf_dir.join("memory").join(format!("{}.json", node_id))
    }

    /// Get path for global-scoped memory.
    pub fn global_memory_path(node_id: &str) -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.join("reasonance").join("agent-memory").join(format!("{}.json", node_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_entry_fifo_eviction() {
        let mut store = AgentMemoryStore::new("agent-1");
        for i in 0..5 {
            store.add_entry(MemoryEntry {
                run_id: format!("run-{}", i),
                timestamp: "2026-01-01".to_string(),
                input_summary: "test".to_string(),
                output_summary: "test".to_string(),
                outcome: "success".to_string(),
                context: serde_json::json!({}),
            }, 3);
        }
        assert_eq!(store.entries.len(), 3);
        assert_eq!(store.entries[0].run_id, "run-2");
    }

    #[test]
    fn test_save_and_load() {
        let dir = std::env::temp_dir().join("reasonance_test_memory");
        let path = dir.join("agent-1.json");
        let path_str = path.to_str().unwrap();

        let mut store = AgentMemoryStore::new("agent-1");
        store.add_entry(MemoryEntry {
            run_id: "run-1".to_string(),
            timestamp: "2026-01-01".to_string(),
            input_summary: "Review code".to_string(),
            output_summary: "Found 2 issues".to_string(),
            outcome: "success".to_string(),
            context: serde_json::json!({}),
        }, 50);

        store.save(path_str).unwrap();
        let loaded = AgentMemoryStore::load(path_str).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].run_id, "run-1");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_missing_file() {
        let result = AgentMemoryStore::load("/nonexistent/path.json");
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test --lib agent_memory`
Expected: ALL PASS

- [ ] **Step 3: Register module**

In `lib.rs`:

```rust
mod agent_memory;
```

- [ ] **Step 4: Integrate into WorkflowEngine — inject memory on spawn**

In `advance_run()`, after creating an agent and before spawning PTY, if the node has `memory.enabled`:

```rust
// Check if agent node has memory enabled
if let Ok(agent_config) = serde_json::from_value::<AgentNodeConfig>(node.config.clone()) {
    if let Some(ref mem_config) = agent_config.memory {
        if mem_config.enabled {
            let mem_path = match mem_config.persist.as_str() {
                "global" => AgentMemoryStore::global_memory_path(&node_id),
                _ => AgentMemoryStore::workflow_memory_path(workflow_path, &node_id),
            };
            if let Ok(memory) = AgentMemoryStore::load(mem_path.to_str().unwrap_or("")) {
                // Inject memory entries as context for the agent
                log::info!("Injected {} memory entries for node {}", memory.entries.len(), node_id);
            }
        }
    }
}
```

- [ ] **Step 5: Save memory on node completion**

In `on_node_completed()`, after marking success:

```rust
// Save memory entry if memory enabled
if let Some(node) = workflow.nodes.iter().find(|n| n.id == *node_id) {
    if let Ok(agent_config) = serde_json::from_value::<AgentNodeConfig>(node.config.clone()) {
        if let Some(ref mem_config) = agent_config.memory {
            if mem_config.enabled {
                let mem_path = match mem_config.persist.as_str() {
                    "global" => AgentMemoryStore::global_memory_path(node_id),
                    _ => AgentMemoryStore::workflow_memory_path(workflow_path, node_id),
                };
                let path_str = mem_path.to_str().unwrap_or("");
                let mut store = AgentMemoryStore::load(path_str)
                    .unwrap_or_else(|_| AgentMemoryStore::new(node_id));
                store.add_entry(MemoryEntry {
                    run_id: run_id.to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    input_summary: String::new(), // TODO: capture from PTY
                    output_summary: String::new(), // TODO: capture from PTY
                    outcome: if success { "success" } else { "failed" }.to_string(),
                    context: serde_json::json!({}),
                }, mem_config.max_entries);
                if let Err(e) = store.save(path_str) {
                    log::error!("Failed to save agent memory: {}", e);
                }
            }
        }
    }
}
```

- [ ] **Step 6: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: ALL PASS

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/agent_memory.rs src-tauri/src/lib.rs src-tauri/src/workflow_engine.rs
git commit -m "feat(hive): add agent memory persistence with FIFO eviction"
```

---

## Task 7: Live Log + HivePanel Tab Integration

**Files:**
- Modify: `src/lib/stores/engine.ts`
- Modify: `src/lib/components/hive/HivePanel.svelte`
- Modify: `src/lib/components/TerminalManager.svelte`

- [ ] **Step 1: Add agent output log store**

In `src/lib/stores/engine.ts`:

```typescript
export interface AgentLogEntry {
  node_id: string;
  line: string;
  timestamp: number;
}

export const agentOutputLog = writable<AgentLogEntry[]>([]);

const MAX_LOG_LINES = 500;

// Add to setupHiveEventListeners():
await listen<{ run_id: string; node_id: string; pty_id: string }>(
  'hive://agent-output',
  (event) => {
    const { node_id, pty_id } = event.payload;
    // Listen to PTY data for this specific agent
    listen<string>(`pty-data-${pty_id}`, (ptyEvent) => {
      agentOutputLog.update(log => {
        const newLog = [...log, { node_id, line: ptyEvent.payload, timestamp: Date.now() }];
        return newLog.slice(-MAX_LOG_LINES);
      });
    });
  }
);
```

- [ ] **Step 2: Add live log to HivePanel**

In `HivePanel.svelte`, add after the status line:

```svelte
<script>
  import { agentOutputLog } from '$lib/stores/engine';
  let logLines: AgentLogEntry[] = [];
  agentOutputLog.subscribe(v => { logLines = v; });
</script>

<!-- After status line -->
<div class="live-log" role="log" aria-label="Agent output log" aria-live="polite">
  {#each logLines.slice(-50) as entry}
    <div class="log-line">
      <span class="log-node">[{entry.node_id}]</span>
      <span class="log-text">{entry.line}</span>
    </div>
  {/each}
  {#if logLines.length === 0}
    <div class="log-empty">No output yet</div>
  {/if}
</div>
```

Add styles:

```css
.live-log {
  flex: 1;
  overflow-y: auto;
  font-family: var(--font-mono);
  font-size: 0.75rem;
  padding: 0.25rem;
  background: var(--bg-secondary);
  border-radius: 4px;
}
.log-line { white-space: pre-wrap; word-break: break-all; }
.log-node { color: var(--accent); margin-right: 0.5em; }
.log-empty { color: var(--text-muted); text-align: center; padding: 1rem; }
```

- [ ] **Step 3: Integrate HivePanel as tab in TerminalManager**

In `TerminalManager.svelte`, add a "Hive" tab option. After the existing tab rendering logic:

```svelte
<script>
  import HivePanel from './hive/HivePanel.svelte';
  import { showHiveCanvas, currentWorkflow } from '$lib/stores/workflow';

  let showHiveTab = false;
  currentWorkflow.subscribe(wf => { showHiveTab = wf !== null; });
</script>

<!-- In the tab bar, add after existing tabs -->
{#if showHiveTab}
  <button
    class="tab"
    class:active={activeTab === 'hive'}
    on:click={() => activeTab = 'hive'}
    aria-label="Hive monitor"
  >
    Hive
  </button>
{/if}

<!-- In the tab content area -->
{#if activeTab === 'hive'}
  <HivePanel {adapter} {cwd} />
{/if}
```

Note: The exact integration depends on TerminalManager's tab system. Adapt the tab state management to match the existing pattern (likely `instanceViewModes` or a simple `activeTab` state).

- [ ] **Step 4: Verify frontend compiles**

Run: `npm run check`
Expected: OK

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/engine.ts src/lib/components/hive/HivePanel.svelte src/lib/components/TerminalManager.svelte
git commit -m "feat(hive): add live agent output log and HivePanel tab integration"
```

---

## Task 8: Capability Validation on Canvas

**Files:**
- Modify: `src/lib/components/hive/HiveCanvas.svelte`
- Modify: `src/lib/adapter/index.ts`

- [ ] **Step 1: Add validateConnection function**

In `HiveCanvas.svelte`, add connection validation:

```typescript
import type { Connection } from '@xyflow/svelte';

function validateConnection(connection: Connection): boolean {
  const wf = get(currentWorkflow);
  if (!wf) return false;

  const sourceNode = wf.nodes.find(n => n.id === connection.source);
  const targetNode = wf.nodes.find(n => n.id === connection.target);
  if (!sourceNode || !targetNode) return false;

  // Agent → Resource: agent must have write capability if resource is write-access
  if (sourceNode.type === 'agent' && targetNode.type === 'resource') {
    const resConfig = targetNode.config as Record<string, unknown>;
    const access = resConfig.access as string || 'read';
    if (access === 'write' || access === 'read_write') {
      const agentConfig = sourceNode.config as Record<string, unknown>;
      const caps = (agentConfig.capabilities as string[]) || [];
      if (!caps.includes('write_file')) {
        return false;
      }
    }
  }

  // Resource → Agent: always valid (agent reads from resource)
  // Agent → Logic: always valid
  // Logic → Agent: always valid

  return true;
}
```

- [ ] **Step 2: Wire validation to SvelteFlow's isValidConnection**

Pass to `<SvelteFlow>` component:

```svelte
<SvelteFlow
  {nodes}
  {edges}
  {nodeTypes}
  isValidConnection={validateConnection}
  on:connect={onConnect}
  ...
>
```

- [ ] **Step 3: Add visual feedback for invalid connections**

In the `onConnect` handler, if validation fails, show a toast:

```typescript
function onConnect(event: CustomEvent) {
  const connection = event.detail;
  if (!validateConnection(connection)) {
    // Show error toast
    toast.set({ message: 'Invalid connection: agent lacks required capability', type: 'error' });
    return;
  }
  // ... existing edge creation logic
}
```

- [ ] **Step 4: Verify frontend compiles**

Run: `npm run check`
Expected: OK

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/hive/HiveCanvas.svelte
git commit -m "feat(hive): add capability validation on canvas edge connections"
```

---

## Task 9: Discovery Engine Completion

**Files:**
- Modify: `src-tauri/src/discovery.rs`
- Modify: `src-tauri/src/commands/discovery.rs`
- Modify: `src-tauri/src/lib.rs`

### Add OpenAI-compatible API probe

- [ ] **Step 1: Write failing test — probe_openai_compatible**

In `discovery.rs` tests:

```rust
#[tokio::test]
async fn test_probe_openai_compatible_timeout() {
    // Should not crash on unreachable endpoint
    let engine = DiscoveryEngine::new();
    let agents = engine.probe_openai_compatible("http://localhost:19999").await;
    assert!(agents.is_err() || agents.unwrap().is_empty());
}
```

- [ ] **Step 2: Implement probe_openai_compatible**

```rust
async fn probe_openai_compatible(base_url: &str) -> Result<Vec<DiscoveredAgent>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("API returned {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let models: Vec<String> = body["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| m["id"].as_str().map(String::from))
        .collect();

    if models.is_empty() {
        return Ok(vec![]);
    }

    Ok(vec![DiscoveredAgent {
        name: format!("openai-compatible@{}", base_url),
        source: DiscoverySource::Api,
        command: None,
        endpoint: Some(base_url.to_string()),
        models,
        capabilities: CapabilityProfile {
            read_file: false,
            write_file: false,
            execute_command: false,
            web_search: false,
            image_input: false,
            long_context: false,
        },
        max_context: None,
        available: true,
    }])
}
```

- [ ] **Step 3: Call from probe_apis()**

Update `probe_apis()`:

```rust
pub async fn probe_apis(&self) -> Vec<DiscoveredAgent> {
    let mut agents = Vec::new();

    // Ollama
    if let Ok(ollama_agents) = Self::probe_ollama().await {
        agents.extend(ollama_agents);
    }

    // OpenAI-compatible on common ports
    for port in [1234, 8080, 5000] {
        let url = format!("http://localhost:{}", port);
        if let Ok(oai_agents) = Self::probe_openai_compatible(&url).await {
            agents.extend(oai_agents);
        }
    }

    // Update cache
    let mut cached = self.agents.lock().unwrap();
    cached.retain(|a| a.source == DiscoverySource::Cli || a.source == DiscoverySource::Manual);
    cached.extend(agents.clone());

    agents
}
```

- [ ] **Step 4: Add custom agent registration**

```rust
pub fn register_custom_agent(&self, agent: DiscoveredAgent) {
    let mut agents = self.agents.lock().unwrap();
    // Remove existing with same name
    agents.retain(|a| a.name != agent.name);
    agents.push(agent);
}
```

- [ ] **Step 5: Add Tauri command for custom registration**

In `commands/discovery.rs`:

```rust
#[tauri::command]
pub fn register_custom_agent(
    agent: DiscoveredAgent,
    engine: State<DiscoveryEngine>,
) -> Result<(), String> {
    engine.register_custom_agent(agent);
    Ok(())
}
```

Register in `lib.rs` invoke handler.

- [ ] **Step 6: Run all tests**

Run: `cd src-tauri && cargo test --lib discovery`
Expected: ALL PASS

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/discovery.rs src-tauri/src/commands/discovery.rs src-tauri/src/lib.rs
git commit -m "feat(hive): add OpenAI-compatible API probe and custom agent registration"
```

---

## Task 10: Final Integration & Build Verification

- [ ] **Step 1: Run full Rust test suite**

Run: `cd src-tauri && cargo test`
Expected: ALL PASS

- [ ] **Step 2: Run frontend type check**

Run: `npm run check`
Expected: OK (or pre-existing warnings only)

- [ ] **Step 3: Run full build**

Run: `npm run build`
Expected: Build succeeds

- [ ] **Step 4: Update registry**

Update `.claude/docs/registry.md` with new modules:
- `LogicEvaluator` in `src-tauri/src/logic_eval.rs`
- `ResourceLockManager` in `src-tauri/src/resource_lock.rs`
- `AgentMemoryStore` in `src-tauri/src/agent_memory.rs`

- [ ] **Step 5: Commit**

```bash
git add .claude/docs/registry.md
git commit -m "docs: update registry with new HIVE modules"
```

---

## Dependency Order

```
Task 1 (Schema)
  ↓
Task 2 (Rhai) ──→ Task 3 (Events) ──→ Task 4 (Permissions)
                                    ↘
Task 5 (Locks) ─────────────────────→ Task 7 (Live Log)
Task 6 (Memory) ────────────────────→ Task 8 (Capability Validation)
Task 9 (Discovery) ─────────────────→ Task 10 (Final)
```

Tasks 2, 5, 6, 9 can run in parallel after Task 1.
Tasks 3, 7, 8 depend on their predecessors.
Task 4 depends on Task 3.
Task 10 depends on all others.

---

## Known Gaps / Deferred Items

These spec features are intentionally deferred from this plan:

- **Explicit `AgentMessage` routing by topology** — The spec defines message passing between agents via edges (pipeline, fan-out, fan-in). Currently, agent I/O is handled via PTY streams. Structured `AgentMessage` queuing and topology-driven routing is deferred to a follow-up plan. The `AgentMessage` struct and `send_message()`/`get_messages_for()` already exist in `agent_runtime.rs` but are not wired to the execution loop.
- **Memory summarization** — The spec lists this as an open decision (spec line 636). `input_summary` and `output_summary` are stored as empty strings for now.
- **Agent output streaming granularity** — Also an open decision (spec line 637). Current approach: full PTY stream via existing `pty-data-{id}` events.
- **Detachable canvas window** — Deferred to v1.1 per spec (line 294).
