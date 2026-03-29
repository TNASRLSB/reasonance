# Wiring & Completion Master Spec

**Created:** 2026-03-27
**Supersedes:** Status dashboard in [master-spec.md](master-spec.md) (which shows 0/52 — incorrect)
**Purpose:** Connect 9 orphaned modules, complete 14 partial implementations, add 4 performance optimizations, bring codebase to full operational status
**Approach:** Bottom-up by dependency graph. No stubs, no shortcuts. Every module fully wired before moving up.
**Constraints:** None. No time, budget, or token limits. Quality and thoroughness are the only metrics.

---

## Audit Summary (2026-03-27)

Before this spec, an audit of all 36 roadmap items (Phase 0-4) revealed:

| Category | Count | Items |
|----------|:-----:|-------|
| DONE | 11 | 0.6, 0.11, 1.2, 2.1, 2.2, 2.4, 4.1, 4.2, 4.3, 4.6, 4.10 |
| BUILT, NOT CONNECTED | 9 | 0.3, 0.4, 0.5, 0.10, 1.1, 1.4, 3.1, 3.2, 3.5 |
| PARTIAL | 14 | 0.1, 0.2, 0.7, 0.9, 1.3, 2.3, 2.5, 3.3, 3.4, 4.4, 4.5, 4.7, 4.8, 4.9 |
| NOT STARTED | 2 | 0.8, 0.12 |

The master-spec dashboard was never updated. The dominant failure pattern: backend implemented and tested, frontend never wired, module later flagged as dead code and deleted.

---

## Status Dashboard

| Phase | Items | Completed | Status |
|-------|:-----:|:---------:|--------|
| W1 — Wire Foundations | 11 | 11 | **complete** |
| W2 — Wire Features | 14 | 14 | **complete** |
| W3 — Complete UX | 6 | 6 | **complete** |
| Phase 5 — Backlog | 2 | 2 | **complete** |
| **Total** | **33** | **33** | **DONE** |

---

## Phase W1 — Wire Foundations

**Gate:** All 12 Phase 0 exit criteria from the original master spec must pass before W2 starts.

### Dependency Graph

```
Track A (event-driven):        Track B (error-driven):        Track C (parallel):
W1.1 EventBus v2               W1.5 Structured errors         W1.8 Perf baselines
  │                               │                              + startup parallelization
  ├→ W1.2 Signal                  ├→ W1.6 Storage
  ├→ W1.3 Weak refs              ├→ W1.7 Zod validation ──→ W1.11 IPC batching
  └→ W1.4 Layered settings       └→ W1.9 Transactions

W1.10 Dead code cleanup ← LAST (after W1.1-W1.9 + W1.11)
```

Tracks A, B, and C are independent and can run in parallel.

### Items

---

#### W1.1 — EventBus v2: Wire + Async Subscribers (0.3)

**Source spec:** [0.3-event-pub-sub.md](phase-0-foundations/0.3-event-pub-sub.md)

**Exists:**
- `src-tauri/src/event_bus_v2.rs` — channel-based pub/sub, `Weak<dyn EventHandler>`, deferred queue (1000 iteration cap), sweep, frontend-visibility flag. 8 tests.
- Registered in `lib.rs` with 11 named channels.

**Work required:**

1. **Async subscriber support.** Add `AsyncEventHandler` trait with `async fn handle()`. Subscribers marked async are dispatched via `tokio::spawn`. Sync `EventHandler` remains for low-latency consumers (frontend emitter). Publisher never blocks on async subscribers.

2. **Backpressure.** Per-channel buffer with configurable max (default 1000). Buffer full + new event → drop oldest + emit `event:dropped` warning on `lifecycle:warning` channel. Slow consumer detection: if sync subscriber takes >100ms, log warning.

3. **Frontend bridge subscriber.** Create `TauriFrontendBridge` implementing `EventHandler`. On event: if channel is `frontend_visible`, call `app_handle.emit(channel_name, &event)`. Register as subscriber on all frontend-visible channels during setup.

4. **Migrate transport subscribers.** Reimplement `FrontendEmitter`, `HistoryRecorder`, `SessionHistoryRecorder`, `AnalyticsCollector` from `transport/event_bus.rs` as `EventHandler` / `AsyncEventHandler` implementations for the new bus. `SessionHistoryRecorder` and `AnalyticsCollector` should be async (they do I/O). `FrontendEmitter` replaced by `TauriFrontendBridge`. `HistoryRecorder` stays sync (in-memory).

5. **Migrate workflow emits.** Replace all `app.emit()` calls in `workflow_engine.rs` with `EventBus::publish()` on appropriate channels (`workflow:node-state`, `workflow:run-status`).

6. **Rename.** `event_bus_v2.rs` → `event_bus.rs`. Remove old `transport/event_bus.rs`. Update all imports. The `AgentEventBus` struct name from the old module must not leak into the new one — use `EventBus` consistently. All renames happen as the final step of W1.1, after all subscribers are migrated and tests pass.

7. **Benchmark.** Criterion bench: ≥10k events/sec with 10 subscribers (mix of sync and async).

**Exit criteria:**
- [ ] EventBus with channel-based pub/sub operational
- [ ] Async subscribers dispatched via `tokio::spawn`, don't block publisher
- [ ] Deferred queue prevents recursion (test: handler that emits)
- [ ] Backpressure drops oldest with warning when buffer full (test)
- [ ] Weak ref subscribers auto-cleaned when dropped (test)
- [ ] ≥10k events/sec with 10 subscribers (benchmark)
- [ ] Frontend bridge receives events via Tauri emit (integration test)
- [ ] All transport subscribers migrated, old `transport/event_bus.rs` deleted
- [ ] All workflow engine emits migrated

---

#### W1.2 — Signal: Wire Consumers (0.4)

**Source spec:** [0.4-background-task-signals.md](phase-0-foundations/0.4-background-task-signals.md)

**Exists:**
- `src-tauri/src/signal.rs` — `Signal<T>` wrapping `tokio::watch::channel`. `send`, `subscribe`, `current`, `modify`. 4 tests.

**Work required:**

1. **Audit polling patterns.** Identify all `setInterval`, `sleep` loops, and periodic checks in both frontend and backend.

2. **Replace backend polling.** Migrate at minimum:
   - Analytics flush interval → `Signal<FlushRequest>`
   - FS watcher debounce → `Signal<Vec<FsEvent>>` (coalesce rapid changes)
   - PTY sweep timer → `Signal<SweepTick>` (triggered by EventBus lifecycle event)
   - CLI updater check interval → `Signal<UpdateCheck>`

3. **Signal → EventBus bridge.** When a `Signal` value changes, optionally publish the change to an EventBus channel. This is configured per-signal, not global.

4. **Coalescing.** Add `Signal::modify_coalesced()` — batches rapid changes within a configurable window (default 100ms) before notifying subscribers. Critical for FS watcher where 50 changes in 10ms should produce 1 notification.

5. **Frontend polling bridge.** `Signal` is Rust-side. For frontend `setInterval` patterns (e.g., PTY sweep in `TerminalManager.svelte`), the Rust Signal emits a Tauri event via EventBus frontend bridge (W1.1). Frontend replaces `setInterval` with `listen()` for the corresponding event. Not all frontend intervals need this — only those that duplicate backend-driven work.

**Exit criteria:**
- [ ] All backend polling intervals replaced with signal-based refresh
- [ ] Signal changes optionally publish to EventBus
- [ ] Coalescing prevents notification storms on rapid changes (test)
- [ ] Frontend polling that duplicates backend work replaced with event listeners
- [ ] Zero `sleep`-based polling loops in backend

---

#### W1.3 — Weak References: Migrate Consumers (0.5)

**Source spec:** [0.5-weak-references.md](phase-0-foundations/0.5-weak-references.md)

**Exists:**
- `src-tauri/src/tracked_map.rs` — `TrackedMap<K,V>` with `Arc<Mutex<V>>` values, `WeakHandle<V>`, `sweep_exclusive()`. 4 tests.

**Work required:**

1. **Audit `Arc<Mutex<HashMap>>` patterns.** Identify all instances across the codebase.

2. **Migrate transport sessions.** `StructuredAgentTransport::sessions` → `TrackedMap<String, AgentSession>`. Callers receive `WeakHandle<AgentSession>` instead of cloning the full map.

3. **Migrate PTY sessions.** `PtyManager::sessions` (or equivalent) → `TrackedMap<String, PtyInstance>`.

4. **Periodic sweep.** Wire `TrackedMap::sweep_exclusive()` to a Signal (from W1.2) or EventBus lifecycle event (from W1.1) for periodic GC.

5. **Document strategy.** Every remaining `Arc<Mutex<HashMap>>` that is NOT migrated must have a doc comment explaining why (e.g., "map is small and bounded, weak refs not needed").

**Exit criteria:**
- [ ] Transport sessions use `TrackedMap` with `WeakHandle`
- [ ] PTY sessions use `TrackedMap` with `WeakHandle`
- [ ] Periodic sweep wired to lifecycle event
- [ ] Every `Arc<Mutex<HashMap>>` documented with weak-ref strategy or justification

---

#### W1.4 — Layered Settings: Register + Wire (0.10)

**Source spec:** [0.10-layered-settings.md](phase-0-foundations/0.10-layered-settings.md)

**Exists:**
- `src-tauri/src/settings/mod.rs` — `LayeredSettings` with 4-layer resolution (builtin > user > project > workspace), `deep_merge`, `set_project_root()`, `reload()`, `get<T>()`. 10 tests.
- `src-tauri/src/settings/defaults.rs` — builtin defaults.

**Work required:**

1. **Register in Tauri state.** Add `LayeredSettings` to `.manage()` in `lib.rs`. Initialize with builtin defaults + user config on setup.

2. **Tauri commands.** Create `get_setting(key: String) -> Value`, `set_setting(key: String, value: Value, layer: String)`, `get_all_settings() -> Value`, `reload_settings()`.

3. **Adapter methods.** Add `getSetting`, `setSetting`, `getAllSettings`, `reloadSettings` to adapter interface and `tauri.ts`.

4. **Migrate config reads.** Replace `commands/config.rs` reads with `LayeredSettings::get()`. Keep backward compat by having `loadInitialConfig` (frontend) read through the new system.

5. **Wire Settings UI.** Settings panel reads/writes through the new adapter methods instead of directly manipulating config stores.

6. **Project root hook.** On project switch, call `LayeredSettings::set_project_root()` to load project-level and workspace-level overrides.

**Exit criteria:**
- [ ] Settings resolution: workspace > project > user > builtin (test with override at each layer)
- [ ] Settings UI reads/writes through LayeredSettings
- [ ] Project switch loads correct overrides
- [ ] Existing config behavior preserved (no regression)

---

#### W1.5 — Structured Errors: Complete Migration (0.1)

**Source spec:** [0.1-structured-error-types.md](phase-0-foundations/0.1-structured-error-types.md)

**Exists:**
- `src-tauri/src/error.rs` — `ReasonanceError` enum, 11 variants, `thiserror`-derived. Used by most modules.

**Work required:**

Migrate 37 remaining `Result<T, String>` in 14 files:
- `workflow_engine.rs` (13 occurrences) — heaviest, convert to `ReasonanceError::Workflow` or appropriate variants
- `normalizer/mod.rs` (3)
- `normalizer_version.rs` (3)
- `analytics/store.rs` (3)
- `discovery.rs` (2)
- `capability.rs` (2)
- `agent_memory.rs` (2)
- `commands/llm.rs` (3) — `call_anthropic`, `call_openai`, `call_ollama_local`
- `resource_lock.rs` (1)
- `logic_eval.rs` (1)
- `workspace_trust.rs` (remaining)
- Other scattered occurrences

Add new error variants to `ReasonanceError` if needed (e.g., `Workflow`, `Discovery`, `Analytics`).

**Exit criteria:**
- [ ] Zero `Result<T, String>` in Rust codebase
- [ ] All Tauri commands return `Result<T, ReasonanceError>`
- [ ] No `unwrap()` in production paths

---

#### W1.6 — Storage Abstraction: Complete + Wire (0.7)

**Source spec:** [0.7-storage-abstraction.md](phase-0-foundations/0.7-storage-abstraction.md)

**Exists:**
- `src-tauri/src/storage/mod.rs` — `StorageBackend` async trait, `TypedStore<T>` wrapper.
- `storage/json_file.rs` — `JsonFileBackend` with atomic writes. 13 tests.
- `storage/in_memory.rs` — `InMemoryBackend`. 7 tests.

**Work required:**

1. **Add migrate/rollback to trait.**
   ```rust
   async fn migrate(&self, namespace: &str, version: u32) -> Result<(), ReasonanceError>;
   async fn rollback(&self, namespace: &str, version: u32) -> Result<(), ReasonanceError>;
   ```

2. **Implement migration for JsonFileBackend.** Version file per namespace, migration scripts as closures.

3. **Wire SessionStore.** Migrate `transport/session_store.rs` from direct JSONL I/O to `StorageBackend`. This is the most natural first consumer.

4. **Wire at least one more consumer.** Analytics store or agent memory config — to prove the abstraction works for multiple use cases.

**Exit criteria:**
- [ ] `StorageBackend` trait includes migrate/rollback
- [ ] save/load/migrate/rollback tested with both backends
- [ ] SessionStore uses `StorageBackend` (not direct file I/O)
- [ ] At least 2 real consumers use the abstraction

---

#### W1.7 — Zod Validation: Migrate Adapter (0.9)

**Source spec:** [0.9-runtime-type-validation.md](phase-0-foundations/0.9-runtime-type-validation.md)

**Exists:**
- `src/lib/schemas/validated.ts` — `validatedInvoke<T>()` and `validatedListen<T>()` wrappers.
- `src/lib/schemas/session.ts` — Zod schemas for session types.

**Work required:**

1. **Write Zod schemas for all adapter return types.** Organize by domain: `schemas/fs.ts`, `schemas/transport.ts`, `schemas/workflow.ts`, `schemas/config.ts`, `schemas/pty.ts`, `schemas/analytics.ts`, etc.

2. **Migrate `tauri.ts`.** Replace every `invoke<T>(cmd, args)` with `validatedInvoke<T>(cmd, args, Schema)`. This is ~83 calls. Systematic, file-by-file.

3. **Migrate event listeners.** Replace `listen<T>()` with `validatedListen<T>()` for all Tauri event subscriptions.

4. **Error handling.** Validation failures log the mismatch details (expected vs received) and throw a typed error that the ErrorBoundary (0.11) can catch gracefully.

5. **Dev vs prod mode.** In production builds, validation can optionally be compiled out (tree-shaken) via an env flag for zero overhead. Default: always validate.

**Exit criteria:**
- [ ] 100% of adapter return types validated at runtime
- [ ] 100% of event listener payloads validated
- [ ] Validation failure logs details and throws typed error
- [ ] Zod schemas cover all Tauri command return types

---

#### W1.8 — Performance Baselines + Startup Parallelization (0.2)

**Source spec:** [0.2-performance-baseline.md](phase-0-foundations/0.2-performance-baseline.md)

**Exists:**
- `src-tauri/benches/baseline.rs` — Criterion micro-benchmarks (JSON, TOML, UUID, SHA).

**Work required:**

1. **App-level baselines.** Instrument and record:
   - Startup to interactive (ms) — from `run()` to first frontend paint
   - Idle memory (MB) — no files open, no sessions
   - Chat render time (ms) — 10 agent events
   - FileTree render (ms) — 1k files
   - Editor open (ms) — 10k line file
   - Bundle size (KB) — track per-build

2. **Baseline recording mechanism.** JSON file in project root (`benchmarks/baselines.json`) with timestamped entries. CI can compare against previous.

3. **Startup parallelization.** Refactor `lib.rs` setup to parallelize independent init steps with `tokio::join!`:
   - Independent: config load, LLM discovery, EventBus init, theme load
   - Sequential: state restore (needs config), transport init (needs EventBus)
   - Measure before/after

4. **CI integration.** Benchmark job in `ci.yml` that records baselines and flags regressions >5%.

**Exit criteria:**
- [ ] All 6 app-level baselines recorded
- [ ] Startup parallelized with measurable improvement
- [ ] Regression detection mechanism in place (manual or CI)
- [ ] No regression >5% from pre-W1 baseline

---

#### W1.9 — Transaction Semantics (0.8)

**Source spec:** [0.8-transaction-semantics.md](phase-0-foundations/0.8-transaction-semantics.md)

**Exists:** Nothing.

**Work required:**

1. **Transaction API.** Add to `StorageBackend`:
   ```rust
   async fn begin_transaction(&self, namespace: &str) -> Result<TransactionId, ReasonanceError>;
   async fn commit(&self, tx: TransactionId) -> Result<(), ReasonanceError>;
   async fn rollback_transaction(&self, tx: TransactionId) -> Result<(), ReasonanceError>;
   ```

2. **Implement for JsonFileBackend.** Write-ahead log (WAL) approach: writes go to `.wal` file, commit atomically renames, rollback deletes `.wal`.

3. **Wire session + event log.** The critical use case: when saving a session, the session metadata and its event log must be atomically consistent. Either both are saved or neither.

4. **Crash-recovery test.** Simulate crash (kill process) mid-transaction, verify recovery produces consistent state.

**Exit criteria:**
- [ ] Transaction API in `StorageBackend` trait
- [ ] `JsonFileBackend` implements transactions with WAL
- [ ] Session + event log atomicity verified
- [ ] Crash-recovery test passes

---

#### W1.10 — Dead Code Cleanup (0.12)

**Source spec:** [0.12-dead-code-cleanup.md](phase-0-foundations/0.12-dead-code-cleanup.md)

**Exists:** 62 `#[allow(dead_code)]` annotations across the Rust codebase.

**MUST be done LAST in W1** — W1.1-W1.9 change what is "dead" by wiring previously-orphaned modules.

**Work required:**

1. **Re-audit after wiring.** Many annotations will be naturally resolved by W1.1-W1.9 (modules are now imported and used).

2. **For remaining annotations:**
   - "Used in tests" → move to `#[cfg(test)]` module
   - "Serde deserialization fields" → `#[allow(dead_code)]` acceptable with doc comment explaining serde usage
   - "Genuinely unused" → remove the code
   - "Planned future use" → NOT acceptable. Either wire it now or remove it.

3. **Clippy clean.** `cargo clippy -- -D warnings` must pass with zero dead_code allowances (except documented serde exceptions).

**Exit criteria:**
- [ ] Zero `#[allow(dead_code)]` except documented serde fields
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] No "planned for future" dead code remains

---

#### W1.11 — IPC Batching (NEW)

**Source:** Performance audit 2026-03-27.

**Exists:** Nothing.

**Work required:**

1. **Compound command.** Rust-side `batch_invoke` Tauri command that accepts `Vec<BatchCall>` where each `BatchCall` is `{ command: String, args: Value }`. Executes all calls, returns `Vec<Result<Value, ReasonanceError>>`.

2. **Frontend batch scheduler.** `batchCall()` in adapter that:
   - Accumulates calls within the same microtask (using `queueMicrotask`)
   - Flushes as single `batch_invoke` at end of microtask
   - Returns individual promises to each caller
   - Falls back to individual `invoke()` for single calls (no overhead)

3. **Common batch patterns.** Identify and batch:
   - File open: `read_file` + `get_git_status` + `set_shadow`
   - Project switch: `get_project_state` + `list_sessions` + `get_app_state`
   - Agent send: `agent_send` stays individual (long-running), but pre-flight checks can batch

4. **Zod integration.** Batched responses still go through `validatedInvoke` schemas per-call.

**Exit criteria:**
- [ ] `batch_invoke` command operational
- [ ] Frontend `batchCall()` transparently batches within microtask
- [ ] File open round-trips reduced from 3-4 to 1
- [ ] No regression in single-call latency
- [ ] Zod validation works per-call within batch

---

### W1 Exit Gate

All original Phase 0 exit criteria PLUS:

- [ ] All Rust commands return `Result<T, ReasonanceError>` with structured error enum
- [ ] Performance baselines recorded: startup (ms), idle memory (MB), bundle size (KB), chat render (ms)
- [ ] Event pub/sub handles ≥10k events/sec without backpressure (benchmark)
- [ ] Async subscribers don't block publisher
- [ ] Signal-based refresh replaces all polling intervals
- [ ] No `Arc<Mutex<HashMap>>` without documented weak-ref strategy
- [ ] TOCTOU window eliminated (already done — 0.6)
- [ ] Storage abstraction passes: save/load/migrate/rollback with ≥2 backends
- [ ] Session state + event log atomicity verified (crash-recovery test)
- [ ] Adapter `invoke<T>()` validates 100% of return types at runtime
- [ ] Settings resolution: `workspace > project > user > builtin` with override test
- [ ] Frontend error boundary catches component throws without app crash (already done — 0.11)
- [ ] Zero `#![allow(dead_code)]` remaining (except documented serde)
- [ ] All baselines re-measured — no regression beyond 5%
- [ ] IPC batching reduces multi-call patterns to single round-trip
- [ ] Startup parallelized with measured improvement

---

## Phase W2 — Wire Features

**Gate:** W1 exit gate passed. All foundation modules operational.

### Dependency Graph

```
W2.1 PermissionEngine (1.1)          W2.4 Agent memory (3.1)
  ├→ W2.2 PermTimeout (1.4)          W2.5 Inter-agent comms (3.2)
  └→ W2.3 Per-tool approval (1.3)    W2.6 Model slots (3.5)

W2.7 PTY reconnection (2.3)     ← independent
W2.8 PTY lifecycle (2.5)        ← independent
W2.9 Node registry (3.3)        ← independent
W2.10 State persistence (3.4)   ← independent
W2.11 Normalizer health (NEW)   ← independent
W2.12 Normalizer versioning (NEW) ← depends on W2.11
W2.13 CLI updater feedback (NEW) ← independent
W2.14 Circuit breaker (NEW)     ← independent
```

All top-level items (W2.1, W2.4, W2.5, W2.6, W2.7-W2.10) are independent and can run in parallel. Only W2.2 and W2.3 depend on W2.1.

### Items

---

#### W2.1 — PermissionEngine: Wire into Transport (1.1)

**Source spec:** [1.1-permission-decision-engine.md](phase-1-security/1.1-permission-decision-engine.md)

**Exists:**
- `src-tauri/src/permission_engine.rs` — 6-layer engine, `evaluate()`, 14 tests. Layer 3 (policy file) and 5 (session memory) are stubs.

**Work required:**

1. **Implement Layer 3 (policy file).** Parse `.reasonance/permissions.toml` for project-level permission rules. File format:
   ```toml
   [tools.write_file]
   decision = "allow"
   scope = "project"

   [tools.execute_command]
   decision = "confirm"
   ```

2. **Implement Layer 5 (session memory).** Integrate `PermissionMemory::lookup()` inside `evaluate()` as a layer, not external.

3. **Wire into transport.** Replace inline trust checks in `transport/mod.rs` `send()` with `PermissionEngine::evaluate()`. The engine becomes the single decision point.

4. **Audit events.** Every `evaluate()` call publishes result to EventBus `permission:decision` channel.

5. **Benchmark.** < 1ms per `evaluate()` invocation.

**Exit criteria:**
- [ ] 6 layers functional (hardcoded, trust, policy file, model config, session memory, default)
- [ ] Destructive commands always denied regardless of any setting
- [ ] Untrusted workspaces restricted to read-only tools
- [ ] All decisions emit audit events on EventBus
- [ ] Benchmark: < 1ms per invocation
- [ ] Inline trust checks in transport replaced by engine

---

#### W2.2 — Permission Timeout: Wire Frontend (1.4)

**Source spec:** [1.4-permission-request-timeout.md](phase-1-security/1.4-permission-request-timeout.md)

**Exists:**
- `PermissionTimeoutConfig` (default 5min), `wait_for_permission_decision` Tauri command with polling + auto-deny.

**Work required:**

1. **Frontend call.** When `PermissionRequestBlock` appears, call `wait_for_permission_decision` in background. If it resolves with auto-deny, update chat flow.

2. **Countdown UI.** Timer countdown in `PermissionRequestBlock.svelte` showing remaining seconds. Visual urgency at <30s.

3. **Settings integration.** Timeout value configurable via LayeredSettings (W1.4). Exposed in Settings UI.

4. **Session resume.** After auto-deny, session continues with denied tool result (same as manual deny).

**Exit criteria:**
- [ ] Auto-deny triggers after configurable timeout
- [ ] Countdown visible in UI
- [ ] Timeout configurable in Settings
- [ ] Session resumes after timeout (not stuck)

---

#### W2.3 — Per-Tool Approval: Wire Frontend to Backend (1.3)

**Source spec:** [1.3-per-tool-approval.md](phase-1-security/1.3-per-tool-approval.md)

**Exists:**
- Backend: `PermissionMemory` (Once/Session/Project), 4 Tauri commands, 10 tests.
- Frontend: `PermissionRequestBlock` with Approve/Remember/Deny, but uses local `Set<string>`.

**Work required:**

1. **Wire frontend to backend.** Replace `sessionApprovedTools` Set with calls to `record_permission_decision` / `lookup_permission_decision`.

2. **Deny scopes.** Add "Deny (session)" and "Deny (project)" options to UI. Backend already supports them.

3. **Project persistence.** `Project` scope decisions written to `.reasonance/permissions.toml` (shared with W2.1 Layer 3).

4. **Approval replay fix.** On re-approval, send only the denied tool call, not replay the full message.

**Exit criteria:**
- [ ] All 4 scopes functional: Allow once, Allow session, Allow project, Deny
- [ ] Project-scope decisions persisted to disk
- [ ] Frontend uses backend PermissionMemory (not local Set)
- [ ] Approval replay sends only the denied tool call

---

#### W2.4 — Agent Memory: Wire Runtime + Frontend (3.1)

**Source spec:** [3.1-agent-memory.md](phase-3-intelligence/3.1-agent-memory.md)

**Exists:**
- `src-tauri/src/agent_memory_v2.rs` — SQLite+FTS5, 3-level scoping, importance eviction, v1 migration, 16 tests, 4 Tauri commands.

**Work required:**

1. **Adapter methods.** `memoryAdd`, `memorySearch`, `memoryList`, `memoryGet` in adapter interface + `tauri.ts`.

2. **Wire in agent runtime.** `AgentRuntime` provides memory read/write to executing nodes. Nodes can store context that persists across workflow runs.

3. **EventBus events.** Publish `memory:added`, `memory:evicted`, `memory:searched` on operations.

4. **Frontend UI.** Memory browse/search panel (can be minimal — list + search input + detail view). Accessible from session context.

5. **Benchmarks.** FTS5 search < 50ms on 10k entries. Hot cache read < 1ms.

**Exit criteria:**
- [ ] 3-level scoping functional (node, project, global)
- [ ] Importance-based eviction operational
- [ ] FTS5 search < 50ms on 10k entries (benchmark)
- [ ] V1 JSON migration works without data loss
- [ ] Events emitted on memory operations
- [ ] Frontend can browse and search memories
- [ ] Agent runtime reads/writes memory during execution

---

#### W2.5 — Inter-Agent Comms: Wire Workflow + Edge-Implicit (3.2)

**Source spec:** [3.2-inter-agent-comms.md](phase-3-intelligence/3.2-inter-agent-comms.md)

**Exists:**
- `src-tauri/src/agent_comms.rs` — direct/broadcast/topic, backpressure, TTL, since_id, reply_to, 10 tests, 6 Tauri commands.

**Work required:**

1. **Edge-implicit subscriptions.** When `WorkflowEngine` loads a workflow, for each edge A→B: auto-subscribe node B to node A's output channel. Channel naming: `node:{node_id}:output`.

2. **Wire in workflow engine.** `WorkflowEngine::execute_node()` publishes node output to `AgentCommsBus`. Downstream nodes receive via subscription.

3. **Adapter methods.** `commsSend`, `commsSubscribe`, `commsReceive` in adapter interface.

4. **HiveInspector visualization.** On the Hive canvas, show message flow indicators on edges (pulse animation on active edges, message count badge).

5. **Benchmark.** 1000 msg/sec with 50 nodes.

**Exit criteria:**
- [ ] Direct, broadcast, and topic channels functional
- [ ] Edge-implicit subscriptions auto-wired from workflow edges
- [ ] Workflow nodes communicate via AgentCommsBus during execution
- [ ] Message throughput: 1000 msg/sec across 50 nodes (benchmark)
- [ ] HiveInspector shows message flow on canvas edges

---

#### W2.6 — Model Slots: Wire Consumers + Settings (3.5)

**Source spec:** [3.5-multi-slot-model.md](phase-3-intelligence/3.5-multi-slot-model.md)

**Exists:**
- `src-tauri/src/model_slots.rs` — 4 slots (Chat, Workflow, Summary, Quick), fallback chain, 3 Tauri commands, 10 tests.

**Work required:**

1. **Adapter methods.** `getModelSlot`, `setModelSlot`, `listModelSlots` in adapter interface.

2. **Wire all consumers.** Transport `send()` resolves model via `ModelSlotRegistry::resolve(slot)` instead of directly using config model. Agent runtime and workflow engine same.

3. **Settings UI.** Model slot configuration panel: for each slot, dropdown of available models. Show fallback chain visually.

4. **Per-project override.** Via LayeredSettings (W1.4): project-level `settings.toml` can override slot assignments.

**Exit criteria:**
- [ ] 4 model slots functional: chat, workflow, summary, quick
- [ ] Fallback chain: quick → summary → chat, workflow → chat
- [ ] All model consumers (transport, agent, workflow) resolve via slots
- [ ] Per-project override via LayeredSettings
- [ ] Settings UI for slot configuration

---

#### W2.7 — PTY Reconnection: Wire Frontend (2.3)

**Source spec:** [2.3-reconnection-backoff.md](phase-2-resilience/2.3-reconnection-backoff.md)

**Exists:**
- `ReconnectConfig` (10 attempts, 1s→30s, backoff 2x), `reconnect_pty` Tauri command.

**Work required:**

1. **Adapter method.** `reconnectPty(ptyId: string)` in adapter interface + `tauri.ts`.

2. **Frontend retry logic.** In `Terminal.svelte`: listen for `pty-exit` event, attempt reconnection using `ReconnectConfig` delays between attempts.

3. **UI feedback.** Overlay on terminal: "Reconnecting... attempt X/10" with countdown to next attempt.

4. **Buffer preservation.** Before killing old PTY, read terminal buffer. After respawn, write buffer to new terminal. (May require xterm.js serialization addon.)

**Exit criteria:**
- [ ] PTY reconnection with exponential backoff (1s→30s, 10 attempts)
- [ ] Terminal buffer preserved across respawn
- [ ] UI shows reconnection status
- [ ] After max attempts, show permanent failure with "Retry" button

---

#### W2.8 — PTY Lifecycle: Kill on App Shutdown (2.5)

**Source spec:** [2.5-pty-lifecycle.md](phase-2-resilience/2.5-pty-lifecycle.md)

**Exists:**
- onDestroy kill, orphan sweep 60s, project-level kill — all wired.
- `kill_all_ptys` Tauri command exists but has zero call sites.

**Work required:**

1. **Rust-side shutdown hook.** In `lib.rs` Tauri builder, add `RunEvent::ExitRequested` handler that calls `PtyManager::kill_all()`. This is more reliable than frontend `onWindowClose` (which may not execute if process is killed).

2. **Frontend backup.** Also call `kill_all_ptys` from `+page.svelte` `onWindowClose` as belt-and-suspenders.

**Exit criteria:**
- [ ] Zero orphan PTYs after app close (verify with `ps` after shutdown)
- [ ] Both Rust-side and frontend-side cleanup wired

---

#### W2.9 — Node Registry: Wire Frontend + Validation (3.3)

**Source spec:** [3.3-node-registry.md](phase-3-intelligence/3.3-node-registry.md)

**Exists:**
- `HiveNodeHandler` trait, 3 built-in handlers, `HiveNodeRegistry`, `get_node_types` command, 17 tests.

**Work required:**

1. **Accept runtime registration.** The spec says `hive_nodes!` macro but runtime `register()` is more flexible and already works. Document this as a deliberate deviation — adding a new node type = implement `HiveNodeHandler` + call `registry.register()` in setup.

2. **Wire frontend palette.** Hive canvas node palette calls `get_node_types` to populate available nodes dynamically instead of hardcoding.

3. **Wire workflow validation.** `WorkflowEngine::load_workflow()` calls `registry.validate_config()` for each node in the workflow. Invalid configs prevent execution with clear error.

**Exit criteria:**
- [ ] Node palette populated from registry (not hardcoded)
- [ ] Config validation on workflow load
- [ ] Adding new node type = implement trait + register (documented)

---

#### W2.10 — State Persistence: Terminal Restore (3.4)

**Source spec:** [3.4-state-persistence.md](phase-3-intelligence/3.4-state-persistence.md)

**Exists:**
- `app_state_store.rs` backend, frontend types + adapter + lifecycle wiring on main.

**Work required:**

1. **Terminal state in ProjectState.** Add `terminals: Vec<TerminalState>` where `TerminalState` includes shell command, cwd, and optionally terminal buffer snapshot.

2. **Save on lifecycle events.** When saving project state, include terminal info.

3. **Restore on project load.** On project open, respawn terminals with saved command + cwd. Restore buffer if available.

4. **Debounce saves.** Add debounced auto-save (every 30s while active) in addition to lifecycle event saves. Prevents data loss on crash.

5. **Benchmarks.** Startup restore < 500ms. Session pagination < 50ms on 1000+ sessions.

**Exit criteria:**
- [ ] Last active project restored on app start
- [ ] Open files, active session, terminals restored per project
- [ ] Terminal sessions respawned with correct command + cwd
- [ ] Debounced auto-save every 30s
- [ ] Startup restore < 500ms (benchmark)

---

#### W2.11 — Normalizer Health: Wire Automatic Checks + EventBus (NEW)

**Exists:**
- `src-tauri/src/normalizer_health.rs` — HealthStatus enum, HealthReport with test results, NormalizerHealth registry per provider, 13 tests. Referenced in commands/capability.rs for `get_health_report()` and `get_all_health_reports()`.

**Work required:**

1. **Automatic health checks on startup.** After normalizers are loaded, run health checks for each provider. Publish results to EventBus `normalizer:health` channel.

2. **Automatic health checks on config reload.** When `reload_normalizers` is called, re-run health checks and publish updated status.

3. **EventBus integration.** Publish `normalizer:health-changed` events when a provider's health transitions (e.g., Healthy → Degraded).

4. **Frontend status indicator.** Show health status (green/yellow/red dot) next to each provider in the settings or sidebar. Use existing `get_health_report` / `get_all_health_reports` commands.

**Exit criteria:**
- [ ] Health checks run automatically on startup and config reload
- [ ] Health status changes published to EventBus
- [ ] Frontend shows per-provider health indicator

---

#### W2.12 — Normalizer Versioning: Wire Automatic Backups + Retention (NEW)

**Exists:**
- `src-tauri/src/normalizer_version.rs` — checksummed backups, restore/rollback, index file, 9 tests. Commands: `get_normalizer_versions()`, `rollback_normalizer()`.

**Work required:**

1. **Automatic backup on config change.** When a normalizer TOML is modified (detected via fs watcher or explicit save), create a versioned backup automatically.

2. **Retention policy.** Keep last 20 versions per provider. On backup creation, prune oldest beyond limit.

3. **EventBus integration.** Publish `normalizer:version-created` and `normalizer:rollback` events.

4. **Frontend rollback UI.** Version history list accessible from provider settings. Each entry shows timestamp, checksum, diff preview. "Rollback" button calls existing command.

**Exit criteria:**
- [ ] Automatic backups on normalizer config change
- [ ] Retention policy (max 20 per provider)
- [ ] EventBus events on version create/rollback
- [ ] Frontend version history with rollback button

---

#### W2.13 — CLI Updater: Wire User Notification + Approval (NEW)

**Exists:**
- `src-tauri/src/cli_updater.rs` — `CliUpdater` with version tracking, `run_background_updates()` spawned on startup, `register_from_configs()` called during setup. Background task runs but has no user-facing feedback.

**Work required:**

1. **EventBus notification.** When `run_background_updates()` detects an available update, publish to EventBus `lifecycle:update-available` channel with provider name and versions.

2. **Frontend notification.** Toast or banner when updates are available. "Update now" / "Dismiss" actions.

3. **Update approval flow.** User clicks "Update now" → triggers update via adapter method → shows progress → confirms success/failure.

4. **Settings integration.** Toggle for "Check for updates automatically" via LayeredSettings. Currently always-on.

**Exit criteria:**
- [ ] User notified of available CLI updates via UI
- [ ] Update approval flow (not silent)
- [ ] Auto-check configurable in Settings
- [ ] EventBus events for update lifecycle

---

#### W2.14 — Circuit Breaker: Wire into Transport (NEW)

**Exists:**
- `src-tauri/src/circuit_breaker.rs` — three-state (Closed/Open/HalfOpen), per-context-id, error deduplication, configurable thresholds, 10 tests. Imported but unused in transport/mod.rs.

**Work required:**

1. **Wire into transport send().** Wrap LLM API calls with circuit breaker per provider. If breaker is Open, fail fast with `ReasonanceError::Transport { retryable: true }` instead of making the API call.

2. **Half-open probes.** When breaker transitions to HalfOpen, allow one probe request through. Success → Close, Failure → Open.

3. **EventBus integration.** Publish `transport:circuit-state` events on state transitions (Closed→Open, Open→HalfOpen, HalfOpen→Closed/Open).

4. **Frontend indicator.** Show provider connection status in sidebar or status bar. Red when circuit is Open (provider down), yellow when HalfOpen (probing).

**Exit criteria:**
- [ ] Circuit breaker wraps transport API calls per provider
- [ ] Fast-fail when circuit Open (no wasted API calls)
- [ ] Half-open probe with automatic recovery
- [ ] State transitions published to EventBus
- [ ] Frontend shows provider circuit status

---

### W2 Exit Gate

All original Phase 1, 2, 3 exit criteria PLUS:

- [ ] Permission engine is sole decision point (no inline checks in transport)
- [ ] All permission decisions audited via EventBus
- [ ] Per-tool approval persisted to disk for project scope
- [ ] Permission timeout auto-denies and session resumes
- [ ] Agent memory searchable with < 50ms FTS5 on 10k entries
- [ ] Inter-agent comms auto-wired from workflow edges
- [ ] Model slots used by all model consumers
- [ ] PTY reconnection with backoff operational
- [ ] Zero orphan PTYs on shutdown
- [ ] Node registry populates Hive palette
- [ ] Terminal sessions restored on project load
- [ ] Normalizer health checks run on startup and config reload
- [ ] Normalizer version backups created automatically, retention policy enforced
- [ ] CLI update notifications shown to user, approval flow functional
- [ ] Circuit breaker wraps transport API calls, fast-fail on Open
- [ ] All W1 baselines maintained (≤ 5% regression)

---

## Phase W3 — Complete UX

**Gate:** W2 exit gate passed.

### Dependency Graph

All items are independent. Can be executed in any order or in parallel.

### Items

---

#### W3.1 — Pace Delta: Reset Countdown + Settings (4.4)

**Source spec:** [4.4-pace-delta.md](phase-4-ux-observability/4.4-pace-delta.md)

**Exists:** `paceMetrics` with burnRate, color coding, tooltip.

**Work required:**
1. **Reset countdown.** Display visible countdown to quota reset (time remaining in quota window).
2. **Settings integration.** Quota window duration configurable via LayeredSettings. Currently hardcoded 5h.

**Exit criteria:**
- [ ] Reset countdown visible in AnalyticsBar
- [ ] Quota window configurable in Settings

---

#### W3.2 — API Value Banner: 30-Day Sparkline (4.5)

**Source spec:** [4.5-api-value-banner.md](phase-4-ux-observability/4.5-api-value-banner.md)

**Exists:** `valueMultiplier` hero card.

**Work required:**
1. **30-day data.** Extend analytics store to retain 30 days of daily value multiplier history.
2. **Sparkline component.** SVG sparkline in hero card showing 30-day trend. Reuse pattern from existing 7-day sparklines.

**Exit criteria:**
- [ ] 30-day sparkline displayed in API value hero card
- [ ] Historical data retained for 30 days

---

#### W3.3 — Git Status Icons: Directory Aggregation (4.7)

**Source spec:** [4.7-git-status-icons.md](phase-4-ux-observability/4.7-git-status-icons.md)

**Exists:** 6 status types with colors, ARIA, debounced refresh.

**Work required:**
1. **Directory aggregation.** After `get_git_status` returns file-level statuses, propagate upward: if any child has status, parent directory shows aggregated indicator (e.g., dot or count).
2. **7th status.** Verify all 7 git statuses are handled (M, A, D, R, U, C, ?). Add any missing.

**Exit criteria:**
- [ ] Parent directories show aggregate status indicator
- [ ] All 7 git statuses displayed correctly

---

#### W3.4 — File Ops Undo: Ctrl+Z Context + Move Op (4.8)

**Source spec:** [4.8-file-operation-undo.md](phase-4-ux-observability/4.8-file-operation-undo.md)

**Exists:** Create/Delete/Rename with undo stack, trash-based delete, context menu.

**Work required:**
1. **Move operation.** Add `Move` op type to `FileOpsManager`. Track source + destination for undo.
2. **Ctrl+Z context-aware.** Global keybinding handler checks active focus:
   - Focus in CodeMirror editor → editor undo (existing)
   - Focus in FileTree or no specific editor focus → file ops undo
3. **EventBus integration.** Publish `fileop:execute`, `fileop:undo`, `fileop:redo` events.

**Exit criteria:**
- [ ] 4 operation types: create, delete, rename, move
- [ ] Ctrl+Z context-aware (editor vs file ops)
- [ ] File operation events emitted via EventBus

---

#### W3.5 — Anchor Search: CodeMirror StateField Tracking (4.9)

**Source spec:** [4.9-anchor-based-search.md](phase-4-ux-observability/4.9-anchor-based-search.md)

**Exists:** Stale detection via mtime, re-search badge, dimmed styling.

**Work required:**

1. **CodeMirror StateField.** Create a `searchAnchors` StateField that stores search result positions as CodeMirror `Pos` markers.

2. **Anchor tracking.** When the document is edited, CodeMirror automatically adjusts `Pos` markers through its change mapping. Search result positions stay correct as the user edits.

3. **Integration with FindInFiles.** When search results are loaded, create anchors in all open files. For files not yet open, fall back to stale detection (mtime) until opened.

4. **Navigation.** Click on search result → navigate to anchor position (correct even after edits).

**Exit criteria:**
- [ ] Search result positions tracked through document edits via anchors
- [ ] Click navigates to correct position after edits
- [ ] Stale detection retained as fallback for unopened files
- [ ] No performance regression on editor with many anchors

---

#### W3.6 — Lazy Component Loading (NEW)

**Source:** Performance audit 2026-03-27.

**Exists:** All components imported statically.

**Work required:**

1. **Identify heavy components.** Candidates: `HiveCanvas` (@xyflow/svelte), `AnalyticsDashboard`, `ThemeEditor`, `Settings` modal. Measure bundle contribution of each.

2. **Dynamic imports.** Replace static imports with `{#await import(...)}` pattern in Svelte. Show skeleton/placeholder during load.

3. **Preload hints.** For components likely to be opened soon (e.g., Settings after clicking gear icon), add `<link rel="modulepreload">` hints.

4. **Measure impact.** Compare first meaningful paint before/after. Target: measurable improvement in initial load time.

**Exit criteria:**
- [ ] Heavy components loaded on demand (not at startup)
- [ ] Skeleton/placeholder shown during component load
- [ ] No visible delay when opening lazy-loaded components (preload hints)
- [ ] First meaningful paint improved (measured)

---

### W3 Exit Gate

All original Phase 4 exit criteria PLUS:

- [ ] Pace delta shows reset countdown, configurable window
- [ ] API value banner shows 30-day sparkline
- [ ] Git status icons with directory aggregation
- [ ] File ops: 4 op types, context-aware Ctrl+Z, EventBus events
- [ ] Anchor-based search results track through edits
- [ ] Heavy components lazy-loaded
- [ ] All baselines maintained (≤ 5% regression from W1 baseline)

---

## Global Exit Criteria

When W3 is complete, ALL of the following must be true:

1. **Zero orphaned modules.** Every Rust module in `lib.rs` has at least one non-test consumer.
2. **Zero `Result<T, String>`.** All commands use `ReasonanceError`.
3. **Zero `#[allow(dead_code)]`** except documented serde fields.
4. **100% adapter validation.** All `invoke` calls use Zod-validated returns.
5. **EventBus is sole event system.** No direct `app.emit()` calls outside the bridge subscriber.
6. **Settings resolve through LayeredSettings.** No direct config file reads outside the settings module.
7. **All benchmarks pass.** No metric regressed >5% from W1 baseline.
8. **Master spec dashboard updated.** Reflects true completion status.

---

## Appendix A: Full Dependency Graph

```
W1 — FOUNDATIONS (strict gate)
════════════════════════════════════════════════════════════════
Track A:                    Track B:                Track C:
W1.1 EventBus v2            W1.5 Errors             W1.8 Baselines
  ├→ W1.2 Signal              ├→ W1.6 Storage          + startup //ism
  ├→ W1.3 Weak refs           ├→ W1.7 Zod
  └→ W1.4 Settings            └→ W1.9 Transactions
        └→ W1.11 IPC batch

                    W1.10 Dead code ← LAST
════════════════════════════════════════════════════════════════
                         ↓ (gate)
W2 — FEATURES
════════════════════════════════════════════════════════════════
W2.1 PermEngine ──→ W2.2 Timeout     W2.4 Memory      W2.7 PTY reconnect
               ──→ W2.3 Per-tool     W2.5 Comms       W2.8 PTY lifecycle
                                      W2.6 Slots       W2.9 Node registry
                                      W2.11 NormHealth  W2.10 State persist
                                        └→ W2.12 Vers  W2.13 CLI updater
                                      W2.14 CircBreaker
════════════════════════════════════════════════════════════════
                         ↓ (gate)
W3 — UX COMPLETION
════════════════════════════════════════════════════════════════
W3.1 Pace delta        W3.3 Git dir-agg      W3.5 Anchor search
W3.2 Sparkline 30d     W3.4 File ops undo    W3.6 Lazy components
                     (all independent)
════════════════════════════════════════════════════════════════
                         ↓
              GLOBAL EXIT CRITERIA MET
```

## Appendix B: Item Cross-Reference

| Wiring Item | Original Roadmap Item | Original Spec |
|-------------|----------------------|---------------|
| W1.1 | 0.3 | [0.3-event-pub-sub.md](phase-0-foundations/0.3-event-pub-sub.md) |
| W1.2 | 0.4 | [0.4-background-task-signals.md](phase-0-foundations/0.4-background-task-signals.md) |
| W1.3 | 0.5 | [0.5-weak-references.md](phase-0-foundations/0.5-weak-references.md) |
| W1.4 | 0.10 | [0.10-layered-settings.md](phase-0-foundations/0.10-layered-settings.md) |
| W1.5 | 0.1 | [0.1-structured-error-types.md](phase-0-foundations/0.1-structured-error-types.md) |
| W1.6 | 0.7 | [0.7-storage-abstraction.md](phase-0-foundations/0.7-storage-abstraction.md) |
| W1.7 | 0.9 | [0.9-runtime-type-validation.md](phase-0-foundations/0.9-runtime-type-validation.md) |
| W1.8 | 0.2 | [0.2-performance-baseline.md](phase-0-foundations/0.2-performance-baseline.md) |
| W1.9 | 0.8 | [0.8-transaction-semantics.md](phase-0-foundations/0.8-transaction-semantics.md) |
| W1.10 | 0.12 | [0.12-dead-code-cleanup.md](phase-0-foundations/0.12-dead-code-cleanup.md) |
| W1.11 | NEW | — |
| W2.1 | 1.1 | [1.1-permission-decision-engine.md](phase-1-security/1.1-permission-decision-engine.md) |
| W2.2 | 1.4 | [1.4-permission-request-timeout.md](phase-1-security/1.4-permission-request-timeout.md) |
| W2.3 | 1.3 | [1.3-per-tool-approval.md](phase-1-security/1.3-per-tool-approval.md) |
| W2.4 | 3.1 | [3.1-agent-memory.md](phase-3-intelligence/3.1-agent-memory.md) |
| W2.5 | 3.2 | [3.2-inter-agent-comms.md](phase-3-intelligence/3.2-inter-agent-comms.md) |
| W2.6 | 3.5 | [3.5-multi-slot-model.md](phase-3-intelligence/3.5-multi-slot-model.md) |
| W2.7 | 2.3 | [2.3-reconnection-backoff.md](phase-2-resilience/2.3-reconnection-backoff.md) |
| W2.8 | 2.5 | [2.5-pty-lifecycle.md](phase-2-resilience/2.5-pty-lifecycle.md) |
| W2.9 | 3.3 | [3.3-node-registry.md](phase-3-intelligence/3.3-node-registry.md) |
| W2.10 | 3.4 | [3.4-state-persistence.md](phase-3-intelligence/3.4-state-persistence.md) |
| W3.1 | 4.4 | [4.4-pace-delta.md](phase-4-ux-observability/4.4-pace-delta.md) |
| W3.2 | 4.5 | [4.5-api-value-banner.md](phase-4-ux-observability/4.5-api-value-banner.md) |
| W3.3 | 4.7 | [4.7-git-status-icons.md](phase-4-ux-observability/4.7-git-status-icons.md) |
| W3.4 | 4.8 | [4.8-file-operation-undo.md](phase-4-ux-observability/4.8-file-operation-undo.md) |
| W3.5 | 4.9 | [4.9-anchor-based-search.md](phase-4-ux-observability/4.9-anchor-based-search.md) |
| W2.11 | NEW | — (normalizer_health.rs) |
| W2.12 | NEW | — (normalizer_version.rs) |
| W2.13 | NEW | — (cli_updater.rs) |
| W2.14 | NEW | — (circuit_breaker.rs) |
| W3.6 | NEW | — |

## Appendix C: Phase 5 Backlog (Post W1-W3)

Items deferred to after all wiring is complete:

| Item | Module | Rationale |
|------|--------|-----------|
| Self-healing normalizers | `self_heal.rs` (192 LOC, 6 tests) | LLM-guided auto-repair of broken TOML configs. Valuable polish feature but not critical path. Keep code in codebase, wire after W3. |
| W1.10 Dead code cleanup | — | Final audit after W2+W3 wiring. Most current "dead code" is built-but-unwired code that W2/W3 will connect. Deferred to avoid deleting needed code. |
