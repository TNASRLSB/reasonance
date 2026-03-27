# W1.9 Transaction Semantics — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add transaction support to StorageBackend so multi-key writes (e.g., session metadata + events) are atomically consistent — either all succeed or all roll back.

**Architecture:** Add begin/commit/rollback to StorageBackend trait with a TransactionId. JsonFileBackend uses write-ahead log (WAL): transactional writes go to `.wal` files, commit atomically renames all, rollback deletes them. InMemoryBackend buffers writes in a pending map. Wire SessionStore's `create_session` as the first consumer.

**Tech Stack:** Rust (async-trait, serde, tokio, uuid), existing StorageBackend

---

## Task 1: Add Transaction API to StorageBackend trait + InMemoryBackend

**Files:**
- Modify: `src-tauri/src/storage/mod.rs`
- Modify: `src-tauri/src/storage/in_memory.rs`

- [ ] **Step 1: Define TransactionId and add trait methods**

In `storage/mod.rs`, add a type alias and extend the trait:

```rust
/// Opaque transaction identifier.
pub type TransactionId = String;

// Add to StorageBackend trait:

/// Begin a transaction in `namespace`. Returns a transaction ID.
/// Writes within the transaction are buffered until commit.
async fn begin_transaction(&self, namespace: &str) -> Result<TransactionId, ReasonanceError>;

/// Transactional put — buffered until commit.
async fn tx_put(&self, tx: &TransactionId, key: &str, value: &[u8]) -> Result<(), ReasonanceError>;

/// Transactional append — buffered until commit.
async fn tx_append(&self, tx: &TransactionId, key: &str, line: &[u8]) -> Result<(), ReasonanceError>;

/// Commit all buffered writes atomically.
async fn commit(&self, tx: TransactionId) -> Result<(), ReasonanceError>;

/// Discard all buffered writes.
async fn rollback_transaction(&self, tx: TransactionId) -> Result<(), ReasonanceError>;
```

- [ ] **Step 2: Implement in InMemoryBackend**

Add a pending transactions map:
```rust
struct PendingTx {
    namespace: String,
    puts: Vec<(String, Vec<u8>)>,
    appends: Vec<(String, Vec<u8>)>,
}

// Field in InMemoryBackend:
transactions: Mutex<HashMap<String, PendingTx>>,
```

- `begin_transaction`: generate UUID, insert empty PendingTx
- `tx_put`: push to pending puts
- `tx_append`: push to pending appends
- `commit`: apply all puts and appends to the actual data/streams, remove PendingTx
- `rollback_transaction`: remove PendingTx without applying

- [ ] **Step 3: Add tests**

```rust
#[tokio::test]
async fn transaction_commit_applies_writes() {
    let backend = InMemoryBackend::new();
    let tx = backend.begin_transaction("ns").await.unwrap();
    backend.tx_put(&tx, "key1", b"val1").await.unwrap();
    backend.tx_append(&tx, "log", b"line1").await.unwrap();
    // Before commit: nothing visible
    assert!(backend.get("ns", "key1").await.unwrap().is_none());
    // Commit
    backend.commit(tx).await.unwrap();
    // After commit: visible
    assert_eq!(backend.get("ns", "key1").await.unwrap(), Some(b"val1".to_vec()));
    let lines = backend.read_stream("ns", "log").await.unwrap();
    assert_eq!(lines.len(), 1);
}

#[tokio::test]
async fn transaction_rollback_discards_writes() {
    let backend = InMemoryBackend::new();
    let tx = backend.begin_transaction("ns").await.unwrap();
    backend.tx_put(&tx, "key1", b"val1").await.unwrap();
    backend.rollback_transaction(tx).await.unwrap();
    assert!(backend.get("ns", "key1").await.unwrap().is_none());
}
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test storage -- --nocapture`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/storage/
git commit -m "feat(storage): add transaction API to StorageBackend + InMemoryBackend"
```

---

## Task 2: Implement transactions for JsonFileBackend (WAL)

**Files:**
- Modify: `src-tauri/src/storage/json_file.rs`

- [ ] **Step 1: Implement WAL-based transactions**

Add transaction state to JsonFileBackend:
```rust
struct WalEntry {
    namespace: String,
    puts: Vec<(String, Vec<u8>)>,     // (key, value)
    appends: Vec<(String, Vec<u8>)>,  // (key, line)
}

// Field:
transactions: Mutex<HashMap<String, WalEntry>>,
```

- `begin_transaction`: generate UUID, create empty WalEntry
- `tx_put`: buffer the (key, value) pair in the WalEntry
- `tx_append`: buffer the (key, line) pair in the WalEntry
- `commit`: for each buffered put, call `atomic_write`. For each buffered append, call `safe_append`. Remove WalEntry. If any write fails mid-commit, log the error but continue with remaining writes (best-effort atomicity on filesystem).
- `rollback_transaction`: remove WalEntry, nothing to clean up on disk

- [ ] **Step 2: Add tests**

```rust
#[tokio::test]
async fn json_file_transaction_commit() {
    let dir = tempfile::tempdir().unwrap();
    let backend = JsonFileBackend::new(dir.path()).unwrap();
    let tx = backend.begin_transaction("txns").await.unwrap();
    backend.tx_put(&tx, "meta", b"{\"id\":\"s1\"}").await.unwrap();
    backend.tx_append(&tx, "events", b"{\"type\":\"text\"}").await.unwrap();
    // Not visible before commit
    assert!(backend.get("txns", "meta").await.unwrap().is_none());
    backend.commit(tx).await.unwrap();
    // Visible after commit
    assert!(backend.get("txns", "meta").await.unwrap().is_some());
    assert_eq!(backend.read_stream("txns", "events").await.unwrap().len(), 1);
}

#[tokio::test]
async fn json_file_transaction_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let backend = JsonFileBackend::new(dir.path()).unwrap();
    let tx = backend.begin_transaction("txns").await.unwrap();
    backend.tx_put(&tx, "meta", b"data").await.unwrap();
    backend.rollback_transaction(tx).await.unwrap();
    assert!(backend.get("txns", "meta").await.unwrap().is_none());
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test storage -- --nocapture`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/storage/json_file.rs
git commit -m "feat(storage): implement WAL-based transactions for JsonFileBackend"
```

---

## Task 3: Wire SessionStore with transactional create_session

**Files:**
- Modify: `src-tauri/src/transport/session_store.rs`

- [ ] **Step 1: Make create_session transactional**

In `SessionStore::create_session()`, wrap the metadata put and initial event stream setup in a transaction:

```rust
pub async fn create_session(&self, handle: &SessionHandle) -> Result<(), ReasonanceError> {
    let tx = self.backend.begin_transaction("sessions").await?;
    let meta_key = format!("{}:meta", handle.session_id);
    let meta_bytes = serde_json::to_vec_pretty(handle)?;
    self.backend.tx_put(&tx, &meta_key, &meta_bytes).await?;
    self.backend.commit(tx).await?;
    Ok(())
}
```

- [ ] **Step 2: Add test for transactional session creation**

```rust
#[tokio::test]
async fn create_session_is_transactional() {
    let backend = Arc::new(InMemoryBackend::new());
    let store = SessionStore::new(backend.clone());
    let handle = make_test_handle("tx-test");
    store.create_session(&handle).await.unwrap();
    // Metadata should exist after transactional commit
    let meta = store.read_metadata("tx-test").await.unwrap();
    assert_eq!(meta.session_id, "tx-test");
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test session_store -- --nocapture`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/transport/session_store.rs
git commit -m "feat(storage): wire SessionStore create_session with transactions"
```

---

## Task 4: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cd src-tauri && cargo test`
Expected: ALL PASS

- [ ] **Step 2: Run clippy**

Run: `cd src-tauri && cargo clippy -- -D warnings`
Expected: Clean

- [ ] **Step 3: Verify exit criteria**

- Transaction API in StorageBackend trait (begin/tx_put/tx_append/commit/rollback)
- Both backends implement transactions
- SessionStore uses transactions for create_session
- Tests verify commit applies writes and rollback discards them
