# Phase 7A: CLI-Based Providers — Design Spec

## Goal

Add 4 new CLI-based LLM providers to the Structured Agent Transport: **Gemini**, **Kimi**, **Qwen**, and **Codex** (OpenAI). Each provider gets a TOML normalizer, a dedicated state machine, real JSON fixtures for testing, and precise error mapping. A shared accumulation helper module eliminates duplication across state machines. Timeout-based flush prevents stuck accumulations. TOML capabilities are wired to Phase 6's CapabilityNegotiator at load time.

## Architecture

### Streaming Protocols

All 4 providers emit JSONL on stdout — one JSON object per line. The existing `stream_reader.rs` reads lines and pipes them through the `NormalizerPipeline` (Rules → State Machine → Content Parser). No changes needed to the stream reader.

**Prerequisite change:** `resolve_path` in `rules_engine.rs` must be extended to support array indexing (e.g., `content[0].text`). The current implementation only splits on `.` and calls `value.get(segment)`, which silently returns `None` for array-indexed paths. This must be done before any TOML with array-indexed mappings can work.

| Provider | Binary | Flags | Protocol |
|----------|--------|-------|----------|
| Gemini | `gemini` | `-p "{prompt}" --output-format stream-json` | JSONL with `type` discriminant |
| Kimi | `kimi` | `-p "{prompt}" --output-format stream-json` | JSONL, Claude-like conventions |
| Qwen | `qwen` | `-p "{prompt}" --output-format stream-json --include-partial-messages` | JSONL, Claude-like conventions |
| Codex | `codex` | `-q --json "{prompt}"` | JSONL, JSON-RPC v2 with `method` discriminant |

### Component Map

```
src-tauri/normalizers/
  claude.toml             (existing — add tool_input_delta + block_stop rules)
  gemini.toml             (new)
  kimi.toml               (new)
  qwen.toml               (new)
  codex.toml              (new)

src-tauri/normalizers/fixtures/
  gemini/basic_text.jsonl  (new — captured CLI output)
  gemini/tool_use.jsonl    (new)
  gemini/error.jsonl       (new)
  kimi/basic_text.jsonl    (new)
  kimi/thinking.jsonl      (new)
  kimi/tool_use.jsonl      (new)
  kimi/error.jsonl         (new)
  qwen/basic_text.jsonl    (new)
  qwen/tool_use.jsonl      (new)
  qwen/error.jsonl         (new)
  codex/basic_text.jsonl   (new)
  codex/reasoning.jsonl    (new)
  codex/tool_use.jsonl     (new)
  codex/error.jsonl        (new)

src-tauri/src/normalizer/state_machines/
  mod.rs                   (modified — 4 new pub mod + accumulator)
  accumulator.rs           (new — shared accumulation primitives)
  claude.rs                (existing — refactor to use shared accumulators)
  generic.rs               (existing)
  gemini.rs                (new)
  kimi.rs                  (new)
  qwen.rs                  (new)
  codex.rs                 (new)

src-tauri/src/normalizer/rules_engine.rs  (modified — array index support in resolve_path)
src-tauri/src/normalizer/mod.rs           (modified — state machine selection in load_from_dir and reload_provider)
src-tauri/src/transport/mod.rs            (modified — provider match for state machine selection)
src-tauri/src/discovery.rs                (modified — add kimi, qwen, codex to CLI scan + builtin profiles)
src-tauri/src/agent_event.rs              (modified — add incomplete field to AgentEventMetadata)
src-tauri/src/normalizer/pipeline.rs      (modified — add incomplete: None to AgentEventMetadata construction)
```

---

## 0. Prerequisite: Array Index Support in `resolve_path`

### Current Limitation

`resolve_path` in `rules_engine.rs` splits on `.` and calls `value.get(segment)`. It cannot handle `content[0].text` — the segment `content[0]` is not a valid JSON key and returns `None`.

### Fix

Extend `resolve_path` to parse `segment[N]` patterns:

```rust
pub fn resolve_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        // Check for array index: "field[N]"
        if let Some(bracket_pos) = segment.find('[') {
            let field = &segment[..bracket_pos];
            let idx_str = &segment[bracket_pos + 1..segment.len() - 1];
            current = current.get(field)?;
            let idx: usize = idx_str.parse().ok()?;
            current = current.get(idx)?;
        } else {
            current = current.get(segment)?;
        }
    }
    Some(current)
}
```

**Tests (3):**
1. `resolve_path(&json, "content[0].text")` → resolves array element
2. `resolve_path(&json, "content[99].text")` → returns None (out of bounds)
3. `resolve_path(&json, "nested.items[1].name")` → resolves deep array access

---

## 1. Shared Accumulator Module

### Purpose

State machines across providers share common patterns: accumulate text deltas, accumulate tool input JSON, flush on a signal event. Instead of reimplementing this in each state machine, a shared `accumulator.rs` module provides tested primitives.

### Types

```rust
/// Accumulates text chunks until flushed.
pub struct TextAccumulator {
    buffer: String,
}

impl TextAccumulator {
    pub fn new() -> Self;
    pub fn push(&mut self, text: &str);
    pub fn take(&mut self) -> String;      // Returns accumulated text, clears buffer
    pub fn is_empty(&self) -> bool;
    pub fn peek(&self) -> &str;
}

/// Accumulates tool input (JSON fragments) with metadata.
pub struct ToolInputAccumulator {
    tool_name: Option<String>,
    tool_id: Option<String>,
    input_buffer: String,
    start_event: Option<AgentEvent>,  // The initial tool_use event
}

impl ToolInputAccumulator {
    pub fn new() -> Self;
    /// Start accumulating for a new tool. If already active, auto-flushes the
    /// pending tool (returns it as Some) before starting the new one.
    pub fn start(&mut self, event: AgentEvent, tool_name: &str, tool_id: Option<&str>) -> Option<AgentEvent>;
    pub fn push_input(&mut self, fragment: &str);
    pub fn is_active(&self) -> bool;
    pub fn finalize(&mut self) -> Option<AgentEvent>;  // Returns assembled tool_use event, clears state
    pub fn reset(&mut self);
}

/// Wraps any accumulator with timeout-based flush.
pub struct TimedFlush {
    last_event_at: Instant,
    timeout: Duration,
}

impl TimedFlush {
    pub fn new(timeout: Duration) -> Self;
    pub fn touch(&mut self);              // Record that an event arrived
    pub fn is_expired(&self) -> bool;     // Has timeout elapsed since last event?
    pub fn elapsed(&self) -> Duration;
}
```

### Flush Timeout Behavior

When the state machine checks `TimedFlush::is_expired()` and it returns `true`:
1. The state machine calls `ToolInputAccumulator::finalize()` or `TextAccumulator::take()`
2. The resulting event gets an `incomplete: true` marker in metadata (new optional field on `AgentEventMetadata`)
3. The state machine resets to idle state

The timeout check happens in the state machine's `process()` method — before processing the new event, check if the previous accumulation has timed out.

**Limitation:** Since `StateMachine::process()` is only called when a new event arrives, the timeout cannot trigger if the provider stops sending events entirely (e.g., hangs). In that case, the stream reader's process termination handling (wait for child exit) covers the stuck case. The timeout's primary purpose is handling partial accumulation when the provider sends a different event type before completing the current one.

Default timeout: **10 seconds** (configurable per provider via TOML `[commands]` section in the future).

### AgentEventMetadata Extension

Add one optional field to `AgentEventMetadata` in `agent_event.rs`:

```rust
pub struct AgentEventMetadata {
    // ... existing 9 fields ...
    pub incomplete: Option<bool>,  // Set to true when event was flushed due to timeout
}
```

**Construction sites that must add `incomplete: None`:**
- `agent_event.rs` → `base_metadata()` helper
- `normalizer/pipeline.rs` → `build_event()` method
- Any test helpers that construct `AgentEventMetadata` directly

> **Note:** `incomplete` is `Option<bool>` (not `bool`) specifically to maintain backward compatibility with serialized events that predate this change — `serde` defaults missing `Option<T>` fields to `None`.

### Mapping Semantics Note

In TOML rule mappings, most values are **JSON paths** resolved via `resolve_path` (e.g., `content = "delta.text"` resolves `$.delta.text`). The exception is `severity`, which is a **literal value** (`"fatal"` or `"recoverable"`) mapped directly to `ErrorSeverity` enum. This distinction is handled in `pipeline.rs::build_event` and does not require TOML-level changes.

---

## 2. TOML Normalizers

### Structure

Each TOML follows the established pattern from `claude.toml`. See the `[cli]`, `[capabilities]`, `[retry]`, `[commands]`, `[session]`, `[[rules]]`, and `[[tests]]` sections.

**Rule ordering:** More specific error rules must appear **before** generic ones. The rules engine uses first-match semantics (`find_matching_rule` returns the first matching rule).

### Claude TOML Updates

Add two missing rules to `claude.toml` (tool input streaming and block stop):

```toml
[[rules]]
name = "tool_input_delta"
when = 'type == "content_block_delta" && delta.type == "input_json_delta"'
emit = "tool_use"
[rules.mappings]
content = "delta.partial_json"

[[rules]]
name = "block_stop"
when = 'type == "content_block_stop"'
emit = "status"
```

### Gemini TOML

```toml
[cli]
name = "gemini"
binary = "gemini"
programmatic_args = ["-p", "{prompt}", "--output-format", "stream-json"]
resume_args = ["--resume", "latest", "-p", "{prompt}", "--output-format", "stream-json"]  # "latest" intentional: Gemini CLI only supports --resume latest, not arbitrary session IDs
version_command = ["gemini", "--version"]
update_command = ["npm", "install", "-g", "@FIXME/gemini-cli@latest"]  # PLACEHOLDER: verify actual npm package name

[capabilities]
streaming = true
session_resume = true
tool_use = true
thinking = false
structured_output = true
diff_output = false

[retry]
max_retries = 3
backoff = { strategy = "exponential", base_ms = 1000, max_ms = 30000 }
retryable_codes = ["RESOURCE_EXHAUSTED", "UNAVAILABLE", "DEADLINE_EXCEEDED"]

[commands]
cancel = { method = "signal", signal = "SIGINT" }

[session]
session_id_path = "session_id"
model_path = "model"

# --- Error rules (specific before generic) ---

[[rules]]
name = "error_resource_exhausted"
when = 'type == "ERROR" && code == "RESOURCE_EXHAUSTED"'
emit = "error"
[rules.mappings]
content = "message"
error_code = "code"
severity = "recoverable"

[[rules]]
name = "error_unavailable"
when = 'type == "ERROR" && code == "UNAVAILABLE"'
emit = "error"
[rules.mappings]
content = "message"
error_code = "code"
severity = "recoverable"

[[rules]]
name = "error_deadline"
when = 'type == "ERROR" && code == "DEADLINE_EXCEEDED"'
emit = "error"
[rules.mappings]
content = "message"
error_code = "code"
severity = "recoverable"

[[rules]]
name = "error_generic"
when = 'type == "ERROR"'
emit = "error"
[rules.mappings]
content = "message"
error_code = "code"
severity = "fatal"

# --- Content rules ---

[[rules]]
name = "text_chunk"
when = 'type == "MESSAGE" && exists(content)'
emit = "text"
[rules.mappings]
content = "content[0].text"

[[rules]]
name = "tool_start"
when = 'type == "TOOL_USE"'
emit = "tool_use"
[rules.mappings]
content = "args"
tool_name = "name"
parent_id = "id"

[[rules]]
name = "tool_result"
when = 'type == "TOOL_RESULT"'
emit = "tool_result"
[rules.mappings]
content = "result"
parent_id = "tool_use_id"

[[rules]]
name = "usage"
when = 'type == "RESULT" && exists(usage)'
emit = "usage"
[rules.mappings]
input_tokens = "usage.input_tokens"
output_tokens = "usage.output_tokens"

[[rules]]
name = "done"
when = 'type == "RESULT"'
emit = "done"

[[tests]]
name = "basic_text"
prompt = "Reply with exactly: REASONANCE_TEST_OK"
max_tokens = 50
expected = [
  { event_type = "text", required = true, validate = { type = "content_matches", pattern = "REASONANCE_TEST_OK" } },
  { event_type = "done", required = true, validate = "exists" },
]

[[tests]]
name = "tool_use"
prompt = "Read the file ./Cargo.toml"
max_tokens = 200
expected = [
  { event_type = "tool_use", required = false, validate = "content_not_empty" },
]
```

### Kimi TOML

```toml
[cli]
name = "kimi"
binary = "kimi"
programmatic_args = ["-p", "{prompt}", "--output-format", "stream-json"]
resume_args = ["-C", "-p", "{prompt}", "--output-format", "stream-json"]
version_command = ["kimi", "--version"]
update_command = ["npm", "install", "-g", "kimi@latest"]

[capabilities]
streaming = true
session_resume = true
tool_use = true
thinking = true
structured_output = true
diff_output = false

[retry]
max_retries = 3
backoff = { strategy = "exponential", base_ms = 1000, max_ms = 30000 }
retryable_codes = ["overloaded", "rate_limit", "timeout"]

[commands]
cancel = { method = "signal", signal = "SIGINT" }

[session]
session_id_path = "message.id"
model_path = "message.model"

# --- Error rules (specific before generic) ---

[[rules]]
name = "error_rate_limit"
when = 'type == "error" && error.type == "overloaded"'
emit = "error"
[rules.mappings]
content = "error.message"
error_code = "error.type"
severity = "recoverable"

[[rules]]
name = "error_generic"
when = 'type == "error"'
emit = "error"
[rules.mappings]
content = "error.message"
error_code = "error.type"
severity = "fatal"

# --- Content rules ---

[[rules]]
name = "text_chunk"
when = 'type == "content_block_delta" && delta.type == "text_delta"'
emit = "text"
[rules.mappings]
content = "delta.text"

[[rules]]
name = "thinking"
when = 'type == "content_block_delta" && delta.type == "thinking_delta"'
emit = "thinking"
[rules.mappings]
content = "delta.thinking"

[[rules]]
name = "tool_start"
when = 'type == "content_block_start" && content_block.type == "tool_use"'
emit = "tool_use"
[rules.mappings]
content = "content_block.input"
tool_name = "content_block.name"
parent_id = "content_block.id"

[[rules]]
name = "tool_input_delta"
when = 'type == "content_block_delta" && delta.type == "input_json_delta"'
emit = "tool_use"
[rules.mappings]
content = "delta.partial_json"

[[rules]]
name = "block_stop"
when = 'type == "content_block_stop"'
emit = "status"

[[rules]]
name = "usage"
when = 'type == "message_delta" && exists(usage)'
emit = "usage"
[rules.mappings]
input_tokens = "usage.input_tokens"
output_tokens = "usage.output_tokens"

[[rules]]
name = "done"
when = 'type == "message_stop"'
emit = "done"

[[tests]]
name = "basic_text"
prompt = "Reply with exactly: REASONANCE_TEST_OK"
max_tokens = 50
expected = [
  { event_type = "text", required = true, validate = { type = "content_matches", pattern = "REASONANCE_TEST_OK" } },
  { event_type = "done", required = true, validate = "exists" },
]

[[tests]]
name = "thinking"
prompt = "Think step by step: what is 2+2?"
max_tokens = 200
expected = [
  { event_type = "thinking", required = false, validate = "content_not_empty" },
]
```

### Qwen TOML

```toml
[cli]
name = "qwen"
binary = "qwen"
programmatic_args = ["-p", "{prompt}", "--output-format", "stream-json", "--include-partial-messages"]
resume_args = ["--continue", "-p", "{prompt}", "--output-format", "stream-json", "--include-partial-messages"]
version_command = ["qwen", "--version"]
update_command = ["npm", "install", "-g", "@qwen-code/qwen-code@latest"]

[capabilities]
streaming = true
session_resume = true
tool_use = true
thinking = false
structured_output = true
diff_output = false

[retry]
max_retries = 3
backoff = { strategy = "exponential", base_ms = 1000, max_ms = 30000 }
retryable_codes = ["overloaded", "rate_limit", "timeout"]

[commands]
cancel = { method = "signal", signal = "SIGINT" }

[session]
session_id_path = "session_id"
model_path = "message.model"

# --- Error rules (specific before generic) ---

[[rules]]
name = "error_rate_limit"
when = 'type == "error" && error.type == "overloaded"'
emit = "error"
[rules.mappings]
content = "error.message"
error_code = "error.type"
severity = "recoverable"

[[rules]]
name = "error_generic"
when = 'type == "error"'
emit = "error"
[rules.mappings]
content = "error.message"
error_code = "error.type"
severity = "fatal"

# --- Content rules ---

[[rules]]
name = "text_chunk"
when = 'type == "content_block_delta" && delta.type == "text_delta"'
emit = "text"
[rules.mappings]
content = "delta.text"

[[rules]]
name = "tool_start"
when = 'type == "content_block_start" && content_block.type == "tool_use"'
emit = "tool_use"
[rules.mappings]
content = "content_block.input"
tool_name = "content_block.name"
parent_id = "content_block.id"

[[rules]]
name = "tool_input_delta"
when = 'type == "content_block_delta" && delta.type == "input_json_delta"'
emit = "tool_use"
[rules.mappings]
content = "delta.partial_json"

[[rules]]
name = "block_stop"
when = 'type == "content_block_stop"'
emit = "status"

[[rules]]
name = "assistant_text"
when = 'type == "assistant" && exists(message.content)'
emit = "text"
[rules.mappings]
content = "message.content[0].text"

[[rules]]
name = "usage"
when = 'type == "result" && exists(usage)'
emit = "usage"
[rules.mappings]
input_tokens = "usage.input_tokens"
output_tokens = "usage.output_tokens"

[[rules]]
name = "done"
when = 'type == "result" && subtype == "success"'
emit = "done"

[[tests]]
name = "basic_text"
prompt = "Reply with exactly: REASONANCE_TEST_OK"
max_tokens = 50
expected = [
  { event_type = "text", required = true, validate = { type = "content_matches", pattern = "REASONANCE_TEST_OK" } },
  { event_type = "done", required = true, validate = "exists" },
]
```

### Codex TOML

```toml
[cli]
name = "codex"
binary = "codex"
programmatic_args = ["-q", "--json", "{prompt}"]
resume_args = ["-q", "--json", "{prompt}"]
version_command = ["codex", "--version"]
update_command = ["npm", "install", "-g", "@openai/codex@latest"]

[capabilities]
streaming = true
session_resume = false
tool_use = true
thinking = true
structured_output = true
diff_output = false

[retry]
max_retries = 3
backoff = { strategy = "exponential", base_ms = 1000, max_ms = 30000 }
retryable_codes = ["rate_limit", "server_error"]

[commands]
cancel = { method = "signal", signal = "SIGINT" }

[session]
session_id_path = "params.thread_id"
model_path = "params.model"

# --- Error rules (specific before generic) ---

[[rules]]
name = "error_rate_limit"
when = 'method == "ErrorNotification" && params.code == "rate_limit"'
emit = "error"
[rules.mappings]
content = "params.message"
error_code = "params.code"
severity = "recoverable"

[[rules]]
name = "error_server"
when = 'method == "ErrorNotification" && params.code == "server_error"'
emit = "error"
[rules.mappings]
content = "params.message"
error_code = "params.code"
severity = "recoverable"

[[rules]]
name = "error_generic"
when = 'method == "ErrorNotification"'
emit = "error"
[rules.mappings]
content = "params.message"
error_code = "params.code"
severity = "fatal"

# --- Content rules ---

[[rules]]
name = "text_delta"
when = 'method == "AgentMessageDeltaNotification"'
emit = "text"
[rules.mappings]
content = "params.delta"

[[rules]]
name = "reasoning_completed"
when = 'method == "ItemCompletedNotification" && params.item.type == "reasoning"'
emit = "thinking"
[rules.mappings]
content = "params.item.content"

[[rules]]
name = "command_execution"
when = 'method == "ItemCompletedNotification" && params.item.type == "commandExecution"'
emit = "tool_use"
[rules.mappings]
content = "params.item.output"
tool_name = "params.item.command"
parent_id = "params.item.id"

[[rules]]
name = "mcp_tool_call"
when = 'method == "ItemCompletedNotification" && params.item.type == "mcpToolCall"'
emit = "tool_use"
[rules.mappings]
content = "params.item.output"
tool_name = "params.item.name"
parent_id = "params.item.id"

[[rules]]
name = "usage"
when = 'method == "ThreadTokenUsageUpdatedNotification"'
emit = "usage"
[rules.mappings]
input_tokens = "params.usage.input_tokens"
output_tokens = "params.usage.output_tokens"

[[rules]]
name = "done"
when = 'method == "TurnCompletedNotification"'
emit = "done"

[[tests]]
name = "basic_text"
prompt = "Reply with exactly: REASONANCE_TEST_OK"
max_tokens = 50
expected = [
  { event_type = "text", required = true, validate = { type = "content_matches", pattern = "REASONANCE_TEST_OK" } },
  { event_type = "done", required = true, validate = "exists" },
]

[[tests]]
name = "reasoning"
prompt = "Think step by step: what is 2+2?"
max_tokens = 200
expected = [
  { event_type = "thinking", required = false, validate = "content_not_empty" },
]
```

---

## 3. State Machines

### Gemini State Machine

Gemini emits tool use as individual events, but may send multi-part tool arguments. The state machine accumulates tool input fragments and flushes on the next non-tool event or timeout.

```rust
pub struct GeminiStateMachine {
    tool_accumulator: ToolInputAccumulator,
    timed_flush: TimedFlush,
}
```

**Behavior:**
- `ToolUse` → start accumulation if not active, push input if active
- Any non-`ToolUse` → flush pending tool if active, then pass through event
- Timeout → flush with `incomplete: true`
- `Text`, `Error`, `Usage`, `Done` → pass through immediately

**Tests (5):**
1. Text events pass through
2. Tool use accumulates and flushes on next event
3. Multiple tool use events accumulate input
4. Reset clears accumulator
5. Timeout flush emits incomplete event

### Kimi State Machine

Mirrors Claude's pattern — `content_block_start` (tool_use) → N deltas → `content_block_stop` (status). Uses shared accumulators.

```rust
pub struct KimiStateMachine {
    tool_accumulator: ToolInputAccumulator,
    timed_flush: TimedFlush,
}
```

**Behavior:**
- `ToolUse` (first) → start accumulation
- `ToolUse` (subsequent) → push input fragment
- `Status` (content_block_stop) → finalize tool, emit assembled event
- `Thinking` → pass through (Kimi supports native thinking)
- Timeout → flush with `incomplete: true`
- Everything else → pass through

**Tests (5):**
1. Text events pass through
2. Tool use accumulates until Status
3. Thinking events pass through
4. Reset clears accumulator
5. Timeout flush emits incomplete event

### Qwen State Machine

Same Claude-like pattern. With `--include-partial-messages`, Qwen emits `content_block_delta` events. Without it, emits `assistant` (complete message) and `result` — both handled by rules.

```rust
pub struct QwenStateMachine {
    tool_accumulator: ToolInputAccumulator,
    timed_flush: TimedFlush,
}
```

**Behavior:**
- Identical to Kimi, minus thinking support
- `ToolUse` → accumulate
- `Status` → finalize tool
- Timeout → flush with `incomplete: true`
- Everything else → pass through

**Tests (5):**
1. Text events pass through
2. Tool use accumulates until Status
3. Assistant-level text events pass through
4. Reset clears accumulator
5. Timeout flush emits incomplete event

### Codex State Machine

The most complex. JSON-RPC emits `AgentMessageDeltaNotification` for text chunks — these need accumulation into complete text blocks. `ItemCompletedNotification` arrives pre-assembled (reasoning, commandExecution, mcpToolCall) — these pass through. Text deltas are accumulated until a non-delta event arrives.

```rust
pub struct CodexStateMachine {
    text_accumulator: TextAccumulator,
    tool_accumulator: ToolInputAccumulator,
    timed_flush: TimedFlush,
}
```

**Behavior:**
- `Text` (from AgentMessageDeltaNotification) → accumulate text
- `Thinking` (from ItemCompletedNotification/reasoning) → flush pending text first, then pass through
- `ToolUse` (from ItemCompletedNotification/command or mcp) → flush pending text first, then pass through (already assembled by rules)
- `Usage`, `Done`, `Error` → flush pending text first, then pass through
- Timeout → flush text and/or tool with `incomplete: true`

**Tests (6):**
1. Single text delta passes through immediately (no following event to trigger flush, but Done will flush)
2. Multiple text deltas accumulate, flush on Done
3. Reasoning (thinking) events pass through, flushing pending text
4. Tool use events pass through (pre-assembled)
5. Reset clears all accumulators
6. Timeout flush emits incomplete text event

### Note on Kimi/Qwen vs Claude Similarity

Kimi and Qwen state machines have nearly identical logic to Claude's (tool_use start → accumulate → status flush). They are kept as separate files because: (a) providers evolve independently and will diverge, (b) isolated files make provider-specific debugging straightforward, (c) the shared `ToolInputAccumulator` already eliminates the actual duplication — the state machine files are thin wrappers (~50-80 lines each).

---

## 4. JSON Fixtures

### Purpose

Each provider gets a `fixtures/<provider>/` directory with `.jsonl` files containing real CLI output captured from actual runs. Pipeline tests replay these fixtures through the full normalizer (TOML rules + state machine) and assert correct AgentEvent output.

### Fixture Format

Each `.jsonl` file is raw CLI stdout — one JSON object per line, exactly as the CLI emits it.

Accompanying each `.jsonl` file is a `.expected.json` file with the expected AgentEvent sequence:

```json
[
  { "event_type": "text", "content_contains": "hello" },
  { "event_type": "usage", "has_input_tokens": true },
  { "event_type": "done" }
]
```

### Required Fixtures Per Provider

| Provider | Fixtures |
|----------|----------|
| Gemini | `basic_text.jsonl`, `tool_use.jsonl`, `error.jsonl` |
| Kimi | `basic_text.jsonl`, `thinking.jsonl`, `tool_use.jsonl`, `error.jsonl` |
| Qwen | `basic_text.jsonl`, `tool_use.jsonl`, `error.jsonl` |
| Codex | `basic_text.jsonl`, `reasoning.jsonl`, `tool_use.jsonl`, `error.jsonl` |

### Fixture Tests

A shared test helper in `normalizer/pipeline.rs` (or a dedicated `tests/fixture_tests.rs`):

```rust
fn run_fixture_test(provider: &str, fixture_name: &str) {
    let fixture_path = format!("normalizers/fixtures/{}/{}.jsonl", provider, fixture_name);
    let expected_path = format!("normalizers/fixtures/{}/{}.expected.json", provider, fixture_name);
    // Load TOML, create pipeline, feed each line, compare output to expected
}
```

### Creating Fixtures

Since not all CLIs may be installed on the build machine, fixtures are committed to the repo as test data. They are created manually by running each CLI once and capturing stdout:

```bash
gemini -p "Reply with exactly: hello" --output-format stream-json > normalizers/fixtures/gemini/basic_text.jsonl
```

If a CLI is not available, synthetic fixtures based on documented JSON schemas are acceptable as initial versions, with a TODO to replace with real captures.

---

## 5. Error Mapping

### Per-Provider Retryable Errors

Each TOML defines `retryable_codes` in the `[retry]` section. The rules engine maps specific error codes to `severity = "recoverable"` (retryable) vs `severity = "fatal"` (non-retryable).

| Provider | Recoverable Codes | Fatal (everything else) |
|----------|-------------------|------------------------|
| Claude | `overloaded`, `rate_limit`, `timeout` | `invalid_request`, `authentication_error` |
| Gemini | `RESOURCE_EXHAUSTED`, `UNAVAILABLE`, `DEADLINE_EXCEEDED` | `INVALID_ARGUMENT`, `PERMISSION_DENIED` |
| Kimi | `overloaded`, `rate_limit`, `timeout` | All others |
| Qwen | `overloaded`, `rate_limit`, `timeout` | All others |
| Codex | `rate_limit`, `server_error` | All others |

### Rule Priority

More specific error rules must appear before generic ones in the TOML. The rules engine uses first-match semantics — a `rate_limit` error matches the specific recoverable rule before the generic fatal rule.

---

## 6. Capabilities Wiring

### At TOML Load Time

When `NormalizerRegistry::load_from_dir()` loads a provider's TOML, the `[capabilities]` section is already parsed into `HashMap<String, toml::Value>`. Phase 7A adds a new method to convert this into `NegotiatedCapabilities` and register it with the `CapabilityNegotiator`.

### Wiring Point

In `lib.rs` setup hook (where `register_from_configs` is already called for `CliUpdater`), add:

```rust
// Register capabilities from TOML configs
for (provider, config) in &configs {
    let features = capabilities_from_toml(&config.capabilities);
    let caps = NegotiatedCapabilities {
        provider: provider.clone(),
        cli_version: String::new(),  // populated on first version check
        cli_mode: CliMode::Structured,
        features,
        negotiated_at: now(),
    };
    negotiator.set_capabilities(provider, caps);
}
```

The `capabilities_from_toml` function maps boolean TOML values to `FeatureSupport`:
- `true` → `FeatureSupport::Full`
- `false` → `FeatureSupport::Unsupported { alternative: None }`

---

## 7. Discovery Extension

### New CLI Entries

Add to `discovery.rs` CLI scan list:

| Binary | Display Name |
|--------|-------------|
| `kimi` | Kimi |
| `qwen` | Qwen Code |
| `codex` | Codex (OpenAI) |

Gemini is already discovered.

### Builtin Capability Profiles

Add entries to `builtin_profiles()` for each new provider:

| Provider | read_file | write_file | execute_command | web_search | image_input | long_context |
|----------|-----------|------------|-----------------|------------|-------------|-------------|
| `kimi` | true | true | true | false | false | true |
| `qwen` | true | true | true | false | false | true |
| `codex` | true | true | true | false | false | true |

---

## 8. Transport Routing

### Provider Match — Two Locations

The state machine selection match must be updated in **two** places:

**1. `transport/mod.rs` (line 80)** — used when spawning a new session:

```rust
let state_machine: Box<dyn StateMachine> = match provider.as_str() {
    "claude" => Box::new(ClaudeStateMachine::new()),
    "gemini" => Box::new(GeminiStateMachine::new()),
    "kimi" => Box::new(KimiStateMachine::new()),
    "qwen" => Box::new(QwenStateMachine::new()),
    "codex" => Box::new(CodexStateMachine::new()),
    _ => Box::new(GenericStateMachine::new()),
};
```

**2. `normalizer/mod.rs`** — used in `load_from_dir` and `reload_provider` when constructing `NormalizerPipeline`:

Same match pattern as above.

The generic fallback remains for any future provider added via TOML-only.

---

## 9. Test Summary

| Component | Tests | Type |
|-----------|-------|------|
| `rules_engine.rs` (array index) | 3 | Unit — resolve_path with `[N]` syntax |
| `accumulator.rs` | 12 | Unit — TextAccumulator (4), ToolInputAccumulator (4, including start-while-active), TimedFlush (4, including fresh/expired/touch/edge) |
| `gemini.rs` | 5 | Unit — state machine behavior |
| `kimi.rs` | 5 | Unit — state machine behavior |
| `qwen.rs` | 5 | Unit — state machine behavior |
| `codex.rs` | 6 | Unit — state machine behavior |
| Fixture tests | 14 | Integration — full pipeline with real JSON |
| **Total** | **50** | |

---

## 10. Deferred

- **Live CLI negotiation** — spawning each CLI to probe capabilities is deferred (Phase 6 laid the infrastructure)
- **DirectApi providers** (Ollama, DeepSeek) — Phase 7B
- **Self-heal orchestration** — Phase 6 infrastructure exists, wiring deferred
- **Codex session resume** — JSON-RPC protocol requires `thread/resume`; not exposed as simple CLI flag
- **Qwen `--output-format` version issues** — GitHub issue #873 reports flag not recognized in some versions; workaround: fall back to non-streaming mode
- **Gemini update command** — npm package name `@anthropic-ai/gemini-cli` is a placeholder; verify actual package name before deployment
