# W1.6 Storage Abstraction: Complete + Wire — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend StorageBackend with append/stream, migrate/rollback, then wire SessionStore and AnalyticsStore as real consumers — eliminating all direct file I/O in those modules.

**Architecture:** Extend the trait with `append()` + `read_stream()` for JSONL patterns, plus `migrate()` + `rollback()` for schema versioning. Implement in both backends (JsonFile, InMemory). Migrate SessionStore first (most complex: metadata + events + index), then AnalyticsStore (simpler: metrics append + cache). Both consumers become backend-agnostic via TypedStore + stream methods.

**Tech Stack:** Rust (async-trait, serde, serde_json, tokio), existing StorageBackend in `src-tauri/src/storage/`

**Master Spec:** [2026-03-27-wiring-completion-master-spec.md](../specs/roadmap/2026-03-27-wiring-completion-master-spec.md)

---

## Task 1: Add append/read_stream to StorageBackend trait + both backends

**Files:**
- Modify: `src-tauri/src/storage/mod.rs`
- Modify: `src-tauri/src/storage/json_file.rs`
- Modify: `src-tauri/src/storage/in_memory.rs`

- [ ] **Step 1: Write tests for append + read_stream in mod.rs**

Add to `#[cfg(test)] mod tests` in `storage/mod.rs`:

```rust
#[tokio::test]
async fn typed_store_append_and_read_stream() {
    let backend = Arc::new(InMemoryBackend::new());
    let store: TypedStore<TestItem> = TypedStore::new(backend, "test");

    let item1 = TestItem { name: "a".to_string(), count: 1 };
    let item2 = TestItem { name: "b".to_string(), count: 2 };
    store.append("log", &item1).await.unwrap();
    store.append("log", &item2).await.unwrap();

    let items = store.read_stream("log").await.unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "a");
    assert_eq!(items[1].name, "b");
}

#[tokio::test]
async fn typed_store_read_stream_empty() {
    let backend = Arc::new(InMemoryBackend::new());
    let store: TypedStore<TestItem> = TypedStore::new(backend, "test");
    let items = store.read_stream::<TestItem>("nonexistent").await.unwrap();
    assert!(items.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test storage -- --nocapture 2>&1 | head -20`
Expected: compilation errors — `append`, `read_stream` don't exist yet.

- [ ] **Step 3: Add append + read_stream to StorageBackend trait**

In `storage/mod.rs`, add to the `StorageBackend` trait:

```rust
/// Append a line of bytes to a stream (JSONL-style) under `key` in `namespace`.
/// Creates the stream if it doesn't exist.
async fn append(&self, namespace: &str, key: &str, line: &[u8]) -> Result<(), ReasonanceError>;

/// Read all lines from a stream under `key` in `namespace`.
/// Returns an empty Vec if the stream doesn't exist.
async fn read_stream(&self, namespace: &str, key: &str) -> Result<Vec<Vec<u8>>, ReasonanceError>;
```

Add corresponding methods to `TypedStore<T>`:

```rust
/// Append a serialized value to a stream (one JSON line per entry).
pub async fn append(&self, key: &str, value: &T) -> Result<(), ReasonanceError> {
    let bytes = serde_json::to_vec(value).map_err(|e| ReasonanceError::Serialization {
        context: "TypedStore::append".to_string(),
        message: e.to_string(),
    })?;
    self.backend.append(&self.namespace, key, &bytes).await
}

/// Read all entries from a stream, deserializing each line.
/// Skips lines that fail to deserialize (corrupted entries).
pub async fn read_stream<U: DeserializeOwned>(&self, key: &str) -> Result<Vec<U>, ReasonanceError> {
    let lines = self.backend.read_stream(&self.namespace, key).await?;
    Ok(lines.iter().filter_map(|bytes| serde_json::from_slice(bytes).ok()).collect())
}
```

- [ ] **Step 4: Implement in InMemoryBackend**

In `storage/in_memory.rs`, change the data structure to support both key-value and streams:

```rust
pub struct InMemoryBackend {
    data: Mutex<HashMap<String, HashMap<String, Vec<u8>>>>,
    streams: Mutex<HashMap<String, HashMap<String, Vec<Vec<u8>>>>>,
}
```

Implement `append`:
```rust
async fn append(&self, namespace: &str, key: &str, line: &[u8]) -> Result<(), ReasonanceError> {
    let mut streams = self.streams.lock().unwrap_or_else(|e| e.into_inner());
    streams
        .entry(namespace.to_string())
        .or_default()
        .entry(key.to_string())
        .or_default()
        .push(line.to_vec());
    Ok(())
}
```

Implement `read_stream`:
```rust
async fn read_stream(&self, namespace: &str, key: &str) -> Result<Vec<Vec<u8>>, ReasonanceError> {
    let streams = self.streams.lock().unwrap_or_else(|e| e.into_inner());
    Ok(streams
        .get(namespace)
        .and_then(|ns| ns.get(key))
        .cloned()
        .unwrap_or_default())
}
```

Update `new()` and `Default` to initialize `streams`.

- [ ] **Step 5: Implement in JsonFileBackend**

In `storage/json_file.rs`, implement `append` using the existing `safe_append()` helper:

```rust
async fn append(&self, namespace: &str, key: &str, line: &[u8]) -> Result<(), ReasonanceError> {
    let path = self.key_path(namespace, key);
    self.ensure_ns_dir(namespace)?;
    safe_append(&path, line)
}
```

Implement `read_stream` by reading JSONL line-by-line:

```rust
async fn read_stream(&self, namespace: &str, key: &str) -> Result<Vec<Vec<u8>>, ReasonanceError> {
    let path = self.key_path(namespace, key);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| {
        ReasonanceError::io(format!("read stream {}/{}", namespace, key), e)
    })?;
    Ok(content.lines().filter(|l| !l.is_empty()).map(|l| l.as_bytes().to_vec()).collect())
}
```

- [ ] **Step 6: Run tests**

Run: `cd src-tauri && cargo test storage -- --nocapture`
Expected: ALL PASS (existing 20+ tests + new stream tests)

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/storage/
git commit -m "feat(storage): add append/read_stream to StorageBackend trait + both backends"
```

---

## Task 2: Add migrate/rollback to StorageBackend trait + both backends

**Files:**
- Modify: `src-tauri/src/storage/mod.rs`
- Modify: `src-tauri/src/storage/json_file.rs`
- Modify: `src-tauri/src/storage/in_memory.rs`

- [ ] **Step 1: Write tests for migrate/rollback**

Add to `storage/mod.rs` tests:

```rust
#[tokio::test]
async fn backend_migrate_stores_version() {
    let backend = Arc::new(InMemoryBackend::new());
    assert_eq!(backend.get_version("sessions").await.unwrap(), 0);
    backend.migrate("sessions", 1).await.unwrap();
    assert_eq!(backend.get_version("sessions").await.unwrap(), 1);
}

#[tokio::test]
async fn backend_rollback_decrements_version() {
    let backend = Arc::new(InMemoryBackend::new());
    backend.migrate("sessions", 1).await.unwrap();
    backend.migrate("sessions", 2).await.unwrap();
    backend.rollback("sessions", 1).await.unwrap();
    assert_eq!(backend.get_version("sessions").await.unwrap(), 1);
}
```

- [ ] **Step 2: Add methods to StorageBackend trait**

```rust
/// Record that `namespace` has been migrated to `version`.
async fn migrate(&self, namespace: &str, version: u32) -> Result<(), ReasonanceError>;

/// Roll back `namespace` to `version`.
async fn rollback(&self, namespace: &str, version: u32) -> Result<(), ReasonanceError>;

/// Get the current schema version for `namespace`. Returns 0 if never migrated.
async fn get_version(&self, namespace: &str) -> Result<u32, ReasonanceError>;
```

- [ ] **Step 3: Implement in InMemoryBackend**

Add `versions: Mutex<HashMap<String, u32>>` field. Implement:
- `migrate`: set version
- `rollback`: set version
- `get_version`: read version, default 0

- [ ] **Step 4: Implement in JsonFileBackend**

Store version in `{base_dir}/{namespace}/_version` as a plain integer string.

- [ ] **Step 5: Run tests**

Run: `cd src-tauri && cargo test storage -- --nocapture`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/storage/
git commit -m "feat(storage): add migrate/rollback/get_version to StorageBackend trait"
```

---

## Task 3: Wire SessionStore to StorageBackend

**Files:**
- Modify: `src-tauri/src/transport/session_store.rs`
- Modify: `src-tauri/src/transport/session_manager.rs` (if it creates SessionStore)
- Modify: `src-tauri/src/lib.rs` (wire backend into SessionStore)

- [ ] **Step 1: Refactor SessionStore to accept StorageBackend**

Change the struct to hold a backend:

```rust
pub struct SessionStore {
    backend: Arc<dyn StorageBackend>,
}
```

Namespace conventions:
- Metadata: namespace `"sessions"`, key `"{session_id}:metadata"` → `TypedStore<SessionHandle>`
- Events: namespace `"sessions"`, key `"{session_id}:events"` → stream (append/read_stream)
- Index: namespace `"sessions"`, key `"_index"` → `TypedStore<Vec<SessionSummary>>`

- [ ] **Step 2: Migrate each method**

Convert all methods from direct fs I/O to backend calls:

- `new(backend)` — just store the backend Arc, no directory creation needed
- `create_session(handle)` — `backend.put("sessions", "{id}:metadata", ...)` + initialize empty stream
- `append_event(session_id, event)` — `backend.append("sessions", "{id}:events", json_bytes)`
- `read_events(session_id)` — `backend.read_stream("sessions", "{id}:events")`, deserialize each line
- `write_metadata(handle)` — `backend.put("sessions", "{id}:metadata", ...)`
- `read_metadata(session_id)` — `backend.get("sessions", "{id}:metadata")`, deserialize
- `write_index(summaries)` — `backend.put("sessions", "_index", ...)`
- `read_index()` — `backend.get("sessions", "_index")`, deserialize, default to empty vec
- `delete_session(session_id)` — `backend.delete("sessions", "{id}:metadata")` + `backend.delete("sessions", "{id}:events")`
- `session_exists(session_id)` — `backend.exists("sessions", "{id}:metadata")`

- [ ] **Step 3: Update SessionManager**

`SessionManager::new()` currently creates a `SessionStore` with a path. Change it to accept an `Arc<dyn StorageBackend>` and pass it to `SessionStore::new()`.

- [ ] **Step 4: Update lib.rs**

In setup, create a `JsonFileBackend` for sessions and pass it to `SessionManager`:

```rust
let sessions_dir = dirs::data_dir()
    .unwrap_or_else(|| std::path::PathBuf::from("."))
    .join("reasonance")
    .join("sessions");
let session_backend = Arc::new(storage::JsonFileBackend::new(&sessions_dir)?);
let session_mgr = transport::session_manager::SessionManager::new(session_backend);
```

- [ ] **Step 5: Run tests**

Run: `cd src-tauri && cargo test session_store session_manager -- --nocapture`
Expected: ALL PASS

- [ ] **Step 6: Run full test suite**

Run: `cd src-tauri && cargo test`
Expected: ALL PASS

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/transport/session_store.rs src-tauri/src/transport/session_manager.rs src-tauri/src/lib.rs
git commit -m "refactor(storage): wire SessionStore to StorageBackend"
```

---

## Task 4: Wire AnalyticsStore to StorageBackend

**Files:**
- Modify: `src-tauri/src/analytics/store.rs`
- Modify: `src-tauri/src/lib.rs` (wire backend)

- [ ] **Step 1: Refactor AnalyticsStore to accept StorageBackend**

Change struct:
```rust
pub struct AnalyticsStore {
    backend: Arc<dyn StorageBackend>,
    completed: Mutex<Vec<SessionMetrics>>,
}
```

Namespace: `"analytics"`, stream key: `"metrics"`

- [ ] **Step 2: Migrate methods**

- `new(backend)` — store backend, load existing metrics via `read_stream`
- `append(metrics)` — `backend.append("analytics", "metrics", json_bytes)` + push to in-memory cache
- `load()` — `backend.read_stream("analytics", "metrics")`, deserialize, populate cache
- `all_completed()` / `with_completed()` — unchanged (read from cache)

- [ ] **Step 3: Update lib.rs**

```rust
let analytics_dir = dirs::data_dir()
    .unwrap_or_else(|| std::path::PathBuf::from("."))
    .join("reasonance")
    .join("analytics");
let analytics_backend = Arc::new(storage::JsonFileBackend::new(&analytics_dir)?);
let analytics_store = Arc::new(analytics::collector::AnalyticsCollector::new(
    analytics::store::AnalyticsStore::new(analytics_backend)?
));
```

Adjust `AnalyticsCollector::new()` if needed to accept the new `AnalyticsStore`.

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test analytics -- --nocapture`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/analytics/store.rs src-tauri/src/lib.rs
git commit -m "refactor(storage): wire AnalyticsStore to StorageBackend"
```

---

## Task 5: Final verification

- [ ] **Step 1: Run full Rust test suite**

Run: `cd src-tauri && cargo test`
Expected: ALL PASS

- [ ] **Step 2: Run clippy**

Run: `cd src-tauri && cargo clippy -- -D warnings`
Expected: Clean

- [ ] **Step 3: Run frontend tests**

Run: `npx svelte-kit sync && npx vitest run`
Expected: ALL PASS

- [ ] **Step 4: Verify exit criteria**

- StorageBackend trait includes append/read_stream + migrate/rollback/get_version
- Both backends implement all methods with tests
- SessionStore uses StorageBackend (not direct file I/O)
- AnalyticsStore uses StorageBackend (not direct file I/O)
- 2 real consumers wired
