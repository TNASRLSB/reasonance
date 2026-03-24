# Codebase Registry

**Last updated:** 2026-03-24

This is my memory. I update it as I learn. I check it before making claims.

---

## Skills

| Name | Location | Purpose |
|------|----------|---------|
| Seurat | `.claude/skills/seurat/` | UI design system, wireframing, page layout, WCAG accessibility |
| Emmet | `.claude/skills/emmet/` | Testing, QA, tech debt audit, functional mapping, unit tests |
| Heimdall | `.claude/skills/heimdall/` | AI-specific security analysis, OWASP Top 10, credential detection |
| Ghostwriter | `.claude/skills/ghostwriter/` | SEO + GEO dual optimization, persuasive copywriting |
| Baptist | `.claude/skills/baptist/` | CRO orchestrator, A/B testing, funnel analysis |
| Orson | `.claude/skills/orson/` | Programmatic video generation, demo recording with audio |
| Scribe | `.claude/skills/scribe/` | Office documents (xlsx, docx, pptx) and PDF handling |
| Forge | `.claude/skills/forge/` | Meta-skill for creating, auditing, and maintaining skills |

---

## Components

| Name | Type | Location | Purpose |
|------|------|----------|---------|
| PermissionRequestBlock | Svelte | `src/lib/components/chat/PermissionRequestBlock.svelte` | Interactive Approve/Deny UI for `ask` permission mode |
| PermissionDenialBlock | Svelte | `src/lib/components/chat/PermissionDenialBlock.svelte` | Info-only denial display for `locked` mode / unsupported providers |
| ChatView | Svelte | `src/lib/components/chat/ChatView.svelte` | Main chat view — manages sessions, permission levels, replay |
| ChatMessages | Svelte | `src/lib/components/chat/ChatMessages.svelte` | Renders agent events including permission denials |
| ChatInput | Svelte | `src/lib/components/chat/ChatInput.svelte` | Chat input with permission level badge |
| StatusBar | Svelte | `src/lib/components/StatusBar.svelte` | Bottom bar — file info, YOLO model warning |
| Toolbar | Svelte | `src/lib/components/Toolbar.svelte` | Top bar — logo, git, settings, window controls |
| MenuBar | Svelte | `src/lib/components/MenuBar.svelte` | Application menu bar |
| Settings | Svelte | `src/lib/components/Settings.svelte` | Settings dialog — LLM config, permission level, allowed tools |
| TerminalManager | Svelte | `src/lib/components/TerminalManager.svelte` | Terminal/chat session manager — PTY and structured modes |
| EditorTabs | Svelte | `src/lib/components/EditorTabs.svelte` | File editor tab bar |
| Toast | Svelte | `src/lib/components/Toast.svelte` | Toast notification system with ARIA live region |
| HiveCanvas | Svelte | `src/lib/components/hive/HiveCanvas.svelte` | Full HIVE canvas — SvelteFlow, dual mode, inspector, controls |
| HivePanel | Svelte | `src/lib/components/hive/HivePanel.svelte` | Compact HIVE monitor — mini-map, live log, status |
| HiveControls | Svelte | `src/lib/components/hive/HiveControls.svelte` | Playback buttons — play/pause/stop/step |
| HiveInspector | Svelte | `src/lib/components/hive/HiveInspector.svelte` | Selected node details + JSON toggle |

---

## Key Functions

| Function | Location | Lines | What it does |
|----------|----------|-------|--------------|
| `LogicEvaluator::evaluate` | `src-tauri/src/logic_eval.rs` | — | Evaluates Rhai rule expression against node output (sandboxed) |
| `ResourceLockManager::acquire` | `src-tauri/src/resource_lock.rs` | — | Acquires read/write lock on resource node |
| `AgentMemoryStore::add_entry` | `src-tauri/src/agent_memory.rs` | — | Adds memory entry with FIFO eviction |
| `WorkflowEngine::spawn_single_node` | `src-tauri/src/workflow_engine.rs` | — | Spawns PTY for single approved node (used by trusted + supervised modes) |
| `migrate` | `src-tauri/src/workflow_store.rs` | — | Schema migration v0→v1 (edge IDs, permissionLevel) |
| `setupHiveEventListeners` | `src/lib/stores/engine.ts` | — | Initializes Tauri event listeners for hive:// namespace |
| `build_permission_args` | `src-tauri/src/transport/mod.rs` | ~287 | Conditionally builds CLI permission flags (only when yolo=true) |
| `build_allowed_tools_args` | `src-tauri/src/transport/mod.rs` | ~300 | Builds `--allowedTools` CLI args from approved tool list |
| `send` | `src-tauri/src/transport/mod.rs` | ~110 | Spawns CLI process with permission + allowed-tools args |
| `build_event` | `src-tauri/src/normalizer/pipeline.rs` | — | Builds AgentEvents from normalizer rules; special JSON handling for PermissionDenial |
| `permission_denial` | `src-tauri/src/agent_event.rs` | — | Constructor for PermissionDenial AgentEvent with JSON denials content |
| `handleApproveTools` | `src/lib/components/chat/ChatView.svelte` | — | Replay mechanism: adds tools to session set, generates new session ID, re-invokes agentSend |
| `parseLlmConfigs` | `src/lib/utils/config-parser.ts` | — | Parses TOML → LlmConfig including permissionLevel and allowedTools |
| `serializeLlmConfigs` | `src/lib/utils/config-bootstrap.ts` | — | Serializes LlmConfig → TOML including permission fields |

---

## API Endpoints

| Method | Route | Handler | Auth required |
|--------|-------|---------|---------------|
| | | | |

---

## Database

### Tables
| Table | Key columns | Used by |
|-------|-------------|---------|
| | | |

### Important queries
| Name | Location | What it does |
|------|----------|--------------|
| | | |

---

## Data Flows

### Per-Model Permission Flow
1. User sends message → ChatView reads `permissionLevel` from model's LlmConfig
2. If `yolo`: `request.yolo = true` → transport passes `--dangerously-skip-permissions`
3. If `ask`/`locked`: `request.yolo = false`, `request.allowed_tools` = session ∪ config tools
4. CLI auto-denies unapproved tools → emits `permission_denials[]` in result event
5. Normalizer `permission_denial` rule → `PermissionDenial` AgentEvent with JSON content
6. ChatMessages renders PermissionRequestBlock (ask) or PermissionDenialBlock (locked)
7. User approves → `handleApproveTools` generates new session ID → replays with `--allowedTools`

---

## External Dependencies

| Package | Version | Used for |
|---------|---------|----------|
| | | |

---

## Environment Variables

| Variable | Required | Purpose |
|----------|----------|---------|
| | | |

---

## Notes

*Project-specific notes go here.*


---

## How I Use This

1. **Before claiming something exists:** `grep "name" .claude/docs/registry.md`
2. **After discovering something:** Add it here immediately
3. **Before implementing:** Check what's already here
4. **After implementing:** Update with new components/functions

**If I'm about to write code that calls a function not listed here, I STOP and verify it exists first.**
