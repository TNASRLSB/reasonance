# Phase 7A: CLI Providers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 4 CLI-based LLM providers (Gemini, Kimi, Qwen, Codex) to the Structured Agent Transport with dedicated state machines, TOML normalizers, fixture-based tests, and error mapping.

**Architecture:** Each provider gets a TOML normalizer (rules), a dedicated state machine (event accumulation), and JSON fixtures (integration tests). A shared accumulator module (`accumulator.rs`) provides `TextAccumulator`, `ToolInputAccumulator`, and `TimedFlush` to eliminate duplication. The `resolve_path` function is extended to support array indexing before any TOML with `content[0].text` paths can work.

**Tech Stack:** Rust, serde_json, TOML normalizer system, Tauri 2 state management

**Spec:** `docs/superpowers/specs/2026-03-22-phase7a-cli-providers-design.md`

---

## File Structure

### New Files
| File | Purpose |
|------|---------|
| `src-tauri/src/normalizer/state_machines/accumulator.rs` | Shared `TextAccumulator`, `ToolInputAccumulator`, `TimedFlush` primitives |
| `src-tauri/src/normalizer/state_machines/gemini.rs` | Gemini state machine — tool use accumulation |
| `src-tauri/src/normalizer/state_machines/kimi.rs` | Kimi state machine — Claude-like block lifecycle |
| `src-tauri/src/normalizer/state_machines/qwen.rs` | Qwen state machine — Claude-like block lifecycle |
| `src-tauri/src/normalizer/state_machines/codex.rs` | Codex state machine — text delta accumulation |
| `src-tauri/normalizers/gemini.toml` | Gemini normalizer rules |
| `src-tauri/normalizers/kimi.toml` | Kimi normalizer rules |
| `src-tauri/normalizers/qwen.toml` | Qwen normalizer rules |
| `src-tauri/normalizers/codex.toml` | Codex normalizer rules |
| `src-tauri/normalizers/fixtures/gemini/basic_text.jsonl` | Gemini text fixture |
| `src-tauri/normalizers/fixtures/gemini/tool_use.jsonl` | Gemini tool use fixture |
| `src-tauri/normalizers/fixtures/gemini/error.jsonl` | Gemini error fixture |
| `src-tauri/normalizers/fixtures/gemini/*.expected.json` | Expected outputs for Gemini |
| `src-tauri/normalizers/fixtures/kimi/basic_text.jsonl` | Kimi text fixture |
| `src-tauri/normalizers/fixtures/kimi/thinking.jsonl` | Kimi thinking fixture |
| `src-tauri/normalizers/fixtures/kimi/tool_use.jsonl` | Kimi tool use fixture |
| `src-tauri/normalizers/fixtures/kimi/error.jsonl` | Kimi error fixture |
| `src-tauri/normalizers/fixtures/kimi/*.expected.json` | Expected outputs for Kimi |
| `src-tauri/normalizers/fixtures/qwen/basic_text.jsonl` | Qwen text fixture |
| `src-tauri/normalizers/fixtures/qwen/tool_use.jsonl` | Qwen tool use fixture |
| `src-tauri/normalizers/fixtures/qwen/error.jsonl` | Qwen error fixture |
| `src-tauri/normalizers/fixtures/qwen/*.expected.json` | Expected outputs for Qwen |
| `src-tauri/normalizers/fixtures/codex/basic_text.jsonl` | Codex text fixture |
| `src-tauri/normalizers/fixtures/codex/reasoning.jsonl` | Codex reasoning fixture |
| `src-tauri/normalizers/fixtures/codex/tool_use.jsonl` | Codex tool use fixture |
| `src-tauri/normalizers/fixtures/codex/error.jsonl` | Codex error fixture |
| `src-tauri/normalizers/fixtures/codex/*.expected.json` | Expected outputs for Codex |

### Modified Files
| File | Changes |
|------|---------|
| `src-tauri/src/normalizer/rules_engine.rs` | Extend `resolve_path` for array index `[N]` syntax |
| `src-tauri/src/agent_event.rs` | Add `incomplete: Option<bool>` to `AgentEventMetadata` + `base_metadata()` |
| `src-tauri/src/normalizer/pipeline.rs` | Add `incomplete: None` to `build_event()` metadata construction |
| `src-tauri/src/normalizer/state_machines/mod.rs` | Add `pub mod accumulator/gemini/kimi/qwen/codex` |
| `src-tauri/src/normalizer/mod.rs` | Add new providers to state machine match in `load_from_dir` and `reload_provider` |
| `src-tauri/src/transport/mod.rs` | Add new providers to state machine match in `send()` |
| `src-tauri/normalizers/claude.toml` | Add `tool_input_delta` and `block_stop` rules |
| `src-tauri/src/discovery.rs` | Add `kimi`, `qwen`, `codex` to CLI scan + builtin profiles |
| `src-tauri/src/lib.rs` | Wire capabilities from TOML configs to `CapabilityNegotiator` |

---

### Task 1: Prerequisite — Array Index Support in `resolve_path`

**Files:**
- Modify: `src-tauri/src/normalizer/rules_engine.rs:4-13`

- [ ] **Step 1: Write 3 failing tests for array indexing**

Add these tests to the existing `mod tests` block in `rules_engine.rs`:

```rust
#[test]
fn test_resolve_path_array_index() {
    let v = json!({
        "content": [
            { "text": "hello", "type": "text" },
            { "text": "world", "type": "text" }
        ]
    });
    assert_eq!(resolve_path(&v, "content[0].text"), Some(&json!("hello")));
}

#[test]
fn test_resolve_path_array_out_of_bounds() {
    let v = json!({
        "content": [{ "text": "hello" }]
    });
    assert_eq!(resolve_path(&v, "content[99].text"), None);
}

#[test]
fn test_resolve_path_nested_array() {
    let v = json!({
        "nested": {
            "items": [
                { "name": "first" },
                { "name": "second" }
            ]
        }
    });
    assert_eq!(resolve_path(&v, "nested.items[1].name"), Some(&json!("second")));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib normalizer::rules_engine::tests::test_resolve_path_array -- --nocapture`
Expected: 3 FAIL (None != Some)

- [ ] **Step 3: Implement array index support in `resolve_path`**

Replace the `resolve_path` function in `rules_engine.rs:4-13` with:

```rust
pub fn resolve_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
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

- [ ] **Step 4: Run all rules_engine tests to verify they pass**

Run: `cd src-tauri && cargo test --lib normalizer::rules_engine::tests -- --nocapture`
Expected: ALL PASS (15 tests including 3 new)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/normalizer/rules_engine.rs
git commit -m "feat: add array index support to resolve_path for [N] syntax"
```

---

### Task 2: AgentEventMetadata `incomplete` Field

**Files:**
- Modify: `src-tauri/src/agent_event.rs:62-72` (metadata struct) and `92-103` (base_metadata)
- Modify: `src-tauri/src/normalizer/pipeline.rs:74-104` (build_event)

- [ ] **Step 1: Add `incomplete` field to `AgentEventMetadata`**

In `agent_event.rs`, add to the struct at line 72 (after `stream_metrics`):

```rust
    #[serde(default)]
    pub incomplete: Option<bool>,
```

- [ ] **Step 2: Add `incomplete: None` to `base_metadata()`**

In `agent_event.rs` function `base_metadata()` (line 92-103), add after `stream_metrics: None,`:

```rust
            incomplete: None,
```

- [ ] **Step 3: Add `incomplete: None` to `pipeline.rs::build_event()`**

In `pipeline.rs`, in the `AgentEventMetadata` struct literal (line 74-104), add after `stream_metrics: None,`:

```rust
            incomplete: None,
```

- [ ] **Step 4: Run all tests to verify nothing breaks**

Run: `cd src-tauri && cargo test --lib -- --nocapture 2>&1 | tail -5`
Expected: ALL PASS (the new field defaults to None, backward-compatible)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/agent_event.rs src-tauri/src/normalizer/pipeline.rs
git commit -m "feat: add incomplete field to AgentEventMetadata for timeout flush"
```

---

### Task 3: Shared Accumulator Module

**Files:**
- Create: `src-tauri/src/normalizer/state_machines/accumulator.rs`
- Modify: `src-tauri/src/normalizer/state_machines/mod.rs`

- [ ] **Step 1: Register the module**

In `src-tauri/src/normalizer/state_machines/mod.rs`, add at line 3 (after `pub mod claude;`):

```rust
pub mod accumulator;
```

- [ ] **Step 2: Write tests for `TextAccumulator`**

Create `src-tauri/src/normalizer/state_machines/accumulator.rs` with tests first:

```rust
use std::time::{Duration, Instant};
use crate::agent_event::{AgentEvent, AgentEventType, EventContent};
use serde_json;

/// Accumulates text chunks until flushed.
pub struct TextAccumulator {
    buffer: String,
}

/// Accumulates tool input (JSON fragments) with metadata.
pub struct ToolInputAccumulator {
    tool_name: Option<String>,
    tool_id: Option<String>,
    input_buffer: String,
    start_event: Option<AgentEvent>,
}

/// Wraps any accumulator with timeout-based flush.
pub struct TimedFlush {
    last_event_at: Instant,
    timeout: Duration,
}

// Placeholder impls — will be filled in next steps
impl TextAccumulator {
    pub fn new() -> Self { Self { buffer: String::new() } }
    pub fn push(&mut self, _text: &str) { }
    pub fn take(&mut self) -> String { String::new() }
    pub fn is_empty(&self) -> bool { true }
    pub fn peek(&self) -> &str { "" }
}

impl ToolInputAccumulator {
    pub fn new() -> Self {
        Self { tool_name: None, tool_id: None, input_buffer: String::new(), start_event: None }
    }
    pub fn start(&mut self, _event: AgentEvent, _tool_name: &str, _tool_id: Option<&str>) -> Option<AgentEvent> { None }
    pub fn push_input(&mut self, _fragment: &str) { }
    pub fn is_active(&self) -> bool { false }
    pub fn finalize(&mut self) -> Option<AgentEvent> { None }
    pub fn reset(&mut self) { }
}

impl TimedFlush {
    pub fn new(timeout: Duration) -> Self { Self { last_event_at: Instant::now(), timeout } }
    pub fn touch(&mut self) { }
    pub fn is_expired(&self) -> bool { false }
    pub fn elapsed(&self) -> Duration { Duration::ZERO }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- TextAccumulator tests ---

    #[test]
    fn test_text_accumulator_push_and_take() {
        let mut acc = TextAccumulator::new();
        acc.push("hello ");
        acc.push("world");
        assert_eq!(acc.take(), "hello world");
        assert!(acc.is_empty());
    }

    #[test]
    fn test_text_accumulator_empty() {
        let acc = TextAccumulator::new();
        assert!(acc.is_empty());
        assert_eq!(acc.peek(), "");
    }

    #[test]
    fn test_text_accumulator_peek() {
        let mut acc = TextAccumulator::new();
        acc.push("hello");
        assert_eq!(acc.peek(), "hello");
        // peek doesn't consume
        assert!(!acc.is_empty());
    }

    #[test]
    fn test_text_accumulator_take_clears() {
        let mut acc = TextAccumulator::new();
        acc.push("data");
        let _ = acc.take();
        assert_eq!(acc.take(), "");
    }

    // --- ToolInputAccumulator tests ---

    #[test]
    fn test_tool_input_start_and_finalize() {
        let mut acc = ToolInputAccumulator::new();
        let event = AgentEvent::tool_use("read_file", "{}", "test");
        let flushed = acc.start(event, "read_file", Some("tool-1"));
        assert!(flushed.is_none()); // nothing pending
        assert!(acc.is_active());
        acc.push_input(r#"{"path":"#);
        acc.push_input(r#""test.rs"}"#);
        let result = acc.finalize();
        assert!(result.is_some());
        let ev = result.unwrap();
        assert_eq!(ev.event_type, AgentEventType::ToolUse);
        assert!(!acc.is_active());
    }

    #[test]
    fn test_tool_input_start_while_active_flushes() {
        let mut acc = ToolInputAccumulator::new();
        let event1 = AgentEvent::tool_use("read_file", "{}", "test");
        acc.start(event1, "read_file", Some("tool-1"));
        acc.push_input(r#"{"path":"a.rs"}"#);

        // Start a new tool while first is active — should auto-flush
        let event2 = AgentEvent::tool_use("write_file", "{}", "test");
        let flushed = acc.start(event2, "write_file", Some("tool-2"));
        assert!(flushed.is_some());
        let flushed_ev = flushed.unwrap();
        assert_eq!(flushed_ev.metadata.tool_name, Some("read_file".to_string()));
    }

    #[test]
    fn test_tool_input_finalize_when_inactive() {
        let mut acc = ToolInputAccumulator::new();
        assert!(acc.finalize().is_none());
    }

    #[test]
    fn test_tool_input_reset() {
        let mut acc = ToolInputAccumulator::new();
        let event = AgentEvent::tool_use("read_file", "{}", "test");
        acc.start(event, "read_file", None);
        acc.reset();
        assert!(!acc.is_active());
        assert!(acc.finalize().is_none());
    }

    // --- TimedFlush tests ---

    #[test]
    fn test_timed_flush_fresh_not_expired() {
        let flush = TimedFlush::new(Duration::from_secs(10));
        assert!(!flush.is_expired());
    }

    #[test]
    fn test_timed_flush_expired() {
        let mut flush = TimedFlush::new(Duration::from_millis(1));
        // Force expiry by sleeping briefly
        std::thread::sleep(Duration::from_millis(5));
        assert!(flush.is_expired());
    }

    #[test]
    fn test_timed_flush_touch_resets() {
        let mut flush = TimedFlush::new(Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(5));
        flush.touch();
        assert!(!flush.is_expired());
    }

    #[test]
    fn test_timed_flush_elapsed() {
        let flush = TimedFlush::new(Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(10));
        assert!(flush.elapsed() >= Duration::from_millis(5));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib normalizer::state_machines::accumulator::tests -- --nocapture`
Expected: FAIL (placeholder impls return wrong values)

- [ ] **Step 4: Implement `TextAccumulator`**

Replace the placeholder `TextAccumulator` impl with:

```rust
impl TextAccumulator {
    pub fn new() -> Self {
        Self { buffer: String::new() }
    }

    pub fn push(&mut self, text: &str) {
        self.buffer.push_str(text);
    }

    pub fn take(&mut self) -> String {
        std::mem::take(&mut self.buffer)
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn peek(&self) -> &str {
        &self.buffer
    }
}
```

- [ ] **Step 5: Implement `ToolInputAccumulator`**

Replace the placeholder `ToolInputAccumulator` impl with:

```rust
impl ToolInputAccumulator {
    pub fn new() -> Self {
        Self {
            tool_name: None,
            tool_id: None,
            input_buffer: String::new(),
            start_event: None,
        }
    }

    /// Start accumulating for a new tool. If already active, auto-flushes the
    /// pending tool (returns it as Some) before starting the new one.
    pub fn start(&mut self, event: AgentEvent, tool_name: &str, tool_id: Option<&str>) -> Option<AgentEvent> {
        let flushed = if self.is_active() {
            self.finalize()
        } else {
            None
        };
        self.tool_name = Some(tool_name.to_string());
        self.tool_id = tool_id.map(|s| s.to_string());
        self.input_buffer.clear();
        self.start_event = Some(event);
        flushed
    }

    pub fn push_input(&mut self, fragment: &str) {
        self.input_buffer.push_str(fragment);
    }

    pub fn is_active(&self) -> bool {
        self.start_event.is_some()
    }

    /// Returns assembled tool_use event with accumulated input, clears state.
    pub fn finalize(&mut self) -> Option<AgentEvent> {
        let mut event = self.start_event.take()?;
        if !self.input_buffer.is_empty() {
            let parsed = serde_json::from_str(&self.input_buffer)
                .unwrap_or(serde_json::Value::String(self.input_buffer.clone()));
            event.content = EventContent::Json { value: parsed };
        }
        event.metadata.tool_name = self.tool_name.take();
        self.tool_id = None;
        self.input_buffer.clear();
        Some(event)
    }

    pub fn reset(&mut self) {
        self.tool_name = None;
        self.tool_id = None;
        self.input_buffer.clear();
        self.start_event = None;
    }
}
```

- [ ] **Step 6: Implement `TimedFlush`**

Replace the placeholder `TimedFlush` impl with:

```rust
impl TimedFlush {
    pub fn new(timeout: Duration) -> Self {
        Self {
            last_event_at: Instant::now(),
            timeout,
        }
    }

    pub fn touch(&mut self) {
        self.last_event_at = Instant::now();
    }

    pub fn is_expired(&self) -> bool {
        self.last_event_at.elapsed() >= self.timeout
    }

    pub fn elapsed(&self) -> Duration {
        self.last_event_at.elapsed()
    }
}
```

- [ ] **Step 7: Run all accumulator tests**

Run: `cd src-tauri && cargo test --lib normalizer::state_machines::accumulator::tests -- --nocapture`
Expected: ALL 12 PASS

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/normalizer/state_machines/accumulator.rs src-tauri/src/normalizer/state_machines/mod.rs
git commit -m "feat: add shared accumulator module (TextAccumulator, ToolInputAccumulator, TimedFlush)"
```

---

### Task 4: Claude TOML Updates

**Files:**
- Modify: `src-tauri/normalizers/claude.toml`

- [ ] **Step 1: Add `tool_input_delta` rule**

Insert after the `tool_start` rule block (after line 61 in `claude.toml`):

```toml
[[rules]]
name = "tool_input_delta"
when = 'type == "content_block_delta" && delta.type == "input_json_delta"'
emit = "tool_use"

[rules.mappings]
content = "delta.partial_json"
```

- [ ] **Step 2: Add `block_stop` rule**

Insert after the `tool_input_delta` rule:

```toml
[[rules]]
name = "block_stop"
when = 'type == "content_block_stop"'
emit = "status"
```

- [ ] **Step 3: Run existing normalizer tests to verify nothing breaks**

Run: `cd src-tauri && cargo test --lib normalizer -- --nocapture 2>&1 | tail -10`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/normalizers/claude.toml
git commit -m "feat: add tool_input_delta and block_stop rules to claude.toml"
```

---

### Task 5: Gemini State Machine + TOML

**Files:**
- Create: `src-tauri/src/normalizer/state_machines/gemini.rs`
- Create: `src-tauri/normalizers/gemini.toml`
- Modify: `src-tauri/src/normalizer/state_machines/mod.rs`

- [ ] **Step 1: Register gemini module**

In `src-tauri/src/normalizer/state_machines/mod.rs`, add:

```rust
pub mod gemini;
```

- [ ] **Step 2: Write 5 failing tests for GeminiStateMachine**

Create `src-tauri/src/normalizer/state_machines/gemini.rs`:

```rust
use super::StateMachine;
use super::accumulator::{ToolInputAccumulator, TimedFlush};
use crate::agent_event::{AgentEvent, AgentEventType, EventContent};
use std::time::Duration;

const FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

pub struct GeminiStateMachine {
    tool_accumulator: ToolInputAccumulator,
    timed_flush: TimedFlush,
}

impl GeminiStateMachine {
    pub fn new() -> Self {
        Self {
            tool_accumulator: ToolInputAccumulator::new(),
            timed_flush: TimedFlush::new(FLUSH_TIMEOUT),
        }
    }

    fn flush_with_incomplete(&mut self) -> Vec<AgentEvent> {
        if let Some(mut event) = self.tool_accumulator.finalize() {
            event.metadata.incomplete = Some(true);
            vec![event]
        } else {
            vec![]
        }
    }
}

impl StateMachine for GeminiStateMachine {
    fn process(&mut self, event: AgentEvent) -> Vec<AgentEvent> {
        // Check timeout before processing
        if self.tool_accumulator.is_active() && self.timed_flush.is_expired() {
            let mut flushed = self.flush_with_incomplete();
            flushed.push(event);
            return flushed;
        }
        self.timed_flush.touch();

        match event.event_type {
            AgentEventType::ToolUse => {
                if self.tool_accumulator.is_active() {
                    // Accumulate input fragment
                    if let EventContent::Text { ref value } = event.content {
                        self.tool_accumulator.push_input(value);
                    } else if let EventContent::Json { ref value } = event.content {
                        self.tool_accumulator.push_input(&value.to_string());
                    }
                    vec![]
                } else {
                    // Start accumulation
                    let tool_name = event.metadata.tool_name.clone().unwrap_or_default();
                    let parent_id = event.parent_id.clone();
                    let flushed = self.tool_accumulator.start(
                        event,
                        &tool_name,
                        parent_id.as_deref(),
                    );
                    flushed.into_iter().collect()
                }
            }
            _ => {
                // Flush pending tool before passing through
                let mut result = Vec::new();
                if self.tool_accumulator.is_active() {
                    if let Some(tool_event) = self.tool_accumulator.finalize() {
                        result.push(tool_event);
                    }
                }
                result.push(event);
                result
            }
        }
    }

    fn reset(&mut self) {
        self.tool_accumulator.reset();
        self.timed_flush = TimedFlush::new(FLUSH_TIMEOUT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::ErrorSeverity;

    #[test]
    fn test_text_passes_through() {
        let mut sm = GeminiStateMachine::new();
        let event = AgentEvent::text("hello", "gemini");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Text);
    }

    #[test]
    fn test_tool_use_accumulates_and_flushes() {
        let mut sm = GeminiStateMachine::new();

        // Start tool
        let start = AgentEvent::tool_use("read_file", r#"{"path":"test.rs"}"#, "gemini");
        let result = sm.process(start);
        assert_eq!(result.len(), 0); // accumulating

        // Non-tool event flushes
        let text = AgentEvent::text("done", "gemini");
        let result = sm.process(text);
        assert_eq!(result.len(), 2); // flushed tool + text
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
        assert_eq!(result[1].event_type, AgentEventType::Text);
    }

    #[test]
    fn test_multiple_tool_input_accumulates() {
        let mut sm = GeminiStateMachine::new();

        let start = AgentEvent::tool_use("read_file", "{}", "gemini");
        sm.process(start);

        // Subsequent tool_use events accumulate input
        let mut chunk = AgentEvent::text(r#"{"path":"test.rs"}"#, "gemini");
        chunk.event_type = AgentEventType::ToolUse;
        sm.process(chunk);

        // Flush on done
        let done = AgentEvent::done("sess", "gemini");
        let result = sm.process(done);
        assert_eq!(result.len(), 2); // tool + done
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
    }

    #[test]
    fn test_reset_clears() {
        let mut sm = GeminiStateMachine::new();
        let start = AgentEvent::tool_use("read_file", "{}", "gemini");
        sm.process(start);
        sm.reset();
        let text = AgentEvent::text("hello", "gemini");
        let result = sm.process(text);
        assert_eq!(result.len(), 1); // just text, no flushed tool
    }

    #[test]
    fn test_timeout_flush_emits_incomplete() {
        let mut sm = GeminiStateMachine::new();
        sm.timed_flush = TimedFlush::new(Duration::from_millis(1));

        let start = AgentEvent::tool_use("read_file", "{}", "gemini");
        sm.process(start);

        std::thread::sleep(Duration::from_millis(5));

        let text = AgentEvent::text("next", "gemini");
        let result = sm.process(text);
        // Should have flushed tool (incomplete) + text
        assert!(result.len() >= 2);
        assert_eq!(result[0].metadata.incomplete, Some(true));
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib normalizer::state_machines::gemini::tests -- --nocapture`
Expected: ALL 5 PASS

- [ ] **Step 4: Create Gemini TOML normalizer**

Create `src-tauri/normalizers/gemini.toml` with the full content from spec Section 2 (Gemini TOML). Copy the exact TOML from the spec.

- [ ] **Step 5: Run full test suite**

Run: `cd src-tauri && cargo test --lib -- --nocapture 2>&1 | tail -5`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/normalizer/state_machines/gemini.rs src-tauri/src/normalizer/state_machines/mod.rs src-tauri/normalizers/gemini.toml
git commit -m "feat: add Gemini state machine and TOML normalizer"
```

---

### Task 6: Kimi State Machine + TOML

**Files:**
- Create: `src-tauri/src/normalizer/state_machines/kimi.rs`
- Create: `src-tauri/normalizers/kimi.toml`
- Modify: `src-tauri/src/normalizer/state_machines/mod.rs`

- [ ] **Step 1: Register kimi module**

In `mod.rs`, add:

```rust
pub mod kimi;
```

- [ ] **Step 2: Create KimiStateMachine with 5 tests**

Create `src-tauri/src/normalizer/state_machines/kimi.rs`:

```rust
use super::StateMachine;
use super::accumulator::{ToolInputAccumulator, TimedFlush};
use crate::agent_event::{AgentEvent, AgentEventType, EventContent};
use std::time::Duration;

const FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

/// Kimi state machine — Claude-like content_block lifecycle.
/// tool_use start → N input deltas → status (content_block_stop) → flush.
pub struct KimiStateMachine {
    tool_accumulator: ToolInputAccumulator,
    timed_flush: TimedFlush,
}

impl KimiStateMachine {
    pub fn new() -> Self {
        Self {
            tool_accumulator: ToolInputAccumulator::new(),
            timed_flush: TimedFlush::new(FLUSH_TIMEOUT),
        }
    }

    fn flush_with_incomplete(&mut self) -> Vec<AgentEvent> {
        if let Some(mut event) = self.tool_accumulator.finalize() {
            event.metadata.incomplete = Some(true);
            vec![event]
        } else {
            vec![]
        }
    }
}

impl StateMachine for KimiStateMachine {
    fn process(&mut self, event: AgentEvent) -> Vec<AgentEvent> {
        // Check timeout before processing
        if self.tool_accumulator.is_active() && self.timed_flush.is_expired() {
            let mut flushed = self.flush_with_incomplete();
            flushed.push(event);
            return flushed;
        }
        self.timed_flush.touch();

        match event.event_type {
            AgentEventType::ToolUse => {
                if self.tool_accumulator.is_active() {
                    // Accumulate input fragment (tool_input_delta)
                    if let EventContent::Text { ref value } = event.content {
                        self.tool_accumulator.push_input(value);
                    } else if let EventContent::Json { ref value } = event.content {
                        self.tool_accumulator.push_input(&value.to_string());
                    }
                    vec![]
                } else {
                    // Start accumulation (content_block_start)
                    let tool_name = event.metadata.tool_name.clone().unwrap_or_default();
                    let parent_id = event.parent_id.clone();
                    let flushed = self.tool_accumulator.start(
                        event,
                        &tool_name,
                        parent_id.as_deref(),
                    );
                    flushed.into_iter().collect()
                }
            }
            AgentEventType::Status => {
                // content_block_stop — finalize tool
                if let Some(tool_event) = self.tool_accumulator.finalize() {
                    vec![tool_event]
                } else {
                    vec![event]
                }
            }
            // Thinking, Text, Error, Usage, Done — pass through
            _ => vec![event],
        }
    }

    fn reset(&mut self) {
        self.tool_accumulator.reset();
        self.timed_flush = TimedFlush::new(FLUSH_TIMEOUT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_passes_through() {
        let mut sm = KimiStateMachine::new();
        let event = AgentEvent::text("hello", "kimi");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Text);
    }

    #[test]
    fn test_tool_accumulates_until_status() {
        let mut sm = KimiStateMachine::new();

        // Start tool
        let start = AgentEvent::tool_use("read_file", "{}", "kimi");
        assert_eq!(sm.process(start).len(), 0);

        // Input delta
        let mut chunk = AgentEvent::text(r#"{"path":"a.rs"}"#, "kimi");
        chunk.event_type = AgentEventType::ToolUse;
        assert_eq!(sm.process(chunk).len(), 0);

        // Status (content_block_stop) flushes
        let status = AgentEvent::status("content_block_stop", "kimi");
        let result = sm.process(status);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
    }

    #[test]
    fn test_thinking_passes_through() {
        let mut sm = KimiStateMachine::new();
        let event = AgentEvent::thinking("reasoning...", "kimi");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Thinking);
    }

    #[test]
    fn test_reset_clears() {
        let mut sm = KimiStateMachine::new();
        let start = AgentEvent::tool_use("read_file", "{}", "kimi");
        sm.process(start);
        sm.reset();
        let text = AgentEvent::text("hello", "kimi");
        let result = sm.process(text);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_timeout_flush_emits_incomplete() {
        let mut sm = KimiStateMachine::new();
        sm.timed_flush = TimedFlush::new(Duration::from_millis(1));

        let start = AgentEvent::tool_use("read_file", "{}", "kimi");
        sm.process(start);

        std::thread::sleep(Duration::from_millis(5));

        let text = AgentEvent::text("next", "kimi");
        let result = sm.process(text);
        assert!(result.len() >= 2);
        assert_eq!(result[0].metadata.incomplete, Some(true));
    }
}
```

- [ ] **Step 3: Create Kimi TOML normalizer**

Create `src-tauri/normalizers/kimi.toml` with the exact content from spec Section 2 (Kimi TOML).

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test --lib normalizer::state_machines::kimi::tests -- --nocapture`
Expected: ALL 5 PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/normalizer/state_machines/kimi.rs src-tauri/src/normalizer/state_machines/mod.rs src-tauri/normalizers/kimi.toml
git commit -m "feat: add Kimi state machine and TOML normalizer"
```

---

### Task 7: Qwen State Machine + TOML

**Files:**
- Create: `src-tauri/src/normalizer/state_machines/qwen.rs`
- Create: `src-tauri/normalizers/qwen.toml`
- Modify: `src-tauri/src/normalizer/state_machines/mod.rs`

- [ ] **Step 1: Register qwen module**

In `mod.rs`, add:

```rust
pub mod qwen;
```

- [ ] **Step 2: Create QwenStateMachine with 5 tests**

Create `src-tauri/src/normalizer/state_machines/qwen.rs`. Same structure as Kimi (Claude-like block lifecycle), but without thinking support:

```rust
use super::StateMachine;
use super::accumulator::{ToolInputAccumulator, TimedFlush};
use crate::agent_event::{AgentEvent, AgentEventType, EventContent};
use std::time::Duration;

const FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

/// Qwen state machine — Claude-like content_block lifecycle.
/// Same pattern as Kimi, minus native thinking support.
pub struct QwenStateMachine {
    tool_accumulator: ToolInputAccumulator,
    timed_flush: TimedFlush,
}

impl QwenStateMachine {
    pub fn new() -> Self {
        Self {
            tool_accumulator: ToolInputAccumulator::new(),
            timed_flush: TimedFlush::new(FLUSH_TIMEOUT),
        }
    }

    fn flush_with_incomplete(&mut self) -> Vec<AgentEvent> {
        if let Some(mut event) = self.tool_accumulator.finalize() {
            event.metadata.incomplete = Some(true);
            vec![event]
        } else {
            vec![]
        }
    }
}

impl StateMachine for QwenStateMachine {
    fn process(&mut self, event: AgentEvent) -> Vec<AgentEvent> {
        if self.tool_accumulator.is_active() && self.timed_flush.is_expired() {
            let mut flushed = self.flush_with_incomplete();
            flushed.push(event);
            return flushed;
        }
        self.timed_flush.touch();

        match event.event_type {
            AgentEventType::ToolUse => {
                if self.tool_accumulator.is_active() {
                    if let EventContent::Text { ref value } = event.content {
                        self.tool_accumulator.push_input(value);
                    } else if let EventContent::Json { ref value } = event.content {
                        self.tool_accumulator.push_input(&value.to_string());
                    }
                    vec![]
                } else {
                    let tool_name = event.metadata.tool_name.clone().unwrap_or_default();
                    let parent_id = event.parent_id.clone();
                    let flushed = self.tool_accumulator.start(
                        event,
                        &tool_name,
                        parent_id.as_deref(),
                    );
                    flushed.into_iter().collect()
                }
            }
            AgentEventType::Status => {
                if let Some(tool_event) = self.tool_accumulator.finalize() {
                    vec![tool_event]
                } else {
                    vec![event]
                }
            }
            _ => vec![event],
        }
    }

    fn reset(&mut self) {
        self.tool_accumulator.reset();
        self.timed_flush = TimedFlush::new(FLUSH_TIMEOUT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_passes_through() {
        let mut sm = QwenStateMachine::new();
        let event = AgentEvent::text("hello", "qwen");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::Text);
    }

    #[test]
    fn test_tool_accumulates_until_status() {
        let mut sm = QwenStateMachine::new();

        let start = AgentEvent::tool_use("read_file", "{}", "qwen");
        assert_eq!(sm.process(start).len(), 0);

        let mut chunk = AgentEvent::text(r#"{"path":"a.rs"}"#, "qwen");
        chunk.event_type = AgentEventType::ToolUse;
        assert_eq!(sm.process(chunk).len(), 0);

        let status = AgentEvent::status("content_block_stop", "qwen");
        let result = sm.process(status);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
    }

    #[test]
    fn test_assistant_text_passes_through() {
        let mut sm = QwenStateMachine::new();
        // Qwen's "assistant" level text events pass through
        let event = AgentEvent::text("full message", "qwen");
        let result = sm.process(event);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_reset_clears() {
        let mut sm = QwenStateMachine::new();
        let start = AgentEvent::tool_use("read_file", "{}", "qwen");
        sm.process(start);
        sm.reset();
        let text = AgentEvent::text("hello", "qwen");
        let result = sm.process(text);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_timeout_flush_emits_incomplete() {
        let mut sm = QwenStateMachine::new();
        sm.timed_flush = TimedFlush::new(Duration::from_millis(1));

        let start = AgentEvent::tool_use("read_file", "{}", "qwen");
        sm.process(start);

        std::thread::sleep(Duration::from_millis(5));

        let text = AgentEvent::text("next", "qwen");
        let result = sm.process(text);
        assert!(result.len() >= 2);
        assert_eq!(result[0].metadata.incomplete, Some(true));
    }
}
```

- [ ] **Step 3: Create Qwen TOML normalizer**

Create `src-tauri/normalizers/qwen.toml` with the exact content from spec Section 2 (Qwen TOML).

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test --lib normalizer::state_machines::qwen::tests -- --nocapture`
Expected: ALL 5 PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/normalizer/state_machines/qwen.rs src-tauri/src/normalizer/state_machines/mod.rs src-tauri/normalizers/qwen.toml
git commit -m "feat: add Qwen state machine and TOML normalizer"
```

---

### Task 8: Codex State Machine + TOML

**Files:**
- Create: `src-tauri/src/normalizer/state_machines/codex.rs`
- Create: `src-tauri/normalizers/codex.toml`
- Modify: `src-tauri/src/normalizer/state_machines/mod.rs`

- [ ] **Step 1: Register codex module**

In `mod.rs`, add:

```rust
pub mod codex;
```

- [ ] **Step 2: Create CodexStateMachine with 6 tests**

Create `src-tauri/src/normalizer/state_machines/codex.rs`:

```rust
use super::StateMachine;
use super::accumulator::{TextAccumulator, ToolInputAccumulator, TimedFlush};
use crate::agent_event::{AgentEvent, AgentEventType, EventContent};
use std::time::Duration;

const FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

/// Codex (OpenAI) state machine — JSON-RPC v2 protocol.
/// Text deltas (AgentMessageDeltaNotification) accumulate until a non-delta event.
/// ItemCompletedNotification events (reasoning, commandExecution, mcpToolCall) arrive
/// pre-assembled and pass through after flushing pending text.
pub struct CodexStateMachine {
    text_accumulator: TextAccumulator,
    tool_accumulator: ToolInputAccumulator,
    timed_flush: TimedFlush,
    provider: String,
}

impl CodexStateMachine {
    pub fn new() -> Self {
        Self {
            text_accumulator: TextAccumulator::new(),
            tool_accumulator: ToolInputAccumulator::new(),
            timed_flush: TimedFlush::new(FLUSH_TIMEOUT),
            provider: "codex".to_string(),
        }
    }

    fn flush_pending_text(&mut self) -> Option<AgentEvent> {
        if !self.text_accumulator.is_empty() {
            let text = self.text_accumulator.take();
            Some(AgentEvent::text(&text, &self.provider))
        } else {
            None
        }
    }

    fn flush_all_with_incomplete(&mut self) -> Vec<AgentEvent> {
        let mut result = Vec::new();
        if !self.text_accumulator.is_empty() {
            let text = self.text_accumulator.take();
            let mut event = AgentEvent::text(&text, &self.provider);
            event.metadata.incomplete = Some(true);
            result.push(event);
        }
        if let Some(mut tool_event) = self.tool_accumulator.finalize() {
            tool_event.metadata.incomplete = Some(true);
            result.push(tool_event);
        }
        result
    }
}

impl StateMachine for CodexStateMachine {
    fn process(&mut self, event: AgentEvent) -> Vec<AgentEvent> {
        // Check timeout
        if self.timed_flush.is_expired() && (!self.text_accumulator.is_empty() || self.tool_accumulator.is_active()) {
            let mut flushed = self.flush_all_with_incomplete();
            flushed.push(event);
            return flushed;
        }
        self.timed_flush.touch();

        match event.event_type {
            AgentEventType::Text => {
                // Accumulate text deltas
                if let EventContent::Text { ref value } = event.content {
                    self.text_accumulator.push(value);
                }
                vec![]
            }
            AgentEventType::Thinking | AgentEventType::ToolUse | AgentEventType::ToolResult => {
                // Flush pending text, then pass through
                let mut result = Vec::new();
                if let Some(text_event) = self.flush_pending_text() {
                    result.push(text_event);
                }
                result.push(event);
                result
            }
            AgentEventType::Usage | AgentEventType::Done | AgentEventType::Error => {
                // Flush pending text, then pass through
                let mut result = Vec::new();
                if let Some(text_event) = self.flush_pending_text() {
                    result.push(text_event);
                }
                result.push(event);
                result
            }
            _ => vec![event],
        }
    }

    fn reset(&mut self) {
        self.text_accumulator = TextAccumulator::new();
        self.tool_accumulator.reset();
        self.timed_flush = TimedFlush::new(FLUSH_TIMEOUT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_text_delta_accumulates() {
        let mut sm = CodexStateMachine::new();
        let event = AgentEvent::text("hello", "codex");
        let result = sm.process(event);
        assert_eq!(result.len(), 0); // accumulating
    }

    #[test]
    fn test_text_deltas_flush_on_done() {
        let mut sm = CodexStateMachine::new();
        sm.process(AgentEvent::text("hello ", "codex"));
        sm.process(AgentEvent::text("world", "codex"));

        let done = AgentEvent::done("sess", "codex");
        let result = sm.process(done);
        assert_eq!(result.len(), 2); // flushed text + done
        assert_eq!(result[0].event_type, AgentEventType::Text);
        if let EventContent::Text { ref value } = result[0].content {
            assert_eq!(value, "hello world");
        } else {
            panic!("Expected Text content");
        }
        assert_eq!(result[1].event_type, AgentEventType::Done);
    }

    #[test]
    fn test_thinking_flushes_pending_text() {
        let mut sm = CodexStateMachine::new();
        sm.process(AgentEvent::text("partial", "codex"));

        let thinking = AgentEvent::thinking("reasoning...", "codex");
        let result = sm.process(thinking);
        assert_eq!(result.len(), 2); // flushed text + thinking
        assert_eq!(result[0].event_type, AgentEventType::Text);
        assert_eq!(result[1].event_type, AgentEventType::Thinking);
    }

    #[test]
    fn test_tool_use_passes_through() {
        let mut sm = CodexStateMachine::new();
        let tool = AgentEvent::tool_use("bash", r#"{"cmd":"ls"}"#, "codex");
        let result = sm.process(tool);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, AgentEventType::ToolUse);
    }

    #[test]
    fn test_reset_clears_all() {
        let mut sm = CodexStateMachine::new();
        sm.process(AgentEvent::text("pending", "codex"));
        sm.reset();
        let done = AgentEvent::done("sess", "codex");
        let result = sm.process(done);
        assert_eq!(result.len(), 1); // just done, no flushed text
    }

    #[test]
    fn test_timeout_flush_emits_incomplete() {
        let mut sm = CodexStateMachine::new();
        sm.timed_flush = TimedFlush::new(Duration::from_millis(1));

        sm.process(AgentEvent::text("partial", "codex"));
        std::thread::sleep(Duration::from_millis(5));

        let next = AgentEvent::text("next", "codex");
        let result = sm.process(next);
        assert!(result.len() >= 1);
        assert_eq!(result[0].metadata.incomplete, Some(true));
    }
}
```

- [ ] **Step 3: Create Codex TOML normalizer**

Create `src-tauri/normalizers/codex.toml` with the exact content from spec Section 2 (Codex TOML).

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test --lib normalizer::state_machines::codex::tests -- --nocapture`
Expected: ALL 6 PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/normalizer/state_machines/codex.rs src-tauri/src/normalizer/state_machines/mod.rs src-tauri/normalizers/codex.toml
git commit -m "feat: add Codex state machine and TOML normalizer"
```

---

### Task 9: Provider Routing — Transport + Normalizer State Machine Selection

**Files:**
- Modify: `src-tauri/src/transport/mod.rs:80-83`
- Modify: `src-tauri/src/normalizer/mod.rs:143-146` and `189-192`

- [ ] **Step 1: Update transport/mod.rs state machine match**

In `src-tauri/src/transport/mod.rs`, replace lines 80-83:

```rust
let state_machine: Box<dyn crate::normalizer::state_machines::StateMachine> = match provider.as_str() {
    "claude" => Box::new(crate::normalizer::state_machines::claude::ClaudeStateMachine::new()),
    _ => Box::new(crate::normalizer::state_machines::generic::GenericStateMachine::new()),
};
```

with:

```rust
let state_machine: Box<dyn crate::normalizer::state_machines::StateMachine> = match provider.as_str() {
    "claude" => Box::new(crate::normalizer::state_machines::claude::ClaudeStateMachine::new()),
    "gemini" => Box::new(crate::normalizer::state_machines::gemini::GeminiStateMachine::new()),
    "kimi" => Box::new(crate::normalizer::state_machines::kimi::KimiStateMachine::new()),
    "qwen" => Box::new(crate::normalizer::state_machines::qwen::QwenStateMachine::new()),
    "codex" => Box::new(crate::normalizer::state_machines::codex::CodexStateMachine::new()),
    _ => Box::new(crate::normalizer::state_machines::generic::GenericStateMachine::new()),
};
```

- [ ] **Step 2: Update normalizer/mod.rs `load_from_dir` state machine match**

In `src-tauri/src/normalizer/mod.rs`, replace lines 143-146:

```rust
let state_machine: Box<dyn StateMachine> = match name.as_str() {
    "claude" => Box::new(ClaudeStateMachine::new()),
    _ => Box::new(GenericStateMachine::new()),
};
```

with:

```rust
let state_machine: Box<dyn StateMachine> = match name.as_str() {
    "claude" => Box::new(ClaudeStateMachine::new()),
    "gemini" => Box::new(state_machines::gemini::GeminiStateMachine::new()),
    "kimi" => Box::new(state_machines::kimi::KimiStateMachine::new()),
    "qwen" => Box::new(state_machines::qwen::QwenStateMachine::new()),
    "codex" => Box::new(state_machines::codex::CodexStateMachine::new()),
    _ => Box::new(GenericStateMachine::new()),
};
```

- [ ] **Step 3: Update normalizer/mod.rs `reload_provider` state machine match**

In `src-tauri/src/normalizer/mod.rs`, replace lines 189-192:

```rust
let state_machine: Box<dyn StateMachine> = match provider {
    "claude" => Box::new(ClaudeStateMachine::new()),
    _ => Box::new(GenericStateMachine::new()),
};
```

with:

```rust
let state_machine: Box<dyn StateMachine> = match provider {
    "claude" => Box::new(ClaudeStateMachine::new()),
    "gemini" => Box::new(state_machines::gemini::GeminiStateMachine::new()),
    "kimi" => Box::new(state_machines::kimi::KimiStateMachine::new()),
    "qwen" => Box::new(state_machines::qwen::QwenStateMachine::new()),
    "codex" => Box::new(state_machines::codex::CodexStateMachine::new()),
    _ => Box::new(GenericStateMachine::new()),
};
```

- [ ] **Step 4: Run full test suite**

Run: `cd src-tauri && cargo test --lib -- --nocapture 2>&1 | tail -5`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/transport/mod.rs src-tauri/src/normalizer/mod.rs
git commit -m "feat: route new providers to dedicated state machines"
```

---

### Task 10: Discovery Extension

**Files:**
- Modify: `src-tauri/src/discovery.rs:48-97` (builtin_profiles) and `111-118` (scan_cli candidates)

- [ ] **Step 1: Add new providers to CLI scan candidates**

In `discovery.rs`, in the `scan_cli` method, add to the `candidates` vec (around line 111-118):

```rust
("Kimi", "kimi"),
("Qwen Code", "qwen"),
("Codex", "codex"),
```

- [ ] **Step 2: Add builtin profiles for new providers**

In `discovery.rs`, in the `builtin_profiles` function, add after the existing entries:

```rust
profiles.insert(
    "kimi".to_string(),
    (
        CapabilityProfile {
            read_file: true,
            write_file: true,
            execute_command: true,
            web_search: false,
            image_input: false,
            long_context: true,
        },
        vec![],
        Some(128_000),
    ),
);
profiles.insert(
    "qwen".to_string(),
    (
        CapabilityProfile {
            read_file: true,
            write_file: true,
            execute_command: true,
            web_search: false,
            image_input: false,
            long_context: true,
        },
        vec![],
        Some(128_000),
    ),
);
profiles.insert(
    "codex".to_string(),
    (
        CapabilityProfile {
            read_file: true,
            write_file: true,
            execute_command: true,
            web_search: false,
            image_input: false,
            long_context: true,
        },
        vec![],
        Some(200_000),
    ),
);
```

- [ ] **Step 3: Run discovery tests**

Run: `cd src-tauri && cargo test --lib discovery::tests -- --nocapture`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/discovery.rs
git commit -m "feat: add kimi, qwen, codex to discovery scan and builtin profiles"
```

---

### Task 11: Capabilities Wiring

**Files:**
- Modify: `src-tauri/src/lib.rs:107-116`

- [ ] **Step 1: Add capabilities wiring in setup hook**

In `src-tauri/src/lib.rs`, after the existing `updater.register_from_configs(&configs);` line (around line 116), add:

```rust
            // Register capabilities from TOML configs
            for (provider, config) in &configs {
                let mut features = std::collections::HashMap::new();
                for (key, val) in &config.capabilities {
                    let support = if val.as_bool() == Some(true) {
                        capability::FeatureSupport::Full
                    } else {
                        capability::FeatureSupport::Unsupported { alternative: None }
                    };
                    features.insert(key.clone(), support);
                }
                let caps = capability::NegotiatedCapabilities {
                    provider: provider.clone(),
                    cli_version: String::new(),
                    cli_mode: transport::request::CliMode::Structured,
                    features,
                    negotiated_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                };
                negotiator.set_capabilities(provider, caps);
            }
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check 2>&1 | tail -5`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: wire TOML capabilities to CapabilityNegotiator at app startup"
```

---

### Task 12: JSON Fixtures + Integration Tests

**Files:**
- Create: `src-tauri/normalizers/fixtures/gemini/*.jsonl` + `*.expected.json`
- Create: `src-tauri/normalizers/fixtures/kimi/*.jsonl` + `*.expected.json`
- Create: `src-tauri/normalizers/fixtures/qwen/*.jsonl` + `*.expected.json`
- Create: `src-tauri/normalizers/fixtures/codex/*.jsonl` + `*.expected.json`
- Create or modify: fixture test runner

- [ ] **Step 1: Create fixture directories**

```bash
mkdir -p src-tauri/normalizers/fixtures/{gemini,kimi,qwen,codex}
```

- [ ] **Step 2: Create Gemini fixtures**

Create `src-tauri/normalizers/fixtures/gemini/basic_text.jsonl`:
```jsonl
{"type":"MESSAGE","content":[{"text":"Hello, I'm Gemini.","type":"text"}],"role":"model"}
{"type":"RESULT","usage":{"input_tokens":10,"output_tokens":5},"session_id":"gemini-sess-1"}
{"type":"RESULT","session_id":"gemini-sess-1"}
```

> **Note:** The rules engine uses first-match semantics — one rule per JSON line. The `usage` rule matches the second line (has `exists(usage)`), and the `done` rule matches the third line (generic `RESULT`). They must be separate lines.

Create `src-tauri/normalizers/fixtures/gemini/basic_text.expected.json`:
```json
[
  {"event_type": "text", "content_contains": "Hello"},
  {"event_type": "usage", "has_input_tokens": true},
  {"event_type": "done"}
]
```

Create `src-tauri/normalizers/fixtures/gemini/tool_use.jsonl`:
```jsonl
{"type":"TOOL_USE","name":"read_file","id":"tool-1","args":"{\"path\":\"test.rs\"}"}
{"type":"TOOL_RESULT","tool_use_id":"tool-1","result":"file contents here"}
{"type":"RESULT","usage":{"input_tokens":20,"output_tokens":10},"session_id":"gemini-sess-2"}
```

Create `src-tauri/normalizers/fixtures/gemini/tool_use.expected.json`:
```json
[
  {"event_type": "tool_use", "has_tool_name": true},
  {"event_type": "tool_result"},
  {"event_type": "usage", "has_input_tokens": true},
  {"event_type": "done"}
]
```

Create `src-tauri/normalizers/fixtures/gemini/error.jsonl`:
```jsonl
{"type":"ERROR","code":"RESOURCE_EXHAUSTED","message":"Quota exceeded"}
```

Create `src-tauri/normalizers/fixtures/gemini/error.expected.json`:
```json
[
  {"event_type": "error", "severity": "recoverable", "error_code": "RESOURCE_EXHAUSTED"}
]
```

- [ ] **Step 3: Create Kimi fixtures**

Create `src-tauri/normalizers/fixtures/kimi/basic_text.jsonl`:
```jsonl
{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello from Kimi"}}
{"type":"message_delta","usage":{"input_tokens":8,"output_tokens":4}}
{"type":"message_stop"}
```

Create `src-tauri/normalizers/fixtures/kimi/basic_text.expected.json`:
```json
[
  {"event_type": "text", "content_contains": "Hello"},
  {"event_type": "usage", "has_input_tokens": true},
  {"event_type": "done"}
]
```

Create `src-tauri/normalizers/fixtures/kimi/thinking.jsonl`:
```jsonl
{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"Let me think..."}}
{"type":"content_block_delta","delta":{"type":"text_delta","text":"The answer is 4"}}
{"type":"message_stop"}
```

Create `src-tauri/normalizers/fixtures/kimi/thinking.expected.json`:
```json
[
  {"event_type": "thinking", "content_contains": "think"},
  {"event_type": "text", "content_contains": "answer"},
  {"event_type": "done"}
]
```

Create `src-tauri/normalizers/fixtures/kimi/tool_use.jsonl`:
```jsonl
{"type":"content_block_start","content_block":{"type":"tool_use","name":"read_file","id":"tool-k1","input":"{}"}}
{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{\"path\":"}}
{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"\"test.rs\"}"}}
{"type":"content_block_stop"}
{"type":"message_stop"}
```

Create `src-tauri/normalizers/fixtures/kimi/tool_use.expected.json`:
```json
[
  {"event_type": "tool_use", "has_tool_name": true},
  {"event_type": "done"}
]
```

Create `src-tauri/normalizers/fixtures/kimi/error.jsonl`:
```jsonl
{"type":"error","error":{"type":"overloaded","message":"Server is overloaded"}}
```

Create `src-tauri/normalizers/fixtures/kimi/error.expected.json`:
```json
[
  {"event_type": "error", "severity": "recoverable", "error_code": "overloaded"}
]
```

- [ ] **Step 4: Create Qwen fixtures**

Create `src-tauri/normalizers/fixtures/qwen/basic_text.jsonl`:
```jsonl
{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello from Qwen"}}
{"type":"result","subtype":"success","usage":{"input_tokens":8,"output_tokens":4}}
{"type":"result","subtype":"success"}
```

> **Note:** Same first-match rule applies — `usage` and `done` must be separate lines since both match `type == "result"` but with different conditions.

Create `src-tauri/normalizers/fixtures/qwen/basic_text.expected.json`:
```json
[
  {"event_type": "text", "content_contains": "Hello"},
  {"event_type": "usage", "has_input_tokens": true},
  {"event_type": "done"}
]
```

Create `src-tauri/normalizers/fixtures/qwen/tool_use.jsonl`:
```jsonl
{"type":"content_block_start","content_block":{"type":"tool_use","name":"read_file","id":"tool-q1","input":"{}"}}
{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{\"path\":\"test.rs\"}"}}
{"type":"content_block_stop"}
{"type":"result","subtype":"success","usage":{"input_tokens":15,"output_tokens":8}}
```

Create `src-tauri/normalizers/fixtures/qwen/tool_use.expected.json`:
```json
[
  {"event_type": "tool_use", "has_tool_name": true},
  {"event_type": "usage", "has_input_tokens": true},
  {"event_type": "done"}
]
```

Create `src-tauri/normalizers/fixtures/qwen/error.jsonl`:
```jsonl
{"type":"error","error":{"type":"overloaded","message":"Rate limit exceeded"}}
```

Create `src-tauri/normalizers/fixtures/qwen/error.expected.json`:
```json
[
  {"event_type": "error", "severity": "recoverable", "error_code": "overloaded"}
]
```

- [ ] **Step 5: Create Codex fixtures**

Create `src-tauri/normalizers/fixtures/codex/basic_text.jsonl`:
```jsonl
{"jsonrpc":"2.0","method":"AgentMessageDeltaNotification","params":{"delta":"Hello "}}
{"jsonrpc":"2.0","method":"AgentMessageDeltaNotification","params":{"delta":"from Codex"}}
{"jsonrpc":"2.0","method":"ThreadTokenUsageUpdatedNotification","params":{"usage":{"input_tokens":10,"output_tokens":5}}}
{"jsonrpc":"2.0","method":"TurnCompletedNotification","params":{}}
```

Create `src-tauri/normalizers/fixtures/codex/basic_text.expected.json`:
```json
[
  {"event_type": "text", "content_contains": "Hello from Codex"},
  {"event_type": "usage", "has_input_tokens": true},
  {"event_type": "done"}
]
```

Create `src-tauri/normalizers/fixtures/codex/reasoning.jsonl`:
```jsonl
{"jsonrpc":"2.0","method":"AgentMessageDeltaNotification","params":{"delta":"Let me think..."}}
{"jsonrpc":"2.0","method":"ItemCompletedNotification","params":{"item":{"type":"reasoning","content":"Step by step analysis"}}}
{"jsonrpc":"2.0","method":"TurnCompletedNotification","params":{}}
```

Create `src-tauri/normalizers/fixtures/codex/reasoning.expected.json`:
```json
[
  {"event_type": "text"},
  {"event_type": "thinking", "content_contains": "Step by step"},
  {"event_type": "done"}
]
```

Create `src-tauri/normalizers/fixtures/codex/tool_use.jsonl`:
```jsonl
{"jsonrpc":"2.0","method":"ItemCompletedNotification","params":{"item":{"type":"commandExecution","command":"ls -la","output":"total 0\ndrwx","id":"cmd-1"}}}
{"jsonrpc":"2.0","method":"TurnCompletedNotification","params":{}}
```

Create `src-tauri/normalizers/fixtures/codex/tool_use.expected.json`:
```json
[
  {"event_type": "tool_use", "has_tool_name": true},
  {"event_type": "done"}
]
```

Create `src-tauri/normalizers/fixtures/codex/error.jsonl`:
```jsonl
{"jsonrpc":"2.0","method":"ErrorNotification","params":{"code":"rate_limit","message":"Rate limit exceeded"}}
```

Create `src-tauri/normalizers/fixtures/codex/error.expected.json`:
```json
[
  {"event_type": "error", "severity": "recoverable", "error_code": "rate_limit"}
]
```

- [ ] **Step 6: Create fixture test runner**

Create `src-tauri/src/normalizer/fixture_tests.rs`:

```rust
//! Integration tests that replay JSON fixtures through the full normalizer pipeline.

use crate::normalizer::NormalizerRegistry;
use crate::agent_event::{AgentEvent, AgentEventType, ErrorSeverity};
use std::path::Path;
use serde_json::Value;

fn run_fixture_test(provider: &str, fixture_name: &str) {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixture_path = format!("{}/normalizers/fixtures/{}/{}.jsonl", base, provider, fixture_name);
    let expected_path = format!("{}/normalizers/fixtures/{}/{}.expected.json", base, provider, fixture_name);

    let fixture_data = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", fixture_path, e));
    let expected_data = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("Failed to read expected {}: {}", expected_path, e));

    let expected: Vec<Value> = serde_json::from_str(&expected_data)
        .unwrap_or_else(|e| panic!("Failed to parse expected JSON {}: {}", expected_path, e));

    // Load registry with all TOMLs
    let normalizers_dir = format!("{}/normalizers", base);
    let mut registry = NormalizerRegistry::load_from_dir(Path::new(&normalizers_dir))
        .expect("Failed to load normalizers");

    // Process each line through the pipeline
    let mut all_events: Vec<AgentEvent> = Vec::new();
    for line in fixture_data.lines() {
        if line.trim().is_empty() { continue; }
        let events = registry.process(provider, line);
        all_events.extend(events);
    }

    // Verify against expected
    let mut event_idx = 0;
    for (i, exp) in expected.iter().enumerate() {
        let exp_type = exp.get("event_type").and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("Expected entry {} missing event_type", i));

        // Find matching event
        let matching_event = loop {
            if event_idx >= all_events.len() {
                panic!(
                    "Expected event_type '{}' (entry {}) but no more events. Got {} events total: {:?}",
                    exp_type, i, all_events.len(),
                    all_events.iter().map(|e| format!("{:?}", e.event_type)).collect::<Vec<_>>()
                );
            }
            let ev = &all_events[event_idx];
            event_idx += 1;
            let type_str = format!("{:?}", ev.event_type).to_lowercase();
            if type_str == exp_type {
                break ev;
            }
        };

        // Validate optional assertions
        if let Some(pattern) = exp.get("content_contains").and_then(|v| v.as_str()) {
            let content_str = match &matching_event.content {
                crate::agent_event::EventContent::Text { value } => value.clone(),
                crate::agent_event::EventContent::Json { value } => value.to_string(),
                _ => String::new(),
            };
            assert!(
                content_str.contains(pattern),
                "Event {} content '{}' doesn't contain '{}'",
                i, content_str, pattern
            );
        }

        if exp.get("has_input_tokens") == Some(&Value::Bool(true)) {
            assert!(
                matching_event.metadata.input_tokens.is_some(),
                "Event {} expected input_tokens", i
            );
        }

        if exp.get("has_tool_name") == Some(&Value::Bool(true)) {
            assert!(
                matching_event.metadata.tool_name.is_some(),
                "Event {} expected tool_name", i
            );
        }

        if let Some(severity_str) = exp.get("severity").and_then(|v| v.as_str()) {
            let expected_severity = match severity_str {
                "recoverable" => Some(ErrorSeverity::Recoverable),
                "fatal" => Some(ErrorSeverity::Fatal),
                "degraded" => Some(ErrorSeverity::Degraded),
                _ => None,
            };
            assert_eq!(
                matching_event.metadata.error_severity, expected_severity,
                "Event {} expected severity {:?}", i, severity_str
            );
        }

        if let Some(code) = exp.get("error_code").and_then(|v| v.as_str()) {
            assert_eq!(
                matching_event.metadata.error_code.as_deref(), Some(code),
                "Event {} expected error_code {}", i, code
            );
        }
    }
}

// --- Gemini ---
#[test]
fn test_gemini_basic_text_fixture() { run_fixture_test("gemini", "basic_text"); }
#[test]
fn test_gemini_tool_use_fixture() { run_fixture_test("gemini", "tool_use"); }
#[test]
fn test_gemini_error_fixture() { run_fixture_test("gemini", "error"); }

// --- Kimi ---
#[test]
fn test_kimi_basic_text_fixture() { run_fixture_test("kimi", "basic_text"); }
#[test]
fn test_kimi_thinking_fixture() { run_fixture_test("kimi", "thinking"); }
#[test]
fn test_kimi_tool_use_fixture() { run_fixture_test("kimi", "tool_use"); }
#[test]
fn test_kimi_error_fixture() { run_fixture_test("kimi", "error"); }

// --- Qwen ---
#[test]
fn test_qwen_basic_text_fixture() { run_fixture_test("qwen", "basic_text"); }
#[test]
fn test_qwen_tool_use_fixture() { run_fixture_test("qwen", "tool_use"); }
#[test]
fn test_qwen_error_fixture() { run_fixture_test("qwen", "error"); }

// --- Codex ---
#[test]
fn test_codex_basic_text_fixture() { run_fixture_test("codex", "basic_text"); }
#[test]
fn test_codex_reasoning_fixture() { run_fixture_test("codex", "reasoning"); }
#[test]
fn test_codex_tool_use_fixture() { run_fixture_test("codex", "tool_use"); }
#[test]
fn test_codex_error_fixture() { run_fixture_test("codex", "error"); }
```

- [ ] **Step 7: Register fixture_tests module**

In `src-tauri/src/normalizer/mod.rs`, add at the bottom (after existing test modules):

```rust
#[cfg(test)]
mod fixture_tests;
```

- [ ] **Step 8: Run fixture tests**

Run: `cd src-tauri && cargo test --lib normalizer::fixture_tests -- --nocapture`
Expected: ALL 14 PASS

- [ ] **Step 9: Commit**

```bash
git add src-tauri/normalizers/fixtures/ src-tauri/src/normalizer/fixture_tests.rs src-tauri/src/normalizer/mod.rs
git commit -m "feat: add JSON fixtures and integration tests for all 4 providers"
```

---

### Task 13: Final Verification

**Files:** None (test-only)

- [ ] **Step 1: Run full test suite**

Run: `cd src-tauri && cargo test --lib -- --nocapture 2>&1 | tail -20`
Expected: ALL PASS — should show ~50+ new tests on top of existing

- [ ] **Step 2: Run cargo check for warnings**

Run: `cd src-tauri && cargo check 2>&1 | grep "warning\|error"`
Expected: No errors. Fix any warnings.

- [ ] **Step 3: Verify all TOMLs load correctly**

Run: `cd src-tauri && cargo test --lib normalizer::tests::test_registry_load -- --nocapture`
Expected: PASS — registry discovers all 5 providers (claude, gemini, kimi, qwen, codex)

- [ ] **Step 4: Count total test results**

Run: `cd src-tauri && cargo test --lib 2>&1 | grep "test result"`
Expected: `test result: ok. N passed; 0 failed` where N includes all new tests
