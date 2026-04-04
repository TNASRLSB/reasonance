# Chat Image Attachments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable users to paste, drag-drop, or pick images in chat, and send them to multimodal LLM providers.

**Architecture:** Two transport paths — Direct API (Anthropic/Google/OpenAI REST) for image messages, CLI flag (`--image`) for Codex. Frontend captures images as base64, backend routes based on normalizer `image_mode` config. Events flow through the existing EventBus so the frontend works identically for both paths.

**Tech Stack:** Svelte 5, TypeScript, Rust/Tauri, reqwest, serde, Zod

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `src-tauri/src/transport/request.rs` | Modify | Add `ImageAttachment` struct and `images` field to `AgentRequest` |
| `src-tauri/src/normalizer/mod.rs` | Modify | Add `image_mode` and `image_arg` to `CliConfig` |
| `src-tauri/src/transport/direct_api.rs` | Create | Direct API calls with multimodal content for Anthropic/Google/OpenAI |
| `src-tauri/src/transport/mod.rs` | Modify | Route image messages to direct_api or cli-flag path |
| `src-tauri/normalizers/claude.toml` | Modify | Add `image_mode = "direct-api"` |
| `src-tauri/normalizers/codex.toml` | Modify | Add `image_mode = "cli-flag"`, `image_arg = "--image"` |
| `src-tauri/normalizers/gemini.toml` | Modify | Add `image_mode = "direct-api"` |
| `src-tauri/normalizers/kimi.toml` | Modify | Add `image_mode = "none"` |
| `src-tauri/normalizers/qwen.toml` | Modify | Add `image_mode = "none"` |
| `src/lib/adapter/index.ts` | Modify | Add `ImageAttachment` type, update `agentSend` signature |
| `src/lib/adapter/tauri.ts` | Modify | Pass images in invoke call |
| `tests/mocks/adapter.ts` | Modify | Update mock `agentSend` signature |
| `src/lib/components/chat/ChatInput.svelte` | Modify | Add paste/drop/picker handlers, image state |
| `src/lib/components/chat/ImageAttachmentStrip.svelte` | Create | Thumbnail strip with accessible remove buttons |
| `src/lib/components/chat/ChatView.svelte` | Modify | Pass images through handleSend, display in user bubbles |
| `src/lib/components/chat/ChatMessages.svelte` | Modify | Render image attachments in user messages |

---

### Task 1: Rust — ImageAttachment type and AgentRequest field

**Files:**
- Modify: `src-tauri/src/transport/request.rs`

- [ ] **Step 1: Write failing test for ImageAttachment serde**

Add at the bottom of `request.rs` inside the existing `#[cfg(test)]` block (or create one if absent):

```rust
#[cfg(test)]
mod image_tests {
    use super::*;

    #[test]
    fn image_attachment_deserializes_from_json() {
        let json = r#"{"data":"iVBOR...","mime_type":"image/png","name":"screenshot.png"}"#;
        let img: ImageAttachment = serde_json::from_str(json).unwrap();
        assert_eq!(img.mime_type, "image/png");
        assert_eq!(img.name, "screenshot.png");
        assert!(img.data.starts_with("iVBOR"));
    }

    #[test]
    fn agent_request_with_images_deserializes() {
        let json = r#"{
            "prompt": "Describe this",
            "provider": "claude",
            "images": [{"data":"abc","mime_type":"image/jpeg","name":"photo.jpg"}]
        }"#;
        let req: AgentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.images.len(), 1);
        assert_eq!(req.images[0].name, "photo.jpg");
    }

    #[test]
    fn agent_request_without_images_has_empty_vec() {
        let json = r#"{"prompt":"hello","provider":"claude"}"#;
        let req: AgentRequest = serde_json::from_str(json).unwrap();
        assert!(req.images.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test image_tests -- --nocapture`
Expected: FAIL — `ImageAttachment` type doesn't exist yet.

- [ ] **Step 3: Add ImageAttachment struct and images field**

In `src-tauri/src/transport/request.rs`, add above `AgentRequest`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAttachment {
    pub data: String,
    pub mime_type: String,
    pub name: String,
}
```

Add the `images` field to `AgentRequest` after the `yolo` field:

```rust
    /// Base64-encoded images attached to this message.
    #[serde(default)]
    pub images: Vec<ImageAttachment>,
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test image_tests -- --nocapture`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/transport/request.rs
git commit -m "feat(transport): add ImageAttachment type to AgentRequest"
```

---

### Task 2: Rust — Add image_mode to normalizer CliConfig

**Files:**
- Modify: `src-tauri/src/normalizer/mod.rs`

- [ ] **Step 1: Write failing test**

Add in the existing test module in `mod.rs`:

```rust
#[test]
fn cli_config_parses_image_mode() {
    let toml_str = r#"
[cli]
name = "test"
binary = "test-cli"
image_mode = "direct-api"
image_arg = "--image"
"#;
    let config: TomlConfig = TomlConfig::parse(toml_str).unwrap();
    assert_eq!(config.cli.image_mode.as_deref(), Some("direct-api"));
    assert_eq!(config.cli.image_arg.as_deref(), Some("--image"));
}

#[test]
fn cli_config_defaults_image_mode_to_none() {
    let toml_str = r#"
[cli]
name = "test"
binary = "test-cli"
"#;
    let config: TomlConfig = TomlConfig::parse(toml_str).unwrap();
    assert!(config.cli.image_mode.is_none());
    assert!(config.cli.image_arg.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test cli_config_parses_image_mode -- --nocapture`
Expected: FAIL — `image_mode` field doesn't exist.

- [ ] **Step 3: Add fields to CliConfig**

In `src-tauri/src/normalizer/mod.rs`, add to `CliConfig` after `read_only_tools`:

```rust
    /// How this provider handles image attachments:
    /// "direct-api" = call provider REST API directly
    /// "cli-flag" = save to temp file, pass via image_arg
    /// "none" or absent = images not supported
    #[serde(default)]
    pub image_mode: Option<String>,
    /// CLI flag for passing image file paths (e.g. "--image"). Only used when image_mode = "cli-flag".
    #[serde(default)]
    pub image_arg: Option<String>,
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test cli_config_parses -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/normalizer/mod.rs
git commit -m "feat(normalizer): add image_mode and image_arg to CliConfig"
```

---

### Task 3: Rust — Direct API image transport module

**Files:**
- Create: `src-tauri/src/transport/direct_api.rs`
- Modify: `src-tauri/src/transport/mod.rs` (add `pub mod direct_api;`)

- [ ] **Step 1: Write failing test**

Create `src-tauri/src/transport/direct_api.rs` with the test at the bottom:

```rust
use crate::agent_event::AgentEvent;
use crate::error::ReasonanceError;
use crate::transport::request::ImageAttachment;
use log::{debug, error, info};
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_anthropic_body_includes_image_block() {
        let images = vec![ImageAttachment {
            data: "iVBOR".to_string(),
            mime_type: "image/png".to_string(),
            name: "test.png".to_string(),
        }];
        let body = build_anthropic_body("Describe this", "claude-sonnet-4-6", &images, 8192);
        let content = &body["messages"][0]["content"];
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Describe this");
        assert_eq!(content[1]["type"], "image");
        assert_eq!(content[1]["source"]["type"], "base64");
        assert_eq!(content[1]["source"]["media_type"], "image/png");
        assert_eq!(content[1]["source"]["data"], "iVBOR");
    }

    #[test]
    fn build_google_body_includes_inline_data() {
        let images = vec![ImageAttachment {
            data: "abc123".to_string(),
            mime_type: "image/jpeg".to_string(),
            name: "photo.jpg".to_string(),
        }];
        let body = build_google_body("What is this?", "gemini-2.5-pro", &images);
        let parts = &body["contents"][0]["parts"];
        assert_eq!(parts[0]["text"], "What is this?");
        assert_eq!(parts[1]["inlineData"]["mimeType"], "image/jpeg");
        assert_eq!(parts[1]["inlineData"]["data"], "abc123");
    }

    #[test]
    fn build_openai_body_includes_image_url() {
        let images = vec![ImageAttachment {
            data: "xyz".to_string(),
            mime_type: "image/png".to_string(),
            name: "s.png".to_string(),
        }];
        let body = build_openai_body("Look", "gpt-4o", &images);
        let content = &body["messages"][0]["content"];
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[1]["type"], "image_url");
        assert!(content[1]["image_url"]["url"].as_str().unwrap().starts_with("data:image/png;base64,"));
    }
}
```

- [ ] **Step 2: Register module and verify test fails**

In `src-tauri/src/transport/mod.rs`, add after the existing `pub mod` lines:

```rust
pub mod direct_api;
```

Run: `cd src-tauri && cargo test direct_api::tests -- --nocapture`
Expected: FAIL — `build_anthropic_body` etc. don't exist.

- [ ] **Step 3: Implement body builders**

In `src-tauri/src/transport/direct_api.rs`, add above the test module:

```rust
pub fn build_anthropic_body(
    prompt: &str,
    model: &str,
    images: &[ImageAttachment],
    max_tokens: u64,
) -> serde_json::Value {
    let mut content = vec![serde_json::json!({"type": "text", "text": prompt})];
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
        "model": model,
        "max_tokens": max_tokens,
        "messages": [{"role": "user", "content": content}]
    })
}

pub fn build_google_body(
    prompt: &str,
    _model: &str,
    images: &[ImageAttachment],
) -> serde_json::Value {
    let mut parts = vec![serde_json::json!({"text": prompt})];
    for img in images {
        parts.push(serde_json::json!({
            "inlineData": {
                "mimeType": img.mime_type,
                "data": img.data,
            }
        }));
    }
    serde_json::json!({
        "contents": [{"parts": parts}]
    })
}

pub fn build_openai_body(
    prompt: &str,
    model: &str,
    images: &[ImageAttachment],
) -> serde_json::Value {
    let mut content = vec![serde_json::json!({"type": "text", "text": prompt})];
    for img in images {
        content.push(serde_json::json!({
            "type": "image_url",
            "image_url": {
                "url": format!("data:{};base64,{}", img.mime_type, img.data),
            }
        }));
    }
    serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": content}]
    })
}
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test direct_api::tests -- --nocapture`
Expected: 3 tests PASS.

- [ ] **Step 5: Implement `send_image_via_api` function**

Add to `direct_api.rs`:

```rust
/// Send a multimodal message via the provider's REST API.
/// Returns a Vec of AgentEvents (Text, Usage, Done) to be published to EventBus.
pub async fn send_image_via_api(
    provider: &str,
    model: &str,
    prompt: &str,
    images: &[ImageAttachment],
    api_key_env: &str,
) -> Result<Vec<AgentEvent>, ReasonanceError> {
    let api_key = if api_key_env.is_empty() {
        String::new()
    } else {
        std::env::var(api_key_env).unwrap_or_default()
    };

    if api_key.is_empty() && provider != "local" {
        return Err(ReasonanceError::config(format!(
            "API key not configured. Set {} environment variable to send images with {}",
            api_key_env, provider
        )));
    }

    let client = reqwest::Client::new();
    let effective_model = if model.is_empty() { default_model(provider) } else { model };

    let (url, body, headers) = match provider {
        "claude" | "anthropic" => {
            let body = build_anthropic_body(prompt, effective_model, images, 8192);
            let url = "https://api.anthropic.com/v1/messages".to_string();
            let mut h = vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("x-api-key".to_string(), api_key.clone()),
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ];
            (url, body, h)
        }
        "gemini" | "google" => {
            let m = if effective_model.is_empty() { "gemini-2.5-flash" } else { effective_model };
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                m
            );
            let body = build_google_body(prompt, m, images);
            let h = vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("x-goog-api-key".to_string(), api_key.clone()),
            ];
            (url, body, h)
        }
        _ => {
            // OpenAI-compatible fallback
            let m = if effective_model.is_empty() { "gpt-4o" } else { effective_model };
            let body = build_openai_body(prompt, m, images);
            let url = "https://api.openai.com/v1/chat/completions".to_string();
            let h = vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), format!("Bearer {}", api_key)),
            ];
            (url, body, h)
        }
    };

    info!("direct_api: sending to {} (model={})", url, effective_model);

    let mut req = client.post(&url).json(&body);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }

    let res = req.send().await.map_err(|e| {
        ReasonanceError::transport(provider, format!("API request failed: {}", e), true)
    })?;

    if !res.status().is_success() {
        let status = res.status().as_u16();
        let text = res.text().await.unwrap_or_default();
        return Err(ReasonanceError::transport(
            provider,
            format!("API error {}: {}", status, text),
            status == 429 || status >= 500,
        ));
    }

    let data: serde_json::Value = res.json().await.map_err(|e| {
        ReasonanceError::transport(provider, format!("Failed to parse response: {}", e), false)
    })?;

    debug!("direct_api: response received from {}", provider);

    // Extract text and usage from response per-provider
    let (text_content, input_tokens, output_tokens) = match provider {
        "claude" | "anthropic" => {
            let text = data["content"][0]["text"].as_str().unwrap_or("").to_string();
            let inp = data["usage"]["input_tokens"].as_u64().unwrap_or(0);
            let out = data["usage"]["output_tokens"].as_u64().unwrap_or(0);
            (text, inp, out)
        }
        "gemini" | "google" => {
            let text = data["candidates"][0]["content"]["parts"][0]["text"]
                .as_str().unwrap_or("").to_string();
            let inp = data["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0);
            let out = data["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0);
            (text, inp, out)
        }
        _ => {
            let text = data["choices"][0]["message"]["content"]
                .as_str().unwrap_or("").to_string();
            let inp = data["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
            let out = data["usage"]["completion_tokens"].as_u64().unwrap_or(0);
            (text, inp, out)
        }
    };

    let mut events = Vec::new();
    events.push(AgentEvent::text(&text_content, provider));
    if input_tokens > 0 || output_tokens > 0 {
        events.push(AgentEvent::usage(input_tokens, output_tokens, provider));
    }

    Ok(events)
}

fn default_model(provider: &str) -> &str {
    match provider {
        "claude" | "anthropic" => "claude-sonnet-4-6",
        "gemini" | "google" => "gemini-2.5-flash",
        _ => "gpt-4o",
    }
}
```

- [ ] **Step 6: Run all transport tests**

Run: `cd src-tauri && cargo test direct_api -- --nocapture`
Expected: 3 body-builder tests PASS. The `send_image_via_api` function compiles (async, no unit test for network calls).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/transport/direct_api.rs src-tauri/src/transport/mod.rs
git commit -m "feat(transport): add direct_api module for multimodal image messages"
```

---

### Task 4: Rust — Route image messages in Transport::send()

**Files:**
- Modify: `src-tauri/src/transport/mod.rs`

- [ ] **Step 1: Write failing test**

Add in the existing test module in `mod.rs`:

```rust
#[test]
fn build_image_cli_args_appends_flag_per_image() {
    let images = vec![
        request::ImageAttachment {
            data: "abc".to_string(),
            mime_type: "image/png".to_string(),
            name: "a.png".to_string(),
        },
        request::ImageAttachment {
            data: "def".to_string(),
            mime_type: "image/jpeg".to_string(),
            name: "b.jpg".to_string(),
        },
    ];
    let args = StructuredAgentTransport::build_image_args("--image", &images, "/tmp");
    // Should produce ["--image", "/tmp/<uuid>.png", "--image", "/tmp/<uuid>.jpg"]
    assert_eq!(args.len(), 4);
    assert_eq!(args[0], "--image");
    assert!(args[1].starts_with("/tmp/"));
    assert!(args[1].ends_with(".png"));
    assert_eq!(args[2], "--image");
    assert!(args[3].ends_with(".jpg"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test build_image_cli_args -- --nocapture`
Expected: FAIL — method doesn't exist.

- [ ] **Step 3: Implement build_image_args and routing logic**

Add to `StructuredAgentTransport` impl block in `mod.rs`:

```rust
    /// Save images to temp files and build CLI args like ["--image", "/tmp/abc.png", "--image", "/tmp/def.jpg"].
    pub fn build_image_args(
        image_arg: &str,
        images: &[request::ImageAttachment],
        temp_dir: &str,
    ) -> Vec<String> {
        use base64::Engine;
        let mut args = Vec::new();
        for img in images {
            let ext = match img.mime_type.as_str() {
                "image/jpeg" => "jpg",
                "image/gif" => "gif",
                "image/webp" => "webp",
                _ => "png",
            };
            let filename = format!("{}/{}.{}", temp_dir, uuid::Uuid::new_v4(), ext);
            // Decode base64 and write to temp file
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&img.data) {
                if std::fs::write(&filename, &bytes).is_ok() {
                    args.push(image_arg.to_string());
                    args.push(filename);
                } else {
                    warn!("Transport: failed to write temp image file {}", filename);
                }
            } else {
                warn!("Transport: failed to decode base64 image {}", img.name);
            }
        }
        args
    }
```

Then in `Transport::send()`, after the CLI spawn loop setup and before `cmd.spawn()`, add image routing. Find the section where `cmd.args(&args)` is called (around line 334) and add before the spawn:

```rust
            // ── Image routing ─────────────────────────────────────
            if !request.images.is_empty() {
                let image_mode = config.cli.image_mode.as_deref().unwrap_or("none");
                match image_mode {
                    "cli-flag" => {
                        if let Some(ref image_arg) = config.cli.image_arg {
                            let image_args = Self::build_image_args(
                                image_arg,
                                &request.images,
                                "/tmp",
                            );
                            cmd.args(&image_args);
                            info!("Transport: appended {} image args", image_args.len());
                        }
                    }
                    "direct-api" => {
                        // Bypass CLI entirely — use direct API
                        let api_key_env = config.cli.api_key_env.clone().unwrap_or_default();
                        let model = request.model.clone().unwrap_or_default();
                        let prompt = request.prompt.clone();
                        let images = request.images.clone();
                        let provider = provider.clone();
                        let session_id_clone = session_id.clone();
                        let event_bus_opt = self.event_bus.lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .clone();

                        if let Some(bus) = event_bus_opt {
                            let bus_clone = bus.clone();
                            let abort = tokio::spawn(async move {
                                match direct_api::send_image_via_api(
                                    &provider, &model, &prompt, &images, &api_key_env,
                                ).await {
                                    Ok(events) => {
                                        for event in &events {
                                            bus_clone.publish(
                                                crate::event_bus::Event::from_agent_event(
                                                    "transport:event",
                                                    &session_id_clone,
                                                    event,
                                                ),
                                            );
                                        }
                                        let done = crate::agent_event::AgentEvent::done(
                                            &session_id_clone, &provider,
                                        );
                                        bus_clone.publish(
                                            crate::event_bus::Event::from_agent_event(
                                                "transport:complete",
                                                &session_id_clone,
                                                &done,
                                            ),
                                        );
                                    }
                                    Err(e) => {
                                        let err_event = crate::agent_event::AgentEvent::error(
                                            &e.to_string(),
                                            "DIRECT_API_ERROR",
                                            crate::agent_event::ErrorSeverity::Fatal,
                                            &provider,
                                        );
                                        bus_clone.publish(
                                            crate::event_bus::Event::from_agent_event(
                                                "transport:error",
                                                &session_id_clone,
                                                &err_event,
                                            ),
                                        );
                                        let done = crate::agent_event::AgentEvent::done(
                                            &session_id_clone, &provider,
                                        );
                                        bus_clone.publish(
                                            crate::event_bus::Event::from_agent_event(
                                                "transport:complete",
                                                &session_id_clone,
                                                &done,
                                            ),
                                        );
                                    }
                                }
                            });
                        }

                        return Ok(session_id);
                    }
                    _ => {
                        // "none" or unknown — publish error event
                        let err_event = AgentEvent::error(
                            &format!("Image attachments are not supported for provider '{}'", provider),
                            "IMAGE_UNSUPPORTED",
                            ErrorSeverity::Recoverable,
                            &provider,
                        );
                        if let Some(bus) = self.event_bus.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                            bus.publish(crate::event_bus::Event::from_agent_event(
                                "transport:error", &session_id, &err_event,
                            ));
                            let done = AgentEvent::done(&session_id, &provider);
                            bus.publish(crate::event_bus::Event::from_agent_event(
                                "transport:complete", &session_id, &done,
                            ));
                        }
                        return Ok(session_id);
                    }
                }
            }
```

Note: This block must be placed AFTER the registry lock is acquired and config is available, but BEFORE the CLI spawn loop. The exact insertion point is after `let allowed_tools_args = ...` (around line 240) and before `let state_machine: Box<dyn ...>` (around line 245). The `"direct-api"` and `"none"` branches return early — only `"cli-flag"` falls through to the existing CLI spawn.

- [ ] **Step 4: Add base64 dependency**

In `src-tauri/Cargo.toml`, add under `[dependencies]`:

```toml
base64 = "0.22"
```

- [ ] **Step 5: Run tests**

Run: `cd src-tauri && cargo test build_image_cli_args -- --nocapture && cargo test transport::tests -- --nocapture`
Expected: All PASS. (The routing code compiles; the early-return paths don't affect existing text-only tests.)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/transport/mod.rs src-tauri/Cargo.toml
git commit -m "feat(transport): route image messages to direct-api or cli-flag path"
```

---

### Task 5: Normalizer TOML updates

**Files:**
- Modify: `src-tauri/normalizers/claude.toml`
- Modify: `src-tauri/normalizers/codex.toml`
- Modify: `src-tauri/normalizers/gemini.toml`
- Modify: `src-tauri/normalizers/kimi.toml`
- Modify: `src-tauri/normalizers/qwen.toml`

- [ ] **Step 1: Add image config to each normalizer**

In `claude.toml`, add to the `[cli]` section:

```toml
image_mode = "direct-api"
```

In `codex.toml`, add to the `[cli]` section:

```toml
image_mode = "cli-flag"
image_arg = "--image"
```

In `gemini.toml`, add to the `[cli]` section:

```toml
image_mode = "direct-api"
```

In `kimi.toml`, add to the `[cli]` section:

```toml
image_mode = "none"
```

In `qwen.toml`, add to the `[cli]` section:

```toml
image_mode = "none"
```

- [ ] **Step 2: Run normalizer tests to verify nothing broke**

Run: `cd src-tauri && cargo test normalizer -- --nocapture`
Expected: All 121+ tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/normalizers/
git commit -m "feat(normalizer): configure image_mode for all providers"
```

---

### Task 6: Frontend — ImageAttachment type and adapter changes

**Files:**
- Modify: `src/lib/adapter/index.ts`
- Modify: `src/lib/adapter/tauri.ts`
- Modify: `tests/mocks/adapter.ts`

- [ ] **Step 1: Add ImageAttachment type and update agentSend signature**

In `src/lib/adapter/index.ts`, add above the `Adapter` interface:

```typescript
export interface ImageAttachment {
  data: string;      // base64 (no data: prefix)
  mimeType: string;  // image/png, image/jpeg, etc.
  name: string;
}
```

Update the `agentSend` line in the `Adapter` interface:

```typescript
  agentSend(prompt: string, provider: string, model?: string, sessionId?: string, cwd?: string, yolo?: boolean, allowedTools?: string[], images?: ImageAttachment[]): Promise<string>;
```

- [ ] **Step 2: Update TauriAdapter**

In `src/lib/adapter/tauri.ts`, update the `agentSend` method signature and invoke call:

```typescript
  async agentSend(prompt: string, provider: string, model?: string, sessionId?: string, cwd?: string, yolo?: boolean, allowedTools?: string[], images?: ImageAttachment[]): Promise<string> {
    const result = await invoke('agent_send', {
      request: {
        prompt, provider, model: model ?? null, context: [],
        session_id: sessionId ?? null, system_prompt: null,
        max_tokens: null, allowed_tools: allowedTools ?? null,
        cwd: cwd ?? null, yolo: yolo ?? false,
        images: (images ?? []).map(img => ({
          data: img.data, mime_type: img.mimeType, name: img.name,
        })),
      }
    });
    return z.string().parse(result);
  }
```

Add the import at the top of `tauri.ts`:

```typescript
import type { ImageAttachment } from './index';
```

- [ ] **Step 3: Update mock adapter**

In `tests/mocks/adapter.ts`, update the `agentSend` signature:

```typescript
    agentSend(_prompt: string, _provider: string, _model?: string, _sessionId?: string, _cwd?: string, _yolo?: boolean, _allowedTools?: string[], _images?: ImageAttachment[]): Promise<string> {
      return Promise.resolve('mock-session-id');
    },
```

Add `ImageAttachment` to the import at the top of the file:

```typescript
import type {
  Adapter,
  // ... existing imports ...
  ImageAttachment,
} from '$lib/adapter/index';
```

- [ ] **Step 4: Run frontend type check**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors.

- [ ] **Step 5: Run tests**

Run: `npx vitest run`
Expected: All 243+ tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/lib/adapter/index.ts src/lib/adapter/tauri.ts tests/mocks/adapter.ts
git commit -m "feat(adapter): add ImageAttachment type, extend agentSend signature"
```

---

### Task 7: Frontend — ChatInput image handlers

**Files:**
- Modify: `src/lib/components/chat/ChatInput.svelte`

- [ ] **Step 1: Add image state and utility functions**

Add to the `<script>` block, after the existing props and state declarations:

```typescript
  import type { ImageAttachment } from '$lib/adapter/index';
  import { showToast } from '$lib/stores/toast';
  import { appAnnouncer } from '$lib/utils/a11y-announcer';

  const MAX_IMAGES = 5;
  const MAX_IMAGE_BYTES = 5 * 1024 * 1024; // 5MB
  const SUPPORTED_TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp'];

  let attachedImages = $state<(ImageAttachment & { id: string; size: number })[]>([]);
  let dragOver = $state(false);
  let fileInput: HTMLInputElement;
```

Update the `onSend` prop type:

```typescript
    onSend: (text: string, images?: ImageAttachment[]) => void;
```

Add image utility functions:

```typescript
  function readFileAsAttachment(file: File): Promise<ImageAttachment & { id: string; size: number }> {
    return new Promise((resolve, reject) => {
      if (!SUPPORTED_TYPES.includes(file.type)) {
        reject(new Error(`Unsupported format: ${file.type}`));
        return;
      }
      if (file.size > MAX_IMAGE_BYTES) {
        reject(new Error(`Image exceeds 5MB limit`));
        return;
      }
      const reader = new FileReader();
      reader.onload = () => {
        const dataUrl = reader.result as string;
        const base64 = dataUrl.split(',')[1] ?? '';
        resolve({
          id: crypto.randomUUID(),
          data: base64,
          mimeType: file.type,
          name: file.name || 'image',
          size: file.size,
        });
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
  }

  async function addImageFiles(files: File[]) {
    for (const file of files) {
      if (attachedImages.length >= MAX_IMAGES) {
        showToast('warning', `Maximum ${MAX_IMAGES} images per message`);
        break;
      }
      try {
        const attachment = await readFileAsAttachment(file);
        attachedImages = [...attachedImages, attachment];
        appAnnouncer.announce(`Image attached: ${attachment.name}`);
      } catch (e: any) {
        showToast('error', e.message ?? 'Failed to read image');
      }
    }
  }

  function removeImage(id: string) {
    const removed = attachedImages.find(img => img.id === id);
    attachedImages = attachedImages.filter(img => img.id !== id);
    if (removed) {
      appAnnouncer.announce(`Image removed: ${removed.name}`);
    }
  }

  function handlePaste(e: ClipboardEvent) {
    const items = Array.from(e.clipboardData?.items ?? []);
    const imageFiles = items
      .filter(item => item.kind === 'file' && item.type.startsWith('image/'))
      .map(item => item.getAsFile())
      .filter((f): f is File => f !== null);
    if (imageFiles.length > 0) {
      e.preventDefault();
      addImageFiles(imageFiles);
    }
  }

  function handleDragOver(e: DragEvent) {
    if (!e.dataTransfer) return;
    const hasImage = Array.from(e.dataTransfer.items).some(
      item => item.kind === 'file' && item.type.startsWith('image/')
    );
    if (!hasImage) return;
    e.preventDefault();
    dragOver = true;
  }

  function handleDragLeave() { dragOver = false; }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const files = Array.from(e.dataTransfer?.files ?? []).filter(f => f.type.startsWith('image/'));
    if (files.length > 0) addImageFiles(files);
  }

  function handleFilePick(e: Event) {
    const input = e.target as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    if (files.length > 0) addImageFiles(files);
    input.value = '';
  }
```

- [ ] **Step 2: Update handleSubmit to pass images**

Replace the existing `handleSubmit`:

```typescript
  function handleSubmit() {
    const trimmed = text.trim();
    if ((!trimmed && attachedImages.length === 0) || disabled || sending) return;
    sending = true;
    if (sendTimer) clearTimeout(sendTimer);
    sendTimer = setTimeout(() => { sending = false; }, 300);
    const images = attachedImages.length > 0
      ? attachedImages.map(({ data, mimeType, name }) => ({ data, mimeType, name }))
      : undefined;
    onSend(trimmed || '(image)', images);
    text = '';
    attachedImages = [];
  }
```

- [ ] **Step 3: Update template with paste/drop/picker and strip**

Replace the template section with:

```svelte
<div
  class="chat-input-wrapper"
  class:drag-over={dragOver}
  role="region"
  aria-label="Chat input area"
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
>
  {#if dragOver}
    <div class="drop-overlay" aria-hidden="true">Drop images here</div>
  {/if}
  {#if showSlashMenu}
    <SlashMenu
      commands={slashFiltered}
      activeIndex={slashActiveIndex}
      onSelect={handleSlashSelect}
      onClose={() => { text = ''; }}
    />
  {/if}

  {#if attachedImages.length > 0}
    <div class="image-strip" role="list" aria-label="Attached images">
      {#each attachedImages as img (img.id)}
        <div class="image-thumb" role="listitem" aria-label="{img.name}, {(img.size / 1024).toFixed(0)}KB">
          <img src="data:{img.mimeType};base64,{img.data}" alt={img.name} width="48" height="48" />
          <button
            class="remove-btn"
            onclick={() => removeImage(img.id)}
            aria-label="Remove {img.name}"
          >&times;</button>
        </div>
      {/each}
    </div>
  {/if}

  <div class="input-row">
    <button
      class="attach-btn"
      onclick={() => fileInput?.click()}
      disabled={disabled || attachedImages.length >= MAX_IMAGES}
      aria-label="Attach images"
      title="Attach images (Ctrl+V to paste)"
    >&#x1F4CE;</button>
    <input
      bind:this={fileInput}
      type="file"
      accept="image/png,image/jpeg,image/gif,image/webp"
      multiple
      onchange={handleFilePick}
      class="sr-only"
      tabindex={-1}
      aria-hidden="true"
    />
    <textarea
      bind:value={text}
      onkeydown={handleKeydown}
      onpaste={handlePaste}
      placeholder={$tr('chat.placeholder')}
      rows="1"
      {disabled}
      role={showSlashMenu ? 'combobox' : undefined}
      aria-autocomplete={showSlashMenu ? 'list' : undefined}
      aria-expanded={showSlashMenu}
      aria-controls={showSlashMenu ? 'slash-menu' : undefined}
      aria-activedescendant={showSlashMenu && slashFiltered.length > 0 ? `slash-cmd-${slashActiveIndex}` : undefined}
      aria-label="Message input"
    ></textarea>
    <button
      class="send-btn"
      onclick={handleSubmit}
      disabled={disabled || sending || (!text.trim() && attachedImages.length === 0)}
      aria-label="Send message"
      aria-busy={sending}
    >
      SEND
    </button>
  </div>

  <!-- existing footer unchanged -->
```

- [ ] **Step 4: Add styles for image strip, drop overlay, attach button**

Add to the `<style>` block:

```css
  .chat-input-wrapper {
    position: relative;
    /* keep existing styles, add: */
  }

  .chat-input-wrapper.drag-over {
    outline: 2px dashed var(--accent);
    outline-offset: -2px;
  }

  .drop-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(124, 106, 247, 0.12);
    font-weight: 600;
    color: var(--accent);
    pointer-events: none;
    z-index: 10;
  }

  .image-strip {
    display: flex;
    gap: var(--space-1);
    padding: 0 var(--space-1);
    overflow-x: auto;
    max-height: 60px;
  }

  .image-thumb {
    position: relative;
    flex-shrink: 0;
    width: 48px;
    height: 48px;
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .image-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .remove-btn {
    position: absolute;
    top: -2px;
    right: -2px;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--text-error, #e55);
    color: white;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .remove-btn:focus-visible {
    outline: 2px solid var(--focus-ring, var(--accent));
    outline-offset: 1px;
  }

  .attach-btn {
    background: transparent;
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-1);
    cursor: pointer;
    font-size: var(--font-size-base);
    color: var(--text-muted);
    align-self: flex-end;
    min-height: 2.75rem;
    min-width: 2.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color var(--transition-fast), border-color var(--transition-fast);
  }

  .attach-btn:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
  }

  .attach-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
```

- [ ] **Step 5: Run type check and tests**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors.

Run: `npx vitest run`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/chat/ChatInput.svelte
git commit -m "feat(chat): add image paste, drop, and picker to ChatInput"
```

---

### Task 8: Frontend — Wire ChatView to pass images

**Files:**
- Modify: `src/lib/components/chat/ChatView.svelte`

- [ ] **Step 1: Update handleSend to accept and pass images**

In `ChatView.svelte`, update the `handleSend` function signature and body:

```typescript
  async function handleSend(text: string, images?: ImageAttachment[]) {
```

Add the import at the top:

```typescript
  import type { ImageAttachment } from '$lib/adapter/index';
```

Update the `agentSend` call (around line 127) to pass images:

```typescript
      await adapter.agentSend(text, provider, model, sessionId, cwd, isYolo, tools, images);
```

Update the user event to include image metadata in the content for rendering:

```typescript
      const userEvent: AgentEvent = {
        id: crypto.randomUUID(),
        parent_id: null,
        event_type: 'text',
        content: { type: 'text', value: text },
        timestamp: Date.now(),
        metadata: {
          session_id: sessionId,
          input_tokens: null,
          output_tokens: null,
          tool_name: null,
          model: null,
          provider: 'user',
          error_severity: null,
          error_code: null,
          stream_metrics: null,
          images: images?.map(img => ({ mimeType: img.mimeType, data: img.data, name: img.name })) ?? null,
        },
      };
```

- [ ] **Step 2: Update ChatInput usage to pass images through**

The `ChatInput` `onSend` prop already changed signature in Task 7. Verify the binding:

```svelte
  <ChatInput
    onSend={handleSend}
    ...
```

No change needed — the callback signature matches.

- [ ] **Step 3: Update ChatMessages to render user images**

In `src/lib/components/chat/ChatMessages.svelte`, find where user messages are rendered (look for `provider === 'user'`). Add image thumbnails above the text content:

```svelte
{#if event.metadata.images?.length}
  <div class="user-images" role="list" aria-label="Attached images">
    {#each event.metadata.images as img}
      <img
        src="data:{img.mimeType};base64,{img.data}"
        alt={img.name ?? 'Attached image'}
        class="user-image-thumb"
        loading="lazy"
      />
    {/each}
  </div>
{/if}
```

Add styles:

```css
  .user-images {
    display: flex;
    gap: var(--space-1);
    flex-wrap: wrap;
    margin-bottom: var(--space-1);
  }

  .user-image-thumb {
    max-width: 200px;
    max-height: 150px;
    border-radius: var(--radius);
    border: var(--border-width) solid var(--border);
    object-fit: contain;
  }
```

- [ ] **Step 4: Update AgentEvent metadata type to include optional images**

In `src/lib/types/agent-event.ts`, add `images` to the metadata interface:

```typescript
  images?: Array<{ mimeType: string; data: string; name: string }> | null;
```

- [ ] **Step 5: Run type check and tests**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors.

Run: `npx vitest run`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/chat/ChatView.svelte src/lib/components/chat/ChatMessages.svelte src/lib/types/agent-event.ts
git commit -m "feat(chat): wire image attachments through ChatView to provider API"
```

---

### Task 9: Full build verification

**Files:** None (verification only)

- [ ] **Step 1: Run Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass (600+).

- [ ] **Step 2: Run frontend tests**

Run: `npx svelte-kit sync && npx svelte-check --tsconfig ./tsconfig.json && npx vitest run`
Expected: 0 errors, all tests pass.

- [ ] **Step 3: Run Rust clippy**

Run: `cd src-tauri && cargo clippy`
Expected: No errors (warnings OK).

- [ ] **Step 4: Commit any fixups**

If clippy or tests required changes, commit them.

- [ ] **Step 5: Final commit with version bump**

```bash
git add -A
git commit -m "feat: chat image attachments (paste, drop, file picker)

Add multimodal image support to chat. Users can paste (Ctrl+V),
drag-drop, or pick image files. Images routed via Direct API
(Anthropic/Google/OpenAI) or CLI flag (Codex --image).
WCAG AA accessible with screen reader announcements."
```
