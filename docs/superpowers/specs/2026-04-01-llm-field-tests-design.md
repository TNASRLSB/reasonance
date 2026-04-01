# LLM Field Tests Design

**Date:** 2026-04-01
**Status:** Approved
**Scope:** 11 field tests exercising the full LLM stack (transport, normalizer, session, agent, UI)

## Context

The field test suite has 67 scenarios, of which 5 were skipped because they require live LLM providers. Those 5 stubs had no real implementation. This spec defines 11 LLM-integrated field tests that replace and extend the original 5.

Both `claude` and `qwen` CLIs are installed and configured with API keys.

## Test Architecture

Tests use a 3-level verification strategy:

1. **IPC / log parsing** — grep on Rust logs (`RUST_LOG=reasonance_lib=trace`) for transport spawn, normalizer rule matches, circuit breaker transitions, permission decisions, session events.
2. **Backend assertions** — verify event types, token counts, session state, pipeline correctness by parsing structured log lines.
3. **UI verification** — dotool automation to send messages via the chat panel + screenshots to confirm rendered responses, streaming indicators, error states.

Each test follows:
```
Setup → Action → Backend assertion → UI assertion → Cleanup
```

## Shared Helper: `tests/field/lib/llm.py`

Utility functions used by all LLM tests:

| Function | Purpose |
|----------|---------|
| `send_chat(ctx, provider, prompt, yolo=True)` | Open session panel, select provider, send prompt via dotool, wait for `Done` event in log |
| `wait_for_event(ctx, event_type, timeout=60)` | Poll log for a specific event pattern |
| `get_session_events(ctx, session_id)` | Extract all events for a session from log |
| `get_cli_pid(ctx)` | Capture CLI child process PID from spawn log line |
| `count_events_by_type(events)` | Count events grouped by type |
| `assert_event_schema(events, required_fields)` | Verify all events have required metadata fields |

## Test Definitions

### Grupo A: Chat base (e2e suite)

#### e2e_10 — Chat with real LLM
- **Provider:** Claude (fallback Qwen)
- **Prompt:** `"Analizza questo snippet Rust e suggerisci un miglioramento: fn add(a: i32, b: i32) -> i32 { return a + b; }"`
- **Backend:** log shows `Transport: send request`, at least 1 `Text` event, 1 `Done` event, `output_tokens > 0`
- **UI:** open session panel, select provider, send message, screenshot before/after, verify chat container has rendered text
- **Timeout:** 60s

#### e2e_10b — Session persistence across restart
- Send prompt to Claude, capture `session_id` from log
- Kill app (`ctx.app.kill()`), relaunch (`ctx.app.launch()`, `wait_ready()`)
- Parse log / session panel for the session appearing in session list
- Restore session, verify previous events are present
- **Timeout:** 150s (includes app restart)

#### e2e_10c — Session fork
- Send prompt, wait for complete response
- Trigger fork (via UI or IPC)
- Verify log shows new session created with partial events from parent
- Send second prompt in fork, verify it diverges
- **Timeout:** 90s

#### e2e_10d — Tool use round-trip
- **Provider:** Claude (yolo mode)
- **Prompt:** `"Leggi il contenuto del file src-tauri/Cargo.toml e dimmi qual è la versione del progetto"`
- **Backend:** verify `ToolUse` event (tool_name contains "Read" or similar) + `ToolResult` + final `Text`
- **UI:** verify chat panel shows rendered tool use block
- **Timeout:** 90s

#### e2e_10e — Permission denial flow
- Configure session in `ask` mode (not yolo)
- Prompt that triggers tool use
- **Backend:** verify log for `PermissionEngine: evaluation` and `PermissionDenial` or `Confirm` event
- **UI:** screenshot of permission dialog if visible
- **Timeout:** 60s

### Group B: Cross-feature

#### cross_49 — Agent writes file, tree updates
- **Provider:** Claude (yolo mode)
- **Prompt:** `"Crea un file chiamato /tmp/reasonance-test-output.txt con il contenuto 'field test ok'"`
- **Backend:** verify `ToolUse` (Write) + `ToolResult` events, verify file exists on disk with expected content
- **UI:** if project includes /tmp in tree, verify update; otherwise verify file on disk only
- **Cleanup:** delete `/tmp/reasonance-test-output.txt`
- **Timeout:** 90s

#### cross_52 — Multi-provider normalizer
- **Prompt (both):** `"Spiega in 2 frasi cos'è un binary search tree"`
- Send to Claude, then to Qwen (sequential)
- **Backend:** verify both produce events with same required fields (`event_type`, `content.type`, `metadata.provider`, `metadata.output_tokens`)
- **Log:** verify each provider's normalizer matched at least 1 rule
- **Timeout:** 120s

#### cross_52b — Normalizer event schema parity
- Extension of cross_52: structural comparison of AgentEvent fields
- Verify `event_type` set is identical (both emit at least Text + Done)
- Verify `metadata` contains `input_tokens`, `output_tokens`, `model` for both
- Optional field differences are warnings, not failures
- **Timeout:** 120s (runs after cross_52, reuses same sessions)

### Group C: Stress & resilience

#### stress_30 — Multiple concurrent agents
- Launch 3 sessions in parallel (2 Claude + 1 Qwen) with distinct prompts
- **Backend:** verify all 3 produce `Done` events without fatal errors
- **Measure:** total wall time, verify no deadlock
- **Timeout:** 120s

#### stress_30b — Circuit breaker trip and recovery
- Force 3 consecutive failures: kill CLI process immediately after spawn, 3 times
- **Backend:** verify log for `CircuitBreaker: state transition Closed -> Open`
- Wait cooldown (~60s)
- Retry: verify `HalfOpen` → successful spawn → `Closed`
- **Cleanup:** ensure circuit breaker returns to Closed state
- **Timeout:** 180s

### Group D: Edge case

#### edge_42 — Network disconnect during chat
- Start Claude session with a prompt that requires a long response
- After 2-3s of streaming, `kill -9` the CLI process PID (captured from spawn log)
- **Backend:** verify session enters `Error` state with appropriate severity
- **UI:** verify interface shows error message (not stuck in infinite loading)
- **Timeout:** 60s

## Execution Order

Tests have implicit dependencies. Execution order:

1. `e2e_10` (establishes that at least one provider works)
2. `e2e_10d` (tool use — same base flow)
3. `e2e_10e` (permission denial — same base flow)
4. `e2e_10c` (session fork — needs a completed session)
5. `cross_49` (agent writes file)
6. `cross_52` (multi-provider)
7. `cross_52b` (schema parity — reuses cross_52 data)
8. `stress_30` (concurrent agents)
9. `edge_42` (kill during chat)
10. `e2e_10b` (session restart — kills and relaunches app)
11. `stress_30b` (circuit breaker — alters transport state, goes last)

## Timeout Strategy

| Category | Timeout | Rationale |
|----------|---------|-----------|
| Single chat | 60s | Claude can take 30s for structured response |
| Tool use | 90s | Read → response round-trip |
| Multi-provider | 120s | Two sequential LLM calls |
| Concurrent | 120s | 3 parallel sessions |
| Circuit breaker | 180s | Includes 60s cooldown wait |
| App restart | 150s | Kill + rebuild + wait_ready |

## Test Isolation

- Every test calls `agent_stop(session_id)` in cleanup to avoid orphan sessions consuming tokens
- Files created by agents (`cross_49`) are deleted in cleanup
- Circuit breaker is reset after `stress_30b` by performing a successful `agent_send`
- Test failure does not block subsequent tests (cleanup runs in `finally` block)

## Failure Handling

- **Retry:** runner supports `--retry N`; single retry is reasonable for LLM tests
- **Timeout ≠ crash:** if provider doesn't respond, test logs TIMEOUT and moves to next
- **Provider fallback:** `e2e_10` tries Claude first, falls back to Qwen
- **Soft assertions:** schema parity tests (`cross_52b`) treat optional field differences as warnings

## Files Changed

| File | Change |
|------|--------|
| `tests/field/lib/llm.py` | NEW — shared LLM test helpers |
| `tests/field/suites/e2e.py` | ADD test_e2e_10, 10b, 10c, 10d, 10e |
| `tests/field/suites/cross.py` | ADD test_cross_49, 52, 52b |
| `tests/field/suites/stress.py` | ADD test_stress_30, 30b |
| `tests/field/suites/edge.py` | ADD test_edge_42 |
| `tests/field/scenarios/*.yaml` | UPDATE requires_llm tests with realistic step definitions |
