# Phase 7B: Analytics Collector — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add session-level analytics collection to the transport layer, enabling per-provider/model usage tracking, trend analysis, and comparison.

**Architecture:** AnalyticsCollector subscribes to EventBus, accumulates per-session metrics (SessionMetrics) from normalized events, persists completed sessions to JSONL. Aggregations (ProviderAnalytics, ModelAnalytics, DailyStats) computed on-demand. New metadata fields in AgentEventMetadata carry cache tokens, duration, cost from CLI output via TOML mappings.

**Tech Stack:** Rust, serde/serde_json, Tauri managed state, JSONL persistence, existing EventBus subscriber pattern.

**Spec:** `docs/superpowers/specs/2026-03-22-phase7b-analytics-collector-design.md`

---

## File Structure

### New files

| File | Responsibility |
|------|---------------|
| `src-tauri/src/analytics/mod.rs` | Data types: `SessionMetrics`, `ErrorRecord`, `ProviderAnalytics`, `ModelAnalytics`, `DailyStats`, `TimeRange` |
| `src-tauri/src/analytics/store.rs` | `AnalyticsStore`: JSONL persistence (append, load, all_completed) |
| `src-tauri/src/analytics/collector.rs` | `AnalyticsCollector`: EventBus subscriber + accumulation logic + query/aggregation methods |
| `src-tauri/src/commands/analytics.rs` | Tauri commands wrapping collector query methods |

### Modified files

| File | What changes |
|------|-------------|
| `src-tauri/src/agent_event.rs:62-106` | Add 10 `Option<T>` fields to `AgentEventMetadata`, update `base_metadata()` |
| `src-tauri/src/normalizer/pipeline.rs:74-105` | Extract new metadata fields in `build_event()` |
| `src-tauri/normalizers/claude.toml:96-108` | Add `result_metrics` rule with cache/duration/cost mappings |
| `src-tauri/normalizers/gemini.toml:90-95` | Add `cache_read_tokens`, `duration_ms` to usage rule |
| `src-tauri/normalizers/kimi.toml:83-89` | Add `context_metrics` rule |
| `src-tauri/normalizers/qwen.toml:83-89` | Add `duration_ms`, `duration_api_ms`, `num_turns` to usage rule |
| `src-tauri/normalizers/codex.toml:88-94` | Add `cache_read_tokens` to usage rule |
| `src-tauri/src/commands/mod.rs:13` | Add `pub mod analytics;` |
| `src-tauri/src/lib.rs:1-4,56-72,81-141,144-212` | Add `mod analytics`, register collector as managed state, wire EventBus, register commands |

---

### Task 1: Extend AgentEventMetadata with new fields

**Files:**
- Modify: `src-tauri/src/agent_event.rs:62-106`

- [ ] **Step 1: Write tests for new metadata fields**

Add to the existing `tests` module at the bottom of `agent_event.rs`:

```rust
#[test]
fn test_metadata_new_fields_default_none() {
    let meta = AgentEventMetadata {
        session_id: None,
        input_tokens: None,
        output_tokens: None,
        tool_name: None,
        model: None,
        provider: "claude".to_string(),
        error_severity: None,
        error_code: None,
        stream_metrics: None,
        incomplete: None,
        cache_creation_tokens: None,
        cache_read_tokens: None,
        duration_ms: None,
        duration_api_ms: None,
        num_turns: None,
        stop_reason: None,
        context_usage: None,
        context_tokens: None,
        max_context_tokens: None,
        total_cost_usd: None,
    };
    assert!(meta.cache_creation_tokens.is_none());
    assert!(meta.total_cost_usd.is_none());
}

#[test]
fn test_metadata_serialization_with_new_fields() {
    let mut meta = AgentEvent::base_metadata("claude");
    meta.cache_creation_tokens = Some(500);
    meta.cache_read_tokens = Some(1000);
    meta.duration_ms = Some(4105);
    meta.total_cost_usd = Some(0.055);
    meta.num_turns = Some(3);
    meta.stop_reason = Some("end_turn".to_string());

    let json = serde_json::to_string(&meta).unwrap();
    assert!(json.contains("cache_creation_tokens"));
    assert!(json.contains("500"));
    assert!(json.contains("total_cost_usd"));

    let deserialized: AgentEventMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.cache_creation_tokens, Some(500));
    assert_eq!(deserialized.total_cost_usd, Some(0.055));
    assert_eq!(deserialized.num_turns, Some(3));
}

#[test]
fn test_metadata_deserialization_without_new_fields() {
    // Old JSON without new fields should deserialize with None defaults
    let json = r#"{"provider":"claude"}"#;
    let meta: AgentEventMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(meta.provider, "claude");
    assert!(meta.cache_creation_tokens.is_none());
    assert!(meta.duration_ms.is_none());
    assert!(meta.total_cost_usd.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test agent_event::tests -- --nocapture 2>&1 | tail -20`
Expected: FAIL — fields don't exist yet.

- [ ] **Step 3: Add 10 new fields to AgentEventMetadata**

In `agent_event.rs`, add these fields to the `AgentEventMetadata` struct after the `incomplete` field (line ~73):

```rust
    #[serde(default)]
    pub cache_creation_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_tokens: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub duration_api_ms: Option<u64>,
    #[serde(default)]
    pub num_turns: Option<u32>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub context_usage: Option<f64>,
    #[serde(default)]
    pub context_tokens: Option<u64>,
    #[serde(default)]
    pub max_context_tokens: Option<u64>,
    #[serde(default)]
    pub total_cost_usd: Option<f64>,
```

- [ ] **Step 4: Update `base_metadata()` to initialize new fields**

In `agent_event.rs`, update `base_metadata()` (around line 94-106) to include all new fields set to `None`:

```rust
    fn base_metadata(provider: &str) -> AgentEventMetadata {
        AgentEventMetadata {
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            tool_name: None,
            model: None,
            provider: provider.to_string(),
            error_severity: None,
            error_code: None,
            stream_metrics: None,
            incomplete: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            duration_ms: None,
            duration_api_ms: None,
            num_turns: None,
            stop_reason: None,
            context_usage: None,
            context_tokens: None,
            max_context_tokens: None,
            total_cost_usd: None,
        }
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test agent_event::tests -- --nocapture 2>&1 | tail -20`
Expected: ALL PASS (including new tests)

- [ ] **Step 6: Run full test suite to check nothing broke**

Run: `cd src-tauri && cargo test 2>&1 | tail -20`
Expected: ALL PASS — the new `Option` fields with `#[serde(default)]` are backward-compatible.

- [ ] **Step 7: Commit**

```bash
cd src-tauri && git add src/agent_event.rs && git commit -m "feat(analytics): extend AgentEventMetadata with cache, duration, cost fields

Add 10 optional fields: cache_creation_tokens, cache_read_tokens,
duration_ms, duration_api_ms, num_turns, stop_reason, context_usage,
context_tokens, max_context_tokens, total_cost_usd.

All fields are Option<T> with #[serde(default)] for backward compat."
```

---

### Task 2: Extract new metadata fields in NormalizerPipeline

**Files:**
- Modify: `src-tauri/src/normalizer/pipeline.rs:74-105`

- [ ] **Step 1: Write a test for new field extraction**

Add to the `tests` module at the bottom of `pipeline.rs`:

```rust
#[test]
fn test_pipeline_extracts_cache_and_duration() {
    let rules = vec![
        Rule {
            name: "result".into(),
            when: r#"type == "result""#.into(),
            emit: "usage".into(),
            mappings: [
                ("cache_creation_tokens".to_string(), "usage.cache_creation_input_tokens".to_string()),
                ("cache_read_tokens".to_string(), "usage.cache_read_input_tokens".to_string()),
                ("duration_ms".to_string(), "duration_ms".to_string()),
                ("duration_api_ms".to_string(), "duration_api_ms".to_string()),
                ("num_turns".to_string(), "num_turns".to_string()),
                ("stop_reason".to_string(), "stop_reason".to_string()),
                ("total_cost_usd".to_string(), "total_cost_usd".to_string()),
            ].into(),
        },
    ];
    let mut pipeline = NormalizerPipeline::new(
        rules,
        Box::new(GenericStateMachine::new()),
        "claude".to_string(),
    );
    let input = r#"{"type":"result","duration_ms":4105,"duration_api_ms":4089,"num_turns":1,"stop_reason":"end_turn","total_cost_usd":0.055,"usage":{"cache_creation_input_tokens":7727,"cache_read_input_tokens":15092}}"#;
    let events = pipeline.process(input);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].metadata.cache_creation_tokens, Some(7727));
    assert_eq!(events[0].metadata.cache_read_tokens, Some(15092));
    assert_eq!(events[0].metadata.duration_ms, Some(4105));
    assert_eq!(events[0].metadata.duration_api_ms, Some(4089));
    assert_eq!(events[0].metadata.num_turns, Some(1));
    assert_eq!(events[0].metadata.stop_reason, Some("end_turn".to_string()));
    assert_eq!(events[0].metadata.total_cost_usd, Some(0.055));
}

#[test]
fn test_pipeline_extracts_context_usage() {
    let rules = vec![
        Rule {
            name: "context".into(),
            when: r#"type == "metrics""#.into(),
            emit: "metrics".into(),
            mappings: [
                ("context_usage".to_string(), "context_usage".to_string()),
                ("context_tokens".to_string(), "context_tokens".to_string()),
                ("max_context_tokens".to_string(), "max_context_tokens".to_string()),
            ].into(),
        },
    ];
    let mut pipeline = NormalizerPipeline::new(
        rules,
        Box::new(GenericStateMachine::new()),
        "kimi".to_string(),
    );
    let input = r#"{"type":"metrics","context_usage":0.75,"context_tokens":96000,"max_context_tokens":128000}"#;
    let events = pipeline.process(input);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].metadata.context_usage, Some(0.75));
    assert_eq!(events[0].metadata.context_tokens, Some(96000));
    assert_eq!(events[0].metadata.max_context_tokens, Some(128000));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test normalizer::pipeline::tests -- --nocapture 2>&1 | tail -20`
Expected: FAIL — fields not extracted yet.

- [ ] **Step 3: Add extraction logic to `build_event()`**

In `pipeline.rs`, inside the `build_event()` method, add the new field extractions to the `AgentEventMetadata` construction (around line 74-105). Add after the existing `incomplete: None,` line:

```rust
            cache_creation_tokens: rule.mappings.get("cache_creation_tokens")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            cache_read_tokens: rule.mappings.get("cache_read_tokens")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            duration_ms: rule.mappings.get("duration_ms")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            duration_api_ms: rule.mappings.get("duration_api_ms")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            num_turns: rule.mappings.get("num_turns")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64().map(|n| n as u32)),
            stop_reason: rule.mappings.get("stop_reason")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            context_usage: rule.mappings.get("context_usage")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_f64()),
            context_tokens: rule.mappings.get("context_tokens")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            max_context_tokens: rule.mappings.get("max_context_tokens")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_u64()),
            total_cost_usd: rule.mappings.get("total_cost_usd")
                .and_then(|path| resolve_path(value, path))
                .and_then(|v| v.as_f64()),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test normalizer::pipeline::tests -- --nocapture 2>&1 | tail -20`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
cd src-tauri && git add src/normalizer/pipeline.rs && git commit -m "feat(analytics): extract new metadata fields in normalizer pipeline

Pipeline now extracts cache_creation_tokens, cache_read_tokens,
duration_ms, duration_api_ms, num_turns, stop_reason, context_usage,
context_tokens, max_context_tokens, total_cost_usd from TOML mappings."
```

---

### Task 3: Update TOML normalizer configs with new mappings

**Files:**
- Modify: `src-tauri/normalizers/claude.toml:96-108`
- Modify: `src-tauri/normalizers/gemini.toml:90-95`
- Modify: `src-tauri/normalizers/kimi.toml:83-89`
- Modify: `src-tauri/normalizers/qwen.toml:83-89`
- Modify: `src-tauri/normalizers/codex.toml:88-94`

- [ ] **Step 1: Update claude.toml — add `result_metrics` rule**

Append after the existing `done` rule (line ~108):

```toml
[[rules]]
name = "result_metrics"
when = 'type == "result"'
emit = "usage"
# NOTE: Do NOT map input_tokens/output_tokens here — they are already
# counted from the message_delta usage rule. Mapping them again would double-count.

[rules.mappings]
cache_creation_tokens = "usage.cache_creation_input_tokens"
cache_read_tokens = "usage.cache_read_input_tokens"
duration_ms = "duration_ms"
duration_api_ms = "duration_api_ms"
num_turns = "num_turns"
stop_reason = "stop_reason"
total_cost_usd = "total_cost_usd"
```

- [ ] **Step 2: Update gemini.toml — add cache/duration to usage rule**

Replace the existing `usage` rule mappings (lines ~90-95) with:

```toml
[[rules]]
name = "usage"
when = 'type == "RESULT" && exists(usage)'
emit = "usage"
[rules.mappings]
input_tokens = "usage.input_tokens"
output_tokens = "usage.output_tokens"
cache_read_tokens = "usage.cached"
duration_ms = "usage.duration_ms"
```

- [ ] **Step 3: Update kimi.toml — add context_metrics rule**

Append after the existing `done` rule (line ~93):

```toml
[[rules]]
name = "context_metrics"
when = 'type == "message_delta" && exists(context_usage)'
emit = "metrics"

[rules.mappings]
context_usage = "context_usage"
context_tokens = "context_tokens"
max_context_tokens = "max_context_tokens"
```

- [ ] **Step 4: Update qwen.toml — add duration/turns to usage rule**

Replace the existing `usage` rule mappings (lines ~83-89) with:

```toml
[[rules]]
name = "usage"
when = 'type == "result" && exists(usage)'
emit = "usage"
[rules.mappings]
input_tokens = "usage.input_tokens"
output_tokens = "usage.output_tokens"
duration_ms = "duration_ms"
duration_api_ms = "duration_api_ms"
num_turns = "num_turns"
```

- [ ] **Step 5: Update codex.toml — add cache_read_tokens to usage rule**

Replace the existing `usage` rule mappings (lines ~88-94) with:

```toml
[[rules]]
name = "usage"
when = 'method == "ThreadTokenUsageUpdatedNotification"'
emit = "usage"
[rules.mappings]
input_tokens = "params.usage.input_tokens"
output_tokens = "params.usage.output_tokens"
cache_read_tokens = "params.usage.cachedInputTokens"
```

- [ ] **Step 6: Run existing tests to verify TOML changes don't break anything**

Run: `cd src-tauri && cargo test 2>&1 | tail -20`
Expected: ALL PASS — new rules just add mappings, existing rules unchanged.

- [ ] **Step 7: Commit**

```bash
cd src-tauri && git add normalizers/ && git commit -m "feat(analytics): add cache, duration, cost mappings to normalizer TOMLs

Claude: result_metrics rule for cache/duration/cost from result event
Gemini: cache_read_tokens, duration_ms in usage rule
Kimi: context_metrics rule (speculative, needs runtime validation)
Qwen: duration_ms, duration_api_ms, num_turns in usage rule
Codex: cache_read_tokens in usage rule"
```

---

### Task 4: Implement AnalyticsStore (JSONL persistence)

**Files:**
- Create: `src-tauri/src/analytics/mod.rs`
- Create: `src-tauri/src/analytics/store.rs`

- [ ] **Step 1: Create `analytics/mod.rs` with data types**

Create `src-tauri/src/analytics/mod.rs`:

```rust
pub mod store;
pub mod collector;

use crate::agent_event::ErrorSeverity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub session_id: String,
    pub provider: String,
    pub model: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub duration_ms: Option<u64>,
    pub duration_api_ms: Option<u64>,
    pub num_turns: u32,
    pub tools_used: HashMap<String, u32>,
    pub stop_reason: Option<String>,
    pub peak_context_usage: Option<f64>,
    pub max_context_tokens: Option<u64>,
    pub total_cost_usd: Option<f64>,
    pub errors: Vec<ErrorRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub timestamp: u64,
    pub code: String,
    pub severity: ErrorSeverity,
    pub recovered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAnalytics {
    pub provider: String,
    pub total_sessions: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub cache_hit_rate: f32,
    pub total_errors: u64,
    pub recovered_errors: u64,
    pub error_rate: f32,
    pub avg_duration_ms: f64,
    pub avg_tokens_per_second: f32,
    pub most_used_model: String,
    pub total_tool_invocations: u64,
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAnalytics {
    pub model: String,
    pub provider: String,
    pub session_count: u64,
    pub avg_input_tokens: f64,
    pub avg_output_tokens: f64,
    pub avg_duration_ms: f64,
    pub avg_tokens_per_second: f32,
    pub error_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub provider: Option<String>,
    pub sessions: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub errors: u64,
    pub avg_duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub from: Option<u64>,
    pub to: Option<u64>,
}

impl SessionMetrics {
    pub fn new(session_id: &str, provider: &str, model: &str, started_at: u64) -> Self {
        Self {
            session_id: session_id.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            started_at,
            ended_at: None,
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            duration_ms: None,
            duration_api_ms: None,
            num_turns: 0,
            tools_used: HashMap::new(),
            stop_reason: None,
            peak_context_usage: None,
            max_context_tokens: None,
            total_cost_usd: None,
            errors: Vec::new(),
        }
    }
}
```

- [ ] **Step 2: Write tests for AnalyticsStore**

Create `src-tauri/src/analytics/store.rs` with the test module first:

```rust
use super::SessionMetrics;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct AnalyticsStore {
    path: PathBuf,
    completed: Mutex<Vec<SessionMetrics>>,
}

impl AnalyticsStore {
    pub fn new(dir: &Path) -> Result<Self, String> {
        todo!()
    }

    pub fn append(&self, metrics: &SessionMetrics) -> Result<(), String> {
        todo!()
    }

    pub fn all_completed(&self) -> Vec<SessionMetrics> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metrics(id: &str, provider: &str) -> SessionMetrics {
        let mut m = SessionMetrics::new(id, provider, "test-model", 1000);
        m.input_tokens = 100;
        m.output_tokens = 50;
        m.ended_at = Some(2000);
        m
    }

    #[test]
    fn test_store_new_creates_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let store_dir = dir.path().join("analytics");
        let store = AnalyticsStore::new(&store_dir).unwrap();
        assert!(store_dir.exists());
        assert!(store.all_completed().is_empty());
    }

    #[test]
    fn test_store_append_and_read() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = AnalyticsStore::new(dir.path()).unwrap();

        let m1 = sample_metrics("s1", "claude");
        let m2 = sample_metrics("s2", "gemini");
        store.append(&m1).unwrap();
        store.append(&m2).unwrap();

        let all = store.all_completed();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].session_id, "s1");
        assert_eq!(all[1].session_id, "s2");
    }

    #[test]
    fn test_store_persists_across_instances() {
        let dir = tempfile::TempDir::new().unwrap();

        {
            let store = AnalyticsStore::new(dir.path()).unwrap();
            store.append(&sample_metrics("s1", "claude")).unwrap();
            store.append(&sample_metrics("s2", "gemini")).unwrap();
        }

        let store2 = AnalyticsStore::new(dir.path()).unwrap();
        assert_eq!(store2.all_completed().len(), 2);
    }

    #[test]
    fn test_store_handles_empty_file() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("metrics.jsonl"), "").unwrap();
        let store = AnalyticsStore::new(dir.path()).unwrap();
        assert!(store.all_completed().is_empty());
    }

    #[test]
    fn test_store_skips_corrupted_lines() {
        let dir = tempfile::TempDir::new().unwrap();
        let m = sample_metrics("s1", "claude");
        let good_line = serde_json::to_string(&m).unwrap();
        let content = format!("{}\nnot valid json\n{}\n", good_line, good_line);
        fs::write(dir.path().join("metrics.jsonl"), content).unwrap();

        let store = AnalyticsStore::new(dir.path()).unwrap();
        assert_eq!(store.all_completed().len(), 2);
    }
}
```

- [ ] **Step 3: Add `mod analytics;` to lib.rs**

Add after `mod self_heal;` (line 17) in `src-tauri/src/lib.rs`:

```rust
mod analytics;
```

This is required for the module to compile before running tests.

- [ ] **Step 4: Run tests to verify they fail**

Run: `cd src-tauri && cargo test analytics::store::tests -- --nocapture 2>&1 | tail -20`
Expected: FAIL — `todo!()` panics.

- [ ] **Step 5: Implement AnalyticsStore**

Replace the `todo!()` implementations in `store.rs`:

```rust
impl AnalyticsStore {
    pub fn new(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| format!("Failed to create analytics dir: {}", e))?;

        let path = dir.to_path_buf();
        let mut store = Self {
            path,
            completed: Mutex::new(Vec::new()),
        };
        store.load()?;
        Ok(store)
    }

    pub fn append(&self, metrics: &SessionMetrics) -> Result<(), String> {
        let json = serde_json::to_string(metrics)
            .map_err(|e| format!("Failed to serialize metrics: {}", e))?;

        let file_path = self.path.join("metrics.jsonl");
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|e| format!("Failed to open metrics file: {}", e))?;

        writeln!(file, "{}", json)
            .map_err(|e| format!("Failed to write metrics: {}", e))?;

        self.completed.lock().unwrap().push(metrics.clone());
        Ok(())
    }

    pub fn all_completed(&self) -> Vec<SessionMetrics> {
        self.completed.lock().unwrap().clone()
    }

    fn load(&mut self) -> Result<(), String> {
        let file_path = self.path.join("metrics.jsonl");
        if !file_path.exists() {
            return Ok(());
        }

        let file = fs::File::open(&file_path)
            .map_err(|e| format!("Failed to open metrics file: {}", e))?;
        let reader = BufReader::new(file);
        let mut completed = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<SessionMetrics>(&line) {
                Ok(metrics) => completed.push(metrics),
                Err(_) => continue, // skip corrupted lines
            }
        }

        *self.completed.lock().unwrap() = completed;
        Ok(())
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd src-tauri && cargo test analytics::store::tests -- --nocapture 2>&1 | tail -20`
Expected: ALL PASS

- [ ] **Step 7: Commit**

```bash
cd src-tauri && git add src/analytics/mod.rs src/analytics/store.rs src/lib.rs && git commit -m "feat(analytics): add SessionMetrics types and AnalyticsStore persistence

SessionMetrics captures per-session token usage, cache, timing, tools,
errors, context pressure, and cost. AnalyticsStore persists completed
sessions as JSONL and loads them at startup."
```

---

### Task 5: Implement AnalyticsCollector (EventBus subscriber + accumulation)

**Files:**
- Create: `src-tauri/src/analytics/collector.rs`

- [ ] **Step 1: Write tests for accumulation logic**

Create `src-tauri/src/analytics/collector.rs`:

```rust
use super::{SessionMetrics, ErrorRecord, ProviderAnalytics, ModelAnalytics, DailyStats, TimeRange};
use super::store::AnalyticsStore;
use crate::agent_event::{AgentEvent, AgentEventType, ErrorSeverity};
use crate::transport::event_bus::{AgentEventSubscriber, EventFilter};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct AnalyticsCollector {
    active: Mutex<HashMap<String, SessionMetrics>>,
    store: Arc<AnalyticsStore>,
}

impl AnalyticsCollector {
    pub fn new(store: Arc<AnalyticsStore>) -> Self {
        Self {
            active: Mutex::new(HashMap::new()),
            store,
        }
    }
}

impl AgentEventSubscriber for AnalyticsCollector {
    fn on_event(&self, session_id: &str, event: &AgentEvent) {
        todo!()
    }

    fn filter(&self) -> Option<EventFilter> {
        Some(EventFilter {
            event_types: Some(vec![
                AgentEventType::Usage,
                AgentEventType::Metrics,
                AgentEventType::ToolUse,
                AgentEventType::Error,
                AgentEventType::Done,
            ]),
            providers: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> (Arc<AnalyticsStore>, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Arc::new(AnalyticsStore::new(dir.path()).unwrap());
        (store, dir) // TempDir kept alive by caller, cleaned up when test ends
    }

    #[test]
    fn test_accumulate_usage_tokens() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        let event = AgentEvent::usage(100, 50, "claude");
        collector.on_event("s1", &event);

        let active = collector.active.lock().unwrap();
        let m = active.get("s1").unwrap();
        assert_eq!(m.input_tokens, 100);
        assert_eq!(m.output_tokens, 50);
    }

    #[test]
    fn test_accumulate_multiple_usage() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::usage(200, 100, "claude"));

        let active = collector.active.lock().unwrap();
        let m = active.get("s1").unwrap();
        assert_eq!(m.input_tokens, 300);
        assert_eq!(m.output_tokens, 150);
    }

    #[test]
    fn test_accumulate_tool_use() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        let event = AgentEvent::tool_use("read_file", r#"{"path":"test"}"#, "claude");
        collector.on_event("s1", &event);
        collector.on_event("s1", &event);

        let active = collector.active.lock().unwrap();
        let m = active.get("s1").unwrap();
        assert_eq!(m.tools_used.get("read_file"), Some(&2));
    }

    #[test]
    fn test_accumulate_error_with_recovery() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        let err = AgentEvent::error("rate limit", "overloaded", ErrorSeverity::Recoverable, "claude");
        collector.on_event("s1", &err);

        // Another event arrives — previous error is recovered
        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));

        let active = collector.active.lock().unwrap();
        let m = active.get("s1").unwrap();
        assert_eq!(m.errors.len(), 1);
        assert!(m.errors[0].recovered);
    }

    #[test]
    fn test_done_flushes_to_store() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store.clone());

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));

        // Active should be empty now
        assert!(collector.active.lock().unwrap().is_empty());

        // Store should have 1 completed session
        assert_eq!(store.all_completed().len(), 1);
        let m = &store.all_completed()[0];
        assert_eq!(m.session_id, "s1");
        assert_eq!(m.input_tokens, 100);
        assert!(m.ended_at.is_some());
    }

    #[test]
    fn test_active_sessions() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s2", &AgentEvent::usage(200, 100, "gemini"));

        let active = collector.get_active_sessions();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_session_metrics_query() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));

        let m = collector.get_session_metrics("s1");
        assert!(m.is_some());
        assert_eq!(m.unwrap().input_tokens, 100);

        assert!(collector.get_session_metrics("nonexistent").is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test analytics::collector::tests -- --nocapture 2>&1 | tail -20`
Expected: FAIL — `todo!()` panic.

- [ ] **Step 3: Implement `on_event` accumulation logic**

Replace the `todo!()` in `on_event`:

```rust
    fn on_event(&self, session_id: &str, event: &AgentEvent) {
        let mut active = self.active.lock().unwrap();

        // Handle Done separately to avoid borrow conflict:
        // entry() borrows `active` mutably, and remove() would need another mutable borrow.
        if event.event_type == AgentEventType::Done {
            if let Some(mut m) = active.remove(session_id) {
                m.ended_at = Some(event.timestamp);
                drop(active); // release lock before I/O
                if let Err(e) = self.store.append(&m) {
                    eprintln!("AnalyticsCollector: failed to flush session {}: {}", session_id, e);
                }
            }
            return;
        }

        // Bootstrap new session on first event
        let metrics = active.entry(session_id.to_string()).or_insert_with(|| {
            SessionMetrics::new(
                session_id,
                &event.metadata.provider,
                event.metadata.model.as_deref().unwrap_or(""),
                event.timestamp,
            )
        });

        match event.event_type {
            AgentEventType::Usage => {
                if let Some(t) = event.metadata.input_tokens {
                    metrics.input_tokens += t;
                }
                if let Some(t) = event.metadata.output_tokens {
                    metrics.output_tokens += t;
                }
                if let Some(t) = event.metadata.cache_creation_tokens {
                    metrics.cache_creation_tokens += t;
                }
                if let Some(t) = event.metadata.cache_read_tokens {
                    metrics.cache_read_tokens += t;
                }
                // Last-write-wins fields
                if let Some(v) = event.metadata.duration_ms {
                    metrics.duration_ms = Some(v);
                }
                if let Some(v) = event.metadata.duration_api_ms {
                    metrics.duration_api_ms = Some(v);
                }
                if let Some(v) = event.metadata.num_turns {
                    metrics.num_turns = v;
                }
                if event.metadata.stop_reason.is_some() {
                    metrics.stop_reason = event.metadata.stop_reason.clone();
                }
                if event.metadata.total_cost_usd.is_some() {
                    metrics.total_cost_usd = event.metadata.total_cost_usd;
                }
            }
            AgentEventType::Metrics => {
                // context_tokens is not stored directly; peak_context_usage captures the derived metric
                if let Some(cu) = event.metadata.context_usage {
                    metrics.peak_context_usage = Some(
                        metrics.peak_context_usage.map_or(cu, |current| current.max(cu))
                    );
                }
                if let Some(v) = event.metadata.max_context_tokens {
                    metrics.max_context_tokens = Some(v);
                }
            }
            AgentEventType::ToolUse => {
                if let Some(ref name) = event.metadata.tool_name {
                    *metrics.tools_used.entry(name.clone()).or_insert(0) += 1;
                }
            }
            AgentEventType::Error => {
                // Mark previous error as recovered (session continued)
                if let Some(last) = metrics.errors.last_mut() {
                    last.recovered = true;
                }
                metrics.errors.push(ErrorRecord {
                    timestamp: event.timestamp,
                    code: event.metadata.error_code.clone().unwrap_or_else(|| "unknown".to_string()),
                    severity: event.metadata.error_severity.clone().unwrap_or(ErrorSeverity::Fatal),
                    recovered: false,
                });
            }
            _ => {}
        }
    }
```

- [ ] **Step 4: Add query methods**

Add to the `impl AnalyticsCollector` block:

```rust
    pub fn get_active_sessions(&self) -> Vec<SessionMetrics> {
        self.active.lock().unwrap().values().cloned().collect()
    }

    pub fn get_session_metrics(&self, session_id: &str) -> Option<SessionMetrics> {
        // Check active first
        if let Some(m) = self.active.lock().unwrap().get(session_id) {
            return Some(m.clone());
        }
        // Then check completed
        self.store.all_completed().into_iter()
            .find(|m| m.session_id == session_id)
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test analytics::collector::tests -- --nocapture 2>&1 | tail -20`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
cd src-tauri && git add src/analytics/collector.rs && git commit -m "feat(analytics): implement AnalyticsCollector with event accumulation

EventBus subscriber that accumulates Usage, ToolUse, Error, Metrics,
Done events into per-session SessionMetrics. Flushes completed sessions
to AnalyticsStore on Done. Supports active session and completed
session queries."
```

---

### Task 6: Implement aggregation query methods

**Files:**
- Modify: `src-tauri/src/analytics/collector.rs`

- [ ] **Step 1: Write tests for aggregation**

Add to the `tests` module in `collector.rs`:

```rust
    #[test]
    fn test_provider_analytics() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        // Two claude sessions
        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));
        collector.on_event("s2", &AgentEvent::usage(200, 100, "claude"));
        collector.on_event("s2", &AgentEvent::done("s2", "claude"));

        let analytics = collector.get_provider_analytics("claude", None);
        assert_eq!(analytics.total_sessions, 2);
        assert_eq!(analytics.total_input_tokens, 300);
        assert_eq!(analytics.total_output_tokens, 150);
        assert_eq!(analytics.error_rate, 0.0);
    }

    #[test]
    fn test_compare_providers() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));
        collector.on_event("s2", &AgentEvent::usage(200, 100, "gemini"));
        collector.on_event("s2", &AgentEvent::done("s2", "gemini"));

        let comparison = collector.compare_providers(None);
        assert_eq!(comparison.len(), 2);
    }

    #[test]
    fn test_model_breakdown() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        // We need events with model set
        let mut event1 = AgentEvent::usage(100, 50, "claude");
        event1.metadata.model = Some("claude-sonnet-4-6".to_string());
        collector.on_event("s1", &event1);
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));

        let mut event2 = AgentEvent::usage(200, 100, "claude");
        event2.metadata.model = Some("claude-opus-4-6".to_string());
        collector.on_event("s2", &event2);
        collector.on_event("s2", &AgentEvent::done("s2", "claude"));

        let breakdown = collector.get_model_breakdown("claude", None);
        assert_eq!(breakdown.len(), 2);
    }

    #[test]
    fn test_daily_stats() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        collector.on_event("s1", &AgentEvent::usage(100, 50, "claude"));
        collector.on_event("s1", &AgentEvent::done("s1", "claude"));

        let stats = collector.get_daily_stats(None, 7);
        assert!(!stats.is_empty());
        assert_eq!(stats[0].sessions, 1);
    }

    #[test]
    fn test_provider_analytics_empty() {
        let (store, _dir) = make_store();
        let collector = AnalyticsCollector::new(store);

        let analytics = collector.get_provider_analytics("nonexistent", None);
        assert_eq!(analytics.total_sessions, 0);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test analytics::collector::tests -- --nocapture 2>&1 | tail -20`
Expected: FAIL — methods don't exist yet.

- [ ] **Step 3: Implement aggregation methods**

Add these methods to `impl AnalyticsCollector`:

```rust
    pub fn get_provider_analytics(&self, provider: &str, range: Option<TimeRange>) -> ProviderAnalytics {
        let sessions: Vec<SessionMetrics> = self.store.all_completed()
            .into_iter()
            .filter(|m| m.provider == provider)
            .filter(|m| Self::in_range(m, &range))
            .collect();

        Self::aggregate_provider(provider, &sessions)
    }

    pub fn compare_providers(&self, range: Option<TimeRange>) -> Vec<ProviderAnalytics> {
        let all = self.store.all_completed();
        let mut by_provider: HashMap<String, Vec<SessionMetrics>> = HashMap::new();
        for m in all {
            if Self::in_range(&m, &range) {
                by_provider.entry(m.provider.clone()).or_default().push(m);
            }
        }
        by_provider.into_iter()
            .map(|(provider, sessions)| Self::aggregate_provider(&provider, &sessions))
            .collect()
    }

    pub fn get_model_breakdown(&self, provider: &str, range: Option<TimeRange>) -> Vec<ModelAnalytics> {
        let sessions: Vec<SessionMetrics> = self.store.all_completed()
            .into_iter()
            .filter(|m| m.provider == provider)
            .filter(|m| Self::in_range(m, &range))
            .collect();

        let mut by_model: HashMap<String, Vec<&SessionMetrics>> = HashMap::new();
        for m in &sessions {
            by_model.entry(m.model.clone()).or_default().push(m);
        }

        by_model.into_iter().map(|(model, sessions)| {
            let count = sessions.len() as u64;
            let total_input: u64 = sessions.iter().map(|s| s.input_tokens).sum();
            let total_output: u64 = sessions.iter().map(|s| s.output_tokens).sum();
            let durations: Vec<u64> = sessions.iter().filter_map(|s| s.duration_ms).collect();
            let avg_dur = if durations.is_empty() { 0.0 } else { durations.iter().sum::<u64>() as f64 / durations.len() as f64 };
            let total_errors: u64 = sessions.iter().map(|s| s.errors.len() as u64).sum();

            let tps = if avg_dur > 0.0 {
                (total_output as f64 / count as f64) / (avg_dur / 1000.0)
            } else { 0.0 };

            ModelAnalytics {
                model,
                provider: provider.to_string(),
                session_count: count,
                avg_input_tokens: total_input as f64 / count as f64,
                avg_output_tokens: total_output as f64 / count as f64,
                avg_duration_ms: avg_dur,
                avg_tokens_per_second: tps as f32,
                error_rate: if count > 0 { total_errors as f32 / count as f32 } else { 0.0 },
            }
        }).collect()
    }

    pub fn get_daily_stats(&self, provider: Option<&str>, days: u32) -> Vec<DailyStats> {
        let all = self.store.all_completed();
        let mut by_date: HashMap<String, Vec<&SessionMetrics>> = HashMap::new();

        for m in &all {
            if let Some(p) = provider {
                if m.provider != p { continue; }
            }
            // Convert ms timestamp to date string
            let secs = m.started_at / 1000;
            let date = unix_secs_to_date_string(secs);
            by_date.entry(date).or_default().push(m);
        }

        let mut stats: Vec<DailyStats> = by_date.into_iter().map(|(date, sessions)| {
            let count = sessions.len() as u64;
            let total_in: u64 = sessions.iter().map(|s| s.input_tokens).sum();
            let total_out: u64 = sessions.iter().map(|s| s.output_tokens).sum();
            let errors: u64 = sessions.iter().map(|s| s.errors.len() as u64).sum();
            let durs: Vec<u64> = sessions.iter().filter_map(|s| s.duration_ms).collect();
            let avg_dur = if durs.is_empty() { 0.0 } else { durs.iter().sum::<u64>() as f64 / durs.len() as f64 };

            DailyStats {
                date,
                provider: provider.map(|s| s.to_string()),
                sessions: count,
                input_tokens: total_in,
                output_tokens: total_out,
                errors,
                avg_duration_ms: avg_dur,
            }
        }).collect();

        stats.sort_by(|a, b| a.date.cmp(&b.date));
        // Keep only last N days
        if stats.len() > days as usize {
            stats = stats.split_off(stats.len() - days as usize);
        }
        stats
    }

    fn in_range(m: &SessionMetrics, range: &Option<TimeRange>) -> bool {
        match range {
            None => true,
            Some(r) => {
                if let Some(from) = r.from {
                    if m.started_at < from { return false; }
                }
                if let Some(to) = r.to {
                    if m.started_at >= to { return false; }
                }
                true
            }
        }
    }

    fn aggregate_provider(provider: &str, sessions: &[SessionMetrics]) -> ProviderAnalytics {
        let count = sessions.len() as u64;
        if count == 0 {
            return ProviderAnalytics {
                provider: provider.to_string(),
                total_sessions: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cache_creation_tokens: 0,
                total_cache_read_tokens: 0,
                cache_hit_rate: 0.0,
                total_errors: 0,
                recovered_errors: 0,
                error_rate: 0.0,
                avg_duration_ms: 0.0,
                avg_tokens_per_second: 0.0,
                most_used_model: String::new(),
                total_tool_invocations: 0,
                total_cost_usd: None,
            };
        }

        let total_in: u64 = sessions.iter().map(|s| s.input_tokens).sum();
        let total_out: u64 = sessions.iter().map(|s| s.output_tokens).sum();
        let total_cache_create: u64 = sessions.iter().map(|s| s.cache_creation_tokens).sum();
        let total_cache_read: u64 = sessions.iter().map(|s| s.cache_read_tokens).sum();
        let total_errors: u64 = sessions.iter().map(|s| s.errors.len() as u64).sum();
        let recovered: u64 = sessions.iter()
            .flat_map(|s| s.errors.iter())
            .filter(|e| e.recovered)
            .count() as u64;
        let total_tools: u64 = sessions.iter()
            .map(|s| s.tools_used.values().sum::<u32>() as u64)
            .sum();

        let durs: Vec<u64> = sessions.iter().filter_map(|s| s.duration_ms).collect();
        let avg_dur = if durs.is_empty() { 0.0 } else { durs.iter().sum::<u64>() as f64 / durs.len() as f64 };

        let cache_denom = total_in + total_cache_read;
        let cache_hit = if cache_denom > 0 { total_cache_read as f32 / cache_denom as f32 } else { 0.0 };

        let tps = if avg_dur > 0.0 {
            ((total_out as f64 / count as f64) / (avg_dur / 1000.0)) as f32
        } else { 0.0 };

        // Most used model
        let mut model_counts: HashMap<&str, u64> = HashMap::new();
        for s in sessions { *model_counts.entry(&s.model).or_insert(0) += 1; }
        let most_used = model_counts.into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(m, _)| m.to_string())
            .unwrap_or_default();

        let cost: Option<f64> = {
            let costs: Vec<f64> = sessions.iter().filter_map(|s| s.total_cost_usd).collect();
            if costs.is_empty() { None } else { Some(costs.iter().sum()) }
        };

        ProviderAnalytics {
            provider: provider.to_string(),
            total_sessions: count,
            total_input_tokens: total_in,
            total_output_tokens: total_out,
            total_cache_creation_tokens: total_cache_create,
            total_cache_read_tokens: total_cache_read,
            cache_hit_rate: cache_hit,
            total_errors,
            recovered_errors: recovered,
            error_rate: total_errors as f32 / count as f32,
            avg_duration_ms: avg_dur,
            avg_tokens_per_second: tps,
            most_used_model: most_used,
            total_tool_invocations: total_tools,
            total_cost_usd: cost,
        }
    }
```

And add this helper function at the module level (outside `impl`):

```rust
/// Convert Unix seconds to "YYYY-MM-DD" date string (no chrono dependency).
fn unix_secs_to_date_string(secs: u64) -> String {
    // Simple date calculation without external dependencies
    // Days since Unix epoch
    let days = secs / 86400;
    let (year, month, day) = days_to_ymd(days as i64);
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test analytics::collector::tests -- --nocapture 2>&1 | tail -20`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
cd src-tauri && git add src/analytics/collector.rs && git commit -m "feat(analytics): add aggregation queries to AnalyticsCollector

Implements get_provider_analytics, compare_providers, get_model_breakdown,
get_daily_stats with TimeRange filtering. Pure functions that aggregate
from session records on-demand."
```

---

### Task 7: Add Tauri commands and wire everything in lib.rs

**Files:**
- Create: `src-tauri/src/commands/analytics.rs`
- Modify: `src-tauri/src/commands/mod.rs:13`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `commands/analytics.rs`**

```rust
use crate::analytics::{ProviderAnalytics, ModelAnalytics, DailyStats, SessionMetrics, TimeRange};
use crate::analytics::collector::AnalyticsCollector;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn analytics_provider(
    provider: String,
    from: Option<u64>,
    to: Option<u64>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<ProviderAnalytics, String> {
    let range = if from.is_some() || to.is_some() {
        Some(TimeRange { from, to })
    } else {
        None
    };
    Ok(collector.get_provider_analytics(&provider, range))
}

#[tauri::command]
pub fn analytics_compare(
    from: Option<u64>,
    to: Option<u64>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<ProviderAnalytics>, String> {
    let range = if from.is_some() || to.is_some() {
        Some(TimeRange { from, to })
    } else {
        None
    };
    Ok(collector.compare_providers(range))
}

#[tauri::command]
pub fn analytics_model_breakdown(
    provider: String,
    from: Option<u64>,
    to: Option<u64>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<ModelAnalytics>, String> {
    let range = if from.is_some() || to.is_some() {
        Some(TimeRange { from, to })
    } else {
        None
    };
    Ok(collector.get_model_breakdown(&provider, range))
}

#[tauri::command]
pub fn analytics_session(
    session_id: String,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Option<SessionMetrics>, String> {
    Ok(collector.get_session_metrics(&session_id))
}

#[tauri::command]
pub fn analytics_daily(
    provider: Option<String>,
    days: Option<u32>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<DailyStats>, String> {
    Ok(collector.get_daily_stats(provider.as_deref(), days.unwrap_or(30)))
}

#[tauri::command]
pub fn analytics_active(
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<SessionMetrics>, String> {
    Ok(collector.get_active_sessions())
}
```

- [ ] **Step 2: Add module declaration in `commands/mod.rs`**

Add after the last `pub mod` line (line 13):

```rust
pub mod analytics;
```

- [ ] **Step 3: Wire AnalyticsCollector in lib.rs**

In `src-tauri/src/lib.rs`:

1. **Skip** — `mod analytics;` already added in Task 4, Step 3

2. Add `.manage()` call after the `normalizer_health` manage block (around line 80). Add before `.setup(|app| {`:

```rust
        .manage({
            let analytics_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance")
                .join("analytics");
            let store = std::sync::Arc::new(
                analytics::store::AnalyticsStore::new(&analytics_dir)
                    .expect("Failed to init analytics store")
            );
            std::sync::Arc::new(analytics::collector::AnalyticsCollector::new(store))
        })
```

3. Inside `.setup(|app| {`, after the `SessionRecorderWrapper` subscription (around line 97), add:

```rust
            // Wire AnalyticsCollector into event bus
            let analytics: tauri::State<'_, std::sync::Arc<analytics::collector::AnalyticsCollector>> = app.state();
            struct AnalyticsWrapper(std::sync::Arc<analytics::collector::AnalyticsCollector>);
            impl transport::event_bus::AgentEventSubscriber for AnalyticsWrapper {
                fn on_event(&self, session_id: &str, event: &crate::agent_event::AgentEvent) {
                    self.0.on_event(session_id, event);
                }
                fn filter(&self) -> Option<transport::event_bus::EventFilter> {
                    self.0.filter()
                }
            }
            transport.event_bus().subscribe(Box::new(AnalyticsWrapper(analytics.inner().clone())));
```

4. Add commands to `.invoke_handler()`. Add after `commands::capability::get_all_health_reports,` (line 211):

```rust
            commands::analytics::analytics_provider,
            commands::analytics::analytics_compare,
            commands::analytics::analytics_model_breakdown,
            commands::analytics::analytics_session,
            commands::analytics::analytics_daily,
            commands::analytics::analytics_active,
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1 | tail -20`
Expected: no errors.

- [ ] **Step 5: Run full test suite**

Run: `cd src-tauri && cargo test 2>&1 | tail -20`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
cd src-tauri && git add src/commands/analytics.rs src/commands/mod.rs src/lib.rs && git commit -m "feat(analytics): add Tauri commands and wire AnalyticsCollector

6 new commands: analytics_provider, analytics_compare,
analytics_model_breakdown, analytics_session, analytics_daily,
analytics_active. Collector registered as Arc<> managed state
and subscribed to EventBus."
```

---

### Task 8: Extend fixture framework and add fixture tests for new metadata fields

**Files:**
- Modify: `src-tauri/src/normalizer/fixture_tests.rs`
- Create: `src-tauri/normalizers/fixtures/claude/result_metrics.jsonl`
- Create: `src-tauri/normalizers/fixtures/claude/result_metrics.expected.json`

- [ ] **Step 1: Create fixtures directory for Claude**

```bash
mkdir -p src-tauri/normalizers/fixtures/claude
```

- [ ] **Step 2: Extend `run_fixture_test()` to support new metadata assertions**

The existing fixture framework only checks `has_input_tokens`, `has_tool_name`, `severity`, `error_code`, `content_contains`. Add support for the new metadata fields. In `fixture_tests.rs`, add these assertion blocks after the existing `error_code` block (line ~111):

```rust
        if let Some(expected_val) = exp.get("has_cache_creation_tokens").and_then(|v| v.as_bool()) {
            if expected_val {
                assert!(
                    matching_event.metadata.cache_creation_tokens.is_some(),
                    "Event {} expected cache_creation_tokens", i
                );
            }
        }

        if let Some(expected_val) = exp.get("cache_creation_tokens").and_then(|v| v.as_u64()) {
            assert_eq!(
                matching_event.metadata.cache_creation_tokens, Some(expected_val),
                "Event {} expected cache_creation_tokens={}", i, expected_val
            );
        }

        if let Some(expected_val) = exp.get("cache_read_tokens").and_then(|v| v.as_u64()) {
            assert_eq!(
                matching_event.metadata.cache_read_tokens, Some(expected_val),
                "Event {} expected cache_read_tokens={}", i, expected_val
            );
        }

        if let Some(expected_val) = exp.get("duration_ms").and_then(|v| v.as_u64()) {
            assert_eq!(
                matching_event.metadata.duration_ms, Some(expected_val),
                "Event {} expected duration_ms={}", i, expected_val
            );
        }

        if let Some(expected_val) = exp.get("duration_api_ms").and_then(|v| v.as_u64()) {
            assert_eq!(
                matching_event.metadata.duration_api_ms, Some(expected_val),
                "Event {} expected duration_api_ms={}", i, expected_val
            );
        }

        if let Some(expected_val) = exp.get("num_turns").and_then(|v| v.as_u64()) {
            assert_eq!(
                matching_event.metadata.num_turns, Some(expected_val as u32),
                "Event {} expected num_turns={}", i, expected_val
            );
        }

        if let Some(expected_val) = exp.get("stop_reason").and_then(|v| v.as_str()) {
            assert_eq!(
                matching_event.metadata.stop_reason.as_deref(), Some(expected_val),
                "Event {} expected stop_reason={}", i, expected_val
            );
        }

        if let Some(expected_val) = exp.get("total_cost_usd").and_then(|v| v.as_f64()) {
            assert!(
                (matching_event.metadata.total_cost_usd.unwrap_or(0.0) - expected_val).abs() < 0.001,
                "Event {} expected total_cost_usd≈{}, got {:?}", i, expected_val, matching_event.metadata.total_cost_usd
            );
        }
```

- [ ] **Step 3: Create Claude result fixture input**

Create `src-tauri/normalizers/fixtures/claude/result_metrics.jsonl`:

```json
{"type":"result","subtype":"success","is_error":false,"duration_ms":4105,"duration_api_ms":4089,"num_turns":1,"stop_reason":"end_turn","total_cost_usd":0.055,"usage":{"cache_creation_input_tokens":7727,"cache_read_input_tokens":15092}}
```

- [ ] **Step 4: Create Claude result fixture expected output**

Create `src-tauri/normalizers/fixtures/claude/result_metrics.expected.json`:

```json
[
  {
    "event_type": "usage",
    "cache_creation_tokens": 7727,
    "cache_read_tokens": 15092,
    "duration_ms": 4105,
    "duration_api_ms": 4089,
    "num_turns": 1,
    "stop_reason": "end_turn",
    "total_cost_usd": 0.055
  }
]
```

- [ ] **Step 5: Add fixture test function**

Add to `fixture_tests.rs` after the Codex tests:

```rust
// --- Claude ---
#[test]
fn test_claude_result_metrics_fixture() { run_fixture_test("claude", "result_metrics"); }
```

- [ ] **Step 6: Run the fixture test**

Run: `cd src-tauri && cargo test fixture_tests -- --nocapture 2>&1 | tail -20`
Expected: PASS — the new `result_metrics` rule in claude.toml extracts the fields correctly, and the extended framework assertions verify the exact values.

- [ ] **Step 7: Commit**

```bash
cd src-tauri && git add normalizers/fixtures/ src/normalizer/fixture_tests.rs && git commit -m "test(analytics): extend fixture framework and add Claude result_metrics test

Extend run_fixture_test to support cache_creation_tokens, cache_read_tokens,
duration_ms, duration_api_ms, num_turns, stop_reason, total_cost_usd
assertions. Add Claude result_metrics fixture validating full field extraction."
```

---

### Task 9: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cd src-tauri && cargo test 2>&1 | tail -30`
Expected: ALL PASS, zero failures.

- [ ] **Step 2: Verify build completes**

Run: `cd src-tauri && cargo build 2>&1 | tail -10`
Expected: build succeeds.

- [ ] **Step 3: Count new tests**

Run: `cd src-tauri && cargo test 2>&1 | grep "test result"`
Expected: test count should be higher than the 263 from Phase 7A.

- [ ] **Step 4: Final commit if any uncommitted changes remain**

```bash
cd src-tauri && git status
```

If clean, proceed to finish.
