# W1.11 IPC Batching Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Batch multiple frontend Tauri invoke calls into single IPC roundtrips, with per-call Zod validation, deduplication, cancellation, and timeout.

**Architecture:** A Rust `batch_invoke` command dispatches to `_inner` functions in parallel via `tokio`. The frontend `TauriAdapter` transparently queues calls per microtask and flushes as a single batch. An explicit `batch()` API guarantees grouping for critical paths. Zod validates each result. AbortController cancels stale requests.

**Tech Stack:** Rust (tokio, serde_json, futures), TypeScript (Zod 4, Tauri invoke API, queueMicrotask, AbortController)

**Spec:** `.claude/docs/specs/2026-03-28-ipc-batching-design.md`

---

## File Structure

### New files
| File | Responsibility |
|------|---------------|
| `src-tauri/src/commands/batch.rs` | `BatchCall`/`BatchCallResult` types, `dispatch()`, `batch_invoke` command |
| `src/lib/adapter/batch-schemas.ts` | Zod schema registry mapping command names to validation schemas |

### Modified files
| File | Changes |
|------|---------|
| `src-tauri/src/error.rs` | Add `timeout()` convenience constructor |
| `src-tauri/src/commands/mod.rs` | Add `pub mod batch;` |
| `src-tauri/src/commands/fs.rs` | Extract `_inner` for `read_file`, `write_file`, `list_dir`, `grep_files` |
| `src-tauri/src/commands/session.rs` | Extract `_inner` for `session_create`, `session_list`, `session_get_events`, `session_restore` |
| `src-tauri/src/commands/app_state.rs` | Extract `_inner` for all 4 commands |
| `src-tauri/src/commands/shadow.rs` | Extract `_inner` for `store_shadow`, `get_shadow` |
| `src-tauri/src/commands/analytics.rs` | Extract `_inner` for `analytics_daily`, `analytics_compare`, `analytics_model_breakdown` |
| `src-tauri/src/commands/workflow.rs` | Extract `_inner` for `load_workflow`, `list_workflows` |
| `src-tauri/src/commands/engine.rs` | Extract `_inner` for `get_run_status` |
| `src-tauri/src/commands/settings.rs` | Extract `_inner` for `get_setting` |
| `src-tauri/src/lib.rs` | Register `batch_invoke` in `generate_handler![]` |
| `src-tauri/Cargo.toml` | Add `futures` crate |
| `src/lib/adapter/tauri.ts` | Add batching layer (enqueue, flush, dedup, abort, batch API), migrate methods |
| `src/lib/adapter/index.ts` | Add `batch()` to `Adapter` interface, add `AbortSignal` to navigable methods |

---

## Task 1: Rust setup — deps, types, timeout constructor

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/error.rs`
- Create: `src-tauri/src/commands/batch.rs`
- Modify: `src-tauri/src/commands/mod.rs`

- [ ] **Step 1: Add `futures` crate to Cargo.toml**

In `src-tauri/Cargo.toml`, add to `[dependencies]`:

```toml
futures = "0.3"
```

- [ ] **Step 2: Add `timeout()` constructor to ReasonanceError**

In `src-tauri/src/error.rs`, after the `transport()` constructor (line ~186), add:

```rust
    pub fn timeout(operation: impl Into<String>, duration_ms: u64) -> Self {
        Self::Timeout {
            operation: operation.into(),
            duration_ms,
        }
    }
```

- [ ] **Step 3: Create batch.rs with types**

Create `src-tauri/src/commands/batch.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::error::ReasonanceError;

#[derive(Debug, Deserialize)]
pub struct BatchCall {
    pub command: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct BatchCallResult {
    pub ok: Option<serde_json::Value>,
    pub err: Option<ReasonanceError>,
}

impl BatchCallResult {
    pub fn success(value: serde_json::Value) -> Self {
        Self { ok: Some(value), err: None }
    }

    pub fn error(err: ReasonanceError) -> Self {
        Self { ok: None, err: Some(err) }
    }
}

/// Extract a typed field from a JSON args object, or return a Validation error.
pub fn extract<T: serde::de::DeserializeOwned>(args: &serde_json::Value, field: &str) -> Result<T, ReasonanceError> {
    serde_json::from_value(
        args.get(field)
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|e| ReasonanceError::validation(field, format!("failed to deserialize '{}': {}", field, e)))
}

/// Extract an optional field — returns Ok(None) if the field is absent or null.
pub fn extract_opt<T: serde::de::DeserializeOwned>(args: &serde_json::Value, field: &str) -> Result<Option<T>, ReasonanceError> {
    match args.get(field) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => serde_json::from_value(v.clone())
            .map(Some)
            .map_err(|e| ReasonanceError::validation(field, format!("failed to deserialize '{}': {}", field, e))),
    }
}
```

- [ ] **Step 4: Register the module**

In `src-tauri/src/commands/mod.rs`, add:

```rust
pub mod batch;
```

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/error.rs src-tauri/src/commands/batch.rs src-tauri/src/commands/mod.rs
git commit -m "feat(batch): add futures dep, BatchCall types, timeout constructor"
```

---

## Task 2: Extract `_inner` functions — fs module

**Files:**
- Modify: `src-tauri/src/commands/fs.rs`

The pattern for each command: extract the body into a `pub fn <name>_inner(...)` that takes `&str` instead of `String` and `&T` instead of `State<'_, T>`. The `#[tauri::command]` wrapper delegates to the inner function.

- [ ] **Step 1: Extract `read_file_inner`**

In `src-tauri/src/commands/fs.rs`, replace the `read_file` function (lines 316-353) with:

```rust
pub fn read_file_inner(
    path: &str,
    state: &ProjectRootState,
) -> Result<String, ReasonanceError> {
    info!("cmd::read_file(path={})", path);
    validate_read_path(Path::new(path), state)?;

    let metadata =
        fs::metadata(path).map_err(|e| ReasonanceError::io(format!("stat '{}'", path), e))?;
    if metadata.len() > MAX_READ_FILE_SIZE {
        error!(
            "cmd::read_file file too large: {} bytes at {}",
            metadata.len(),
            path
        );
        return Err(ReasonanceError::validation(
            "file_size",
            format!(
                "File too large ({:.1} MB). Maximum allowed size is {:.0} MB.",
                metadata.len() as f64 / (1024.0 * 1024.0),
                MAX_READ_FILE_SIZE as f64 / (1024.0 * 1024.0),
            ),
        ));
    }

    fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::InvalidData {
            error!("cmd::read_file error for {}: binary file", path);
            ReasonanceError::validation(
                "file_content",
                "Cannot open binary file: the file contains non-UTF-8 data.",
            )
        } else {
            error!("cmd::read_file error for {}: {}", path, e);
            ReasonanceError::io(format!("read '{}'", path), e)
        }
    })
}

#[tauri::command]
pub fn read_file(
    path: String,
    state: State<'_, ProjectRootState>,
) -> Result<String, ReasonanceError> {
    read_file_inner(&path, &state)
}
```

- [ ] **Step 2: Extract `write_file_inner`**

Replace `write_file` (lines 356-394) with:

```rust
pub fn write_file_inner(
    path: &str,
    content: &str,
    state: &ProjectRootState,
) -> Result<(), ReasonanceError> {
    info!("cmd::write_file(path={})", path);
    validate_write_path(Path::new(path), state)?;

    let target = Path::new(path);
    let parent = target.parent().ok_or_else(|| {
        ReasonanceError::validation("path", format!("No parent directory for '{}'", path))
    })?;
    let file_name = target.file_name().ok_or_else(|| {
        ReasonanceError::validation("path", format!("No file name for '{}'", path))
    })?;

    let tmp_name = format!(".{}.tmp", file_name.to_string_lossy());
    let tmp_path = parent.join(&tmp_name);

    fs::write(&tmp_path, content).map_err(|e| {
        error!(
            "cmd::write_file failed to write temp file {}: {}",
            tmp_path.display(),
            e
        );
        ReasonanceError::io(format!("write temp file '{}'", tmp_path.display()), e)
    })?;
    fs::rename(&tmp_path, target).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        error!(
            "cmd::write_file failed to rename temp file to {}: {}",
            path, e
        );
        ReasonanceError::io(format!("rename temp file to '{}'", path), e)
    })
}

#[tauri::command]
pub fn write_file(
    path: String,
    content: String,
    state: State<'_, ProjectRootState>,
) -> Result<(), ReasonanceError> {
    write_file_inner(&path, &content, &state)
}
```

- [ ] **Step 3: Extract `list_dir_inner`**

Replace `list_dir` (lines 397-453) with:

```rust
pub fn list_dir_inner(
    path: &str,
    respect_gitignore: bool,
    state: &ProjectRootState,
) -> Result<Vec<FileEntry>, ReasonanceError> {
    info!("cmd::list_dir(path={})", path);
    validate_read_path(Path::new(path), state)?;
    let entries =
        fs::read_dir(path).map_err(|e| ReasonanceError::io(format!("read dir '{}'", path), e))?;

    let gitignore = if respect_gitignore {
        ignore::gitignore::Gitignore::new(Path::new(path).join(".gitignore")).0
    } else {
        ignore::gitignore::Gitignore::empty()
    };

    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| ReasonanceError::io("read dir entry", e))?;
        let metadata = entry
            .metadata()
            .map_err(|e| ReasonanceError::io("read entry metadata", e))?;

        let is_ignored = if respect_gitignore {
            let matched = gitignore.matched_path_or_any_parents(&entry.path(), metadata.is_dir());
            matched.is_ignore()
        } else {
            false
        };

        let modified = metadata
            .modified()
            .map_err(|e| ReasonanceError::io("read modified time", e))?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        result.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified,
            is_gitignored: is_ignored,
        });
    }
    result.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    debug!(
        "cmd::list_dir returned {} entries for {}",
        result.len(),
        path
    );
    Ok(result)
}

#[tauri::command]
pub fn list_dir(
    path: String,
    respect_gitignore: bool,
    state: State<'_, ProjectRootState>,
) -> Result<Vec<FileEntry>, ReasonanceError> {
    list_dir_inner(&path, respect_gitignore, &state)
}
```

- [ ] **Step 4: Extract `grep_files_inner`**

Replace `grep_files` (lines 464-513) with:

```rust
pub fn grep_files_inner(
    path: &str,
    pattern: &str,
    respect_gitignore: bool,
    state: &ProjectRootState,
) -> Result<Vec<GrepResult>, ReasonanceError> {
    info!("cmd::grep_files(path={}, pattern={})", path, pattern);
    validate_read_path(Path::new(path), state)?;
    use ignore::WalkBuilder;
    use std::io::BufRead;

    let mut results = Vec::new();
    let walker = WalkBuilder::new(path)
        .git_ignore(respect_gitignore)
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }
        let file_path = entry.path().to_owned();
        if let Ok(file) = std::fs::File::open(&file_path) {
            let reader = std::io::BufReader::new(file);
            for (i, line_result) in reader.lines().enumerate() {
                if let Ok(line) = line_result {
                    if line.contains(pattern) {
                        results.push(GrepResult {
                            path: file_path.to_string_lossy().to_string(),
                            line_number: i + 1,
                            line,
                        });
                        if results.len() >= 500 {
                            debug!(
                                "cmd::grep_files hit 500 result limit for pattern={}",
                                pattern
                            );
                            return Ok(results);
                        }
                    }
                }
            }
        }
    }
    debug!(
        "cmd::grep_files found {} matches for pattern={}",
        results.len(),
        pattern
    );
    Ok(results)
}

#[tauri::command]
pub fn grep_files(
    path: String,
    pattern: String,
    respect_gitignore: bool,
    state: State<'_, ProjectRootState>,
) -> Result<Vec<GrepResult>, ReasonanceError> {
    grep_files_inner(&path, &pattern, respect_gitignore, &state)
}
```

Note: `get_git_status` takes no `State` — no `_inner` needed. The dispatcher calls it directly.

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: compiles, no errors

- [ ] **Step 6: Run existing tests**

Run: `cd src-tauri && cargo test -- commands::fs`
Expected: all existing tests pass

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/fs.rs
git commit -m "refactor(fs): extract _inner functions for batch dispatch"
```

---

## Task 3: Extract `_inner` functions — session, app_state, shadow

**Files:**
- Modify: `src-tauri/src/commands/session.rs`
- Modify: `src-tauri/src/commands/app_state.rs`
- Modify: `src-tauri/src/commands/shadow.rs`

- [ ] **Step 1: Extract session `_inner` functions**

Replace `src-tauri/src/commands/session.rs` with:

```rust
use crate::agent_event::AgentEvent;
use crate::error::ReasonanceError;
use crate::transport::session_handle::{SessionHandle, SessionSummary, ViewMode};
use crate::transport::session_manager::SessionManager;
use log::{debug, error, info};
use tauri::State;

pub async fn session_create_inner(
    provider: &str,
    model: &str,
    session_manager: &SessionManager,
) -> Result<String, ReasonanceError> {
    info!(
        "cmd::session_create(provider={}, model={})",
        provider, model
    );
    let result = session_manager.create_session(provider, model).await;
    match &result {
        Ok(id) => debug!("cmd::session_create created session_id={}", id),
        Err(e) => error!("cmd::session_create failed: {}", e),
    }
    result
}

#[tauri::command]
pub async fn session_create(
    provider: String,
    model: String,
    session_manager: State<'_, SessionManager>,
) -> Result<String, ReasonanceError> {
    session_create_inner(&provider, &model, &session_manager).await
}

pub async fn session_restore_inner(
    session_id: &str,
    session_manager: &SessionManager,
) -> Result<SessionHandle, ReasonanceError> {
    info!("cmd::session_restore(session_id={})", session_id);
    let (handle, _events) = session_manager
        .restore_session(session_id)
        .await
        .map_err(|e| {
            error!(
                "cmd::session_restore failed for session_id={}: {}",
                session_id, e
            );
            e
        })?;
    Ok(handle)
}

#[tauri::command]
pub async fn session_restore(
    session_id: String,
    session_manager: State<'_, SessionManager>,
) -> Result<SessionHandle, ReasonanceError> {
    session_restore_inner(&session_id, &session_manager).await
}

pub async fn session_get_events_inner(
    session_id: &str,
    session_manager: &SessionManager,
) -> Result<Vec<AgentEvent>, ReasonanceError> {
    debug!("cmd::session_get_events(session_id={})", session_id);
    let store = session_manager.store();
    store.read_events(session_id).await
}

#[tauri::command]
pub async fn session_get_events(
    session_id: String,
    session_manager: State<'_, SessionManager>,
) -> Result<Vec<AgentEvent>, ReasonanceError> {
    session_get_events_inner(&session_id, &session_manager).await
}

pub async fn session_list_inner(
    session_manager: &SessionManager,
) -> Result<Vec<SessionSummary>, ReasonanceError> {
    info!("cmd::session_list called");
    Ok(session_manager.list_sessions())
}

#[tauri::command]
pub async fn session_list(
    session_manager: State<'_, SessionManager>,
) -> Result<Vec<SessionSummary>, ReasonanceError> {
    session_list_inner(&session_manager).await
}

pub async fn session_delete_inner(
    session_id: &str,
    session_manager: &SessionManager,
) -> Result<(), ReasonanceError> {
    info!("cmd::session_delete(session_id={})", session_id);
    session_manager.delete_session(session_id).await
}

#[tauri::command]
pub async fn session_delete(
    session_id: String,
    session_manager: State<'_, SessionManager>,
) -> Result<(), ReasonanceError> {
    session_delete_inner(&session_id, &session_manager).await
}

const MAX_SESSION_TITLE_LENGTH: usize = 200;

pub async fn session_rename_inner(
    session_id: &str,
    title: &str,
    session_manager: &SessionManager,
) -> Result<(), ReasonanceError> {
    info!("cmd::session_rename(session_id={})", session_id);
    if title.len() > MAX_SESSION_TITLE_LENGTH {
        error!(
            "cmd::session_rename title too long ({} chars) for session_id={}",
            title.len(),
            session_id
        );
        return Err(ReasonanceError::validation(
            "title",
            format!(
                "Session title too long ({} chars). Maximum allowed is {} characters.",
                title.len(),
                MAX_SESSION_TITLE_LENGTH,
            ),
        ));
    }
    session_manager.rename_session(session_id, title).await
}

#[tauri::command]
pub async fn session_rename(
    session_id: String,
    title: String,
    session_manager: State<'_, SessionManager>,
) -> Result<(), ReasonanceError> {
    session_rename_inner(&session_id, &title, &session_manager).await
}

pub async fn session_fork_inner(
    session_id: &str,
    fork_event_index: u32,
    session_manager: &SessionManager,
) -> Result<String, ReasonanceError> {
    info!(
        "cmd::session_fork(session_id={}, fork_event_index={})",
        session_id, fork_event_index
    );
    session_manager
        .fork_session(session_id, fork_event_index)
        .await
}

#[tauri::command]
pub async fn session_fork(
    session_id: String,
    fork_event_index: u32,
    session_manager: State<'_, SessionManager>,
) -> Result<String, ReasonanceError> {
    session_fork_inner(&session_id, fork_event_index, &session_manager).await
}

pub async fn session_set_view_mode_inner(
    session_id: &str,
    mode: ViewMode,
    session_manager: &SessionManager,
) -> Result<(), ReasonanceError> {
    info!(
        "cmd::session_set_view_mode(session_id={}, mode={:?})",
        session_id, mode
    );
    session_manager.set_view_mode(session_id, mode).await
}

#[tauri::command]
pub async fn session_set_view_mode(
    session_id: String,
    mode: ViewMode,
    session_manager: State<'_, SessionManager>,
) -> Result<(), ReasonanceError> {
    session_set_view_mode_inner(&session_id, mode, &session_manager).await
}
```

- [ ] **Step 2: Extract app_state `_inner` functions**

Replace `src-tauri/src/commands/app_state.rs` with:

```rust
use crate::app_state_store::{AppState, AppStateStore, ProjectState};
use crate::error::ReasonanceError;
use tauri::State;

pub fn get_app_state_inner(store: &AppStateStore) -> Result<AppState, ReasonanceError> {
    Ok(store.get_app_state())
}

#[tauri::command]
pub fn get_app_state(store: State<'_, AppStateStore>) -> Result<AppState, ReasonanceError> {
    get_app_state_inner(&store)
}

pub fn save_app_state_inner(
    store: &AppStateStore,
    state: &AppState,
) -> Result<(), ReasonanceError> {
    store.save_app_state(state)
}

#[tauri::command]
pub fn save_app_state(
    store: State<'_, AppStateStore>,
    state: AppState,
) -> Result<(), ReasonanceError> {
    save_app_state_inner(&store, &state)
}

pub fn get_project_state_inner(
    store: &AppStateStore,
    project_id: &str,
) -> Result<ProjectState, ReasonanceError> {
    Ok(store.get_project_state(project_id))
}

#[tauri::command]
pub fn get_project_state(
    store: State<'_, AppStateStore>,
    project_id: String,
) -> Result<ProjectState, ReasonanceError> {
    get_project_state_inner(&store, &project_id)
}

pub fn save_project_state_inner(
    store: &AppStateStore,
    project_id: &str,
    state: &ProjectState,
) -> Result<(), ReasonanceError> {
    store.save_project_state(project_id, state)
}

#[tauri::command]
pub fn save_project_state(
    store: State<'_, AppStateStore>,
    project_id: String,
    state: ProjectState,
) -> Result<(), ReasonanceError> {
    save_project_state_inner(&store, &project_id, &state)
}
```

- [ ] **Step 3: Extract shadow `_inner` functions**

Replace `src-tauri/src/commands/shadow.rs` with:

```rust
use crate::error::ReasonanceError;
use crate::shadow_store::ShadowStore;
use log::{debug, info};
use tauri::State;

pub fn store_shadow_inner(
    path: &str,
    content: &str,
    store: &ShadowStore,
) -> Result<(), ReasonanceError> {
    info!("cmd::store_shadow(path={})", path);
    store.store(path, content);
    Ok(())
}

#[tauri::command]
pub fn store_shadow(
    path: String,
    content: String,
    store: State<'_, ShadowStore>,
) -> Result<(), ReasonanceError> {
    store_shadow_inner(&path, &content, &store)
}

pub fn get_shadow_inner(
    path: &str,
    store: &ShadowStore,
) -> Result<Option<String>, ReasonanceError> {
    debug!("cmd::get_shadow(path={})", path);
    Ok(store.get(path))
}

#[tauri::command]
pub fn get_shadow(
    path: String,
    store: State<'_, ShadowStore>,
) -> Result<Option<String>, ReasonanceError> {
    get_shadow_inner(&path, &store)
}
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: compiles, no errors

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/session.rs src-tauri/src/commands/app_state.rs src-tauri/src/commands/shadow.rs
git commit -m "refactor(session,app_state,shadow): extract _inner functions for batch dispatch"
```

---

## Task 4: Extract `_inner` functions — analytics, workflow, engine, settings

**Files:**
- Modify: `src-tauri/src/commands/analytics.rs`
- Modify: `src-tauri/src/commands/workflow.rs`
- Modify: `src-tauri/src/commands/engine.rs`
- Modify: `src-tauri/src/commands/settings.rs`

- [ ] **Step 1: Extract analytics `_inner` functions**

In `src-tauri/src/commands/analytics.rs`, add `_inner` variants for the 3 batchable commands. The pattern is the same — take `&Arc<AnalyticsCollector>` instead of `State<'_, Arc<AnalyticsCollector>>`:

```rust
use crate::analytics::collector::AnalyticsCollector;
use crate::analytics::{DailyStats, ModelAnalytics, ProviderAnalytics, SessionMetrics, TimeRange};
use crate::error::ReasonanceError;
use log::{debug, info};
use std::sync::Arc;
use tauri::State;

pub fn analytics_daily_inner(
    provider: Option<&str>,
    days: Option<u32>,
    collector: &Arc<AnalyticsCollector>,
) -> Result<Vec<DailyStats>, ReasonanceError> {
    info!(
        "cmd::analytics_daily(provider={:?}, days={:?})",
        provider, days
    );
    Ok(collector.get_daily_stats(provider, days.unwrap_or(30)))
}

#[tauri::command]
pub fn analytics_daily(
    provider: Option<String>,
    days: Option<u32>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<DailyStats>, ReasonanceError> {
    analytics_daily_inner(provider.as_deref(), days, &collector)
}

pub fn analytics_compare_inner(
    from: Option<u64>,
    to: Option<u64>,
    collector: &Arc<AnalyticsCollector>,
) -> Result<Vec<ProviderAnalytics>, ReasonanceError> {
    info!("cmd::analytics_compare called");
    let range = if from.is_some() || to.is_some() {
        Some(TimeRange { from, to })
    } else {
        None
    };
    Ok(collector.compare_providers(range))
}

#[tauri::command]
pub fn analytics_compare(
    from: Option<u64>,
    to: Option<u64>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<ProviderAnalytics>, ReasonanceError> {
    analytics_compare_inner(from, to, &collector)
}

pub fn analytics_model_breakdown_inner(
    provider: &str,
    from: Option<u64>,
    to: Option<u64>,
    collector: &Arc<AnalyticsCollector>,
) -> Result<Vec<ModelAnalytics>, ReasonanceError> {
    info!("cmd::analytics_model_breakdown(provider={})", provider);
    let range = if from.is_some() || to.is_some() {
        Some(TimeRange { from, to })
    } else {
        None
    };
    Ok(collector.get_model_breakdown(provider, range))
}

#[tauri::command]
pub fn analytics_model_breakdown(
    provider: String,
    from: Option<u64>,
    to: Option<u64>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<ModelAnalytics>, ReasonanceError> {
    analytics_model_breakdown_inner(&provider, from, to, &collector)
}

#[tauri::command]
pub fn analytics_provider(
    provider: String,
    from: Option<u64>,
    to: Option<u64>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<ProviderAnalytics, ReasonanceError> {
    info!("cmd::analytics_provider(provider={})", provider);
    let range = if from.is_some() || to.is_some() {
        Some(TimeRange { from, to })
    } else {
        None
    };
    Ok(collector.get_provider_analytics(&provider, range))
}

#[tauri::command]
pub fn analytics_session(
    session_id: String,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Option<SessionMetrics>, ReasonanceError> {
    info!("cmd::analytics_session(session_id={})", session_id);
    Ok(collector.get_session_metrics(&session_id))
}

#[tauri::command]
pub fn analytics_active(
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<SessionMetrics>, ReasonanceError> {
    debug!("cmd::analytics_active called");
    Ok(collector.get_active_sessions())
}
```

- [ ] **Step 2: Extract workflow `_inner` functions**

In `src-tauri/src/commands/workflow.rs`, add `_inner` for `load_workflow` and `list_workflows`. These commands take two `State` params:

After the existing imports, before the `load_workflow` command, add:

```rust
pub fn load_workflow_inner(
    file_path: &str,
    store: &WorkflowStore,
    state: &ProjectRootState,
) -> Result<Workflow, ReasonanceError> {
    info!("cmd::load_workflow(path={})", file_path);
    validate_workflow_path(Path::new(file_path), state, false)?;
    store.load(file_path).map_err(|e| {
        error!("cmd::load_workflow failed for {}: {}", file_path, e);
        e
    })
}
```

Then change `load_workflow` to:

```rust
#[tauri::command]
pub fn load_workflow(
    file_path: String,
    store: State<'_, WorkflowStore>,
    state: State<'_, ProjectRootState>,
) -> Result<Workflow, ReasonanceError> {
    load_workflow_inner(&file_path, &store, &state)
}
```

Similarly for `list_workflows`:

```rust
pub fn list_workflows_inner(
    dir: &str,
    state: &ProjectRootState,
) -> Result<Vec<String>, ReasonanceError> {
    info!("cmd::list_workflows(dir={})", dir);
    validate_workflow_dir(Path::new(dir), state)?;
    WorkflowStore::list_workflows(dir)
}

#[tauri::command]
pub fn list_workflows(
    dir: String,
    state: State<'_, ProjectRootState>,
) -> Result<Vec<String>, ReasonanceError> {
    list_workflows_inner(&dir, &state)
}
```

- [ ] **Step 3: Extract engine `get_run_status_inner`**

In `src-tauri/src/commands/engine.rs`, add before `get_run_status`:

```rust
pub fn get_run_status_inner(run_id: &str, engine: &WorkflowEngine) -> Option<WorkflowRun> {
    debug!("cmd::get_run_status(run_id={})", run_id);
    engine.get_run(run_id)
}
```

Change `get_run_status` to:

```rust
#[tauri::command]
pub fn get_run_status(run_id: String, engine: State<'_, WorkflowEngine>) -> Option<WorkflowRun> {
    get_run_status_inner(&run_id, &engine)
}
```

- [ ] **Step 4: Extract settings `get_setting_inner`**

In `src-tauri/src/commands/settings.rs`, add before `get_setting`:

```rust
pub fn get_setting_inner(
    settings: &Mutex<LayeredSettings>,
    key: &str,
) -> Result<Option<serde_json::Value>, ReasonanceError> {
    info!("cmd::get_setting key={}", key);
    let s = settings.lock().unwrap_or_else(|e| e.into_inner());
    match s.get_value(key) {
        Some(toml_val) => {
            let json =
                serde_json::to_value(toml_val).map_err(|e| ReasonanceError::Serialization {
                    context: format!("setting key={}", key),
                    message: e.to_string(),
                })?;
            Ok(Some(json))
        }
        None => Ok(None),
    }
}
```

Change `get_setting` to:

```rust
#[tauri::command]
pub fn get_setting(
    settings: State<'_, Mutex<LayeredSettings>>,
    key: String,
) -> Result<Option<serde_json::Value>, ReasonanceError> {
    get_setting_inner(&settings, &key)
}
```

- [ ] **Step 5: Verify compilation and tests**

Run: `cd src-tauri && cargo check && cargo test`
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/analytics.rs src-tauri/src/commands/workflow.rs src-tauri/src/commands/engine.rs src-tauri/src/commands/settings.rs
git commit -m "refactor(analytics,workflow,engine,settings): extract _inner functions for batch dispatch"
```

---

## Task 5: Build dispatcher and `batch_invoke` command

**Files:**
- Modify: `src-tauri/src/commands/batch.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the dispatcher tests**

Add to the bottom of `src-tauri/src/commands/batch.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_string() {
        let args = serde_json::json!({"path": "/test/file.txt"});
        let path: String = extract(&args, "path").unwrap();
        assert_eq!(path, "/test/file.txt");
    }

    #[test]
    fn test_extract_missing_field() {
        let args = serde_json::json!({});
        let result: Result<String, _> = extract(&args, "path");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_opt_present() {
        let args = serde_json::json!({"provider": "claude"});
        let val: Option<String> = extract_opt(&args, "provider").unwrap();
        assert_eq!(val, Some("claude".to_string()));
    }

    #[test]
    fn test_extract_opt_absent() {
        let args = serde_json::json!({});
        let val: Option<String> = extract_opt(&args, "provider").unwrap();
        assert_eq!(val, None);
    }

    #[test]
    fn test_batch_call_result_success() {
        let r = BatchCallResult::success(serde_json::json!("hello"));
        assert!(r.ok.is_some());
        assert!(r.err.is_none());
    }

    #[test]
    fn test_batch_call_result_error() {
        let r = BatchCallResult::error(ReasonanceError::validation("test", "fail"));
        assert!(r.ok.is_none());
        assert!(r.err.is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cd src-tauri && cargo test -- commands::batch`
Expected: all 6 tests pass

- [ ] **Step 3: Add the dispatcher and batch_invoke**

Add imports and the full dispatch + batch_invoke implementation to `src-tauri/src/commands/batch.rs` (above the tests module):

```rust
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;

use crate::analytics::collector::AnalyticsCollector;
use crate::app_state_store::AppStateStore;
use crate::commands::{
    analytics, app_state, engine, fs, session, settings, shadow, workflow,
};
use crate::error::ReasonanceError;
use crate::event_bus::{Event, EventBus};
use crate::settings::LayeredSettings;
use crate::shadow_store::ShadowStore;
use crate::transport::session_manager::SessionManager;
use crate::workflow_engine::WorkflowEngine;
use crate::workflow_store::WorkflowStore;

// (BatchCall, BatchCallResult, extract, extract_opt structs remain above)

const BATCH_CALL_TIMEOUT: Duration = Duration::from_secs(5);

async fn dispatch(app: &AppHandle, cmd: &str, args: Value) -> Result<Value, ReasonanceError> {
    match cmd {
        // ── fs ──
        "read_file" => {
            let path: String = extract(&args, "path")?;
            let state = app.state::<fs::ProjectRootState>();
            let result = fs::read_file_inner(&path, &state)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "write_file" => {
            let path: String = extract(&args, "path")?;
            let content: String = extract(&args, "content")?;
            let state = app.state::<fs::ProjectRootState>();
            fs::write_file_inner(&path, &content, &state)?;
            Ok(Value::Null)
        }
        "list_dir" => {
            let path: String = extract(&args, "path")?;
            let respect_gitignore: bool = extract_opt(&args, "respectGitignore")?.unwrap_or(true);
            let state = app.state::<fs::ProjectRootState>();
            let result = fs::list_dir_inner(&path, respect_gitignore, &state)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "grep_files" => {
            let path: String = extract(&args, "path")?;
            let pattern: String = extract(&args, "pattern")?;
            let respect_gitignore: bool = extract_opt(&args, "respectGitignore")?.unwrap_or(true);
            let state = app.state::<fs::ProjectRootState>();
            let result = fs::grep_files_inner(&path, &pattern, respect_gitignore, &state)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_git_status" => {
            let project_root: String = extract(&args, "projectRoot")?;
            let result = fs::get_git_status(project_root).await?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── session ──
        "session_create" => {
            let provider: String = extract(&args, "provider")?;
            let model: String = extract(&args, "model")?;
            let sm = app.state::<SessionManager>();
            let result = session::session_create_inner(&provider, &model, &sm).await?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "session_list" => {
            let sm = app.state::<SessionManager>();
            let result = session::session_list_inner(&sm).await?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "session_get_events" => {
            let session_id: String = extract(&args, "sessionId")?;
            let sm = app.state::<SessionManager>();
            let result = session::session_get_events_inner(&session_id, &sm).await?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "session_restore" => {
            let session_id: String = extract(&args, "sessionId")?;
            let sm = app.state::<SessionManager>();
            let result = session::session_restore_inner(&session_id, &sm).await?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── app_state ──
        "get_app_state" => {
            let store = app.state::<AppStateStore>();
            let result = app_state::get_app_state_inner(&store)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_project_state" => {
            let project_id: String = extract(&args, "projectId")?;
            let store = app.state::<AppStateStore>();
            let result = app_state::get_project_state_inner(&store, &project_id)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "save_app_state" => {
            let state: crate::app_state_store::AppState = serde_json::from_value(
                args.get("state").cloned().unwrap_or(Value::Null),
            ).map_err(|e| ReasonanceError::validation("state", e.to_string()))?;
            let store = app.state::<AppStateStore>();
            app_state::save_app_state_inner(&store, &state)?;
            Ok(Value::Null)
        }
        "save_project_state" => {
            let project_id: String = extract(&args, "projectId")?;
            let state: crate::app_state_store::ProjectState = serde_json::from_value(
                args.get("state").cloned().unwrap_or(Value::Null),
            ).map_err(|e| ReasonanceError::validation("state", e.to_string()))?;
            let store = app.state::<AppStateStore>();
            app_state::save_project_state_inner(&store, &project_id, &state)?;
            Ok(Value::Null)
        }

        // ── workflow / engine ──
        "get_run_status" => {
            let run_id: String = extract(&args, "runId")?;
            let we = app.state::<WorkflowEngine>();
            let result = engine::get_run_status_inner(&run_id, &we);
            Ok(serde_json::to_value(result).unwrap())
        }
        "load_workflow" => {
            let file_path: String = extract(&args, "filePath")?;
            let store = app.state::<WorkflowStore>();
            let root = app.state::<fs::ProjectRootState>();
            let result = workflow::load_workflow_inner(&file_path, &store, &root)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "list_workflows" => {
            let dir: String = extract(&args, "dir")?;
            let root = app.state::<fs::ProjectRootState>();
            let result = workflow::list_workflows_inner(&dir, &root)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── analytics ──
        "analytics_daily" => {
            let provider: Option<String> = extract_opt(&args, "provider")?;
            let days: Option<u32> = extract_opt(&args, "days")?;
            let collector = app.state::<Arc<AnalyticsCollector>>();
            let result = analytics::analytics_daily_inner(provider.as_deref(), days, &collector)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "analytics_compare" => {
            let from: Option<u64> = extract_opt(&args, "from")?;
            let to: Option<u64> = extract_opt(&args, "to")?;
            let collector = app.state::<Arc<AnalyticsCollector>>();
            let result = analytics::analytics_compare_inner(from, to, &collector)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "analytics_model_breakdown" => {
            let provider: String = extract(&args, "provider")?;
            let from: Option<u64> = extract_opt(&args, "from")?;
            let to: Option<u64> = extract_opt(&args, "to")?;
            let collector = app.state::<Arc<AnalyticsCollector>>();
            let result = analytics::analytics_model_breakdown_inner(&provider, from, to, &collector)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── shadow ──
        "store_shadow" => {
            let path: String = extract(&args, "path")?;
            let content: String = extract(&args, "content")?;
            let store = app.state::<ShadowStore>();
            shadow::store_shadow_inner(&path, &content, &store)?;
            Ok(Value::Null)
        }
        "get_shadow" => {
            let path: String = extract(&args, "path")?;
            let store = app.state::<ShadowStore>();
            let result = shadow::get_shadow_inner(&path, &store)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── config / settings ──
        "read_config" => {
            let result = super::config::read_config()?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_setting" => {
            let key: String = extract(&args, "key")?;
            let s = app.state::<std::sync::Mutex<LayeredSettings>>();
            let result = settings::get_setting_inner(&s, &key)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        other => Err(ReasonanceError::validation(
            "command",
            format!("not batchable: {}", other),
        )),
    }
}

#[tauri::command]
pub async fn batch_invoke(calls: Vec<BatchCall>, app: AppHandle) -> Vec<BatchCallResult> {
    let call_count = calls.len();
    let call_names: Vec<String> = calls.iter().map(|c| c.command.clone()).collect();
    let start = std::time::Instant::now();

    let futures = calls.into_iter().map(|c| {
        let app = app.clone();
        async move {
            match tokio::time::timeout(
                BATCH_CALL_TIMEOUT,
                dispatch(&app, &c.command, c.args),
            )
            .await
            {
                Ok(Ok(value)) => BatchCallResult::success(value),
                Ok(Err(e)) => BatchCallResult::error(e),
                Err(_) => BatchCallResult::error(ReasonanceError::timeout(
                    &c.command,
                    BATCH_CALL_TIMEOUT.as_millis() as u64,
                )),
            }
        }
    });

    let results = join_all(futures).await;
    let elapsed = start.elapsed();
    let error_count = results.iter().filter(|r| r.err.is_some()).count();

    info!(
        "batch_invoke: {} calls in {}ms ({} errors)",
        call_count,
        elapsed.as_millis(),
        error_count
    );

    // Publish telemetry to EventBus
    if let Ok(event_bus) = app.try_state::<Arc<EventBus>>() {
        event_bus.publish(Event::new(
            "ipc:batch_executed",
            serde_json::json!({
                "batch_size": call_count,
                "duration_ms": elapsed.as_millis() as u64,
                "commands": call_names,
                "errors": error_count,
            }),
            "batch_invoke",
        ));
    }

    results
}
```

- [ ] **Step 4: Register `batch_invoke` in lib.rs**

In `src-tauri/src/lib.rs`, add `commands::batch::batch_invoke` to the `generate_handler![]` macro invocation.

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: compiles with no errors

- [ ] **Step 6: Run all Rust tests**

Run: `cd src-tauri && cargo test`
Expected: all pass

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/batch.rs src-tauri/src/lib.rs
git commit -m "feat(batch): add batch_invoke command with parallel dispatch, timeout, and EventBus telemetry"
```

---

## Task 6: Frontend Zod schema registry

**Files:**
- Create: `src/lib/adapter/batch-schemas.ts`

- [ ] **Step 1: Create the schema registry**

Create `src/lib/adapter/batch-schemas.ts`:

```typescript
import { z } from 'zod';

// -- Reusable sub-schemas --

const FileEntrySchema = z.object({
  name: z.string(),
  path: z.string(),
  isDir: z.boolean(),
  size: z.number(),
  modified: z.number(),
  isGitignored: z.boolean(),
});

const GrepResultSchema = z.object({
  path: z.string(),
  line_number: z.number(),
  line: z.string(),
});

const SessionSummarySchema = z.object({
  id: z.string(),
  title: z.string().nullable(),
  provider: z.string(),
  model: z.string(),
  created_at: z.number(),
  updated_at: z.number(),
  event_count: z.number(),
  view_mode: z.string(),
});

const NodeRunStateSchema = z.object({
  node_id: z.string(),
  agent_id: z.string().nullable(),
  state: z.string(),
});

const WorkflowRunSchema = z.object({
  id: z.string(),
  workflow_path: z.string(),
  status: z.string(),
  node_states: z.record(z.string(), NodeRunStateSchema),
  started_at: z.string().nullable(),
  finished_at: z.string().nullable(),
}).nullable();

const DailyStatsSchema = z.object({
  date: z.string(),
  provider: z.string().nullable(),
  sessions: z.number(),
  input_tokens: z.number(),
  output_tokens: z.number(),
  errors: z.number(),
  avg_duration_ms: z.number(),
  total_cost_usd: z.number().optional(),
});

const ProviderAnalyticsSchema = z.object({
  provider: z.string(),
  total_sessions: z.number(),
  total_input_tokens: z.number(),
  total_output_tokens: z.number(),
  total_cache_creation_tokens: z.number(),
  total_cache_read_tokens: z.number(),
  cache_hit_rate: z.number(),
  total_errors: z.number(),
  recovered_errors: z.number(),
  error_rate: z.number(),
  avg_duration_ms: z.number(),
  avg_tokens_per_second: z.number(),
  most_used_model: z.string(),
  total_tool_invocations: z.number(),
  total_cost_usd: z.number().nullable(),
});

const ModelAnalyticsSchema = z.object({
  model: z.string(),
  provider: z.string(),
  session_count: z.number(),
  avg_input_tokens: z.number(),
  avg_output_tokens: z.number(),
  avg_duration_ms: z.number(),
  avg_tokens_per_second: z.number(),
  error_rate: z.number(),
});

// AppState / ProjectState are pass-through (complex nested types, validated by Rust)
// We use z.object with passthrough for the top-level shape check only.
const AppStateSchema = z.object({
  last_active_project_id: z.string().nullable(),
  recent_projects: z.array(z.object({ path: z.string(), label: z.string(), last_opened: z.number() })),
  window_state: z.object({
    width: z.number(), height: z.number(), x: z.number(), y: z.number(), maximized: z.boolean(),
  }).nullable(),
});

const ProjectStateSchema = z.object({
  active_session_id: z.string().nullable(),
  open_files: z.array(z.object({
    path: z.string(), cursor_line: z.number(), cursor_column: z.number(), scroll_offset: z.number(),
  })),
  active_file_path: z.string().nullable(),
  panel_layout: z.object({
    sidebar_visible: z.boolean(), sidebar_width: z.number(),
    bottom_panel_visible: z.boolean(), bottom_panel_height: z.number(),
  }).nullable(),
  last_model_used: z.string().nullable(),
});

/**
 * Maps Tauri command names to Zod schemas for per-call validation in batch results.
 * Commands without an entry skip validation (result passed through as-is).
 */
export const batchSchemas: Record<string, z.ZodType> = {
  // fs
  read_file: z.string(),
  write_file: z.null(),
  list_dir: z.array(FileEntrySchema),
  grep_files: z.array(GrepResultSchema),
  get_git_status: z.record(z.string(), z.string()),

  // session (complex return types — pass-through, validated by Rust)
  session_create: z.string(),
  session_list: z.array(SessionSummarySchema),
  // session_get_events and session_restore have complex nested types — skip Zod, trust Rust

  // app_state
  get_app_state: AppStateSchema,
  get_project_state: ProjectStateSchema,
  save_app_state: z.null(),
  save_project_state: z.null(),

  // workflow / engine
  get_run_status: WorkflowRunSchema,
  list_workflows: z.array(z.string()),
  // load_workflow has complex Workflow type — skip Zod, trust Rust

  // analytics
  analytics_daily: z.array(DailyStatsSchema),
  analytics_compare: z.array(ProviderAnalyticsSchema),
  analytics_model_breakdown: z.array(ModelAnalyticsSchema),

  // shadow
  store_shadow: z.null(),
  get_shadow: z.string().nullable(),

  // config / settings
  read_config: z.string(),
  get_setting: z.unknown().nullable(),
};
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | head -20`
Expected: no errors related to batch-schemas.ts

- [ ] **Step 3: Commit**

```bash
git add src/lib/adapter/batch-schemas.ts
git commit -m "feat(batch): add Zod schema registry for batch result validation"
```

---

## Task 7: TauriAdapter batching layer

**Files:**
- Modify: `src/lib/adapter/tauri.ts`

- [ ] **Step 1: Write the batching tests**

Create `src/lib/adapter/__tests__/batch.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';

// We'll test the batching logic via the adapter. Mock the Tauri invoke.
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

// Import after mock setup
const { TauriAdapter } = await import('../tauri');

describe('IPC batching', () => {
  let adapter: InstanceType<typeof TauriAdapter>;

  beforeEach(() => {
    adapter = new TauriAdapter();
    invokeMock.mockReset();
  });

  it('batches calls in the same microtask into a single invoke', async () => {
    invokeMock.mockResolvedValueOnce([
      { ok: 'file content', err: null },
      { ok: { 'src/main.rs': 'modified' }, err: null },
    ]);

    const p1 = adapter.readFile('/test.rs');
    const p2 = adapter.getGitStatus('/project');

    const [file, git] = await Promise.all([p1, p2]);

    expect(file).toBe('file content');
    expect(git).toEqual({ 'src/main.rs': 'modified' });
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('batch_invoke', {
      calls: [
        { command: 'read_file', args: { path: '/test.rs' } },
        { command: 'get_git_status', args: { projectRoot: '/project' } },
      ],
    });
  });

  it('deduplicates identical calls in the same batch', async () => {
    invokeMock.mockResolvedValueOnce([
      { ok: 'content', err: null },
    ]);

    const p1 = adapter.readFile('/same.txt');
    const p2 = adapter.readFile('/same.txt');

    const [r1, r2] = await Promise.all([p1, p2]);

    expect(r1).toBe('content');
    expect(r2).toBe('content');
    // Only 1 call in the batch (deduped)
    expect(invokeMock.mock.calls[0][1].calls).toHaveLength(1);
  });

  it('rejects with error when Rust returns err for a call', async () => {
    invokeMock.mockResolvedValueOnce([
      { ok: null, err: { type: 'NotFound', details: { resource_type: 'file', identifier: '/nope' } } },
    ]);

    await expect(adapter.readFile('/nope')).rejects.toEqual(
      expect.objectContaining({ type: 'NotFound' }),
    );
  });

  it('rejects pre-aborted signals immediately without sending', async () => {
    const controller = new AbortController();
    controller.abort();

    invokeMock.mockResolvedValueOnce([]);

    await expect(adapter.readFile('/test', controller.signal)).rejects.toThrow('Aborted');
    // The batch should not include the aborted call
    // Give microtask a chance to flush
    await new Promise((r) => setTimeout(r, 10));
    // If there were no other calls, invoke might not even be called
  });

  it('handles partial failures: one fails, others succeed', async () => {
    invokeMock.mockResolvedValueOnce([
      { ok: 'ok1', err: null },
      { ok: null, err: { type: 'Timeout', details: { operation: 'slow', duration_ms: 5000 } } },
    ]);

    const p1 = adapter.readFile('/a.txt');
    const p2 = adapter.readFile('/b.txt');

    await expect(p1).resolves.toBe('ok1');
    await expect(p2).rejects.toEqual(expect.objectContaining({ type: 'Timeout' }));
  });

  it('long-running commands bypass batching', async () => {
    invokeMock.mockResolvedValueOnce('session-123');

    const result = await adapter.agentSend('hello', 'claude');

    expect(invokeMock).toHaveBeenCalledWith('agent_send', expect.any(Object));
    expect(result).toBe('session-123');
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/lib/adapter/__tests__/batch.test.ts`
Expected: FAIL — readFile doesn't use enqueue yet

- [ ] **Step 3: Add batching infrastructure to TauriAdapter**

In `src/lib/adapter/tauri.ts`, add the imports and batching fields/methods. At the top of the file, add:

```typescript
import { batchSchemas } from './batch-schemas';
```

Add the `PendingCall` interface and `BatchCallResult` type before the class:

```typescript
interface PendingCall {
  command: string;
  args: Record<string, unknown>;
  resolve: (value: unknown) => void;
  reject: (error: unknown) => void;
  signal?: AbortSignal;
}

interface BatchCallResult {
  ok: unknown;
  err: unknown;
}
```

At the start of the `TauriAdapter` class, add the private fields and methods:

```typescript
export class TauriAdapter implements Adapter {
  private queue: PendingCall[] = [];
  private flushScheduled = false;

  private enqueue(
    command: string,
    args: Record<string, unknown>,
    signal?: AbortSignal,
  ): Promise<unknown> {
    return new Promise((resolve, reject) => {
      if (signal?.aborted) {
        reject(new DOMException('Aborted', 'AbortError'));
        return;
      }
      const entry: PendingCall = { command, args, resolve, reject, signal };
      signal?.addEventListener('abort', () => {
        reject(new DOMException('Aborted', 'AbortError'));
      }, { once: true });
      this.queue.push(entry);
      if (!this.flushScheduled) {
        this.flushScheduled = true;
        queueMicrotask(() => this.flush());
      }
    });
  }

  private async flush(): Promise<void> {
    const batch = this.queue;
    this.queue = [];
    this.flushScheduled = false;

    if (batch.length === 0) return;

    // Deduplicate by (command, args) key
    const keyMap = new Map<string, { call: PendingCall; subscribers: PendingCall[] }>();
    for (const entry of batch) {
      if (entry.signal?.aborted) continue; // skip already-aborted
      const key = `${entry.command}::${JSON.stringify(entry.args)}`;
      const existing = keyMap.get(key);
      if (existing) {
        existing.subscribers.push(entry);
      } else {
        keyMap.set(key, { call: entry, subscribers: [entry] });
      }
    }
    const groups = [...keyMap.values()];
    if (groups.length === 0) return;

    const t0 = performance.now();
    let results: BatchCallResult[];
    try {
      results = await invoke<BatchCallResult[]>('batch_invoke', {
        calls: groups.map(g => ({ command: g.call.command, args: g.call.args })),
      });
    } catch (e) {
      // Transport-level failure — reject all
      for (const g of groups) {
        for (const sub of g.subscribers) {
          if (!sub.signal?.aborted) sub.reject(e);
        }
      }
      return;
    }
    const elapsed = performance.now() - t0;

    if (import.meta.env.DEV) {
      console.debug(
        `[batch] ${groups.length} calls (${batch.length} enqueued) in ${elapsed.toFixed(1)}ms`,
        groups.map(g => g.call.command),
      );
    }

    for (let i = 0; i < groups.length; i++) {
      const r = results[i];
      const { subscribers } = groups[i];

      for (const sub of subscribers) {
        if (sub.signal?.aborted) continue;

        if (r.err) {
          sub.reject(r.err);
        } else {
          const schema = batchSchemas[sub.command];
          if (schema) {
            const parsed = schema.safeParse(r.ok);
            if (!parsed.success) {
              console.error(`[batch] Zod validation failed for ${sub.command}:`, parsed.error);
              sub.reject(parsed.error);
              continue;
            }
            sub.resolve(parsed.data);
          } else {
            sub.resolve(r.ok);
          }
        }
      }
    }
  }

  async batch<T extends unknown[]>(
    fn: (ctx: TauriAdapter) => [...{ [K in keyof T]: Promise<T[K]> }],
  ): Promise<T> {
    const saved = this.queue;
    this.queue = [];
    const promises = fn(this);
    const batch = this.queue;
    this.queue = saved;

    if (batch.length === 0) return Promise.all(promises) as Promise<T>;

    let results: BatchCallResult[];
    try {
      results = await invoke<BatchCallResult[]>('batch_invoke', {
        calls: batch.map(c => ({ command: c.command, args: c.args })),
      });
    } catch (e) {
      for (const entry of batch) entry.reject(e);
      return Promise.all(promises) as Promise<T>;
    }

    for (let i = 0; i < batch.length; i++) {
      const r = results[i];
      if (r.err) {
        batch[i].reject(r.err);
      } else {
        const schema = batchSchemas[batch[i].command];
        if (schema) {
          const parsed = schema.safeParse(r.ok);
          if (!parsed.success) { batch[i].reject(parsed.error); continue; }
          batch[i].resolve(parsed.data);
        } else {
          batch[i].resolve(r.ok);
        }
      }
    }

    return Promise.all(promises) as Promise<T>;
  }

  // ... existing methods below
```

- [ ] **Step 4: Migrate batchable adapter methods to `enqueue()`**

Replace each batchable method's `invoke()` call with `this.enqueue()`. Methods that represent user-navigable state get an optional `signal` param:

```typescript
  async readFile(path: string, signal?: AbortSignal): Promise<string> {
    return this.enqueue('read_file', { path }, signal) as Promise<string>;
  }
  async writeFile(path: string, content: string): Promise<void> {
    return this.enqueue('write_file', { path, content }) as Promise<void>;
  }
  async listDir(path: string, respectGitignore?: boolean, signal?: AbortSignal): Promise<FileEntry[]> {
    return this.enqueue('list_dir', { path, respectGitignore: respectGitignore ?? true }, signal) as Promise<FileEntry[]>;
  }
  async getGitStatus(projectRoot: string, signal?: AbortSignal): Promise<Record<string, string>> {
    return this.enqueue('get_git_status', { projectRoot }, signal) as Promise<Record<string, string>>;
  }
  async grepFiles(path: string, pattern: string, respectGitignore: boolean): Promise<GrepResult[]> {
    return this.enqueue('grep_files', { path, pattern, respectGitignore }) as Promise<GrepResult[]>;
  }
  async readConfig(): Promise<string> {
    return this.enqueue('read_config', {}) as Promise<string>;
  }
  async storeShadow(path: string, content: string): Promise<void> {
    return this.enqueue('store_shadow', { path, content }) as Promise<void>;
  }
  async getShadow(path: string): Promise<string | null> {
    return this.enqueue('get_shadow', { path }) as Promise<string | null>;
  }
  async loadWorkflow(filePath: string): Promise<Workflow> {
    return this.enqueue('load_workflow', { filePath }) as Promise<Workflow>;
  }
  async listWorkflows(dir: string): Promise<string[]> {
    return this.enqueue('list_workflows', { dir }) as Promise<string[]>;
  }
  async getRunStatus(runId: string): Promise<WorkflowRun | null> {
    return this.enqueue('get_run_status', { runId }) as Promise<WorkflowRun | null>;
  }
  async sessionCreate(provider: string, model: string): Promise<string> {
    return this.enqueue('session_create', { provider, model }) as Promise<string>;
  }
  async sessionRestore(sessionId: string): Promise<SessionHandle> {
    return this.enqueue('session_restore', { sessionId }) as Promise<SessionHandle>;
  }
  async sessionGetEvents(sessionId: string): Promise<AgentEvent[]> {
    return this.enqueue('session_get_events', { sessionId }) as Promise<AgentEvent[]>;
  }
  async sessionList(): Promise<SessionSummary[]> {
    return this.enqueue('session_list', {}) as Promise<SessionSummary[]>;
  }
  async getAppState(): Promise<AppState> {
    return this.enqueue('get_app_state', {}) as Promise<AppState>;
  }
  async saveAppState(state: AppState): Promise<void> {
    return this.enqueue('save_app_state', { state }) as Promise<void>;
  }
  async getProjectState(projectId: string): Promise<ProjectState> {
    return this.enqueue('get_project_state', { projectId }) as Promise<ProjectState>;
  }
  async saveProjectState(projectId: string, state: ProjectState): Promise<void> {
    return this.enqueue('save_project_state', { projectId, state }) as Promise<void>;
  }
  async getSetting(key: string): Promise<unknown> {
    return this.enqueue('get_setting', { key });
  }
  async analyticsDaily(provider?: string, days?: number): Promise<DailyStats[]> {
    return this.enqueue('analytics_daily', { provider, days }) as Promise<DailyStats[]>;
  }
  async analyticsCompare(from?: number, to?: number): Promise<ProviderAnalytics[]> {
    return this.enqueue('analytics_compare', { from, to }) as Promise<ProviderAnalytics[]>;
  }
  async analyticsModelBreakdown(provider: string, from?: number, to?: number): Promise<ModelAnalytics[]> {
    return this.enqueue('analytics_model_breakdown', { provider, from, to }) as Promise<ModelAnalytics[]>;
  }
```

All other methods (agentSend, spawnProcess, writePty, killProcess, etc.) keep their existing `invoke()` calls unchanged.

- [ ] **Step 5: Update the Adapter interface**

In `src/lib/adapter/index.ts`, add `batch()` to the `Adapter` interface and add `signal?: AbortSignal` to navigable methods:

```typescript
  readFile(path: string, signal?: AbortSignal): Promise<string>;
  listDir(path: string, respectGitignore?: boolean, signal?: AbortSignal): Promise<FileEntry[]>;
  getGitStatus(projectRoot: string, signal?: AbortSignal): Promise<Record<string, string>>;
  // Add to the interface:
  batch<T extends unknown[]>(fn: (ctx: Adapter) => [...{ [K in keyof T]: Promise<T[K]> }]): Promise<T>;
```

- [ ] **Step 6: Run the tests**

Run: `npx vitest run src/lib/adapter/__tests__/batch.test.ts`
Expected: all tests pass

- [ ] **Step 7: Run svelte-check**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5`
Expected: no new errors

- [ ] **Step 8: Commit**

```bash
git add src/lib/adapter/tauri.ts src/lib/adapter/index.ts src/lib/adapter/__tests__/batch.test.ts
git commit -m "feat(batch): add transparent IPC batching with dedup, abort, Zod validation, and explicit batch API"
```

---

## Task 8: Update call sites for AbortSignal and explicit batching

**Files:**
- Varies by component (FileTree, project switch flows)

This task adds `AbortSignal` usage and explicit `adapter.batch()` at the critical paths identified in the spec. The exact call sites depend on how the components currently invoke the adapter. The worker should:

- [ ] **Step 1: Find file-open patterns**

Search for call sites where `adapter.readFile` and `adapter.getGitStatus` are called in sequence (FileTree.svelte, DiffBlock.svelte, etc.):

Run: `grep -rn 'adapter\.\(readFile\|getGitStatus\)' src/lib/`

- [ ] **Step 2: Add AbortController to file-open flows**

In components that load file content on user navigation (e.g., clicking a file in FileTree), add an `AbortController` that cancels the previous request when a new file is selected:

```typescript
let abortController: AbortController | null = null;

async function openFile(path: string) {
  abortController?.abort();
  abortController = new AbortController();

  try {
    const [content, git] = await adapter.batch((ctx) => [
      ctx.readFile(path, abortController!.signal),
      ctx.getGitStatus(projectRoot, abortController!.signal),
    ]);
    // update stores
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') return; // navigated away
    throw e;
  }
}
```

- [ ] **Step 3: Find project-switch patterns**

Search for call sites where project state, sessions, and app state are loaded together:

Run: `grep -rn 'getProjectState\|sessionList\|getAppState' src/lib/`

- [ ] **Step 4: Add explicit batch to project switch**

Where the project switch loads multiple pieces of state, wrap them in `adapter.batch()`:

```typescript
const [projectState, sessions, appState] = await adapter.batch((ctx) => [
  ctx.getProjectState(projectId),
  ctx.sessionList(),
  ctx.getAppState(),
]);
```

- [ ] **Step 5: Run full test suite**

Run: `npx svelte-kit sync && npx vitest run`
Expected: all tests pass

- [ ] **Step 6: Run svelte-check**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5`
Expected: no errors

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(batch): wire AbortController and explicit batch() at critical call sites"
```

---

## Task 9: Full validation

- [ ] **Step 1: Run the complete Rust test suite**

Run: `cd src-tauri && cargo test`
Expected: all pass

- [ ] **Step 2: Run cargo clippy**

Run: `cd src-tauri && cargo clippy -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Run the complete frontend test suite**

Run: `npx svelte-kit sync && npx vitest run`
Expected: all pass

- [ ] **Step 4: Run svelte-check**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json`
Expected: no errors

- [ ] **Step 5: Run vite build**

Run: `vite build`
Expected: builds successfully

- [ ] **Step 6: Final commit if any remaining changes**

```bash
git status
# If clean: nothing to do
# If changes remain: stage and commit with appropriate message
```
