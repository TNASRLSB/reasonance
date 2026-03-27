# W1.5 Structured Errors: Complete Migration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate all `Result<T, String>` error types and dangerous `unwrap()` calls from production code, making `ReasonanceError` the sole error type across the Rust codebase.

**Architecture:** Bottom-up migration by dependency order. Add `From<String> for ReasonanceError` to smooth gradual migration. Convert leaf modules first, then dependents, then workflow_engine (heaviest). Fix dangerous unwraps in the same pass per file. Each task produces a compiling, tested state.

**Tech Stack:** Rust (thiserror, serde), existing `ReasonanceError` enum in `src-tauri/src/error.rs`

**Master Spec:** [2026-03-27-wiring-completion-master-spec.md](../specs/roadmap/2026-03-27-wiring-completion-master-spec.md)

---

## Task 1: Add From<String> for ReasonanceError + convenience constructors

**Files:**
- Modify: `src-tauri/src/error.rs`

- [ ] **Step 1: Add From<String> impl and new constructors**

In `src-tauri/src/error.rs`, add after the existing `From` impls:

```rust
impl From<String> for ReasonanceError {
    fn from(s: String) -> Self {
        Self::Internal { message: s }
    }
}
```

Also add convenience constructors to the `impl ReasonanceError` block:

```rust
pub fn workflow(workflow_id: impl Into<String>, node_id: impl Into<String>, message: impl Into<String>) -> Self {
    Self::Workflow {
        workflow_id: workflow_id.into(),
        node_id: node_id.into(),
        message: message.into(),
    }
}

pub fn serialization(context: impl Into<String>, message: impl Into<String>) -> Self {
    Self::Serialization {
        context: context.into(),
        message: message.into(),
    }
}

pub fn transport(provider: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
    Self::Transport {
        provider: provider.into(),
        message: message.into(),
        retryable,
    }
}
```

- [ ] **Step 2: Add tests for new impls**

```rust
#[test]
fn test_from_string() {
    let err: ReasonanceError = "something failed".to_string().into();
    assert!(matches!(err, ReasonanceError::Internal { .. }));
    assert!(err.to_string().contains("something failed"));
}

#[test]
fn test_workflow_constructor() {
    let err = ReasonanceError::workflow("wf-1", "node-a", "cycle detected");
    assert!(matches!(err, ReasonanceError::Workflow { .. }));
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test error -- --nocapture`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/error.rs
git commit -m "feat(errors): add From<String>, workflow/serialization/transport constructors"
```

---

## Task 2: Migrate leaf modules (resource_lock, logic_eval, settings, agent_memory)

**Files:**
- Modify: `src-tauri/src/resource_lock.rs:24` — `acquire() -> Result<(), String>`
- Modify: `src-tauri/src/logic_eval.rs:22` — `evaluate() -> Result<bool, String>`
- Modify: `src-tauri/src/settings/mod.rs:169` — `set() -> Result<(), String>`
- Modify: `src-tauri/src/agent_memory.rs:29,36` — `load/save -> Result<_, String>`

- [ ] **Step 1: Migrate resource_lock.rs**

Change `acquire()` signature from `Result<(), String>` to `Result<(), ReasonanceError>`. Replace `Err(format!(...))` with `Err(ReasonanceError::workflow("", resource_id, message))`.

Add `use crate::error::ReasonanceError;` if not present.

- [ ] **Step 2: Migrate logic_eval.rs**

Change `evaluate()` from `Result<bool, String>` to `Result<bool, ReasonanceError>`. Replace `Err(format!(...))` with `Err(ReasonanceError::validation("expression", message))`.

- [ ] **Step 3: Migrate settings/mod.rs**

Change `set()` from `Result<(), String>` to `Result<(), ReasonanceError>`. Replace `Err("Empty path".to_string())` with `Err(ReasonanceError::validation("path", "Empty path"))` and `Err("Not a table")` with `Err(ReasonanceError::config("Setting path is not a table"))`.

Update caller in `commands/settings.rs` — remove the `.map_err(ReasonanceError::internal)` since `set()` now returns `ReasonanceError` directly.

- [ ] **Step 4: Migrate agent_memory.rs**

Change `load()` and `save()` from `Result<_, String>` to `Result<_, ReasonanceError>`. Replace `Err(format!(...))` with `Err(ReasonanceError::io(...))` for I/O and `Err(ReasonanceError::Serialization { ... })` for JSON errors. Use `?` with existing `From` impls where possible.

- [ ] **Step 5: Run tests**

Run: `cd src-tauri && cargo test resource_lock logic_eval settings agent_memory -- --nocapture`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/resource_lock.rs src-tauri/src/logic_eval.rs src-tauri/src/settings/mod.rs src-tauri/src/agent_memory.rs src-tauri/src/commands/settings.rs
git commit -m "refactor(errors): migrate leaf modules to ReasonanceError"
```

---

## Task 3: Migrate storage/data modules (analytics, normalizer, capability, discovery, workspace_trust, normalizer_version)

**Files:**
- Modify: `src-tauri/src/analytics/store.rs:14,27,70` — `new/append/load -> Result<_, String>`
- Modify: `src-tauri/src/normalizer/mod.rs:129,158,224` — `parse/load_from_dir/reload_provider -> Result<_, String>`
- Modify: `src-tauri/src/normalizer_version.rs:96` — `save_index -> Result<(), String>`
- Modify: `src-tauri/src/capability.rs:104,116` — `save_cache/load_cache -> Result<(), String>`
- Modify: `src-tauri/src/discovery.rs:230,275` — `probe_openai_compatible/probe_ollama -> Result<_, String>`
- Modify: `src-tauri/src/workspace_trust.rs:210` — `folder_info -> Result<_, String>`

- [ ] **Step 1: Migrate analytics/store.rs**

Change `new()`, `append()`, `load()` from `Result<_, String>` to `Result<_, ReasonanceError>`. Use `ReasonanceError::io(...)` for file ops, `?` with serde `From` impls for JSON. Replace `map_err(|e| e.to_string())` with `map_err(|e| ReasonanceError::io("analytics store", e))`.

- [ ] **Step 2: Migrate normalizer/mod.rs**

Change `TomlConfig::parse()`, `NormalizerRegistry::load_from_dir()`, `reload_provider()` from `Result<_, String>` to `Result<_, ReasonanceError>`. Use `ReasonanceError::config(...)` for parse errors, `ReasonanceError::io(...)` for file reads.

**Important:** `load_from_dir` is called in `transport/mod.rs` constructors which already return `ReasonanceError`. The `.map_err(ReasonanceError::config)` wrapper there becomes unnecessary — use `?` directly.

- [ ] **Step 3: Migrate normalizer_version.rs**

Change `save_index()` from `Result<(), String>` to `Result<(), ReasonanceError>`. Use `ReasonanceError::serialization(...)` for JSON errors, `ReasonanceError::io(...)` for file writes.

- [ ] **Step 4: Migrate capability.rs**

Change `save_cache()` and `load_cache()` from `Result<(), String>` to `Result<(), ReasonanceError>`. Same pattern as normalizer_version.

- [ ] **Step 5: Migrate discovery.rs**

Change `probe_openai_compatible()` and `probe_ollama()` from `Result<_, String>` to `Result<_, ReasonanceError>`. Use `ReasonanceError::transport("openai-compatible", msg, true)` for HTTP errors, `ReasonanceError::Serialization` for JSON parse errors.

- [ ] **Step 6: Migrate workspace_trust.rs**

Change `folder_info()` from `Result<FolderInfo, String>` to `Result<FolderInfo, ReasonanceError>`. Use `ReasonanceError::validation(...)` for path validation errors.

- [ ] **Step 7: Update callers**

Check all callers of the above functions. Where they previously did `.map_err(|e| ...)` to convert String to ReasonanceError, simplify to use `?` directly. Key callers:
- `transport/mod.rs` line calling `load_from_dir`
- `commands/transport.rs` calling discovery methods
- `lib.rs` setup calling analytics/capability init

- [ ] **Step 8: Run tests**

Run: `cd src-tauri && cargo test analytics normalizer capability discovery workspace_trust -- --nocapture`
Expected: ALL PASS

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/analytics/store.rs src-tauri/src/normalizer/mod.rs src-tauri/src/normalizer_version.rs src-tauri/src/capability.rs src-tauri/src/discovery.rs src-tauri/src/workspace_trust.rs src-tauri/src/transport/mod.rs src-tauri/src/lib.rs
git commit -m "refactor(errors): migrate storage/data modules to ReasonanceError"
```

---

## Task 4: Migrate workflow_engine.rs (13 functions + 6 dangerous unwraps)

**Files:**
- Modify: `src-tauri/src/workflow_engine.rs` — 13 `Result<_, String>` functions + 6 unwraps

- [ ] **Step 1: Change all 13 function signatures**

Change every function returning `Result<T, String>` to `Result<T, ReasonanceError>`. The functions are:
- `topological_sort`, `create_run`, `stop_run`, `pause_run`, `resume_run`
- `update_node_state`, `check_run_complete`, `finalize_run`
- `spawn_single_node`, `advance_run`, `on_node_completed`
- `handle_failure`, `step_run`

Replace all `Err(format!(...))` patterns:
- Run not found → `Err(ReasonanceError::not_found("run", run_id))`
- Node not found → `Err(ReasonanceError::not_found("node", node_id))`
- Cycle detected → `Err(ReasonanceError::workflow(workflow_id, "", "Cycle detected"))`
- Invalid state → `Err(ReasonanceError::workflow(workflow_id, node_id, message))`

- [ ] **Step 2: Fix 6 dangerous unwraps**

Replace HashMap `.get().unwrap()` on run_id (lines ~410, 546, 837, 1043):
```rust
// Before:
let wf_path = &runs.get(run_id).unwrap().workflow_path;
// After:
let run = runs.get(run_id).ok_or_else(|| ReasonanceError::not_found("run", run_id))?;
let wf_path = &run.workflow_path;
```

Replace iterator `.find().unwrap()` on node_id (lines ~945, 981):
```rust
// Before:
let node = workflow.nodes.iter().find(|n| n.id == node_id).unwrap();
// After:
let node = workflow.nodes.iter().find(|n| n.id == node_id)
    .ok_or_else(|| ReasonanceError::not_found("node", node_id))?;
```

- [ ] **Step 3: Update callers in commands/engine.rs**

The engine commands call workflow_engine methods and convert errors. Since the methods now return `ReasonanceError` directly, simplify the error handling — remove `.map_err(...)` wrappers that converted String to error types.

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test workflow_engine engine -- --nocapture`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/workflow_engine.rs src-tauri/src/commands/engine.rs
git commit -m "refactor(errors): migrate workflow_engine to ReasonanceError, fix 6 dangerous unwraps"
```

---

## Task 5: Fix remaining dangerous unwraps + Mutex poison recovery

**Files:**
- Modify: `src-tauri/src/file_ops.rs` — 8 `lock().unwrap()` → `lock().unwrap_or_else(|e| e.into_inner())`
- Modify: `src-tauri/src/normalizer/pipeline.rs:77` — guarded unwrap → pattern match

- [ ] **Step 1: Fix file_ops.rs Mutex locks**

Replace all `self.undo_stack.lock().unwrap()` and `self.project_root.lock().unwrap()` with the poison-recovery pattern:
```rust
// Before:
let mut stack = self.undo_stack.lock().unwrap();
// After:
let mut stack = self.undo_stack.lock().unwrap_or_else(|e| e.into_inner());
```

Apply to all ~8 occurrences in file_ops.rs.

- [ ] **Step 2: Fix normalizer/pipeline.rs**

Replace the guarded unwrap with a safe pattern:
```rust
// Before:
Some(arr) if arr.is_array() => arr.as_array().unwrap(),
// After:
Some(arr) => match arr.as_array() {
    Some(array) => array,
    None => return events,
},
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test file_ops normalizer -- --nocapture`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/file_ops.rs src-tauri/src/normalizer/pipeline.rs
git commit -m "fix(safety): replace dangerous unwraps with error handling and poison recovery"
```

---

## Task 6: Final sweep + verification

**Files:**
- All modified files from Tasks 1-5

- [ ] **Step 1: Verify zero Result<T, String> in error positions**

Run: `cd src-tauri && grep -rn 'Result<.*,\s*String>' --include='*.rs' src/ | grep -v '#\[cfg(test)\]' | grep -v 'mod tests' | grep -v 'Result<String'`

Expected: Zero matches (or only in test code / success-type positions).

- [ ] **Step 2: Run full test suite**

Run: `cd src-tauri && cargo test`
Expected: ALL PASS (570+)

- [ ] **Step 3: Run clippy**

Run: `cd src-tauri && cargo clippy -- -D warnings`
Expected: Clean (no warnings)

- [ ] **Step 4: Run frontend tests**

Run: `npx svelte-kit sync && npx vitest run`
Expected: ALL PASS (238)

- [ ] **Step 5: Commit any remaining fixes**

If clippy or tests revealed issues, fix and commit.
