# Phase 7B: Analytics Collector — Design Spec

## Goal

Add session-level analytics collection to REASONANCE's transport layer, enabling per-provider/model usage tracking, trend analysis, and comparison — with zero data loss from premature aggregation.

## Architecture

The AnalyticsCollector is an EventBus subscriber that records per-session metrics (`SessionMetrics`) from normalized `AgentEvent` streams. Aggregations (`ProviderAnalytics`, `ModelAnalytics`, `DailyStats`) are computed on-demand when the frontend requests them. Raw session records are persisted to disk as JSONL.

This builds on the normalizer pipeline from Phase 7A — the pipeline extracts rich metrics from CLI output into `AgentEventMetadata`, and the AnalyticsCollector reads those normalized fields.

## Tech Stack

- Rust (backend, Tauri managed state)
- serde/serde_json (serialization)
- JSONL file storage (persistence)
- Existing EventBus subscriber pattern

---

## 1. AgentEventMetadata Extension

### New fields

All new fields are `Option<T>` with `#[serde(default)]` — zero impact on existing code.

```rust
// In agent_event.rs, added to AgentEventMetadata:

/// Cache tokens created during this request (Claude)
#[serde(default)]
pub cache_creation_tokens: Option<u64>,

/// Cache tokens read/reused during this request (Claude, Gemini)
#[serde(default)]
pub cache_read_tokens: Option<u64>,

/// Total wall-clock duration of the CLI session in ms
#[serde(default)]
pub duration_ms: Option<u64>,

/// API-only duration (excludes tool execution time)
#[serde(default)]
pub duration_api_ms: Option<u64>,

/// Number of conversation turns in this session
#[serde(default)]
pub num_turns: Option<u32>,

/// Why the session stopped (end_turn, max_tokens, tool_use, etc.)
#[serde(default)]
pub stop_reason: Option<String>,

/// Context window utilization as fraction 0.0–1.0 (Kimi)
#[serde(default)]
pub context_usage: Option<f64>,

/// Current context token count
#[serde(default)]
pub context_tokens: Option<u64>,

/// Maximum context window size for the model
#[serde(default)]
pub max_context_tokens: Option<u64>,

/// Cost in USD as reported by the CLI (Claude provides this directly)
#[serde(default)]
pub total_cost_usd: Option<f64>,
```

### Why these fields

| Field | Source CLIs | User value |
|-------|-----------|------------|
| `cache_creation_tokens` | Claude | Understand cache investment |
| `cache_read_tokens` | Claude, Gemini | Cache hit rate → speed + cost savings |
| `duration_ms` | Claude, Gemini, Qwen | Real-world speed comparison |
| `duration_api_ms` | Claude, Qwen | API vs tool overhead |
| `num_turns` | Claude, Qwen | Session complexity tracking |
| `stop_reason` | Claude | Why did it stop? Context limit vs done |
| `context_usage` | Kimi | Context pressure monitoring |
| `context_tokens` | Kimi | Raw context consumption |
| `max_context_tokens` | Kimi | Context window headroom |
| `total_cost_usd` | Claude | Exact cost per session (CLI-computed) |

### Deferred fields from master spec

The master spec (§10) defines `capabilities_used: HashMap<String, u64>` and `estimated_cost_usd: Option<f64>`. Both are **deferred**:

- **`capabilities_used`**: Tracked implicitly via `tools_used` in `SessionMetrics` and event types (thinking, tool_use). A dedicated capabilities breakdown can be derived from session records in Phase 7C when the frontend analytics UI is built.
- **`estimated_cost_usd`**: The master spec assumed this required a pricing table. However, Claude CLI provides `total_cost_usd` directly in its `result` event, so we capture it as `total_cost_usd` in metadata. For other providers, cost estimation from token counts is deferred to Phase 7C (requires a pricing table per model).

### Pipeline integration

The `NormalizerPipeline::build_event()` method already extracts metadata fields via `rule.mappings`. The new fields follow the exact same pattern — each TOML rule maps a JSON path to a metadata field name, and `build_event()` resolves it.

New extraction logic in `build_event()`:
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

---

## 2. TOML Mapping Updates

### Claude — `claude.toml`

Add cache token mappings to the existing `usage` rule, and add a new `result_metrics` rule for the CLI-level `result` event that carries duration/turns/cost.

**Verified against real Claude CLI output** (v2.1.81, `--output-format stream-json --verbose`). The CLI emits these event types in order:
1. Anthropic API events: `message_start`, `content_block_*`, `message_delta` (with `usage`), `message_stop`
2. CLI-level events: `assistant` (with per-message `usage` including cache tokens), `rate_limit_event`, `result` (final summary)

Sample `result` event from real CLI output:
```json
{"type":"result","subtype":"success","is_error":false,"duration_ms":4105,"duration_api_ms":4089,"num_turns":1,"result":"...","stop_reason":"end_turn","total_cost_usd":0.05597975,"usage":{"input_tokens":3,"cache_creation_input_tokens":7727,"cache_read_input_tokens":15092,"output_tokens":5},"modelUsage":{"claude-opus-4-6[1m]":{"contextWindow":1000000,"maxOutputTokens":64000}}}
```

```toml
[[rules]]
name = "usage"
when = 'type == "message_delta" && exists(usage)'
emit = "usage"
[rules.mappings]
input_tokens = "usage.input_tokens"
output_tokens = "usage.output_tokens"

[[rules]]
name = "result_metrics"
when = 'type == "result"'
emit = "usage"
[rules.mappings]
cache_creation_tokens = "usage.cache_creation_input_tokens"
cache_read_tokens = "usage.cache_read_input_tokens"
duration_ms = "duration_ms"
duration_api_ms = "duration_api_ms"
num_turns = "num_turns"
stop_reason = "stop_reason"
total_cost_usd = "total_cost_usd"
```

**Note:** The `result_metrics` rule intentionally does NOT map `input_tokens`/`output_tokens` — these are already accumulated from the streaming `message_delta` usage events. Mapping them here would cause double-counting. The `result` event carries cache tokens, timing, and cost which are not available in `message_delta`.

### Gemini — `gemini.toml`

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

### Kimi — `kimi.toml`

**Speculative:** Kimi's wire protocol (from `kimi_cli/wire/types.py`) defines `StatusUpdate` events with `context_usage` (float), `context_tokens`, and `max_context_tokens`. The exact JSON field names in stream-json output need verification against actual CLI output. The rule below is based on the wire protocol types and will be validated during implementation with a real Kimi session or updated if the actual format differs.

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

### Qwen — `qwen.toml`

Qwen's `result` event contains usage and timing:

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

### Codex — `codex.toml`

Codex reports usage via `ThreadTokenUsageUpdatedNotification`. Note: Codex does not provide `stop_reason` or `duration_ms` in its notification protocol — the `TurnCompletedNotification` has `status` (completed/interrupted/failed) but no timing data.

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

---

## 3. SessionMetrics — Per-Session Record

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub session_id: String,
    pub provider: String,
    pub model: String,
    pub started_at: u64,      // ms since epoch
    pub ended_at: Option<u64>,

    // Token usage (accumulated from all Usage events in session)
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,

    // Timing (from final result/done event)
    pub duration_ms: Option<u64>,
    pub duration_api_ms: Option<u64>,

    // Interaction
    pub num_turns: u32,
    pub tools_used: HashMap<String, u32>,  // tool_name → invocation count
    pub stop_reason: Option<String>,

    // Context window pressure
    pub peak_context_usage: Option<f64>,   // highest 0.0–1.0 seen
    pub max_context_tokens: Option<u64>,

    // Cost (Claude provides directly; others: None until pricing table exists)
    pub total_cost_usd: Option<f64>,

    // Errors during this session
    pub errors: Vec<ErrorRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub timestamp: u64,
    pub code: String,                        // from AgentEventMetadata.error_code; "unknown" if None
    pub severity: crate::agent_event::ErrorSeverity,  // reuses existing enum (Recoverable/Degraded/Fatal)
    pub recovered: bool,                     // true if session continued after this error
}
```

### Accumulation logic

The AnalyticsCollector maintains a `HashMap<String, SessionMetrics>` for active sessions. On each event:

| Event type | Action |
|-----------|--------|
| `Usage` | `input_tokens += meta.input_tokens`; same for output, cache_creation, cache_read. **Last-write-wins** for: `duration_ms`, `duration_api_ms`, `num_turns`, `stop_reason`, `total_cost_usd` (these come from the final `result` event, not intermediate streaming events). |
| `Metrics` | Update `peak_context_usage = max(current, meta.context_usage)`. Set `max_context_tokens`. |
| `ToolUse` | If `meta.tool_name` is `Some(name)`: `tools_used[name] += 1`. If `None`: skip (malformed event, don't track). |
| `Error` | Push `ErrorRecord` with `code = meta.error_code.unwrap_or("unknown")`, `severity = meta.error_severity` (reuses `crate::agent_event::ErrorSeverity`). Mark previous error in this session as `recovered = true` (session continued after it). |
| `Done` | Set `ended_at`. Mark last error as `recovered = false` if it was the final event before Done. Flush to disk. Remove from active map. |

### First event bootstrapping

When the AnalyticsCollector receives an event for an unknown `session_id`, it creates a new `SessionMetrics` with `started_at = event.timestamp`, `provider = event.metadata.provider`, and `model = event.metadata.model.unwrap_or_default()`.

---

## 4. AnalyticsCollector — EventBus Subscriber

```rust
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
        // Get or create SessionMetrics for this session
        // Accumulate based on event type
        // On Done: flush to store, remove from active
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
```

### Ownership pattern (Tauri state + EventBus subscriber)

The `AnalyticsCollector` must be both Tauri managed state (for commands to query) and an EventBus subscriber (for event ingestion). This creates a dual-ownership requirement. The pattern follows `SessionHistoryRecorder`:

```rust
// In lib.rs .manage():
let collector = Arc::new(AnalyticsCollector::new(store));
// ... later register as managed state via a wrapper or Arc directly

// In .setup():
let collector_ref = collector.clone();  // Arc clone
struct CollectorWrapper(Arc<AnalyticsCollector>);
impl AgentEventSubscriber for CollectorWrapper {
    fn on_event(&self, session_id: &str, event: &AgentEvent) {
        self.0.on_event(session_id, event);
    }
    fn filter(&self) -> Option<EventFilter> {
        self.0.filter()
    }
}
transport.event_bus().subscribe(Box::new(CollectorWrapper(collector_ref)));
```

Tauri's `.manage()` accepts `Arc<T>` via the `State<'_, Arc<AnalyticsCollector>>` pattern, or the collector can implement the subscriber trait directly and use the wrapper only for the EventBus subscription.

### Thread safety

All access to `active` goes through `Mutex`. The EventBus already serializes calls to `on_event` per subscriber (single lock in `publish()`), so contention is minimal.

---

## 5. AnalyticsStore — Persistence

```rust
pub struct AnalyticsStore {
    path: PathBuf,                                    // ~/.local/share/reasonance/analytics/
    completed: Mutex<Vec<SessionMetrics>>,             // in-memory cache of all completed sessions
}

impl AnalyticsStore {
    /// Creates dir if needed, then calls load() to populate completed sessions from disk.
    pub fn new(dir: &Path) -> Result<Self, String>;
    pub fn append(&self, metrics: &SessionMetrics) -> Result<(), String>;  // append to metrics.jsonl
    fn load(&mut self) -> Result<(), String>;          // called by new(); populates completed from JSONL
    pub fn all_completed(&self) -> Vec<SessionMetrics>; // clone of all completed sessions
}
```

File format: `metrics.jsonl` — one JSON object per line, one line per completed session. Loaded into memory at startup for fast aggregation.

### Storage estimate

Each `SessionMetrics` serializes to ~500 bytes. 10,000 sessions = ~5MB. Fits comfortably in memory.

---

## 6. Aggregation Functions (On-Demand)

These are methods on `AnalyticsCollector`, not stored state.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAnalytics {
    pub provider: String,
    pub total_sessions: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub cache_hit_rate: f32,           // cache_read / (input + cache_read)
    pub total_errors: u64,
    pub recovered_errors: u64,
    pub error_rate: f32,               // errors / sessions
    pub avg_duration_ms: f64,
    pub avg_tokens_per_second: f32,    // output_tokens / (duration_ms/1000)
    pub most_used_model: String,
    pub total_tool_invocations: u64,
    pub total_cost_usd: Option<f64>,  // sum of sessions where cost was reported (Claude)
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
    pub date: String,              // "2026-03-22"
    pub provider: Option<String>,  // None = all providers
    pub sessions: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub errors: u64,
    pub avg_duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub from: Option<u64>,   // ms since epoch, inclusive
    pub to: Option<u64>,     // ms since epoch, exclusive
}
```

### Query methods

```rust
impl AnalyticsCollector {
    /// Per-provider aggregate, optionally filtered by time range
    pub fn get_provider_analytics(
        &self, provider: &str, range: Option<TimeRange>
    ) -> ProviderAnalytics;

    /// All providers compared
    pub fn compare_providers(
        &self, range: Option<TimeRange>
    ) -> Vec<ProviderAnalytics>;

    /// Model breakdown within a provider
    pub fn get_model_breakdown(
        &self, provider: &str, range: Option<TimeRange>
    ) -> Vec<ModelAnalytics>;

    /// Single session detail
    pub fn get_session_metrics(
        &self, session_id: &str
    ) -> Option<SessionMetrics>;

    /// Daily stats for trend charts
    pub fn get_daily_stats(
        &self, provider: Option<&str>, days: u32
    ) -> Vec<DailyStats>;

    /// Active sessions (not yet Done)
    pub fn get_active_sessions(&self) -> Vec<SessionMetrics>;
}
```

---

## 7. Tauri Commands

New file: `src-tauri/src/commands/analytics.rs`

```rust
#[tauri::command]
pub fn analytics_provider(
    provider: String,
    from: Option<u64>,
    to: Option<u64>,
    collector: tauri::State<'_, AnalyticsCollector>,
) -> Result<ProviderAnalytics, String>;

#[tauri::command]
pub fn analytics_compare(
    from: Option<u64>,
    to: Option<u64>,
    collector: tauri::State<'_, AnalyticsCollector>,
) -> Result<Vec<ProviderAnalytics>, String>;

#[tauri::command]
pub fn analytics_model_breakdown(
    provider: String,
    from: Option<u64>,
    to: Option<u64>,
    collector: tauri::State<'_, AnalyticsCollector>,
) -> Result<Vec<ModelAnalytics>, String>;

#[tauri::command]
pub fn analytics_session(
    session_id: String,
    collector: tauri::State<'_, AnalyticsCollector>,
) -> Result<Option<SessionMetrics>, String>;

#[tauri::command]
pub fn analytics_daily(
    provider: Option<String>,
    days: Option<u32>,
    collector: tauri::State<'_, AnalyticsCollector>,
) -> Result<Vec<DailyStats>, String>;

#[tauri::command]
pub fn analytics_active(
    collector: tauri::State<'_, AnalyticsCollector>,
) -> Result<Vec<SessionMetrics>, String>;
```

### Registration in lib.rs

```rust
// In lib.rs, .manage():
.manage({
    let analytics_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("reasonance")
        .join("analytics");
    let store = std::sync::Arc::new(
        analytics::AnalyticsStore::new(&analytics_dir)
            .expect("Failed to init analytics store")
    );
    std::sync::Arc::new(analytics::AnalyticsCollector::new(store))
})

// In .setup(), wire to EventBus using Arc wrapper:
let collector: tauri::State<'_, std::sync::Arc<analytics::AnalyticsCollector>> = app.state();
struct CollectorWrapper(std::sync::Arc<analytics::AnalyticsCollector>);
impl transport::event_bus::AgentEventSubscriber for CollectorWrapper {
    fn on_event(&self, session_id: &str, event: &crate::agent_event::AgentEvent) {
        self.0.on_event(session_id, event);
    }
    fn filter(&self) -> Option<transport::event_bus::EventFilter> {
        self.0.filter()
    }
}
transport.event_bus().subscribe(Box::new(CollectorWrapper(collector.inner().clone())));

// In .invoke_handler(), add:
commands::analytics::analytics_provider,
commands::analytics::analytics_compare,
commands::analytics::analytics_model_breakdown,
commands::analytics::analytics_session,
commands::analytics::analytics_daily,
commands::analytics::analytics_active,
```

**Note:** Tauri commands will receive `State<'_, Arc<AnalyticsCollector>>` and call methods via the Arc.

---

## 8. File Impact Map

### New files

| File | Purpose |
|------|---------|
| `src-tauri/src/analytics/mod.rs` | Module root: SessionMetrics, ErrorRecord, ProviderAnalytics, ModelAnalytics, DailyStats, TimeRange |
| `src-tauri/src/analytics/collector.rs` | AnalyticsCollector (EventBus subscriber + query methods) |
| `src-tauri/src/analytics/store.rs` | AnalyticsStore (JSONL persistence) |
| `src-tauri/src/commands/analytics.rs` | Tauri commands |

### Modified files

| File | Changes |
|------|---------|
| `src-tauri/src/agent_event.rs` | Add 10 optional fields to `AgentEventMetadata`, update `base_metadata()` |
| `src-tauri/src/normalizer/pipeline.rs` | Extract new metadata fields in `build_event()` |
| `src-tauri/normalizers/claude.toml` | Add cache/duration mappings, add `result_metrics` rule |
| `src-tauri/normalizers/gemini.toml` | Add cache/duration mappings to usage rule |
| `src-tauri/normalizers/kimi.toml` | Add `context_metrics` rule |
| `src-tauri/normalizers/qwen.toml` | Add duration/turns mappings to usage rule |
| `src-tauri/src/lib.rs` | Register AnalyticsCollector as managed state, wire to EventBus, register commands |

---

## 9. Testing Strategy

### Unit tests

- `SessionMetrics` accumulation: verify token counting, cache tracking, error recording, tool counting
- `AnalyticsStore`: write/read roundtrip, empty file handling, corrupted line handling
- Aggregation functions: `get_provider_analytics` with known data, `get_daily_stats` grouping, `compare_providers` sorting
- `ErrorRecord.recovered` logic: error followed by more events → recovered = true; error as last event → recovered = false

### Integration tests

- Extended fixture tests: update existing fixture `.expected.json` files to verify new metadata fields (cache_creation_tokens, duration_ms, etc.)
- New fixtures for `result` events (Claude, Qwen) that produce duration/num_turns
- End-to-end: create AnalyticsCollector → feed events → query aggregations → verify

### Edge cases

- Session with zero Usage events (provider crashed immediately)
- Multiple Usage events per session (accumulation correctness)
- Session never receives Done event (process killed — active sessions API)
- Empty analytics store (first-run experience)
- Very large session (100K+ tokens — numeric overflow safety with u64)

---

## 10. Design Decisions

### Why session-level records, not pre-aggregated counters?

Pre-aggregated counters (total_input_tokens, total_sessions) lose the temporal dimension. Users can't answer "was this week worse than last week?" or drill into individual expensive sessions. Session-level records preserve everything; aggregations are computed when needed.

### Why JSONL, not SQLite?

- JSONL is append-only, crash-safe (no WAL corruption risk)
- At our scale (thousands of sessions, not millions), full-scan aggregation is <10ms
- No additional dependency (SQLite would add ~1MB to binary)
- Human-readable for debugging
- If we outgrow this, migration to SQLite is straightforward (deserialize → insert)

### Why extend AgentEventMetadata, not a separate RawMetrics blob?

Type safety. Every field has a concrete type, not `serde_json::Value`. The pipeline extracts them via the same mapping mechanism. The frontend receives typed data. No duplicate parsing logic.

### Why filter on 5 event types, not all?

The AnalyticsCollector only needs Usage (tokens, timing), Metrics (context), ToolUse (tool tracking), Error (error tracking), and Done (session finalization). Text and Thinking events carry no analytics-relevant data and would add noise.
