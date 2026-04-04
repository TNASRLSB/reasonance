# Chat Image Attachments

**Date:** 2026-04-04
**Status:** Draft

## Problem

Users cannot paste, drop, or attach images in the Reasonance chat. Every major coding IDE extension (Claude Code for VS Code, Codex, Gemini Code Assist) supports this. Common use cases: pasting screenshots for UI implementation, sharing error screenshots, attaching design mockups.

## Approach

Two transport paths depending on provider capabilities:

1. **Direct API path** (Anthropic, Google, OpenAI): When images are attached, bypass the CLI and call the provider REST API directly with multimodal content blocks. The existing `call_llm_api` Rust module is extended to accept images and stream responses via EventBus.

2. **CLI flag path** (Codex): Providers with native `--image <FILE>` CLI support save images to temp files and pass them as CLI arguments. The existing normalizer pipeline processes the response.

3. **Unsupported providers** (Kimi, Qwen): Show a toast explaining images aren't supported for this provider.

### Trade-off: Direct API vs CLI

The Direct API path for image messages loses CLI-specific features (tool use, session resume, permissions). This is acceptable because:
- Image messages are typically "look at this" / "implement this UI" queries
- Text follow-ups in the same session continue through the CLI path with full features
- The alternative (stdin stream-json) is unreliable for single-turn and would require a major transport rewrite

## Architecture

### Data Flow

```
User pastes/drops/picks image
  -> ChatInput captures File/Blob
  -> Convert to base64 + detect MIME type
  -> ChatView.handleSend(text, images[])
  -> adapter.agentSend(text, provider, ..., images)
  -> invoke('agent_send', { request: { ..., images } })
  -> Transport::send()
     |
     +-- images.is_empty() -> CLI path (unchanged)
     |
     +-- images.not_empty() + provider has image_cli_arg
     |     -> save to temp files
     |     -> append --image <path> to CLI args
     |     -> CLI path with normalizer pipeline
     |
     +-- images.not_empty() + provider has direct_api
           -> call provider REST API with multimodal content
           -> stream response chunks via EventBus as AgentEvents
           -> frontend processes normally
```

### Frontend Changes

#### ChatInput.svelte

Add three image input methods:

**Paste (Ctrl+V):** `onpaste` handler on the textarea. Checks `event.clipboardData.items` for `type.startsWith('image/')`. Converts matching items to base64 via `FileReader.readAsDataURL()`.

**Drag-drop:** `ondragover` (prevent default, show drop indicator) and `ondrop` handlers on `.chat-input-wrapper`. Same base64 conversion as paste.

**File picker:** A button with `aria-label` that triggers a hidden `<input type="file" accept="image/png,image/jpeg,image/gif,image/webp" multiple>`.

**State:** `attachedImages: Array<{id: string, data: string, mimeType: string, name: string, size: number}>`. Max 5 images, max 5MB each. Images exceeding the limit show a toast error.

**ImageAttachmentStrip:** New component rendered below the textarea when `attachedImages.length > 0`. Shows a horizontal strip of thumbnails (48x48px, object-fit cover) with:
- Remove button per image (X icon)
- Image name (truncated) and size
- Total count badge when > 3 images

#### Accessibility (WCAG AA)

- Drop zone: `ondragover` sets `aria-label="Drop images here"` on wrapper, visual border highlight
- File picker button: `aria-label="Attach images"`, visible label text (clip icon + "Attach")
- Strip: `role="list"`, each thumbnail `role="listitem"` with `aria-label="{name}, {size}. Press Delete to remove"`
- Remove buttons: `aria-label="Remove {name}"`, focus moves to next image or textarea after removal
- Screen reader announcements via `appAnnouncer`: "Image attached: {name}" / "Image removed: {name}" / "Images not supported for {provider}"
- Keyboard: Tab navigates strip items, Delete/Backspace removes focused image
- Reduced motion: no transition animations on strip if `prefers-reduced-motion`

#### ChatView.svelte

- `handleSend(text, images?)` passes images to adapter
- User message events include image metadata for display
- `ChatMessages` renders attached images inline in user message bubbles (thumbnail + "View" link that opens full size in a modal or native viewer)

#### Adapter

```typescript
// adapter/index.ts - extend interface
interface ImageAttachment {
  data: string;      // base64 (no data: prefix)
  mimeType: string;  // image/png, image/jpeg, etc.
  name: string;
}

agentSend(prompt: string, provider: string, model?: string, sessionId?: string,
  cwd?: string, yolo?: boolean, allowedTools?: string[],
  images?: ImageAttachment[]): Promise<string>;
```

```typescript
// adapter/tauri.ts - pass images in request
const result = await invoke('agent_send', {
  request: {
    prompt, provider, model: model ?? null, context: [],
    session_id: sessionId ?? null, system_prompt: null,
    max_tokens: null, allowed_tools: allowedTools ?? null,
    cwd: cwd ?? null, yolo: yolo ?? false,
    images: images ?? [],
  }
});
```

### Backend Changes (Rust)

#### transport/request.rs

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAttachment {
    pub data: String,      // base64
    pub mime_type: String,
    pub name: String,
}

// Add to AgentRequest:
pub images: Vec<ImageAttachment>,
```

Default: `images: Vec::new()` (empty = text-only, existing behavior unchanged).

#### Normalizer TOML config

Add optional fields to `[cli]` section:

```toml
# claude.toml - uses direct API for images
[cli]
image_mode = "direct-api"
direct_api_provider = "anthropic"

# codex.toml - uses CLI flag
[cli]
image_mode = "cli-flag"
image_arg = "--image"

# gemini.toml
[cli]
image_mode = "direct-api"
direct_api_provider = "google"

# kimi.toml / qwen.toml
[cli]
image_mode = "none"
```

#### transport/mod.rs — send() changes

When `request.images.is_not_empty()`:

1. Read `image_mode` from normalizer config
2. If `"cli-flag"`:
   - Save each image to temp file (`/tmp/reasonance-img-{uuid}.{ext}`)
   - Append `["--image", "/tmp/reasonance-img-{uuid}.png"]` per image to CLI args
   - Proceed with normal CLI spawn + stream reader
   - Schedule temp file cleanup after stream completes
3. If `"direct-api"`:
   - Read `direct_api_provider` to determine API format
   - Call `send_with_direct_api()` (new method)
   - This builds the multimodal request, calls the API, and publishes synthetic AgentEvents through EventBus
4. If `"none"`:
   - Publish a synthetic Error event: "Image attachments not supported for {provider}"
   - Return session_id (don't spawn CLI)

#### New: transport/direct_api.rs

Handles the direct API path for image messages. Responsibilities:
- Build multimodal request body per provider (Anthropic, Google, OpenAI)
- Make the HTTP request via `reqwest`
- Parse the streaming or non-streaming response
- Create `AgentEvent`s (Text, Usage, Done) from the response
- Publish each event to EventBus channel `transport:event`
- The existing `TauriFrontendBridge` forwards them as `agent-event` to the frontend

**API formats:**

Anthropic:
```json
{
  "model": "claude-sonnet-4-6",
  "max_tokens": 8192,
  "messages": [{
    "role": "user",
    "content": [
      {"type": "text", "text": "prompt text"},
      {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "..."}}
    ]
  }]
}
```

Google:
```json
{
  "contents": [{
    "parts": [
      {"text": "prompt text"},
      {"inlineData": {"mimeType": "image/png", "data": "..."}}
    ]
  }]
}
```

OpenAI:
```json
{
  "model": "gpt-4o",
  "messages": [{
    "role": "user",
    "content": [
      {"type": "text", "text": "prompt text"},
      {"type": "image_url", "image_url": {"url": "data:image/png;base64,..."}}
    ]
  }]
}
```

The API key is read from the normalizer's `api_key_env` config field (e.g., `ANTHROPIC_API_KEY`).

### commands/llm.rs changes

Not changed. The direct API image path goes through `transport/direct_api.rs` which has access to EventBus. The existing `call_llm_api` stays for non-chat use (settings, self-heal, etc.).

## Image Validation

- **Supported formats:** PNG, JPEG, GIF, WebP (validated by MIME type)
- **Max size:** 5MB per image (Anthropic limit). Frontend validates before sending.
- **Max count:** 5 images per message (reasonable UX limit)
- **Resize:** If image exceeds 5MB, frontend resizes using Canvas API to fit within limit while preserving aspect ratio

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Unsupported provider | Toast: "Images not supported for {provider}" |
| Image too large (after resize attempt) | Toast: "Image exceeds 5MB limit" |
| Too many images | Toast: "Maximum 5 images per message" |
| Invalid format | Toast: "Unsupported image format. Use PNG, JPEG, GIF, or WebP" |
| API key missing for direct-api path | Error event in chat: "API key not configured for {provider}. Set {env_var} to send images." |
| API call fails | Error event in chat with provider error message |
| Temp file write fails | Error event in chat: "Failed to save image for upload" |

## Files Changed

### New files
- `src/lib/components/chat/ImageAttachmentStrip.svelte` — thumbnail strip component
- `src-tauri/src/transport/direct_api.rs` — direct API path for multimodal messages

### Modified files
- `src/lib/components/chat/ChatInput.svelte` — paste/drop/picker handlers, image state
- `src/lib/components/chat/ChatView.svelte` — pass images through handleSend
- `src/lib/components/chat/ChatMessages.svelte` — render user image attachments
- `src/lib/adapter/index.ts` — ImageAttachment type, agentSend signature
- `src/lib/adapter/tauri.ts` — pass images in invoke
- `tests/mocks/adapter.ts` — mock updated signature
- `src-tauri/src/transport/request.rs` — ImageAttachment struct, AgentRequest field
- `src-tauri/src/transport/mod.rs` — route to direct_api or cli-flag based on config
- `src-tauri/src/normalizer/mod.rs` — parse image_mode, image_arg, direct_api_provider from TOML
- `src-tauri/normalizers/claude.toml` — add image_mode + direct_api_provider
- `src-tauri/normalizers/codex.toml` — add image_mode + image_arg
- `src-tauri/normalizers/gemini.toml` — add image_mode + direct_api_provider
- `src-tauri/normalizers/kimi.toml` — add image_mode = "none"
- `src-tauri/normalizers/qwen.toml` — add image_mode = "none"

## Testing

- **Unit tests:** ImageAttachmentStrip render, image validation (size, format, count)
- **Rust tests:** Direct API request building per provider, temp file management, ImageAttachment serde
- **Mock adapter tests:** agentSend with images parameter
- **Manual test:** Paste screenshot in chat with Claude provider, verify image renders in user bubble and AI responds about the image content

## Future Enhancements (not in scope)

- Stream-json stdin mode for Claude CLI (enables tool use + images)
- Image OCR/description caching
- Image compression options in settings
- Multi-turn image context (current: images only in the sent message)
