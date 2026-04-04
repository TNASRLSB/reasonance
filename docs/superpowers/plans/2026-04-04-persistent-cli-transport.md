# Persistent CLI Transport Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace spawn-per-message CLI transport with a persistent process per session for Claude, enabling image context continuity and prompt caching.

**Architecture:** Add `persistent_sessions` map to `StructuredAgentTransport`. When `transport_mode = "persistent"`, spawn CLI once with `--input-format stream-json` and keep stdin open. Messages are newline-delimited JSON. The stream reader runs continuously, emitting Done on `type:"result"` events instead of EOF.

**Tech Stack:** Rust/Tokio async, serde_json, existing normalizer pipeline

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `src-tauri/src/normalizer/mod.rs` | Modify | Add `transport_mode`, `persistent_args` to CliConfig |
| `src-tauri/normalizers/claude.toml` | Modify | Add `transport_mode = "persistent"` |
| `src-tauri/src/transport/persistent.rs` | Create | PersistentSession struct + message building |
| `src-tauri/src/transport/mod.rs` | Modify | Add persistent_sessions field, persistent path in send(), cleanup in stop() |
| `src-tauri/src/transport/stream_reader.rs` | Modify | Emit Done on result events, not just EOF |

---

### Task 1: Add transport_mode to normalizer CliConfig

**Files:**
- Modify: `src-tauri/src/normalizer/mod.rs`
- Modify: `src-tauri/normalizers/claude.toml`

- [ ] **Step 1: Write failing test**

In `src-tauri/src/normalizer/mod.rs`, add to the existing `mod tests` block:

```rust
    #[test]
    fn cli_config_parses_transport_mode() {
        let toml_str = r#"
[cli]
name = "test"
binary = "test-cli"
transport_mode = "persistent"
persistent_args = ["--input-format", "stream-json", "--output-format", "stream-json", "--verbose"]
"#;
        let config: TomlConfig = TomlConfig::parse(toml_str).unwrap();
        assert_eq!(config.cli.transport_mode.as_deref(), Some("persistent"));
        assert_eq!(config.cli.persistent_args.len(), 6);
    }

    #[test]
    fn cli_config_defaults_transport_mode_to_none() {
        let toml_str = r#"
[cli]
name = "test"
binary = "test-cli"
"#;
        let config: TomlConfig = TomlConfig::parse(toml_str).unwrap();
        assert!(config.cli.transport_mode.is_none());
        assert!(config.cli.persistent_args.is_empty());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test cli_config_parses_transport_mode -- --nocapture`
Expected: FAIL — `transport_mode` field doesn't exist.

- [ ] **Step 3: Add fields to CliConfig**

In `src-tauri/src/normalizer/mod.rs`, add to `CliConfig` after the `image_args_template` field:

```rust
    /// Transport mode: "persistent" = keep CLI process alive for the session,
    /// "spawn" or absent = spawn a new process per message (default).
    #[serde(default)]
    pub transport_mode: Option<String>,
    /// CLI args for persistent mode (replaces programmatic_args).
    #[serde(default)]
    pub persistent_args: Vec<String>,
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test cli_config_parses_transport -- --nocapture`
Expected: 2 tests PASS.

- [ ] **Step 5: Update claude.toml**

In `src-tauri/normalizers/claude.toml`, add to the `[cli]` section:

```toml
transport_mode = "persistent"
persistent_args = ["--output-format", "stream-json", "--verbose", "--input-format", "stream-json"]
```

- [ ] **Step 6: Run normalizer tests**

Run: `cd src-tauri && cargo test normalizer -- --nocapture`
Expected: All pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/normalizer/mod.rs src-tauri/normalizers/claude.toml
git commit -m "feat(normalizer): add transport_mode and persistent_args to CliConfig"
```

---

### Task 2: Create PersistentSession module

**Files:**
- Create: `src-tauri/src/transport/persistent.rs`
- Modify: `src-tauri/src/transport/mod.rs` (add `pub mod persistent;`)

- [ ] **Step 1: Create the module with struct and message builder**

Create `src-tauri/src/transport/persistent.rs`:

```rust
use crate::transport::request::ImageAttachment;
use log::{error, info};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::Mutex as TokioMutex;

/// A persistent CLI session that stays alive across multiple messages.
/// The CLI process is spawned with --input-format stream-json and
/// messages are written as newline-delimited JSON to stdin.
pub struct PersistentSession {
    /// Async stdin handle (wrapped in TokioMutex for Send + shared access)
    stdin: Arc<TokioMutex<ChildStdin>>,
    /// Provider name
    pub provider: String,
}

impl PersistentSession {
    pub fn new(stdin: ChildStdin, provider: String) -> Self {
        Self {
            stdin: Arc::new(TokioMutex::new(stdin)),
            provider,
        }
    }

    /// Write a user message to the CLI's stdin.
    /// The message is JSON-serialized and terminated with a newline.
    pub async fn write_message(
        &self,
        prompt: &str,
        images: &[ImageAttachment],
    ) -> Result<(), std::io::Error> {
        let msg = build_user_message(prompt, images);
        let msg_str = serde_json::to_string(&msg).unwrap_or_default();
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(msg_str.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        info!(
            "PersistentSession[{}]: wrote message ({} bytes, {} images)",
            self.provider,
            msg_str.len(),
            images.len()
        );
        Ok(())
    }

    /// Close stdin, signaling the CLI to exit.
    pub async fn close(self) {
        let mut stdin = self.stdin.lock().await;
        let _ = stdin.shutdown().await;
        info!("PersistentSession[{}]: stdin closed", self.provider);
    }
}

/// Build the VS Code extension-compatible stream-json user message.
pub fn build_user_message(
    prompt: &str,
    images: &[ImageAttachment],
) -> serde_json::Value {
    let mut content = vec![serde_json::json!({
        "type": "text",
        "text": prompt,
    })];
    for img in images {
        content.push(serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": img.mime_type,
                "data": img.data,
            }
        }));
    }
    serde_json::json!({
        "type": "user",
        "uuid": uuid::Uuid::new_v4().to_string(),
        "session_id": "",
        "message": {
            "role": "user",
            "content": content,
        },
        "parent_tool_use_id": null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_user_message_text_only() {
        let msg = build_user_message("hello", &[]);
        assert_eq!(msg["type"], "user");
        assert_eq!(msg["message"]["role"], "user");
        let content = msg["message"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "hello");
    }

    #[test]
    fn build_user_message_with_image() {
        let images = vec![ImageAttachment {
            data: "abc123".to_string(),
            mime_type: "image/png".to_string(),
            name: "test.png".to_string(),
        }];
        let msg = build_user_message("describe", &images);
        let content = msg["message"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[1]["type"], "image");
        assert_eq!(content[1]["source"]["media_type"], "image/png");
        assert_eq!(content[1]["source"]["data"], "abc123");
    }

    #[test]
    fn build_user_message_has_uuid() {
        let msg = build_user_message("hi", &[]);
        assert!(msg["uuid"].is_string());
        assert!(!msg["uuid"].as_str().unwrap().is_empty());
    }
}
```

- [ ] **Step 2: Register module**

In `src-tauri/src/transport/mod.rs`, add after `pub mod direct_api;`:

```rust
pub mod persistent;
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test persistent::tests -- --nocapture`
Expected: 3 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/transport/persistent.rs src-tauri/src/transport/mod.rs
git commit -m "feat(transport): add PersistentSession module with message builder"
```

---

### Task 3: Modify stream reader to emit Done on result events

**Files:**
- Modify: `src-tauri/src/transport/stream_reader.rs`

This is the critical change: the stream reader must emit a Done event when the normalizer produces a Usage event (which comes from `type:"result"` lines), not only when stdout closes. This allows the persistent process to signal "response complete" without exiting.

- [ ] **Step 1: Write failing test**

Add to the existing test module in `stream_reader.rs`:

```rust
    #[test]
    fn stream_reader_emits_done_after_usage_event() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let bus = Arc::new(crate::event_bus::EventBus::new(
                tokio::runtime::Handle::current(),
            ));
            bus.register_channel("transport:event", true);
            bus.register_channel("transport:complete", true);

            let recorder = Arc::new(crate::subscribers::history::HistoryRecorder::new());
            bus.subscribe("transport:event", recorder.clone());
            bus.subscribe("transport:complete", recorder.clone());

            // Simulate CLI output with a result line (which produces Usage)
            let lines = vec![
                r#"{"type":"assistant","message":{"model":"test","content":[{"type":"text","text":"hello"}]}}"#,
                r#"{"type":"result","subtype":"success","usage":{"input_tokens":10,"output_tokens":5},"session_id":"s1"}"#,
            ];
            let combined = lines.join("\n") + "\n";
            let cursor = std::io::Cursor::new(combined.into_bytes());
            let stdout = tokio::process::ChildStdout::from_std(
                unsafe { std::os::unix::io::FromRawFd::from_raw_fd(-1) }
            );
            // We can't easily create a ChildStdout from bytes in a unit test.
            // This test is better done as an integration test — skip for now.
        });
    }
```

Actually, testing the stream reader directly requires a ChildStdout which can't be easily mocked. Instead, modify the stream_reader to accept a flag for "emit done on usage" and test the pipeline change.

- [ ] **Step 1 (revised): Modify stream reader to emit Done after Usage events**

In `src-tauri/src/transport/stream_reader.rs`, in the main loop where events are published (around line 90-102), add logic to emit a Done event after publishing a Usage event:

Find this block:
```rust
                    for event in &events {
                        events_count += 1;
                        log::debug!(
                            "StreamReader[{}]: emitting {:?}",
                            session_id,
                            event.event_type
                        );
                        event_bus.publish(crate::event_bus::Event::from_agent_event(
                            "transport:event",
                            &session_id,
                            event,
                        ));
                    }
```

Replace with:
```rust
                    for event in &events {
                        events_count += 1;
                        log::debug!(
                            "StreamReader[{}]: emitting {:?}",
                            session_id,
                            event.event_type
                        );
                        event_bus.publish(crate::event_bus::Event::from_agent_event(
                            "transport:event",
                            &session_id,
                            event,
                        ));
                    }

                    // If any event was Usage (from a result line), emit Done.
                    // This signals "response complete" for persistent sessions
                    // where stdout doesn't close between messages.
                    let has_usage = events
                        .iter()
                        .any(|e| e.event_type == crate::agent_event::AgentEventType::Usage);
                    if has_usage {
                        let done = crate::agent_event::AgentEvent::done(&session_id, "system");
                        events_count += 1;
                        log::debug!(
                            "StreamReader[{}]: emitting Done (after Usage)",
                            session_id
                        );
                        event_bus.publish(crate::event_bus::Event::from_agent_event(
                            "transport:complete",
                            &session_id,
                            &done,
                        ));
                    }
```

- [ ] **Step 2: Verify existing tests still pass**

Run: `cd src-tauri && cargo test stream_reader -- --nocapture`
Expected: All existing tests pass. The change is additive — the EOF Done still works for spawn-per-message sessions.

- [ ] **Step 3: Run all transport tests**

Run: `cd src-tauri && cargo test transport -- --nocapture`
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/transport/stream_reader.rs
git commit -m "feat(stream_reader): emit Done after Usage events for persistent sessions"
```

---

### Task 4: Add persistent_sessions map to Transport and implement persistent path

**Files:**
- Modify: `src-tauri/src/transport/mod.rs`

This is the main task. Add the `persistent_sessions` field and the persistent code path in `send()`.

- [ ] **Step 1: Add persistent_sessions field to StructuredAgentTransport**

In the struct definition (around line 31), add:

```rust
    /// Persistent CLI sessions keyed by Reasonance session ID.
    /// Only used for providers with transport_mode = "persistent".
    persistent_sessions: Arc<std::sync::Mutex<HashMap<String, persistent::PersistentSession>>>,
```

Update `empty()` and `new()` to initialize it:

```rust
persistent_sessions: Arc::new(std::sync::Mutex::new(HashMap::new())),
```

- [ ] **Step 2: Capture transport_mode before registry drop**

In `send()`, alongside the existing pre-drop captures (image_mode, image_arg, etc.), add:

```rust
        let transport_mode = config.cli.transport_mode.clone();
        let persistent_args = config.cli.persistent_args.clone();
```

- [ ] **Step 3: Add persistent path at the TOP of send(), before image routing**

After session registration and before the image routing block, add:

```rust
        // ── Persistent session path ──────────────────────────────
        if transport_mode.as_deref() == Some("persistent") {
            // Check for existing persistent session
            {
                let persistent = self.persistent_sessions.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(session) = persistent.get(&session_id) {
                    info!("Transport[persistent]: writing to existing session={}", session_id);
                    let sess = session.clone_stdin();
                    let prompt = request.prompt.clone();
                    let images = request.images.clone();
                    let provider_name = session.provider.clone();
                    drop(persistent); // release lock before async work
                    tokio::spawn(async move {
                        if let Err(e) = persistent::PersistentSession::write_to(
                            &sess, &prompt, &images,
                        ).await {
                            log::error!(
                                "Transport[persistent]: write failed for session={}: {}",
                                provider_name, e
                            );
                        }
                    });
                    return Ok(session_id);
                }
            }

            // No existing session — spawn new persistent process
            info!(
                "Transport[persistent]: spawning new {} for session={}",
                binary, session_id
            );

            let mut cli_args = persistent_args.clone();
            cli_args.extend(permission_args.iter().cloned());
            cli_args.extend(allowed_tools_args.iter().cloned());
            if let Some(ref cwd) = request.cwd {
                if !cwd.is_empty() {
                    info!("Transport[persistent]: cwd={}", cwd);
                }
            }

            let mut cmd = Command::new(&binary);
            cmd.args(&cli_args);
            cmd.stdin(Stdio::piped());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
            if let Some(ref cwd) = request.cwd {
                if !cwd.is_empty() {
                    cmd.current_dir(cwd);
                }
            }

            match cmd.spawn() {
                Ok(mut child) => {
                    let stdin = child.stdin.take().expect("stdin must be piped");
                    let persistent_session =
                        persistent::PersistentSession::new(stdin, provider.clone());

                    // Build normalizer pipeline
                    let sm: Box<dyn crate::normalizer::state_machines::StateMachine> =
                        match provider.as_str() {
                            "claude" => Box::new(
                                crate::normalizer::state_machines::claude::ClaudeStateMachine::new(),
                            ),
                            "gemini" => Box::new(
                                crate::normalizer::state_machines::gemini::GeminiStateMachine::new(),
                            ),
                            _ => Box::new(
                                crate::normalizer::state_machines::generic::GenericStateMachine::new(),
                            ),
                        };
                    let pl = Arc::new(Mutex::new(
                        crate::normalizer::pipeline::NormalizerPipeline::new(
                            rules.clone(),
                            sm,
                            provider.clone(),
                        ),
                    ));

                    // Start stream reader (runs for entire session lifetime)
                    if let Some(stdout) = child.stdout.take() {
                        let bus = self
                            .event_bus
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .clone()
                            .expect("EventBus must be set");
                        let sid_clone = session_id.clone();
                        spawn_stream_reader(
                            stdout,
                            pl,
                            bus,
                            sid_clone,
                            session_id_path.clone(),
                            cli_sid_arc.clone(),
                            None,
                        );
                    }

                    // Write the first message
                    let prompt = request.prompt.clone();
                    let images = request.images.clone();
                    let stdin_ref = persistent_session.clone_stdin();
                    tokio::spawn(async move {
                        if let Err(e) =
                            persistent::PersistentSession::write_to(&stdin_ref, &prompt, &images)
                                .await
                        {
                            log::error!("Transport[persistent]: first write failed: {}", e);
                        }
                    });

                    // Store the persistent session
                    self.persistent_sessions
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(session_id.clone(), persistent_session);

                    // Wait for child exit in background (cleanup on crash)
                    let persistent_sessions_ref = self.persistent_sessions.clone();
                    let sid_for_cleanup = session_id.clone();
                    tokio::spawn(async move {
                        let status = child.wait().await;
                        info!(
                            "Transport[persistent]: process exited for session={}: {:?}",
                            sid_for_cleanup, status
                        );
                        persistent_sessions_ref
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .remove(&sid_for_cleanup);
                    });

                    if let Some(pid) = child.id() {
                        info!("Transport[persistent]: child pid={}", pid);
                    }
                }
                Err(e) => {
                    error!("Transport[persistent]: spawn failed: {}", e);
                    let err_event = AgentEvent::error(
                        &format!("Failed to start {}: {}", binary, e),
                        "PERSISTENT_SPAWN_ERROR",
                        ErrorSeverity::Fatal,
                        &provider,
                    );
                    if let Some(bus) = self
                        .event_bus
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .as_ref()
                    {
                        bus.publish(crate::event_bus::Event::from_agent_event(
                            "transport:error",
                            &session_id,
                            &err_event,
                        ));
                        let done = AgentEvent::done(&session_id, &provider);
                        bus.publish(crate::event_bus::Event::from_agent_event(
                            "transport:complete",
                            &session_id,
                            &done,
                        ));
                    }
                }
            }
            return Ok(session_id);
        }

        // ── Image routing (existing, for non-persistent providers) ──
```

- [ ] **Step 4: Update stop() to clean up persistent sessions**

In the `stop()` method (around line 870), add persistent session cleanup before the existing session handling:

```rust
    pub fn stop(&self, session_id: &str) -> Result<(), crate::error::ReasonanceError> {
        info!("Transport: stopping session={}", session_id);

        // Close persistent session if one exists (drops stdin → CLI exits)
        if let Some(persistent) = self
            .persistent_sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(session_id)
        {
            info!("Transport: closing persistent session={}", session_id);
            let stdin = persistent.clone_stdin();
            tokio::spawn(async move {
                let mut s = stdin.lock().await;
                let _ = s.shutdown().await;
            });
        }

        // existing session cleanup...
```

- [ ] **Step 5: Add clone_stdin and write_to to PersistentSession**

In `persistent.rs`, add these methods:

```rust
    /// Get a clone of the stdin Arc for async writes from other contexts.
    pub fn clone_stdin(&self) -> Arc<TokioMutex<ChildStdin>> {
        self.stdin.clone()
    }

    /// Write a message using a shared stdin reference.
    pub async fn write_to(
        stdin: &Arc<TokioMutex<ChildStdin>>,
        prompt: &str,
        images: &[ImageAttachment],
    ) -> Result<(), std::io::Error> {
        let msg = build_user_message(prompt, images);
        let msg_str = serde_json::to_string(&msg).unwrap_or_default();
        let mut s = stdin.lock().await;
        s.write_all(msg_str.as_bytes()).await?;
        s.write_all(b"\n").await?;
        s.flush().await?;
        log::info!(
            "PersistentSession: wrote message ({} bytes, {} images)",
            msg_str.len(),
            images.len()
        );
        Ok(())
    }
```

- [ ] **Step 6: Run cargo check**

Run: `cd src-tauri && cargo check`
Expected: Compiles. There may be warnings about unused variables.

- [ ] **Step 7: Run all transport tests**

Run: `cd src-tauri && cargo test transport -- --nocapture`
Expected: All existing tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/transport/mod.rs src-tauri/src/transport/persistent.rs
git commit -m "feat(transport): add persistent CLI session path for Claude"
```

---

### Task 5: Full verification

**Files:** None (verification only)

- [ ] **Step 1: Run all Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All 630+ tests pass.

- [ ] **Step 2: Run clippy**

Run: `cd src-tauri && cargo clippy`
Expected: No errors.

- [ ] **Step 3: Run frontend checks**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json && npx vitest run`
Expected: 0 errors, all tests pass.

- [ ] **Step 4: Commit any fixups**

- [ ] **Step 5: Manual test — launch app and test image + follow-up**

Run: `npx tauri dev`

Test flow:
1. Open chat, send text message → verify response appears
2. Attach image, send with "describe this image" → verify image-aware response
3. Follow-up: "what was in the image I sent?" → verify model remembers the image
4. Check logs: `grep "persistent" ~/.local/share/com.reasonance.app/logs/Reasonance.log`

Expected: Session uses persistent process, follow-ups preserve image context.
