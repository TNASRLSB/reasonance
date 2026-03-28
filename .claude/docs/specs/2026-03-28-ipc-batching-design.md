# W1.11 — IPC Batching Design

**Date**: 2026-03-28
**Status**: Approved
**Depends on**: W1.1 (EventBus), W1.3 (Weak refs), W1.7 (Zod)
**Blocks**: Nothing (W1.10 dead code cleanup is last)

## Problem

The frontend makes multiple sequential Tauri `invoke()` calls for common operations — file open (3 calls), project switch (3 calls), workflow control (2 calls). Each is a separate IPC roundtrip (~2.5ms each). This creates perceptible micro-latency and partial-render frames (e.g., file content appears before git status).

## Solution

A hybrid batching layer: **transparent microtask batching** (automatic, covers all calls in the same tick) plus an **explicit batch API** for critical paths where grouping must be guaranteed.

Both converge on a single Rust `batch_invoke` command that executes calls in parallel via `tokio`.

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Intra-batch execution | All parallel | Microtask boundary naturally prevents dependent calls from being batched (`await` breaks the tick) |
| Zod validation | Included in W1.11 | Marginal cost to add during batch result mapping; avoids reopening the path later |
| Single-call fallback | No fallback — always goes through `batch_invoke` | Single code path: one validation point, one logging point, one test suite |
| Dispatch scope | ~20 high-frequency commands at launch | Long-running commands (`agent_send`, `call_llm_api`, `spawn_process`) excluded by design |
| Cancellation | AbortController per-call | Prevents stale content flash when user navigates faster than IPC roundtrip |
| Deduplication | Intra-batch, by `(command, args)` key | Multiple components requesting same data in one tick share a single IPC call |
| Timeout | Per-call, 5s default, Rust-side | A hung command returns `ReasonanceError::Timeout`, doesn't block other calls in batch |

## Architecture

### Rust: `batch_invoke` command

**File**: `src-tauri/src/commands/batch.rs`

```rust
#[derive(Deserialize)]
pub struct BatchCall {
    command: String,
    args: serde_json::Value,
}

#[derive(Serialize)]
pub struct BatchCallResult {
    ok: Option<serde_json::Value>,
    err: Option<ReasonanceError>,
}

#[tauri::command]
pub async fn batch_invoke(
    calls: Vec<BatchCall>,
    app: AppHandle,
) -> Vec<BatchCallResult>
```

The command **never fails atomically** — it always returns `Vec<BatchCallResult>`, one per input call. Each call succeeds or fails independently.

### Rust: Dispatch via `_inner` pattern

Commands are refactored into outer wrapper + inner function:

```rust
// Inner: pure logic, plain types, reusable
pub fn read_file_inner(path: &str, root: &ProjectRootState) -> Result<String, ReasonanceError> { ... }

// Outer: Tauri wrapper, extracts State, calls inner
#[tauri::command]
pub fn read_file(path: String, state: State<'_, ProjectRootState>) -> Result<String, ReasonanceError> {
    read_file_inner(&path, &state)
}
```

The dispatcher uses `app.state::<T>()` to access managed state and calls `_inner` functions:

```rust
async fn dispatch(app: &AppHandle, cmd: &str, args: Value) -> Result<Value, ReasonanceError> {
    match cmd {
        "read_file" => {
            let path: String = extract(&args, "path")?;
            let state = app.state::<ProjectRootState>();
            Ok(to_value(fs::read_file_inner(&path, &state))?)
        }
        // ~20 commands at launch
        other => Err(ReasonanceError::validation("command", format!("not batchable: {other}")))
    }
}
```

Parallel execution via `futures::future::join_all`, with per-call timeout (5s default):

```rust
const BATCH_CALL_TIMEOUT: Duration = Duration::from_secs(5);

let futures = calls.into_iter().map(|c| {
    let app = app.clone();
    async move {
        match tokio::time::timeout(BATCH_CALL_TIMEOUT, dispatch(&app, &c.command, c.args)).await {
            Ok(result) => result,
            Err(_) => Err(ReasonanceError::timeout(&c.command, BATCH_CALL_TIMEOUT.as_millis() as u64)),
        }
    }
});
let results = futures::future::join_all(futures).await;
```

A timed-out call returns `ReasonanceError::Timeout` for that slot only — other calls in the batch are unaffected since they run in parallel.

### Rust: Batchable commands (initial set)

**File ops**: `read_file`, `write_file`, `list_dir`, `grep_files`, `get_git_status`
**Session**: `session_create`, `session_list`, `session_get_events`, `session_restore`
**App state**: `get_app_state`, `get_project_state`, `save_app_state`, `save_project_state`
**Workflow**: `get_run_status`, `load_workflow`, `list_workflows`
**Analytics**: `analytics_daily`, `analytics_compare`, `analytics_model_breakdown`
**Other**: `store_shadow`, `get_shadow`, `read_config`, `get_setting`

Commands **excluded** (long-running or streaming): `agent_send`, `call_llm_api`, `spawn_process`, `write_pty`, `resize_pty`, `kill_process`.

### Frontend: Transparent batching in TauriAdapter

**File**: `src/lib/adapter/tauri.ts`

```typescript
interface PendingCall {
  command: string;
  args: Record<string, unknown>;
  resolve: (value: unknown) => void;
  reject: (error: unknown) => void;
  signal?: AbortSignal;
}

class TauriAdapter implements Adapter {
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

    // Deduplicate: group by (command, args) key
    const keyMap = new Map<string, { call: PendingCall; subscribers: PendingCall[] }>();
    for (const entry of batch) {
      const key = `${entry.command}::${JSON.stringify(entry.args)}`;
      const existing = keyMap.get(key);
      if (existing) {
        existing.subscribers.push(entry);
      } else {
        keyMap.set(key, { call: entry, subscribers: [entry] });
      }
    }
    const groups = [...keyMap.values()];

    const t0 = performance.now();
    const results = await invoke<BatchCallResult[]>('batch_invoke', {
      calls: groups.map(g => ({ command: g.call.command, args: g.call.args })),
    });
    const elapsed = performance.now() - t0;

    if (import.meta.env.DEV) {
      console.debug(
        `[batch] ${groups.length} calls (${batch.length} deduped) in ${elapsed.toFixed(1)}ms`,
        groups.map(g => g.call.command),
      );
    }

    for (let i = 0; i < groups.length; i++) {
      const r = results[i];
      const { subscribers } = groups[i];

      for (const sub of subscribers) {
        // Skip aborted calls — promise already rejected by abort listener
        if (sub.signal?.aborted) continue;

        if (r.err) {
          sub.reject(r.err);
        } else {
          const schema = batchSchemas[sub.command];
          if (schema) {
            const parsed = schema.safeParse(r.ok);
            if (!parsed.success) {
              console.error(`[batch] Zod failed for ${sub.command}:`, parsed.error);
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
}
```

Adapter methods migrate from `invoke()` to `this.enqueue()`. Methods that represent user-navigable state (file reads, project loads) accept an optional `AbortSignal`:

```typescript
// Batchable commands → enqueue, with optional cancellation
async readFile(path: string, signal?: AbortSignal): Promise<string> {
  return this.enqueue('read_file', { path }, signal) as Promise<string>;
}

async getGitStatus(path: string, signal?: AbortSignal): Promise<GitStatus> {
  return this.enqueue('get_git_status', { path }, signal) as Promise<GitStatus>;
}

// Long-running commands → direct invoke (unchanged)
async agentSend(request: AgentRequest): Promise<string> {
  return invoke<string>('agent_send', { request });
}
```

### Frontend: Explicit batch API

For critical paths where grouping must be guaranteed. `BatchContext` is a type alias for the adapter itself — the callback receives `this`, so all adapter methods are available:

```typescript
type BatchContext = TauriAdapter;

async batch<T extends unknown[]>(
  fn: (ctx: BatchContext) => [...{ [K in keyof T]: Promise<T[K]> }]
): Promise<T> {
  const saved = this.queue;
  this.queue = [];
  const promises = fn(this as unknown as BatchContext);
  const batch = this.queue;
  this.queue = saved;

  const results = await invoke<BatchCallResult[]>('batch_invoke', {
    calls: batch.map(c => ({ command: c.command, args: c.args })),
  });

  for (let i = 0; i < batch.length; i++) {
    const r = results[i];
    if (r.err) batch[i].reject(r.err);
    else {
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
```

Usage at call sites:

```typescript
// Project switch — guaranteed single roundtrip
const [state, sessions, app] = await adapter.batch((ctx) => [
  ctx.getProjectState(id),
  ctx.listSessions(id),
  ctx.getAppState(),
]);

// File open — guaranteed single roundtrip
const [content, git] = await adapter.batch((ctx) => [
  ctx.readFile(path),
  ctx.getGitStatus(path),
]);
```

### Frontend: Zod schema registry

```typescript
// src/lib/adapter/batch-schemas.ts
import { z } from 'zod';

export const batchSchemas: Record<string, z.ZodType> = {
  read_file: z.string(),
  get_git_status: GitStatusSchema,
  list_dir: z.array(DirEntrySchema),
  session_list: z.array(SessionSchema),
  get_app_state: AppStateSchema,
  get_project_state: ProjectStateSchema,
  get_run_status: RunStatusSchema,
  analytics_daily: AnalyticsDailySchema,
  // ... one entry per batchable command
};
```

## Error Handling

Three error levels per call in a batch:

1. **Rust error** → `BatchCallResult.err` present → promise rejected with `ReasonanceError`
2. **Rust success, Zod failure** → value arrives but fails schema → promise rejected with `ZodError`
3. **Full success** → promise resolved with validated value

The batch itself never fails atomically. The only transport-level failure is malformed JSON in the `Vec<BatchCall>` payload, which indicates a code bug, not a runtime condition.

## Observability

### Rust: EventBus telemetry

After each batch execution, publish to EventBus:

```rust
event_bus.publish(Event {
    channel: "ipc:batch_executed".into(),
    payload: json!({
        "batch_size": calls.len(),
        "duration_ms": elapsed.as_millis(),
        "commands": call_names,
        "errors": error_count,
    }),
    ..
});
```

Integrates with W1.8 perf baselines — the baseline recorder already subscribes to EventBus channels.

### Frontend: Dev console logging

In DEV mode, each flush logs batch size, elapsed time, and command names. Production builds have zero logging overhead.

## Testing

### Rust tests

- **Dispatcher unit tests**: each supported command with valid args → success; with invalid args → `ReasonanceError::Validation`
- **Unknown command**: dispatcher returns `Validation` error, not panic
- **Parallel execution**: N calls with `tokio::time::sleep(100ms)` each — total time must be ~100ms, not N×100ms
- **Partial failure**: batch of 3 where one fails — other 2 succeed with correct values
- **Timeout**: call with `tokio::time::sleep(10s)` against 5s timeout → `ReasonanceError::Timeout`, other calls in batch unaffected

### Frontend tests (Vitest)

- **Microtask batching**: two `enqueue` calls in same tick → single `invoke('batch_invoke')` call
- **Explicit batch API**: `adapter.batch()` forces grouping, doesn't interfere with transparent queue
- **Zod rejection**: mock result that fails schema → promise rejected with `ZodError`
- **Partial error**: one `err` in batch results → only that promise rejected, others resolve
- **Long-running exclusion**: `agentSend` still calls `invoke()` directly, not `enqueue()`
- **Deduplication**: two `readFile('/same')` in same tick → single IPC call, both promises resolve with same value
- **Abort before flush**: enqueue with pre-aborted signal → immediate reject, call not sent in batch
- **Abort during flight**: enqueue, abort signal after flush starts → promise rejected with `AbortError`, result discarded

## Migration Strategy

Four phases, each independently committable and deployable:

### Phase 1 — Rust infrastructure
- Create `commands/batch.rs` with `batch_invoke` + `dispatch()`
- Refactor ~20 target commands to `_inner` pattern
- Register `batch_invoke` in `generate_handler![]`
- Rust unit tests

### Phase 2 — Frontend layer
- Add `enqueue()`, `flush()`, `batch()` to `TauriAdapter`
- Create `batch-schemas.ts` with Zod registry
- Vitest unit tests

### Phase 3 — Wiring
- Migrate batchable adapter methods from `invoke()` to `this.enqueue()`
- Long-running commands stay on `invoke()` direct
- Add explicit `adapter.batch()` at critical call sites (file open, project switch)

### Phase 4 — Validation
- Verify roundtrip reduction on target patterns (3→1, 2→1)
- Dev logging, EventBus metrics
- No regression on single-call latency

## Exit Criteria

- `batch_invoke` operational with ~20 commands dispatched
- File open: 3→1 roundtrips
- Project switch: 3→1 roundtrips
- No single-call regression (single code path, ~0.1ms overhead max)
- Zod validation per-call within batch results
- Partial failure isolation (one call fails, others succeed)
- EventBus telemetry publishing batch metrics
- Intra-batch deduplication: duplicate `(command, args)` pairs send one IPC call
- AbortController support: aborted calls rejected without applying stale state
- Per-call timeout (5s): hung commands return `Timeout` error, don't block batch
