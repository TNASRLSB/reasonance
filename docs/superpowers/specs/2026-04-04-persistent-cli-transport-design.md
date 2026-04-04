# Persistent CLI Transport

**Date:** 2026-04-04
**Status:** Draft

## Problem

Reasonance spawns a new CLI process for every chat message. This causes:
1. **Image context lost on follow-ups** — each process has its own conversation, `--resume` doesn't reliably carry images
2. **No prompt caching** — the ~20KB system prompt is sent fresh every time, degrading model performance on images
3. **Slow responses** — CLI startup + hook loading adds 2-3s per message
4. **Architectural divergence** — the VS Code extension uses a persistent process; Reasonance's spawn-per-message approach is fundamentally different

## Solution

Replace the spawn-per-message transport with a persistent CLI process per chat session. The process is spawned once when the user sends the first message and stays alive for the entire session. All messages (text and images) are written as newline-delimited JSON to stdin. Responses are read from stdout through the existing normalizer pipeline.

## Architecture

### Data Flow

```
ChatInput → adapter.agentSend(text, images) → invoke('agent_send')
  → Transport::send()
      → PersistentSession exists for this session_id?
          YES → write message JSON to session's stdin
          NO  → spawn CLI process with stream-json args
              → store stdin handle + child in PersistentSession
              → start stream reader on stdout
              → write message JSON to stdin
      → Stream reader emits AgentEvents through EventBus
      → Frontend receives events via listen('agent-event')
```

### New Struct: PersistentSession

```rust
pub struct PersistentSession {
    /// Tokio stdin handle for writing messages
    stdin: tokio::process::ChildStdin,
    /// CLI session ID captured from the first system/init event
    cli_session_id: Option<String>,
    /// Provider name (for logging and config lookup)
    provider: String,
    /// Whether the process is currently processing a message
    busy: bool,
}
```

Stored in a new field on `StructuredAgentTransport`:
```rust
persistent_sessions: Arc<Mutex<HashMap<String, PersistentSession>>>,
```

Keyed by the Reasonance session ID (the UUID from the frontend).

### Message Format

All messages use the VS Code extension's stream-json format:

```json
{"type":"user","uuid":"<random-uuid>","session_id":"","message":{"role":"user","content":[{"type":"text","text":"hello"}]},"parent_tool_use_id":null}
```

For images:
```json
{"type":"user","uuid":"<random-uuid>","session_id":"","message":{"role":"user","content":[{"type":"text","text":"describe this"},{"type":"image","source":{"type":"base64","media_type":"image/png","data":"..."}}]},"parent_tool_use_id":null}
```

Written as a single line + `\n` to stdin.

### CLI Spawn Args

```
claude --output-format stream-json --verbose --input-format stream-json [permission_args] [allowed_tools_args]
```

No `-p` flag (persistent mode, not print mode). No `--resume` (session lives in the process). The `--session-id` flag can optionally be passed to set a specific CLI session ID.

### Stream Reader Changes

The existing `spawn_stream_reader` reads stdout line by line and emits events. Currently it emits a synthetic "Done" event when stdout closes (EOF). With a persistent process:

1. **stdout never closes** between messages (process stays alive)
2. **"Done" must be emitted when the CLI outputs a `type:"result"` event** instead of on EOF
3. The stream reader loop continues running after emitting Done, waiting for the next message's output

Change: in the normalizer pipeline, when a `result` rule matches, emit BOTH a `Usage` event AND a `Done` event. The stream reader no longer emits Done on its own — only the pipeline does.

The EOF branch (stdout closes) becomes the session cleanup path: log a warning, emit a final Done if the session was active, and remove the PersistentSession from the map.

### Transport::send() Changes

The `send()` method changes from "build args + spawn process" to:

```rust
fn send(&self, request: AgentRequest, ...) -> Result<String, ReasonanceError> {
    // ... permission checks (unchanged) ...
    // ... model slot resolution (unchanged) ...

    let session_id = request.session_id.unwrap_or_else(|| uuid());

    // Build the user message
    let user_msg = build_user_message(&request);

    // Check for existing persistent session
    let mut persistent = self.persistent_sessions.lock();
    if let Some(session) = persistent.get_mut(&session_id) {
        // Write to existing session's stdin
        session.write_message(&user_msg)?;
        return Ok(session_id);
    }

    // No persistent session — spawn new CLI process
    let (stdin, child) = self.spawn_persistent_cli(&provider, &request, ...)?;

    // Start stream reader on stdout (runs for entire session lifetime)
    spawn_stream_reader(stdout, pipeline, event_bus, session_id, ...);

    // Write the first message
    stdin.write_message(&user_msg)?;

    // Store the persistent session
    persistent.insert(session_id.clone(), PersistentSession {
        stdin,
        cli_session_id: None,
        provider,
        busy: true,
    });

    Ok(session_id)
}
```

### Session Lifecycle

1. **Create**: First message in a new session spawns the CLI process
2. **Message**: Subsequent messages write to the existing stdin
3. **Close**: When the user closes the chat tab, Reasonance calls `agent_stop(session_id)` which:
   - Drops the stdin handle (sends EOF to CLI)
   - Removes the PersistentSession from the map
   - The stream reader detects EOF and emits final Done
4. **Crash recovery**: If the CLI process exits unexpectedly (crash, error), the stream reader detects EOF, emits an Error event, and removes the PersistentSession. The next message will spawn a new process.

### Backward Compatibility

- The spawn-per-message path remains for providers that DON'T support `--input-format stream-json` (Codex, Gemini, Kimi, Qwen). Only Claude uses the persistent process.
- The normalizer TOML config determines which path to use via the existing `image_mode` field or a new `transport_mode` field:
  - `transport_mode = "persistent"` → persistent process (Claude)
  - `transport_mode = "spawn"` → spawn per message (default, all others)
- The `direct-api` and `cli-flag` image paths remain as fallbacks for non-Claude providers.

### Normalizer Config

```toml
# claude.toml
[cli]
transport_mode = "persistent"
persistent_args = ["--output-format", "stream-json", "--verbose", "--input-format", "stream-json"]
```

### What Changes vs What Stays

| Component | Changes? | Details |
|-----------|----------|---------|
| `transport/mod.rs` | YES | New persistent session path in `send()`, `agent_stop()` cleanup |
| `transport/request.rs` | NO | ImageAttachment already exists |
| `stream_reader.rs` | YES | Emit Done on `result` event, handle persistent EOF |
| `normalizer/mod.rs` | YES | Add `transport_mode`, `persistent_args` to CliConfig |
| `normalizer/pipeline.rs` | YES | Emit Done event when result rule matches |
| `claude.toml` | YES | Add transport_mode + persistent_args |
| `event_bus.rs` | NO | Unchanged |
| `ChatInput.svelte` | NO | Already handles images |
| `ChatView.svelte` | NO | Already passes images |
| `adapter/tauri.ts` | NO | Already passes images |

### Error Handling

| Scenario | Behavior |
|----------|----------|
| CLI process crashes | Stream reader detects EOF, emits Error + Done, removes PersistentSession. Next message spawns new process. |
| stdin write fails | Emit Error event in chat, remove PersistentSession, next message spawns new. |
| CLI hangs (no response) | Existing 5-min line timeout in stream reader fires, emits Error + Done. |
| User closes tab | `agent_stop()` drops stdin, process exits cleanly. |
| App shutdown | `kill_all()` in PtyManager already handles cleanup. Add similar cleanup for persistent sessions. |

### Testing

- **Unit**: PersistentSession write/read, message formatting
- **Integration**: Spawn persistent CLI, send text, verify response; send image, verify response; send follow-up, verify context preserved
- **Existing tests**: All current transport tests must still pass (spawn-per-message path unchanged for non-Claude providers)

## Files Changed

### New files
- None (PersistentSession struct added to `transport/mod.rs`)

### Modified files
- `src-tauri/src/transport/mod.rs` — PersistentSession struct, persistent path in send(), cleanup in agent_stop()
- `src-tauri/src/transport/stream_reader.rs` — Emit Done on result event, handle persistent EOF
- `src-tauri/src/normalizer/mod.rs` — Add transport_mode, persistent_args to CliConfig
- `src-tauri/normalizers/claude.toml` — Add transport_mode = "persistent"

## Future

- Extend persistent mode to other providers as they add `--input-format stream-json` support
- Multi-turn image context (re-send images in follow-ups for providers without persistent mode)
- Image compression/resize before sending (like the VS Code extension's image-processor.node)
