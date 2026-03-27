# W1.8 Performance Baselines + Startup Parallelization — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Instrument app-level performance baselines, parallelize startup init, and add CI regression detection.

**Architecture:** Add timing instrumentation to `lib.rs` setup, parallelize independent init steps with `tokio::join!`, record baselines to JSON, add Criterion benchmarks for key paths, integrate into CI.

**Tech Stack:** Rust (criterion, tokio, serde, std::time::Instant), GitHub Actions CI

---

## Task 1: Startup instrumentation + parallelization

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add startup timing instrumentation**

At the start of `setup()`, record the start time:
```rust
let setup_start = std::time::Instant::now();
```

Add timing points after each major init block. Log results:
```rust
info!("⏱ Startup: EventBus init {}ms", t1.elapsed().as_millis());
info!("⏱ Startup: SessionManager init {}ms", t2.elapsed().as_millis());
// etc.
info!("⏱ Startup: total setup {}ms", setup_start.elapsed().as_millis());
```

- [ ] **Step 2: Parallelize independent init with tokio::join!**

Identify independent init steps in setup():
- **Independent (can parallelize):**
  - SessionManager init (creates JsonFileBackend + async new)
  - AnalyticsCollector init (creates JsonFileBackend + async new)
  - NormalizerVersionStore init
  - AgentMemory init
  - TrustStore init
  - AppStateStore init
  - LayeredSettings init (already done in .manage(), but could move)

- **Sequential (must run after bus):**
  - EventBus creation + channel registration (must be first)
  - Subscriber wiring (needs EventBus + SessionManager + Analytics)
  - Transport + WorkflowEngine bus wiring (needs EventBus)
  - Lifecycle signals (needs EventBus + transport + PTY)

Refactor: group the independent async inits into a single `tokio::join!`:

```rust
let rt = tokio::runtime::Handle::current();
let (session_mgr, analytics_collector) = rt.block_on(async {
    let (sm_result, ac_result) = tokio::join!(
        async {
            let sessions_dir = dirs::data_dir()...;
            let backend = Arc::new(storage::JsonFileBackend::new(&sessions_dir)?);
            transport::session_manager::SessionManager::new(backend).await
        },
        async {
            let analytics_dir = dirs::data_dir()...;
            let backend = Arc::new(storage::JsonFileBackend::new(&analytics_dir)?);
            let store = Arc::new(analytics::store::AnalyticsStore::new(backend).await?);
            Ok::<_, ReasonanceError>(Arc::new(
                analytics::collector::AnalyticsCollector::new(store),
            ))
        },
    );
    Ok::<_, ReasonanceError>((sm_result?, ac_result?))
}).expect("Failed parallel init");
app.manage(session_mgr);
app.manage(analytics_collector);
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "perf(startup): parallelize independent init steps with tokio::join!, add timing"
```

---

## Task 2: App-level Criterion benchmarks

**Files:**
- Modify: `src-tauri/benches/baseline.rs`

- [ ] **Step 1: Add EventBus benchmark**

```rust
fn bench_event_bus_publish(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let bus = std::sync::Arc::new(reasonance_lib::event_bus::EventBus::new(rt.handle().clone()));
    bus.register_channel("bench", false);

    c.bench_function("event_bus_publish", |b| {
        b.iter(|| {
            bus.publish(reasonance_lib::event_bus::Event::new(
                "bench",
                serde_json::json!(null),
                "bench",
            ));
        })
    });
}
```

- [ ] **Step 2: Add StorageBackend benchmarks**

```rust
fn bench_storage_put_get(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let backend = std::sync::Arc::new(reasonance_lib::storage::InMemoryBackend::new());
    let data = serde_json::to_vec(&serde_json::json!({"key": "value", "count": 42})).unwrap();

    c.bench_function("storage_inmemory_put", |b| {
        b.iter(|| {
            rt.block_on(backend.put("bench", "key", &data)).unwrap();
        })
    });

    c.bench_function("storage_inmemory_get", |b| {
        b.iter(|| {
            rt.block_on(backend.get("bench", "key")).unwrap();
        })
    });
}
```

- [ ] **Step 3: Run benchmarks**

Run: `cd src-tauri && cargo bench -- --output-format bencher 2>&1 | head -30`
Expected: Benchmark results printed

- [ ] **Step 4: Commit**

```bash
git add src-tauri/benches/baseline.rs
git commit -m "bench: add EventBus and StorageBackend Criterion benchmarks"
```

---

## Task 3: Baseline recording mechanism + CI integration

**Files:**
- Create: `src-tauri/src/perf.rs` — baseline recording utility
- Modify: `src-tauri/src/lib.rs` — wire startup time recording
- Modify: `.github/workflows/ci.yml` — add benchmark job

- [ ] **Step 1: Create perf.rs baseline recorder**

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupBaseline {
    pub timestamp: String,
    pub git_commit: String,
    pub total_setup_ms: u64,
    pub event_bus_ms: u64,
    pub parallel_init_ms: u64,
    pub subscriber_wiring_ms: u64,
}

pub fn record_startup(baseline: &StartupBaseline) {
    let path = Path::new("benchmarks/baselines.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut entries: Vec<StartupBaseline> = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    entries.push(baseline.clone());
    // Keep last 100 entries
    if entries.len() > 100 {
        entries.drain(..entries.len() - 100);
    }
    let _ = std::fs::write(path, serde_json::to_string_pretty(&entries).unwrap_or_default());
}
```

Register module in lib.rs: `mod perf;`

- [ ] **Step 2: Record startup baseline at end of setup()**

In `lib.rs` at the end of setup(), record the baseline using the timing data collected in Task 1.

- [ ] **Step 3: Add benchmark job to CI**

In `.github/workflows/ci.yml`, add a benchmark step:
```yaml
  bench:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run benchmarks
        run: cd src-tauri && cargo bench -- --output-format bencher
      - name: Check for regressions
        run: |
          echo "Benchmark results recorded. Manual review for >5% regressions."
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test perf -- --nocapture`
Expected: PASS (or no tests if perf.rs has none — that's OK)

Run: `cd src-tauri && cargo check`
Expected: Clean

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/perf.rs src-tauri/src/lib.rs .github/workflows/ci.yml
git commit -m "feat(perf): add baseline recording, startup instrumentation, CI benchmark job"
```

---

## Task 4: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cd src-tauri && cargo test`
Expected: ALL PASS

- [ ] **Step 2: Run clippy**

Run: `cd src-tauri && cargo clippy -- -D warnings`
Expected: Clean

- [ ] **Step 3: Run frontend tests**

Run: `npx svelte-kit sync && npx vitest run`
Expected: ALL PASS

- [ ] **Step 4: Verify exit criteria**

- Startup timing logged on every launch
- Independent init steps parallelized via tokio::join!
- Criterion benchmarks for EventBus + Storage
- Baseline recording to benchmarks/baselines.json
- CI benchmark job in ci.yml
