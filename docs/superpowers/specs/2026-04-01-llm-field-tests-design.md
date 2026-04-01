# LLM Field Tests Design v2

**Date:** 2026-04-01
**Status:** Approved (v2 — revised after self-review)
**Scope:** 15 field tests exercising the full LLM stack (transport, normalizer, session, agent, analytics, UI)

## Context

The field test suite has 67 scenarios, of which 5 were skipped because they require live LLM providers. Those 5 stubs had no real implementation. This spec defines 15 LLM-integrated field tests that replace and extend the original 5.

Both `claude` and `qwen` CLIs are installed and configured with API keys.

## v2 Changes from v1

Fixed 14 issues identified in self-review:
1. Log patterns now extracted from actual Rust source (verified, not assumed)
2. Permission denial test redesigned to handle CLI stdin blocking
3. Session restart test uses explicit app re-launch protocol
4. cross_52 and cross_52b merged into single test with progressive assertions
5. Concurrent agents test uses `threading.Thread` for real parallelism
6. UI verification uses frontend `console.debug` from `[batch]` log + log markers, not screenshot OCR
7. Added analytics pipeline verification test
8. Added streaming UX verification test
9. Added session rename/delete lifecycle test
10. Added cache token verification
11. Tool use test made provider-aware (Claude-only, Qwen has different tools)
12. Circuit breaker cleanup uses `finally` block even on test failure
13. All log grep patterns verified against source and documented in appendix
14. e2e_10c fork trigger changed from ambiguous "UI or IPC" to explicit IPC-via-log approach

## Test Architecture

Tests use a 3-level verification strategy:

1. **Log parsing** — grep on Rust logs (`RUST_LOG=reasonance_lib=trace`) for verified patterns (see Appendix A). This is the primary backend verification mechanism.
2. **Backend assertions** — verify event types, token counts, session state, pipeline correctness by parsing structured log lines.
3. **UI verification** — dotool automation to send messages via the chat panel + screenshots. Frontend console logs (`[batch]` debug lines) confirm IPC calls succeeded. For "text rendered" checks, verify the `[batch]` log shows `agent_get_events` returning events with `output_tokens > 0`, plus screenshot shows non-empty chat area.

Each test follows:
```
Setup → Action → Backend assertion (log) → UI assertion (screenshot + console) → Cleanup (finally)
```

## Shared Helper: `tests/field/lib/llm.py`

| Function | Purpose |
|----------|---------|
| `send_chat(ctx, provider, prompt, yolo=True)` | Open session panel, select provider, send prompt via dotool, return when log shows `Transport: session=.* started` or timeout |
| `wait_for_log(ctx, pattern, timeout=60)` | Poll `ctx.app.logs()` for a regex pattern, return match or raise on timeout |
| `extract_session_id(ctx)` | Regex on log for `Transport: send request .* session_id=(\S+)` |
| `extract_cli_pid(ctx, session_id)` | Regex on log for `Transport: spawning CLI .* attempt=` then get PID from process list |
| `wait_for_done(ctx, session_id, timeout=60)` | Poll log for `StreamReader\[{session_id}\]: emitting.*Done` |
| `count_log_events(ctx, session_id)` | Count `StreamReader\[{session_id}\]: emitting (\S+)` lines grouped by type |
| `assert_log_has(ctx, pattern, msg)` | Assert pattern exists in log, with descriptive failure message |
| `assert_analytics_recorded(ctx, session_id)` | Verify `Session ended: session_id={session_id}` appears in log with `output_tokens > 0` |
| `cleanup_session(ctx, session_id)` | Best-effort stop: send `agent_stop` via UI keybinding or log the session_id for manual cleanup |

## Test Definitions

### Group A: Chat base (e2e suite)

#### e2e_10 — Chat with real LLM
- **Provider:** Claude (fallback Qwen if Claude spawn fails)
- **Prompt:** `"Analyze this Rust snippet and suggest one improvement: fn add(a: i32, b: i32) -> i32 { return a + b; }"`
- **Backend assertions:**
  - Log contains `Transport: send request provider=claude`
  - Log contains `Transport: spawning CLI`
  - Log contains `StreamReader[...]: emitting` with at least 1 `Text` type
  - Log contains `StreamReader[...]: emitting` with `Done` type
  - Log contains `Session ended:` with `output_tokens=N` where N > 0
- **UI:** screenshot before/after send. Verify `[batch]` log shows `agent_get_events` call succeeded (no error in batch result).
- **Timeout:** 60s
- **Cleanup:** `cleanup_session`

#### e2e_10b — Session persistence across restart
- Phase 1: send prompt to Claude via `send_chat`, capture `session_id` via `extract_session_id`, wait for `Done`
- Phase 2: `ctx.app.kill()` → `ctx.app.launch()` → `ctx.app.wait_ready(timeout=120)`
- Phase 3: search NEW log for `SessionManager: loaded .* existing sessions` with count >= 1
- Phase 4: verify the old `session_id` appears in `SessionManager: restoring session=` when session panel is opened
- **Note:** this test creates a fresh app instance. The runner's `ctx.app` is reused — `kill()` and `launch()` are called on the same `ReasonanceApp` object, which handles log file rotation.
- **Timeout:** 150s
- **Cleanup:** `cleanup_session`

#### e2e_10c — Session fork (via log-driven IPC)
- Phase 1: send prompt, wait for `Done`, capture `session_id`
- Phase 2: trigger fork by simulating Ctrl+Shift+H (open session panel) → find fork button via dotool
- Phase 3: verify log shows `SessionManager: forking session={session_id}` and `SessionManager: forked session=.* -> new session=(\S+) with (\d+) events`
- Phase 4: capture forked `session_id`, verify event count < original
- **Timeout:** 90s
- **Cleanup:** cleanup both sessions

#### e2e_10d — Tool use round-trip (Claude only)
- **Provider:** Claude (yolo mode) — Qwen CLI doesn't support the same file tools
- **Prompt:** `"Read the file src-tauri/Cargo.toml and tell me the project version number"`
- **Backend assertions:**
  - Log contains `Pipeline[claude]: matched rule` at least 2 times (tool_use + text)
  - `count_log_events` shows at least 1 `ToolUse` and 1 `ToolResult`
  - Final `Text` event references a version number
- **UI:** screenshot shows tool use block in chat
- **Timeout:** 90s
- **Cleanup:** `cleanup_session`

#### e2e_10e — Permission flow (observe, don't block)
- **Redesign:** instead of `ask` mode (which blocks on stdin), use `yolo` mode but verify the permission_args are constructed. The test verifies the permission ENGINE evaluates correctly, not the stdin prompt.
- **Provider:** Claude (yolo mode)
- **Prompt:** simple prompt that triggers tool use
- **Backend assertions:**
  - Log contains `Transport: permission_args=` showing the constructed args
  - Log contains `Transport: trust_level=` showing trust evaluation
  - If any `Permission:` debug lines appear, verify they resolve to Allow (since yolo)
- **Note:** a full stdin permission test would require PTY interaction with the CLI process, which is beyond field test scope. Permission denial is already covered by 620 Rust unit tests.
- **Timeout:** 60s
- **Cleanup:** `cleanup_session`

#### e2e_10f — Streaming UX verification (NEW)
- **Provider:** Claude
- **Prompt:** `"Write a detailed explanation of how quicksort works, step by step"` (forces a long response)
- **Backend assertions:**
  - Multiple `StreamReader[...]: emitting Text` lines appear BEFORE the `Done` line (proves streaming, not batch)
  - Time delta between first `Text` emission and `Done` emission is > 2s (confirms progressive delivery)
- **UI:** take 2 screenshots: one 3s after sending (should show partial response), one after `Done` (should show complete response). Both should show non-empty chat area.
- **Timeout:** 90s
- **Cleanup:** `cleanup_session`

#### e2e_10g — Session rename and delete lifecycle (NEW)
- Phase 1: send prompt, wait for `Done`, capture `session_id`
- Phase 2: rename via UI (session panel → rename action)
- Phase 3: verify log shows `SessionManager: renaming session=` and `SessionManager: session=.* renamed`
- Phase 4: delete the session via UI
- Phase 5: verify log shows `SessionManager: deleting session=` and `SessionManager: session=.* deleted`
- **Timeout:** 90s
- **No cleanup needed** (session is deleted as part of the test)

### Group B: Cross-feature

#### cross_49 — Agent writes file, tree updates
- **Provider:** Claude (yolo mode)
- **Prompt:** `"Create a file at /tmp/reasonance-test-output.txt with the content 'field test ok'"`
- **Backend assertions:**
  - `count_log_events` shows `ToolUse` + `ToolResult`
  - File `/tmp/reasonance-test-output.txt` exists on disk
  - File content is `field test ok` (or contains it)
- **UI:** screenshot after completion
- **Timeout:** 90s
- **Cleanup:** `os.unlink('/tmp/reasonance-test-output.txt')` in `finally` block

#### cross_52 — Multi-provider normalizer + schema parity (merged from cross_52 + cross_52b)
- **Prompt:** `"Explain in 2 sentences what a binary search tree is"`
- Phase 1: send to Claude, capture `session_id_claude`, wait for `Done`
- Phase 2: send to Qwen, capture `session_id_qwen`, wait for `Done`
- **Backend assertions (both providers):**
  - Log contains `Pipeline[claude]: matched rule` and `Pipeline[qwen]: matched rule` (each matched >= 1 rule)
  - `count_log_events` for each session shows at least `Text` and `Done`
  - Both have `Session ended:` log with `output_tokens > 0`
- **Schema parity assertions (warnings, not failures):**
  - Both sessions produced same set of event types (Text, Done at minimum)
  - Both have `input_tokens` and `output_tokens` in their `Session ended:` line
  - Log any differences in event type sets as warnings
- **Timeout:** 120s
- **Cleanup:** cleanup both sessions

### Group C: Stress & resilience

#### stress_30 — Multiple concurrent agents
- Launch 3 sessions in parallel using `threading.Thread`:
  - Thread 1: Claude, prompt `"List 5 sorting algorithms"`
  - Thread 2: Claude, prompt `"List 5 data structures"`
  - Thread 3: Qwen, prompt `"List 5 design patterns"`
- Each thread calls `send_chat` + `wait_for_done` independently
- **Backend assertions:**
  - 3 distinct `Transport: send request` lines in log
  - 3 distinct `Session ended:` lines (one per session_id)
  - No `ERROR` or `panicked` lines between the first spawn and last Done
- **Measure:** total wall time logged. Verify < 120s (no deadlock).
- **Timeout:** 120s
- **Cleanup:** cleanup all 3 sessions

#### stress_30b — Circuit breaker trip and recovery
- Phase 1 (trip): force 3 consecutive failures
  - Send `agent_send` to Claude, immediately `extract_cli_pid` and `kill -9` the PID
  - Repeat 3 times, waiting 1s between each
  - After 3 kills, verify log shows circuit breaker open state (provider spawn fails with "circuit open" or similar transport error)
- Phase 2 (cooldown): sleep 65s (circuit breaker default cooldown is 60s)
- Phase 3 (recovery): send normal `agent_send` to Claude
  - Verify spawn succeeds (log shows `Transport: spawning CLI`)
  - Wait for `Done` event
- **Cleanup (in finally):** always send a successful prompt at the end to ensure circuit breaker returns to Closed state, even if test assertions failed partway through
- **Timeout:** 180s

### Group D: Edge case

#### edge_42 — Process kill during chat
- **Provider:** Claude
- **Prompt:** `"Write a very detailed 500-word essay about the history of computing"` (forces long streaming response)
- Phase 1: send prompt, wait for first `StreamReader[...]: emitting Text` (streaming has started)
- Phase 2: `extract_cli_pid`, then `os.kill(pid, signal.SIGKILL)`
- Phase 3: verify within 10s:
  - Log shows `StreamReader[...]:` error or EOF
  - Session status transitions away from Active
  - No `panicked` or `SIGABRT` in log (graceful handling)
- **UI:** screenshot after kill. Chat area should show an error indicator or message, not infinite spinner.
- **Timeout:** 60s
- **Cleanup:** `cleanup_session`

### Group E: Analytics & metrics (NEW)

#### e2e_10h — Analytics pipeline verification (NEW)
- **Provider:** Claude
- **Prompt:** `"What is 2+2?"` (minimal, just to generate analytics)
- **Backend assertions:**
  - Log contains `Session tracking started: session_id=.*, provider=claude`
  - After `Done`, log contains `Session ended: session_id=.*, input_tokens=(\d+), output_tokens=(\d+)` with both > 0
  - Cache tokens: if `cache_creation_tokens` or `cache_read_tokens` appear in the `Session ended` line, verify they are >= 0 (not negative or malformed)
- **Note:** this is a focused test for the AnalyticsCollector pipeline. Other tests also verify `Session ended:` but this one specifically checks all numeric fields.
- **Timeout:** 60s
- **Cleanup:** `cleanup_session`

## Execution Order

1. `e2e_10` — establishes that at least one provider works
2. `e2e_10f` — streaming UX (needs working provider)
3. `e2e_10d` — tool use round-trip (Claude only)
4. `e2e_10e` — permission flow
5. `e2e_10h` — analytics pipeline
6. `e2e_10g` — session rename/delete lifecycle
7. `e2e_10c` — session fork
8. `cross_49` — agent writes file
9. `cross_52` — multi-provider + schema parity
10. `stress_30` — concurrent agents
11. `edge_42` — process kill during chat
12. `e2e_10b` — session restart (kills and relaunches app)
13. `stress_30b` — circuit breaker (alters transport state, goes last)

## Timeout Strategy

| Category | Timeout | Rationale |
|----------|---------|-----------|
| Simple chat | 60s | Claude can take 30s for structured response |
| Streaming/long | 90s | Needs time for progressive delivery |
| Tool use | 90s | Read → response round-trip |
| Multi-provider | 120s | Two sequential LLM calls |
| Concurrent | 120s | 3 parallel sessions |
| Circuit breaker | 180s | Includes 65s cooldown wait |
| App restart | 150s | Kill + rebuild + wait_ready |

## Test Isolation

- Every test calls `cleanup_session` in a `finally` block — ensures orphan sessions don't consume tokens even if the test crashes
- Files created by agents (`cross_49`) are deleted in `finally`
- Circuit breaker cleanup in `stress_30b` uses an inner `finally` that always sends a successful prompt to restore Closed state
- `e2e_10b` (restart test) re-launches the app via `ctx.app`, which the runner can continue to use after the test

## Failure Handling

- **Retry:** runner's `--retry N` flag. Single retry for LLM tests.
- **Timeout → FAIL:** if provider doesn't respond, test fails (not skip) to surface the issue
- **Provider fallback:** only `e2e_10` falls back to Qwen. Other tests are provider-specific.
- **Soft assertions:** schema parity in `cross_52` logs differences as warnings, only fails if no events at all

## Files Changed

| File | Change |
|------|--------|
| `tests/field/lib/llm.py` | NEW — shared LLM test helpers |
| `tests/field/suites/e2e.py` | ADD test_e2e_10, 10b, 10c, 10d, 10e, 10f, 10g, 10h |
| `tests/field/suites/cross.py` | ADD test_cross_49, 52 |
| `tests/field/suites/stress.py` | ADD test_stress_30, 30b |
| `tests/field/suites/edge.py` | ADD test_edge_42 |
| `tests/field/scenarios/e2e.yaml` | ADD e2e_10f, 10g, 10h entries with `requires_llm: true` |

## Appendix A: Verified Log Patterns

These patterns were extracted directly from the Rust source code on 2026-04-01 and are used as grep targets in the test helpers.

### Transport (transport/mod.rs)
```
Transport: send request provider=(\S+) model=(\S+) session_id=(\S+)
Transport: trust_level=(\S+) yolo=(\S+)
Transport: send blocked by PermissionEngine layer=(\S+) reason=(.+)
Transport: permission_args=(.+)
Transport: spawning CLI binary=(\S+) .* attempt=(\d+)
Transport: spawn retry attempt (\d+) after (\d+)ms
Transport: session=(\S+) stored CLI session ID=(\S+)
Transport: session=(\S+) started for provider=(\S+)
Transport: session=(\S+) stopped
Transport: stopping session=(\S+)
```

### Stream Reader (transport/stream_reader.rs)
```
StreamReader\[(\S+)\]: raw line type=(\S+) len=(\d+)
StreamReader\[(\S+)\]: captured CLI session ID: (\S+)
StreamReader\[(\S+)\]: emitting (\S+)
```

### Normalizer Pipeline (normalizer/pipeline.rs)
```
Pipeline\[(\S+)\]: matched rule '(\S+)' .* emit '(\S+)'
Pipeline\[(\S+)\]: built event type=(\S+) content_len=(\d+)
```

### Session Manager (transport/session_manager.rs)
```
SessionManager: creating session provider=(\S+) model=(\S+)
SessionManager: session created session_id=(\S+)
SessionManager: restoring session=(\S+)
SessionManager: session=(\S+) restored with (\d+) events
SessionManager: forking session=(\S+) at event_index=(\d+)
SessionManager: forked session=(\S+) -> new session=(\S+) with (\d+) events
SessionManager: deleting session=(\S+)
SessionManager: session=(\S+) deleted
SessionManager: renaming session=(\S+) to (.+)
SessionManager: session=(\S+) renamed
SessionManager: loaded (\d+) existing sessions from index
```

### Analytics (analytics/collector.rs)
```
Session ended: session_id=(\S+), input_tokens=(\d+), output_tokens=(\d+)
Session tracking started: session_id=(\S+), provider=(\S+)
```

### Permission Engine (permission_engine.rs)
```
Permission: hardcoded rule -> (\S+)
Permission: trust level -> (\S+)
Permission: policy file -> (\S+)
Permission: session memory -> (\S+)
Permission: default -> Confirm
```

### Frontend Console (via Tauri attachConsole, DEV mode only)
```
\[batch\] (\d+) calls .* in (\d+\.\d+)ms
```
