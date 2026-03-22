# CTO Technical Audit Report

**Date:** 2026-03-22
**Persona:** Technical leader evaluating production adoption
**Judgment:** Would I trust this in production for my team?
**Codebase:** Reasonance v0.5.0 -- Svelte 5 + Tauri 2 + Rust desktop IDE

---

## Executive Summary

Reasonance is a well-architected multi-provider AI IDE with a clean layered Rust backend and a reactive Svelte 5 frontend. The architecture demonstrates strong design instincts: proper separation of concerns, a declarative normalizer pipeline with TOML-driven configuration, session persistence with atomic writes, and thoughtful security boundaries (path validation, command allowlists, env var allowlists).

However, production readiness is gated by three systemic issues: (1) pervasive `lock().unwrap()` on mutexes (89 occurrences) means any poisoned mutex causes a process panic -- this is the single biggest risk; (2) absence of error boundaries in the Svelte frontend means a single rendering exception can blank the entire UI; (3) the `list_dir` and `grep_files` IPC commands lack path validation, creating a directory traversal vulnerability that bypasses the otherwise solid project-root sandboxing.

The codebase is at a strong beta quality level. With targeted hardening on the three issues above, it would be production-ready.

---

## Architecture Assessment

### Strengths

1. **Clean layered architecture.** The Rust backend is organized into well-defined layers: `transport` (process management, streaming), `normalizer` (protocol translation), `analytics` (metrics collection), `commands` (IPC boundary). Each layer has a single responsibility and clear interfaces.

2. **Declarative normalizer system.** The three-stage pipeline (Rules Engine -> State Machine -> Content Parser) with TOML-driven configuration is excellent. Adding a new LLM provider requires only a TOML file and optionally a state machine implementation. The `NormalizerRegistry` supports hot-reload via `reload_provider()`. This is production-grade extensibility.

3. **Security-conscious design.** Multiple SEC-annotated mitigations are in place:
   - `validate_read_path` / `validate_write_path` enforce project-root sandboxing (`commands/fs.rs:45-103`)
   - `ENV_VAR_ALLOWLIST` prevents arbitrary env var leakage (`commands/system.rs:52-70`)
   - `ALLOWED_SHELLS` + config-derived command allowlist prevents arbitrary command execution (`commands/pty.rs:6-51`)
   - `open_external` rejects non-HTTP schemes (`commands/system.rs:43-48`)
   - Config files store env var names, not actual secrets (`config.rs:48-53`)

4. **Robust session persistence.** `SessionStore` uses atomic writes (write-to-tmp-then-rename) for metadata and index files (`session_store.rs:87-95`). Events are append-only JSONL. The `SessionHistoryRecorder` batches metadata writes every 10 events to reduce I/O (`event_bus.rs:175`). Fork support with event slicing is well-implemented.

5. **Comprehensive Rust test coverage.** 284 `#[test]` functions across 42 test modules covering all backend layers -- transport, normalizer, analytics, commands, state machines, session management.

6. **Proper CI/CD pipeline.** Conventional-commit-based semantic versioning, cross-platform builds (Linux/macOS/Windows), artifact signing with `TAURI_SIGNING_PRIVATE_KEY`, auto-updater manifest (`latest.json`), and `[skip ci]` support.

### Concerns

1. **Mutex poisoning is unhandled everywhere.** Every `lock().unwrap()` (89 in production code) will panic if any thread holding that mutex panics first. In a multi-threaded Tauri app where the event bus fan-out can trigger subscriber panics, this creates cascading failure risk. The correct pattern is `lock().unwrap_or_else(|e| e.into_inner())` or explicit error propagation.

2. **No frontend error boundaries.** There are zero Svelte error boundary components. A single runtime exception in any component (e.g., a malformed event from a new provider) will crash the entire webview. For a desktop app that manages live LLM sessions, this is unacceptable.

3. **In-memory analytics scaling.** `AnalyticsStore` loads all historical metrics into a `Vec<SessionMetrics>` on startup (`store.rs:47-70`) and `all_completed()` clones the entire vector on every query (`store.rs:43-45`). For a power user with thousands of sessions, this will degrade to OOM or multi-second freezes. The module name references SQLite but uses JSONL -- this should be migrated.

4. **Svelte stores create new `Map` instances on every update.** The `agentEvents` and `agentSessions` stores (`agent-events.ts:60-65`, `agent-session.ts:32-36`) create `new Map(map)` on every event. During active streaming (hundreds of events/second), this produces significant GC pressure and potential frame drops.

### Risks

1. **Path validation gap:** `list_dir` (`commands/fs.rs:135`) and `grep_files` (`commands/fs.rs:188`) do NOT call `validate_read_path`. A compromised or malicious frontend webview could enumerate any directory on the filesystem. Since Tauri IPC is the trust boundary, this is a security vulnerability.

2. **Stderr is silenced for CLI processes.** `cmd.stderr(Stdio::null())` (`transport/mod.rs:99`) means CLI error output is permanently lost. If a provider CLI prints errors to stderr (most do), the user gets no feedback -- the session just hangs or terminates silently.

3. **No backpressure in the event bus.** `AgentEventBus::publish` (`event_bus.rs:48-59`) holds the subscriber lock for the entire fan-out. A slow subscriber (e.g., disk I/O in `SessionHistoryRecorder`) blocks all other subscribers, including the `FrontendEmitter`. This can cause visible UI freezes during disk contention.

---

## Layer-by-Layer Findings

### Rust Backend (`lib.rs`, `main.rs`, `agent_runtime.rs`, `config.rs`, `discovery.rs`)

**Module structure:** Clean, 18 modules declared in `lib.rs`. The `run()` function in `lib.rs` is a single chain of `.manage()` and `.plugin()` calls -- readable and auditable.

**Startup panics:** Four `.expect()` calls during initialization (`lib.rs:62-82`) for normalizer loading, session manager, and analytics store. These are acceptable -- the app cannot function without these subsystems. However, the `unwrap_or_else(|| PathBuf::from("."))` fallback for `dirs::data_dir()` (`lib.rs:66`) is questionable: writing session data to the CWD (which may be read-only or `/`) will silently fail.

**Agent runtime state machine:** Well-designed with explicit valid transitions and exhaustive matching (`agent_runtime.rs:59-70`). The `lock().unwrap()` pattern is used throughout but the state is simple enough that poisoning is unlikely in isolation.

**Discovery:** `scan_cli()` (`discovery.rs:155-201`) calls `which`/`where` synchronously on the main thread for 9 candidates. This takes ~100ms typically but could block longer. `probe_apis()` has a proper 3-second timeout for Ollama detection.

### Transport Layer

**Process lifecycle:** The transport spawns CLI processes via `tokio::process::Command`, pipes stdout through a `BufReader` line-by-line into the normalizer pipeline, and publishes events via the event bus (`transport/mod.rs:96-130`). Process cleanup uses `AbortHandle` on the spawned tokio task. This is correct but note: aborting the task does not necessarily kill the child process -- the child may become orphaned.

**Stream reader:** Clean implementation (`stream_reader.rs`). Uses a oneshot channel to communicate the final `StreamResult` back. Error on I/O sets the error field but does not panic. Empty lines are skipped. The pipeline `Mutex` is locked per-line, which is fine for line-by-line processing.

**Retry policy:** Well-structured with exponential backoff, configurable via TOML, and `saturating_mul`/`saturating_pow` to prevent overflow (`retry.rs:74-76`). However, the retry logic is defined but never actually invoked -- `StructuredAgentTransport::send()` does not implement retry loops. The `RetryPolicy` is constructed and stored (`transport/mod.rs:37-41`) but never called. This is dead code.

**Session management:** `SessionManager` properly separates concerns between the in-memory recorder, disk store, and index. `finalize_session` carefully acquires locks in a defined order to avoid deadlocks (`session_manager.rs:141-177`). The `fork_session` implementation correctly validates the fork index and copies events up to the fork point.

**Event bus:** Synchronous fan-out with no backpressure. The lock is held during `publish()` which means subscribers cannot be added/removed during event delivery. `FrontendEmitter` uses `let _ = self.app_handle.emit(...)` to fire-and-forget, which is correct -- frontend emission should never block the pipeline.

### Normalizer Layer

**Pipeline design:** The three-stage pipeline (Rules -> StateMachine -> ContentParser) is well-factored (`pipeline.rs`). JSON parsing failures return empty events rather than errors, which is the correct behavior for a streaming normalizer.

**Rules engine:** Custom expression evaluator supporting `==`, `!=`, `&&`, `||`, `exists()`. The `split_operator` function handles quoted strings correctly (`rules_engine.rs:56-69`). Array indexing is supported (`content[0].text`). The evaluator is simple but sufficient -- no regex or complex predicates needed at this level.

**State machines:** Five provider-specific implementations (Claude, Gemini, Kimi, Qwen, Codex) plus Generic pass-through. All share a common `ToolInputAccumulator` via the `accumulator.rs` module, which is good code reuse. The `TimedFlush` mechanism (10-second timeout) prevents tool events from being stuck forever if the CLI crashes mid-sequence -- this is a thoughtful edge case handler.

**Content parser:** Detects code fences and unified diffs in text content (`content_parser.rs`). The `parse_content_blocks` function has one `unwrap()` on line 8 that is safe (guaranteed by `blocks.len() == 1` check), but should still be replaced with `expect("single block guaranteed")` for documentation.

**Extensibility:** Adding a new provider requires: (1) a TOML config in `normalizers/`, (2) optionally a state machine in `state_machines/`. The registry auto-loads all `.toml` files from the directory. This is production-grade plugin architecture.

### Analytics

**Collector design:** Event-driven via the `AgentEventSubscriber` trait, with an `EventFilter` that only subscribes to Usage/Metrics/ToolUse/Error/Done events (`collector.rs:346-357`). This is efficient -- text events are not processed.

**Data integrity:** The collector correctly handles the "error recovery" pattern: when a non-error event follows an error, the previous error is marked as recovered (`collector.rs:279-285`). The `Done` event flushes active metrics to the store and drops the lock before performing I/O (`collector.rs:260`).

**Query performance:** All queries (provider analytics, model breakdown, daily stats) iterate the full `all_completed()` vector with cloning. For small datasets (<1000 sessions), this is fine. At scale, this needs indexing -- either SQLite migration or at minimum an in-memory index by provider/date.

**Cost tracking:** `unix_secs_to_date_string` implements the Hinnant date algorithm directly rather than pulling in chrono for this single use case (`collector.rs:230-249`). This is commendable -- no unnecessary dependency for a simple conversion. However, the budget system in the frontend (`analytics.ts:211`) uses a crude `tokens * 0.00001` cost estimate rather than the actual `total_cost_usd` from the backend.

### Commands / IPC

**Trust boundary analysis:** Tauri IPC is the trust boundary. The frontend is untrusted (webview). Analysis of all 50+ commands:

- **Properly validated:** `read_file`, `write_file` (project-root sandboxed), `spawn_process` (command allowlist), `open_external` (scheme allowlist), `get_env_var` (env var allowlist)
- **Missing validation:** `list_dir` and `grep_files` accept arbitrary paths without calling `validate_read_path`. `grep_files` has a 500-result cap but no path restriction.
- **Implicitly safe:** Session/analytics/capability commands operate on UUIDs and opaque identifiers -- no path traversal risk. `agent_send` passes through to the transport which validates the provider name.

**Input validation gaps:**
- `session_rename` accepts arbitrary strings for `title` with no length limit
- `agent_send` passes `AgentRequest.prompt` directly into CLI args via template substitution (`transport/mod.rs:173-177`). If the prompt contains shell metacharacters and the CLI binary doesn't handle them safely, this could be a command injection vector. The `Stdio::piped()` approach mitigates this somewhat since the prompt goes through arg passing, not shell expansion.

### Svelte State Management

**Store architecture:** 13 stores using Svelte's `writable`/`derived`. All use immutable update patterns (spread operators, `new Map()`). No direct mutations.

**Reactivity correctness:** The `derived` stores in `engine.ts` chain properly: `currentRun -> nodeStates -> completedNodeCount/activeNodeCount/errorNodeCount -> statusSummary`. This is correct Svelte 5 reactivity.

**Race conditions:** The `processAgentEvent` function in `agent-events.ts:56-96` updates multiple stores (`agentEvents`, `agentSessions`) non-atomically. If two events arrive near-simultaneously on different Tauri event listeners, the Map copy-on-write pattern prevents data corruption but may cause stale intermediate renders. In practice, Svelte's microtask batching mitigates this.

**Memory leaks:**
- `agentEvents` (`agent-events.ts:6`) is a `Map<string, AgentEvent[]>` that grows unboundedly. Events are never evicted for active sessions. For long-running sessions (thousands of events), this will consume significant memory.
- `clearSessionEvents` exists but must be called explicitly -- there is no automatic cleanup on session close.
- The `startLiveTracking` function in `analytics.ts:125-201` properly returns an unsubscribe function, and the `costHistory` array is time-windowed to 30 seconds. This is correct.

**Subscription cleanup:** Only 7 `onDestroy` calls found across all Svelte components. The Tauri event listener registered in `processAgentEvent` integration is the main concern -- if the component that sets up the listener is destroyed without cleanup, events will continue to be processed against stale stores.

### Build Pipeline

**CI/CD quality:** The release workflow (`release.yml`) is well-structured:
- Conventional commit analysis for semantic versioning
- Cross-platform matrix build (Linux, macOS ARM64, Windows)
- Rust cache via `swatinem/rust-cache`
- Artifact signing with Tauri keys
- Auto-updater manifest generation

**Gaps:**
- No `cargo test` or `npm test` in the CI pipeline. The `build` job compiles but never runs tests. Tests exist (284 Rust tests, 46 frontend test files) but are not gated.
- No `cargo clippy` or linting step.
- No `cargo audit` in CI -- the 19 warnings (all unmaintained GTK3 bindings from Tauri's dependency tree) would be caught but the 0 vulnerabilities result confirms this is not urgent.
- Build triggers on every push to main, not on tags -- this means the version bump commit triggers another build (mitigated by `[skip ci]` in the commit message, but fragile).

---

## Code Quality Metrics

| Metric | Value | Assessment |
|--------|-------|------------|
| Production `unwrap()` count | 104 total (89 mutex locks, 15 other) | CONCERN -- mutex panics cascade |
| `expect()` count (non-test) | 4 (all in `lib.rs` startup) | ACCEPTABLE -- startup-only |
| Rust test functions | 284 across 42 modules | STRONG |
| Frontend test files | 46 (unit + a11y + visual + interaction) | STRONG |
| Tests in CI | 0 (not wired into release pipeline) | CRITICAL GAP |
| npm audit | 3 low severity (cookie in @sveltejs/kit) | LOW RISK |
| cargo audit vulnerabilities | 0 | CLEAN |
| cargo audit warnings | 19 (unmaintained GTK3 bindings via Tauri) | EXPECTED -- Tauri dependency |
| Error boundaries (frontend) | 0 | MISSING |
| `Stdio::null()` on stderr | 1 (transport/mod.rs:99) | CONCERN -- silent failures |
| Dead code (retry logic) | 1 module (retry.rs used but never invoked) | MINOR |
| Path validation coverage | 2/4 FS commands validated | SECURITY GAP |
| IPC commands total | 50+ | Reasonable surface area |

---

## Production Readiness Verdict

**Conditional YES** -- with three mandatory fixes before production deployment:

1. **Fix mutex unwrap pattern** (effort: 2-3 days). Replace all 89 `lock().unwrap()` calls with either `lock().unwrap_or_else(|e| e.into_inner())` (if poisoned state is recoverable) or propagate errors via `Result`. Priority: the event bus and transport session map, where concurrent access is highest.

2. **Add path validation to `list_dir` and `grep_files`** (effort: 1 hour). Both need a `validate_read_path` call or equivalent project-root check. Without this, the IPC sandbox has a hole.

3. **Add a Svelte error boundary** (effort: 1 day). Wrap the main layout in an error boundary component that catches render exceptions and shows a recovery UI rather than blanking the webview.

**Strongly recommended but not blocking:**

4. Wire `cargo test` and `npm test` into CI before the build step.
5. Capture stderr from CLI processes (at minimum log it, ideally surface to the user as a warning event).
6. Implement the retry loop in the transport layer -- the policy infrastructure exists but is unused.
7. Add event eviction or windowing to the `agentEvents` store to prevent memory growth in long sessions.
8. Migrate analytics from JSONL+Vec to SQLite for query performance at scale.

---

## Technical Debt Register

| # | Severity | Location | Issue | Suggested Fix |
|---|----------|----------|-------|---------------|
| 1 | CRITICAL | `src-tauri/src/**/*.rs` (89 sites) | `lock().unwrap()` on all Mutex acquisitions -- poisoned mutex = process panic | Replace with `lock().unwrap_or_else(\|e\| e.into_inner())` or propagate `Result` |
| 2 | HIGH | `commands/fs.rs:135,188` | `list_dir` and `grep_files` skip `validate_read_path` -- directory traversal via IPC | Add `validate_read_path` check using `ProjectRootState` |
| 3 | HIGH | Frontend (all components) | No Svelte error boundary -- single exception blanks entire UI | Add `<ErrorBoundary>` wrapper component |
| 4 | HIGH | `.github/workflows/release.yml` | No tests in CI pipeline despite 284 Rust + 46 JS test files | Add `cargo test` and `npm test` steps before build |
| 5 | MEDIUM | `transport/mod.rs:99` | `stderr(Stdio::null())` silences all CLI error output | Capture stderr, surface as warning events or log |
| 6 | MEDIUM | `analytics/store.rs:43-45` | `all_completed()` clones entire metrics vector on every query | Migrate to SQLite or add indexed queries |
| 7 | MEDIUM | `agent-events.ts:60-65` | `new Map(map)` on every event during streaming -- GC pressure | Use mutable Map with explicit `invalidate()` trigger, or batch updates |
| 8 | MEDIUM | `agent-events.ts:6` | `agentEvents` Map grows unboundedly -- no eviction | Add event windowing (e.g., keep last 5000 per session) |
| 9 | MEDIUM | `transport/mod.rs`, `retry.rs` | RetryPolicy is constructed and stored but never invoked in send() | Implement retry loop in `send()` or remove dead code |
| 10 | MEDIUM | `transport/mod.rs:110-122` | `child.wait()` does not guarantee child process termination on abort | Send SIGTERM/SIGKILL to child PID before aborting task |
| 11 | LOW | `lib.rs:66,77,89` | `dirs::data_dir().unwrap_or(PathBuf::from("."))` -- fallback to CWD may be read-only | Fail explicitly or use a guaranteed writable fallback |
| 12 | LOW | `normalizer/content_parser.rs:8` | `blocks.into_iter().next().unwrap()` -- safe but undocumented invariant | Replace with `.expect("guaranteed by len check")` |
| 13 | LOW | `analytics.ts:211` | Budget cost uses `tokens * 0.00001` estimate instead of actual `total_cost_usd` | Use `total_cost_usd` from backend metrics |
| 14 | LOW | `commands/llm.rs:128-129` | Google API key passed in URL query parameter (visible in logs/network traces) | Use request header authentication if supported |
| 15 | LOW | `release.yml` | No `cargo clippy` or lint step in CI | Add `cargo clippy -- -D warnings` step |
| 16 | LOW | `Cargo.toml` | 19 unmaintained GTK3 crate warnings from Tauri's Linux dependency tree | Track Tauri's migration to GTK4 (not actionable now) |
| 17 | LOW | `commands/session.rs:49-55` | `session_rename` accepts unbounded title length | Add title length validation (e.g., max 500 chars) |
