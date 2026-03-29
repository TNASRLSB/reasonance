# Changelog

## [2.6.0] - 2026-03-29

### Features

- feat(memory): add browse/search panel for agent memory v2

### Other

- chore: frontend cleanup — remove unused components, exports, and tests
- - Remove createFocusTrap (unused), keep findFocusFallback (used by layerManager)
- - Remove colorblindDistinct (unused dev utility)
- - Remove corresponding tests
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- sort controls, paginated listing, and detail view. Accessible from toolbar MEMORY
- button or Ctrl+Shift+M. Includes full i18n coverage across all 9 locales.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [2.5.0] - 2026-03-29

### Features

- feat(self-heal): wire LLM-guided normalizer repair with iterative feedback loop

### Other

- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [2.4.0] - 2026-03-29

### Features

- feat(search): add CodeMirror anchor-based search position tracking
- feat(fileops): add context-aware Ctrl+Z, Move op, and EventBus integration
- feat(git): add directory-level git status aggregation in FileTree
- feat(analytics): add 30-day sparkline to API value hero card
- feat(analytics): add quota reset countdown to AnalyticsBar

### Other

- as absolute CodeMirror offsets. As the user edits the document, CM6
- maps positions through change sets so they stay correct after insertions
- and deletions. FindInFiles dispatches setAnchors when results are ready
- for the active file, and uses pendingAnchorIndex for precise navigation
- rather than falling back to the (now-stale) original line number.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- inside .cm-editor the event passes through for native editor undo. Adds
- Move operation type with undo support. All file-ops commands (delete,
- undo, create, rename, move) now publish fileop:execute / fileop:undo
- events to the EventBus.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- a dot badge when any descendant has changes. Adds 'copied' and 'ignored'
- status handling to cover all 7 git status types.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- recorded cost_usd or a blended token-rate estimate) and render a 120×30
- SVG polyline sparkline next to the value multiplier in the hero banner.
- Always fetch at least 30 days of daily history regardless of the selected
- period so the sparkline always has full coverage.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- defaults) instead of hardcoding, and display a visible "↺ Xh Ym"
- countdown next to the pace-delta metric showing time until quota reset.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [2.3.0] - 2026-03-29

### Features

- feat(circuit): complete circuit breaker wiring with EventBus state events
- feat(updater): publish update-available events to EventBus
- feat(normalizer): auto health checks + versioned backups with retention
- feat(state): add terminal state persistence with debounced auto-save
- feat(registry): wire node palette from get_node_types, add workflow validation
- feat(pty): add kill-all-ptys on app shutdown (Rust + frontend)
- feat(pty): wire reconnection with exponential backoff and UI overlay
- feat(slots): wire ModelSlotRegistry into transport, workflow engine, and frontend adapter
- feat(comms): add comms adapter methods, batch dispatch, and Zod schemas
- feat(comms): wire AgentCommsBus into WorkflowEngine for edge-implicit messaging
- feat(memory): replace v1 AgentMemoryStore with v2 in WorkflowEngine
- feat(memory): add v2 adapter methods, batch dispatch, and Zod schemas

### Other

- state pairs so callers can detect transitions. In transport send(), publish
- transport:circuit-state events on all transitions — at spawn-failure site
- (sync path), at CLI-completion site (async task), and at successful spawn.
- Register the transport:circuit-state channel as frontend-visible in lib.rs.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- EventBus when a CLI version change is detected post-update. Add
- updates.auto_check setting (default true) that skips the entire update
- cycle when disabled. Register the lifecycle:update-available channel as
- frontend-visible in lib.rs.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- stored in NormalizerHealth and published to normalizer:health EventBus
- channel (frontend-visible).
- 
- W2.12: backup_with_retention (max 20) on startup and reload_normalizers;
- version ID published to normalizer:version-created EventBus channel.
- 
- Both channels registered in lib.rs setup(). Fixed unused-import warning
- in batch.rs (pre-existing HiveNodeRegistry import).
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- and TypeScript, saves active PTY terminals on lifecycle events and project
- switches, restores them on load via a custom event to TerminalManager, and
- auto-saves project state every 30s via setInterval.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- via Builder::build().run(callback) and adds a frontend backup call to
- killAllPtys() inside the onWindowClose handler in +page.svelte.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- wires handlePtyExit in Terminal.svelte with 10-attempt 1s→30s×2 backoff,
- shows a corner overlay during reconnect, and adds Zod schema + mock stub.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- stack. Transport send() now resolves through LayeredSettings then
- ModelSlotRegistry when no explicit model is provided. WorkflowEngine
- resolves node LLM values through slots when the config value is a slot
- name. Frontend adapter exposes getModelForSlot, setModelSlot, and
- listModelSlots with batch dispatch and Zod validation.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- commsGetTopicMessages, commsGetBroadcastMessages, commsSweep,
- commsClearWorkflow). Implement in TauriAdapter via enqueue(), add batch
- dispatch arms, Zod validation schemas, and mock adapter stubs.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Direct channel in on_node_completed and spawn_single_node. Add Broadcast
- messages for HiveInspector visualization. Clean up CommsBus on workflow
- stop/finalize. Register comms:message_published EventBus channel.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- spawn_single_node (load) and on_node_completed (save). Add memory:added
- and memory:evicted EventBus events. Mark v1 module as retained for
- migration only.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- memory_list, memory_get) into the frontend adapter layer with batched IPC
- dispatch, Zod response validation, and mock stubs for testing.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [2.2.0] - 2026-03-29

### Features

- feat(permissions): add deny session/project scopes, reuse sessionId for resume-capable replay

### Other

- expand pattern: "Deny..." reveals once/session/project scope buttons. The
- handleApproveTools path in ChatView already passes the existing sessionId
- (not a new UUID), enabling the transport layer's resume_args_template
- path for --resume-capable CLIs.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [2.1.0] - 2026-03-29

### Features

- feat(permissions): add countdown timer with auto-deny on timeout to PermissionRequestBlock

### Other

- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [2.0.0] - 2026-03-29

### Features

- feat(permissions): redesign PermissionRequestBlock for per-tool approval with 4 scopes
- feat(permissions): add permission adapter methods with batch dispatch and Zod schemas
- feat(permissions): route Project scope to PolicyFile instead of PermissionMemory
- feat(permissions): wire engine into transport, replace inline trust checks, add audit events
- feat(permissions): add EvaluationResult, integrate Layer 3 + Layer 5 in evaluate()
- feat(permissions): add PolicyFile with TOML parsing, regex patterns, caching

### Other

- refactor(permissions): remove sessionApprovedTools, use PermissionMemory via adapter
- recorded through adapter.recordPermissionDecision from PermissionRequestBlock.
- handleSend and handleApproveTools simplified to use only configAllowedTools.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Allow project / Deny buttons. Decisions are recorded via adapter.recordPermissionDecision
- and decided rows are dimmed. ChatMessages passes sessionId and adapter through.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs: add W2.1 permission engine wiring implementation plan
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs(W2.1): pure evaluate(), regex patterns, single source of truth, pre-loaded policy
- - Pattern matching via compiled regex (no substring false positives)
- - Project scope in permissions.toml only (not PermissionMemory)
- - Policy file pre-loaded at startup + fs event reload
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs: add W2.1 permission engine wiring design spec
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.9.1] - 2026-03-28

### Other

- docs: add W2.11-W2.14 (normalizer health, versioning, CLI updater, circuit breaker) + Phase 5 backlog
- - W2.12: normalizer_version auto-backups + retention
- - W2.13: cli_updater user notification + approval
- - W2.14: circuit_breaker wire into transport
- - Phase 5 backlog: self_heal.rs, W1.10 dead code cleanup (deferred)
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.9.0] - 2026-03-28

### Features

- feat(batch): wire AbortController and explicit batch() at critical call sites
- feat(batch): add transparent IPC batching with dedup, abort, Zod validation, and explicit batch API
- feat(batch): add Zod schema registry for batch result validation
- feat(batch): add batch_invoke command with parallel dispatch, timeout, and EventBus telemetry
- feat(batch): add futures dep, BatchCall types, timeout constructor

### Bug Fixes

- fix(batch): add abort check in result loop, add Zod and abort-during-flight tests
- fix(batch): use extract_opt for respectGitignore, extract BATCH_CALL_TIMEOUT constant

### Other

-   stale requests when user navigates to a different file or project root changes
- - SearchPalette: AbortController on readFile, abort on palette close to prevent
-   stale file content from landing after the palette is dismissed
- 
- No explicit batch() call sites added — project-switch state (getProjectState,
- sessionList, getAppState) is not co-located in any single component; the
- existing auto-batcher in TauriAdapter covers sequential restoreSession reads.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- refactor(analytics,workflow,engine,settings): extract _inner functions for batch dispatch
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- refactor(session,app_state,shadow): extract _inner functions for batch dispatch
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- refactor(fs): extract _inner functions for batch dispatch
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- ReasonanceError, and the new commands/batch module with BatchCall types,
- BatchCallResult, and extract/extract_opt JSON helpers.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs: add W1.11 IPC batching implementation plan
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs(W1.11): add cancellation, dedup, and timeout to IPC batching spec
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs: add W1.11 IPC batching design spec
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.8.0] - 2026-03-28

### Features

- feat(storage): add transaction semantics to StorageBackend + wire SessionStore
- feat(perf): parallelize startup, add timing instrumentation and baseline recording
- feat(storage): add append/read_stream + migrate/rollback to StorageBackend trait
- feat(errors): add From<String>, workflow/serialization/transport constructors
- feat(settings): migrate Settings UI to use LayeredSettings adapter
- feat(settings): wire frontend adapter methods + project root hook
- feat(settings): add Tauri commands for layered settings CRUD
- feat(weak-refs): wire periodic sweep, document remaining HashMap patterns
- feat(signal): replace frontend polling with EventBus-bridged signals
- feat(signal): add EventBus bridge and coalescing documentation
- feat(event-bus): dual-emit workflow events through EventBus v2 channels
- feat(event-bus): wire new EventBus into transport (dual-bus coexistence)
- feat(event-bus): add new EventHandler subscriber implementations (history, session writer)
- feat(event-bus): add AsyncEventHandler trait, backpressure, and mixed subscriber support
- feat(file-ops): initialize FileOpsManager when project root changes
- feat(file-ops): add delete with undo to FileTree context menu and Delete key
- feat(file-ops): add file operations adapter methods (delete, undo, create, rename)
- feat(state): wire app state persistence to mount/close/switch lifecycle
- feat(state): add app state persistence types, adapter methods, and utility helpers

### Bug Fixes

- fix(clippy): reduce type complexity in InMemoryBackend with type alias
- fix(event-bus): restore frontend event compatibility for agent-event listener
- fix(event-bus): keep subscriber Arcs alive via managed state, dual-publish stderr, subscribe writer to errors
- fix(subscribers): revert accidental transport formatting, release lock before I/O
- fix(event-bus): correct backpressure to track pending async events, restore read lock, emit dropped events
- fix: resolve all clippy warnings (Default impls, collapsible replace, dead_code annotations)
- fix(tests): add scrollIntoView and CSS.escape polyfills for jsdom
- fix(tests): add ResizeObserver polyfill, fix DiffView assertions, update mock adapter
- fix: prevent Hive event listener leaks with proper cleanup
- fix: critical fixes (gemini TOML, about version, normalizer done rules, CSP unsafe-eval)

### Other

- refactor: extract shared utilities, remove dead code, add CI pipeline
- - Replace inline focus traps with shared trapFocus (WorkspaceTrust, ProjectDisconnected)
- - Add validatePermissionLevel to config-parser, DRY permission validation
- - Remove dead code: a11y-focus.ts (FocusManager), OpenFile interface, isRTL export
- - Add GitHub Actions CI workflow (frontend + Rust checks)
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs: add W1.9 transactions implementation plan
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs: add W1.8 perf baselines implementation plan
- 
- refactor(storage): wire SessionStore to StorageBackend
- All methods are now async. Metadata uses put/get, events use
- append/read_stream, index uses put/get. SessionManager accepts
- Arc<dyn StorageBackend> instead of directory path. Tests use
- InMemoryBackend.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- refactor(storage): wire AnalyticsStore to StorageBackend
- The store now uses async append/read_stream for metrics persistence,
- with an in-memory cache for sync query access. Collector's on_event
- uses block_in_place for the sync-to-async bridge when flushing
- completed sessions.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- get_version) on JsonFileBackend, using the existing safe_append and atomic_write
- helpers. Adds 4 integration tests covering all new methods.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs: add W1.6 storage abstraction implementation plan
- migrate/rollback, then wire SessionStore and AnalyticsStore as real
- consumers.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- refactor(errors): migrate workflow_engine and workspace_trust to ReasonanceError
- with Result<T, ReasonanceError>, using typed constructors (not_found, workflow,
- internal) and eliminating 6 dangerous .unwrap() calls on HashMap lookups.
- Simplify commands/engine.rs callers to use ? directly. Change workspace_trust
- folder_info to return ReasonanceError.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- refactor(errors): migrate storage/data/command modules to ReasonanceError
- across analytics, normalizer, normalizer_version, capability, discovery,
- and all command modules. Also migrated many supporting modules (file_ops,
- config, shadow_store, model_slots, theme_manager, etc.).
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- refactor(errors): migrate leaf modules to ReasonanceError
- resource_lock, logic_eval, settings/mod, and agent_memory.
- Update commands/settings to use ? instead of map_err(internal).
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs: add W1.5 structured errors implementation plan
- functions across 10 files) and fix 14 dangerous unwraps in production
- code. Bottom-up by dependency order.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- LayeredSettings backend via adapter.getAllSettings(), falling back to
- Svelte store values. LLM configs remain on readConfig/writeConfig.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- interface, TauriAdapter, and mock adapter. Wire set_project_root to
- call LayeredSettings::set_project_root() for loading project-level
- and workspace-level settings overrides.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Tauri commands with JSON↔TOML conversion. Add LayeredSettings::set()
- for per-layer value updates with automatic re-resolve. Register
- LayeredSettings in Tauri managed state.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- on both transport sessions and PTY instances, removing entries with
- no external strong refs. All remaining Arc<Mutex<HashMap>> patterns
- documented with justification (bounded, no lifecycle, different
- semantics).
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- refactor(pty): migrate instances to TrackedMap with weak handles
- Session-level locking now allows concurrent access to different PTYs.
- project_map stays as plain HashMap (documented: small, bounded, no
- lifecycle tracking needed). Add instances_map() accessor for periodic
- sweep wiring.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- refactor(transport): migrate sessions to TrackedMap with weak handles
- Arc<Mutex<TrackedMap<String, AgentSession>>>. Spawned tasks now hold
- an Arc to their specific session instead of the whole map, reducing
- lock contention. Also add Borrow-based get/remove to TrackedMap for
- HashMap-compatible ergonomics (&str for String keys).
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Rust setup, bridge them to EventBus, and replace frontend setInterval
- patterns with Tauri event listeners.
- 
- - TerminalManager: listen('lifecycle:sweep') replaces setInterval
- - updater.ts: listen('lifecycle:update-check') replaces setInterval
- - Both lifecycle channels now frontend-visible via TauriFrontendBridge
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- an EventBus channel via a spawned tokio task. Document that tokio::watch
- naturally coalesces rapid updates (no additional debouncing needed).
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- bench(event-bus): add Criterion benchmark for publish throughput
- and 10 sync-only) using noop handlers; both benchmarks run well under
- 1μs per publish (~257ns and ~269ns respectively).
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- inner payload ({session_id, event}) matching what the frontend expects,
- and maps workflow:* channels to their legacy hive:// names for engine.ts.
- Also removes stale (v2) suffixes from subscriber log messages and IDs.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- refactor(event-bus): remove old transport/event_bus.rs, rename event_bus_v2 to event_bus
- system. The old AgentEventBus, FrontendEmitter, HistoryRecorder (v1),
- and SessionHistoryRecorder are deleted. All event publishing now goes
- exclusively through the channel-based EventBus with weak-ref subscribers.
- 
- Key changes:
- - Delete transport/event_bus.rs (old bus, traits, and all subscribers)
- - Rename event_bus_v2.rs to event_bus.rs, update all imports
- - Add subscribers/analytics.rs to bridge AnalyticsCollector to new bus
- - Convert AnalyticsCollector.on_event from trait impl to direct method
- - Remove old bus fields from StructuredAgentTransport and WorkflowEngine
- - Remove all dual-publish calls from stream_reader and transport/mod.rs
- - Remove all app.emit("hive://...") calls from workflow_engine.rs
- - Update SessionManager to use SessionHistoryWriter instead of old recorder
- - Update agent_get_events command to read from v2 HistoryRecorder
- - Rewrite all transport tests to use the new EventBus
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- app.emit("hive://...") calls. All 15 workflow event emit sites now
- publish through both the old Tauri emit path and the new EventBus v2
- channels (workflow:node-state, workflow:run-status, workflow:agent-output,
- workflow:permission-request). Registers the two new channels and wires
- the bus into WorkflowEngine during app setup.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- managed state so they outlive setup(). The EventBus holds Weak refs
- that would otherwise go dead immediately. Also dual-publish stderr
- error events to the v2 bus and subscribe SessionHistoryWriter to
- transport:error so errors are persisted to session history.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- receive every event during the migration period. The old bus continues
- working unchanged; stream_reader now dual-publishes to both buses using
- transport:event, transport:complete, and transport:error channels.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- their pre-fmt state (cosmetic-only changes from cargo fmt reverted).
- 
- Refactor SessionHistoryWriter::handle to drop the handles Mutex before
- calling store.write_metadata(), eliminating lock-held disk I/O.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- that work with the new EventBus channel-based pub/sub system:
- 
- - HistoryRecorder: sync EventHandler that stores AgentEvents in-memory
-   per session, extracted from generic Event payloads
- - SessionHistoryWriter: async AsyncEventHandler that appends events to
-   disk via SessionStore and periodically persists session metadata
- 
- These coexist with the old transport::event_bus subscribers until the
- full migration cutover in sub-task 1.6.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- refactor(event-bus): fix TOCTOU race, remove unsafe impls, clean up Channel construction
-   concurrent threads from both passing the processing check
- - I1: Remove unsafe impl Send/Sync for EventBus (all fields auto-derive)
- - I2: Remove dead Channel.name field and its #[allow(dead_code)]
- - I3: Extract Channel::new() constructor, replacing 4 duplicated inline constructions
- - I4: Capture pending_count once in dispatch() backpressure check to avoid
-   TOCTOU between the comparison and the log message
- - M2: Extract magic numbers into named constants (DEFAULT_CHANNEL_BUFFER,
-   MAX_DEFERRED_ITERATIONS, SLOW_HANDLER_THRESHOLD)
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- and mixed sync/async subscriber storage. This is the foundation for migrating
- all event consumers to the unified EventBus.
- 
- - Add AsyncEventHandler trait (via async-trait) with tokio::spawn dispatch
- - Add Subscriber enum wrapping both Sync and Async weak refs
- - Change EventBus::new() to take tokio::runtime::Handle for async spawning
- - Add subscribe_async(), register_channel_with_buffer(), drop_count() methods
- - Add per-channel publish_count tracking with configurable max_buffer_size
- - Add slow sync handler detection (>100ms warning)
- - Update lib.rs call site to pass runtime handle
- - All 8 existing tests updated and passing
- - 5 new tests: async handler, backpressure, mixed subscribers, drop_count, custom buffer
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- in FileTree with trash-based undo, context menu danger button, and Delete key handler.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- chore: data/config fixes (shortcut i18n, console.log, deep-link, file watcher, CSS)
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- chore: remove dead code (2 stores, 1 component, 3 schema files, ~25 unused exports)
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- in HiveCanvas.svelte onDestroy. Also deduplicate pty-data listeners
- with a Map to prevent accumulation on repeated agent-output events.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.7.0] - 2026-03-26

### Features

- feat(phase-4): add i18n pluralization via Intl.PluralRules
- feat(phase-4): add stale detection for search results
- feat(phase-4): add file operation undo with trash-based delete
- feat(phase-4): add git status icons in FileTree
- feat(phase-4): add auto-fold for single-child directory chains
- feat(phase-4): add API value hero banner to AnalyticsDashboard
- feat(phase-4): add pace delta (quota burn rate) to AnalyticsBar
- feat(phase-4): add DiffBlock conflict detection via context-line validation
- feat(phase-4): virtualize FileTree with flat list + scroll buffer

### Bug Fixes

- fix(phase-4): fix editor memory leak + optimize gutter to O(viewport)

### Other

- Added 6 plural keys across all 9 locales with proper forms per language — Arabic gets all 6
- CLDR forms, Chinese gets only 'other', others get one/other. Replaced inline ternary plurals
- in DiffBlock (hunk count) and ChatMessages (earlier messages load-more button).
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- file modified since search. Re-search button for stale results.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- .reasonance/.trash/, undo restores. Redo cleared on new operation.
- 5 Tauri commands: file_ops_delete, file_ops_undo, file_ops_record_create,
- file_ops_record_rename, file_ops_set_project. 8 tests.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- with color coding. 2s debounced refresh on fs-change.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Expand/collapse operates on final directory in chain.
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Hidden when multiplier ≤ 1.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Reset countdown. claude-lens formula adapted.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Conflict UI with Apply anyway / Dismiss options.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- GPU-accelerated translateY positioning. ARIA tree semantics preserved.
- Keyboard nav works on flat index. childrenCache cleaned on project switch.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- pre-split once on load, eliminating split('
') on every gutter frame.
- Gutter now iterates only visible viewport lines instead of all file lines.
- Add LRU eviction at 100 entries and clean up entries when tabs are closed.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.6.0] - 2026-03-26

### Features

- feat(phase-3): add multi-slot model selection with fallback chain
- feat(phase-3): add app state persistence across restart
- feat(phase-3): add HiveNodeHandler trait and node registry
- feat(phase-3): add channel-based inter-agent communication
- feat(phase-3): add SQLite + FTS5 agent memory with scoped search

### Other

- Per-provider configs. Tauri commands for frontend. 10 tests.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Per-project state isolation. JSON file persistence. 8 tests.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- NodeDescriptor for frontend palette. get_node_types Tauri command.
- 17 tests passing (all green).
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- since_id filtering. Replaces Vec mailbox. 10 tests.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Features: importance-based eviction, 3-level scoping (node/project/global),
- FTS5 full-text keyword search, WAL mode for concurrent reads. Old
- agent_memory.rs retained for backward compat. 16 tests.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.5.0] - 2026-03-26

### Features

- feat(phase-2): add PTY lifecycle management with orphan sweep
- feat(phase-2): add WebGL context loss recovery with canvas fallback
- feat(phase-2): add PTY reconnection with exponential backoff
- feat(phase-2): wire retry logic into transport with circuit breaker gating
- feat(phase-2): add three-state circuit breaker with fingerprint dedup
- feat(phase-1): add permission request timeout with auto-deny
- feat(phase-1): add PermissionMemory for per-tool approval decisions
- feat(phase-1): add symlink escape detection with resolve_safe_path
- feat(phase-1): add permission decision engine with 6-layer evaluation

### Other

- TerminalManager via adapter.sweepPtys(). Backend: list_active_ptys(),
- sweep_dead_ptys() (probe-resize heuristic), kill_all() for shutdown.
- New Tauri commands sweep_ptys and kill_all_ptys. Three unit tests.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- canvas renderer. Proactive check on window re-focus after 30s blur.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- pty_manager.rs. Adds reconnect_pty Tauri command to commands/pty.rs —
- kills the dead PTY and spawns a fresh one with the same shell and cwd,
- returning the new ID for the frontend to track. Backoff timing and
- custom-config tests all pass (477 total, 0 failures).
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- rejects requests when provider circuit is OPEN. Retryable spawn failures
- get exponential backoff per provider's retry policy. Non-retryable errors
- fail immediately. Async completion handler records success/failure in
- circuit breaker for CLI-level errors.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Per-fingerprint error dedup within 10s window using SHA-256. 13 tests.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- with ReasonanceError::Timeout. 3 test cases covering default config,
- decision-before-deadline, and auto-deny-on-expiry.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Tauri commands for frontend integration (record, lookup, list, clear).
- 10 test cases covering all scopes, consumption, clearing, and overwrite.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Prevents path traversal via symlinks or relative paths.
- Returns Security::PathTraversal error instead of PermissionDenied
- for clearer security violation classification. 7 new tests.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- policy file -> model config -> session memory -> default Confirm.
- 13 test cases cover all layers and priority ordering.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.4.0] - 2026-03-26

### Features

- feat(phase-0): add ErrorBoundary component for panel isolation
- feat(phase-0): add 4-layer settings (builtin > user > project > workspace)
- feat(phase-0): add Zod runtime validation at adapter boundary
- feat(phase-0): add atomic writes, safe JSONL append, and crash recovery
- feat(phase-0): add StorageBackend trait with InMemory and JsonFile backends
- feat(phase-0): add TrackedMap with WeakHandle for lifecycle tracking
- feat(phase-0): add Signal<T> wrapper for watch-based updates
- feat(phase-0): add general-purpose EventBus with deferred queue
- feat(phase-0): add performance baseline infrastructure
- feat(phase-0): add structured error types (ReasonanceError)

### Bug Fixes

- fix(phase-0): dead code cleanup — remove #![allow(dead_code)]
- fix(phase-0): eliminate TOCTOU window in transport session locking

### Other

- Delete truly dead RunStatusEvent struct from workflow_engine.
- Add targeted #[allow(dead_code)] with comments for:
- - Serde-deserialized struct fields (populated by TOML parsing)
- - Test-only convenience constructors on AgentEvent
- - Public API methods not yet called from production code
- - Roadmap features (retry, self-heal, capability cache)
- 
- All 428 tests pass, zero warnings remain.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 3-strike persistent error detection. ARIA role=alert.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 13 test cases covering merge semantics, layer precedence, and error handling.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Foundation for migrating all invoke() calls to validated versions.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- (truncates partial lines). 5 new test cases, 416 total passing.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- InMemoryBackend for tests, JsonFileBackend with atomic writes. 24 tests.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Merged the two separate lock scopes (check-status + set-active) into a
- single atomic block. Pre-capture resume_args template before dropping
- registry lock so CLI args can be built inside the session lock scope.
- Restore session status to Error on spawn failure.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 4 test cases. Replaces raw Arc<Mutex<HashMap>> pattern.
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- Replaces polling patterns. 4 test cases.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- recursion prevention, frontend visibility flag. 8 test cases.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- baselines.json for tracking metrics across phases.
- 
- Benchmark results (median):
- - json_serialize_event: 966 ns
- - json_deserialize_event: 931 ns
- - toml_parse_config: 6,110 ns
- - uuid_v4_generate: 78 ns
- - sha256_10kb: 4,704 ns
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- with a typed ReasonanceError enum. Adds thiserror derive, tagged JSON
- serialization, is_retryable(), severity(), From impls for io/serde/toml
- errors, and 11 unit tests. Migrated 29 files (16 commands, 8 core
- modules, theme_manager, project_manager, fs_watcher) with zero business
- logic changes.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.3.2] - 2026-03-25

### Bug Fixes

- fix: resolve production deployment issues across all platforms

### Other

-   install paths instead of relying on relative CWD path. Gracefully start
-   with empty registry if normalizers are missing (no more panic).
- - Bundle: add normalizers/* to tauri.conf.json resources so they ship
-   with macOS .app, Windows MSI, and Linux deb/rpm/AppImage bundles.
- - AUR: install normalizer configs to /usr/share/reasonance/normalizers/.
- - AUR-bin: fix .deb extraction (extract ar archive before data.tar.*).
- - WelcomeScreen: fix theme toggle using new loadBuiltinTheme API.
- - ProjectSidebar: always visible (not hidden when single project open).
- - App: add CSS for sidebar container so it renders with correct dimensions.
- - Fix forge.toml → llms.toml comment in config-bootstrap.ts.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.3.1] - 2026-03-25

### Bug Fixes

- fix: catch unhandled setTitle promise rejection on early webview init

### Other

- before the webview was ready, silently crashing the renderer and preventing
- the window from appearing.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.3.0] - 2026-03-25

### Features

- feat(multi-project): scope terminal tabs per project, preserve background buffers
- feat(multi-project): update page startup for multi-project lifecycle
- feat(multi-project): wire CLI path argument to open project
- feat(multi-project): update MenuBar with Recent projects and Close Project
- feat(multi-project): add ProjectAddMenu, QuickSwitcher, and DisconnectedDialog
- feat(multi-project): integrate sidebar into App layout with shortcuts
- feat(multi-project): add ProjectSidebar component
- feat(multi-project): add sidebar CSS variables to theme system (schema v2)
- feat(multi-project): extend adapter with project-aware methods
- feat(multi-project): Rust project manager with multi-project state
- feat(multi-project): tag agent sessions with projectId
- feat(multi-project): shim files.ts and terminals.ts to re-export from namespace layer
- feat(multi-project): add namespace layer with derived stores and action wrappers
- feat(multi-project): add sidebar summary and status derived stores
- feat(multi-project): add projects store index with re-exports
- feat(multi-project): add project registry store with lifecycle actions
- feat(multi-project): add project context type definitions

### Bug Fixes

- fix(multi-project): resolve remaining type errors in components and sidebar
- fix(multi-project): update test theme schema version to 2
- fix(multi-project): migrate test files to new store API
- fix: use matches! macro instead of match block (clippy)
- fix(multi-project): resolve source component type errors
- fix(multi-project): remove re-created conflicting projects.ts
- fix(multi-project): remove conflicting projects.ts file (shadowed projects/ directory)
- fix(multi-project): migrate all Writable callers to namespace actions

### Other

- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- the projects registry). Replaced direct .set() calls with registry-based
- setup via a shared setupTestProject() helper. Updated addOpenFile calls
- to use the new openFile(path, content) signature, added projectId to
- TerminalInstanceMeta objects, and removed addRecentProject tests.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs: update registry with multi-project components and data flows
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- allInstances across all projects so xterm.js buffers stay mounted when
- switching projects (hidden via display:none). Tab bar remains
- project-scoped through the existing terminalInstances shim.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- - Add reasonance:closeProject listener: kills processes, removes from backend + store
- - Add cli-open-project Tauri event listener to add projects from CLI
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- of the non-existent updateProjectRoot export from the projects/ namespace store.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- killProjectProcesses methods to TauriAdapter to invoke the new Rust backend commands.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- roots in the backend. Extend PtyManager with project_map tracking so
- PTY instances can be associated with and bulk-killed per project.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- activeFilePath, projectRoot, activeInstanceId) with namespace action
- functions (updateFileContent, updateFileState, setActiveFile, addProject,
- setActiveTerminal). Remove addRecentProject usage — handled automatically
- by addProject. Update session restore to use new project/file APIs.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- .set() directly must migrate to action functions. addRecentProject removed
- (handled by registry lifecycle).
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- 
- 
- 
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs: add multi-project sidebar design spec and implementation plan
- Covers namespace layer, session persistence v2, theme system updates,
- and project sidebar component.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.2.0] - 2026-03-25

### Features

- feat(ci): add reasonance-bin AUR package + decouple aur-publish from build

### Other

- - aur-publish (source) now runs right after version bump, parallel to build
- - aur-publish-bin runs after release, when binary artifacts are available
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.1.1] - 2026-03-25

### Other

- docs: bump compliance docs version to 1.1.0
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- docs: update README, VPAT, EN 301 549 to v1.0.2 + add docs/compliance
-   find in files, help panel, session management, provider health,
-   image drop, welcome screen) and dedicated accessibility section
- - Update VPAT 2.4 and EN 301 549 from v0.11.0 to v1.0.2
- - Resolve TerminalManager "+" button known issue (now has i18n aria-label)
- - Upgrade EN 301 549 Chapter 12 to "Supports" (in-app help panel)
- - Move compliance docs to docs/compliance/ (tracked in git)
- - Keep docs/audit/ in .gitignore (generated artifacts)
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.1.0] - 2026-03-25

### Features

- feat(a11y): WCAG Phase 1B-3 — new functionality for keyboard trap, drag alternatives, status messages, non-color indicators, and aria-labels

### Other

- 4.1.3 (appAnnouncer for file save/theme/session/workflow events), 1.4.1 (DiffView headings,
- LogicNode state label), and 4.1.2 (aria-label on 7 title-only elements + permission badge).
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.0.2] - 2026-03-25

### Bug Fixes

- fix(ci): handle orphan tags from failed runs + force-push tags

### Other

- the next run would fail with "tag already exists". Now uses
- git tag -f to overwrite orphan tags.
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [1.0.1] - 2026-03-25

### Bug Fixes

- fix(ci): every push to main triggers a release + AUR update
- fix(tests): update theme tests for AAA color values and new layer scale
- fix: resolve all svelte-check errors and warnings (10→0 errors, 15→0 warnings)

### Other

- - Every push = new version (never skip release)
- - Default bump is patch; feat: → minor; breaking → major
- - Rollover versioning: patch >9 bumps minor, minor >9 bumps major
-   (e.g., 0.0.9 → 0.1.0, 0.9.9 → 1.0.0)
- - Force push with no new commits still triggers patch release
- - AUR publish now runs on every release (was being skipped)
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- - accent: #3b82f6 → #8ab8ff (AAA contrast)
- - toolbar-height: 44px → 52px (AAA target size)
- - layer-modal: 4 → 2000 (realistic z-index scale from Phase 0)
- - layer-toast: 5 → 5000 (same)
- 
- These broke CI builds on all platforms.
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- 
- - Settings: simplify openFileDialog return type (string
- - HiveCanvas: add required version/schemaVersion to Workflow init
- - ThemeEditor: $derived → $derived.by, remove () invocation
- - ThemeJsonView: remove invalid autocorrect HTML attribute
- - Toast: remove deleted component imports from aria.test.ts, delete Toast.test.ts
- - theme.test.ts: replace Node.js fs/path with Vite JSON import
- 
- Warnings fixed:
- - HiveInspector: associate 4 labels with controls via for/id
- - ThemeStartDialog + ThemeEditor: add tabindex="-1" to dialog roles
- - AgentNode/ResourceNode/LogicNode: fix $state prop capture with $effect sync
- - ThemeJsonView: initialize $state to empty, let $effect populate
- - StatusBar: add svelte-ignore for stopPropagation-only click handlers
- - Settings: fix orphaned CSS selector .provider-section-header h3 → legend
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>



## [0.17.0] - 2026-03-25

### Features

- feat(a11y): WCAG Phase 1B-2 — ARIA attributes, semantic HTML, i18n lang

### Other

- 
- Icon buttons (4.1.2):
- - Add aria-labels to 11 icon-only buttons (HiveControls ×5, FileTree ×2,
-   WelcomeScreen ×4) that only had title attributes
- 
- Decorative icons (1.1.1):
- - Add aria-hidden="true" to StatusBar progress symbols and notification
-   type icons (6 instances) — adjacent text provides meaning
- 
- EditorTabs restructuring (4.1.2, 1.3.1):
- - CRITICAL: Fix nested interactive violation — buttons (save/close) moved
-   from inside role="tab" to sibling position (.tab → .tab-wrapper/.tab-label)
- - Move role="tablist" to .tabs-scroll for correct ARIA ownership
- - Add tabpanel wrapper with aria-labelledby in +page.svelte
- - Add sanitizeId() utility for valid HTML IDs from file paths
- - Fix keyboard handler to use .closest('[role="tablist"]')
- - Update a11y tests for new structure
- 
- Settings (1.3.1, 3.3.1):
- - Convert 6 sections to fieldset/legend with CSS reset
- - Add aria-invalid + aria-describedby on command/endpoint inputs
- - Add role="alert" to import error, aria-hidden to connection test icons
- - Add aria-pressed to theme selection buttons
- 
- ThemeEditor (4.1.2):
- - Add aria-pressed to mode toggle buttons (theme/modifier)
- 
- i18n (3.1.1):
- - Update document.documentElement.lang on locale change
- 
- 0 new svelte-check errors. All verification checks pass.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>



## [0.16.0] - 2026-03-25

### Features

- feat(a11y): WCAG Phase 1B-1 — target size, focus, text spacing, link underlines

### Other

- 
- Target size (2.5.8 AA / 2.5.5 AAA):
- - Global rule in app.css: button/input/select/textarea get
-   min-height: var(--interactive-min, 24px) — 44px in default themes
- - Fix 4 min-height:unset overrides (Toolbar, AnalyticsDashboard ×3)
- - Add min-width to 6 icon-only buttons (EditorTabs, FileTree, HiveControls,
-   TerminalToolbar, TerminalManager)
- - EditorTabs container height now uses var(--interactive-min)
- - Verify global rule covers HivePanel, HiveInspector, SessionPanel buttons
- 
- Focus not obscured (2.4.11):
- - Add scroll-padding-top to StatusBar .notif-flyout for sticky header
- 
- Text spacing (1.4.12):
- - Replace overflow:hidden → overflow:auto on 23 text truncation selectors
-   in 19 files, including 3 parent-fix cases (EditorTabs, TerminalManager,
-   FileTree)
- 
- Link underlines (1.4.1):
- - MarkdownPreview and ResponsePanel: text-decoration: none → underline
- - App.svelte skip-link:focus: add underline
- 
- 0 new svelte-check errors. All verification checks pass.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>



## [0.15.1] - 2026-03-25

### Bug Fixes

- fix(a11y): correct oklch contrast values + replace accent backgrounds with accent-btn

### Other

- - Bump dark theme oklch values verified via Playwright browser rendering:
-   text-disabled/placeholder/state-idle: 0.70→0.73, state-idle-text: 0.72→0.74,
-   state-retrying: 0.72→0.75, border-disabled: 0.52→0.54
- - Fix light theme accent from #174990 to #16488e (7.09:1 on tertiary vs 6.97)
- - Replace background: var(--accent) → var(--accent-btn) in 25 components
-   where white text appears on accent background (dark theme accent #8ab8ff
-   is too light for white text at 2.0:1, accent-btn #1e40af gives 8.7:1 AAA)
- - Keep 4 decorative accent backgrounds (progress bars, markers, divider)
- - All oklch contrast verified via Playwright canvas pixel extraction
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>



## [0.15.0] - 2026-03-25

### Features

- feat(a11y): upgrade default themes to WCAG 2.2 AAA contrast + 44px target size

### Other

- pair meets 7:1 (AAA normal) or 4.5:1 (AAA large) contrast. Borders and
- focus indicators meet 3:1 non-text contrast. Add --interactive-min: 44px
- for AAA target size (2.5.5). Align fallback-theme.ts and fix its var()
- reference in --overlay-border.
- 
- Dark theme: lighter accent (#8ab8ff), success (#22c55e), danger (#fa8080),
- warning (#d4a020), border (#7a7a7a), secondary/muted text lightened.
- Light theme: darker accent (#16488e), success (#096028), warning (#6e4c00),
- danger (#a51a1a), border (#808080), disabled/placeholder darkened.
- Both: toolbar 52px, statusbar 52px, interactive-min 44px.
- 
- 67/67 contrast checks pass. 0 new svelte-check errors.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- refactor(theme): separate theme/app CSS — wire all hardcoded values to theme variables
- and fallback. Replace ~110 hardcoded rgba/hex/z-index values in ~35 components
- with var() references, making every visual aspect fully themeable.
- 
- Changes:
- - Add shadows (sm/md/lg/overlay), overlays (bg/border), highlights (subtle/hover/diff/terminal-selection) sections
- - Add --font-size-xs to typography, redesign z-index layers from 0-5 to realistic scale (0-9999)
- - Replace overlay backgrounds in 8 dialogs with var(--overlay-bg)
- - Replace box-shadow in 5 components with var(--shadow-*)
- - Replace highlight/diff rgba in 6 components with var(--highlight-*/--diff-*)
- - Wire terminal selection to var(--terminal-selection) via getToken()
- - Replace z-index literals in 21 components with var(--layer-*)
- - Strip ~165 dead hex fallbacks from var() calls in 20 components
- - Fix --color-error → --danger, remove --font-size-xs fallbacks
- - Update theme-types.ts (14 sections), theme-schema.json, all theme JSON files
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>



## [0.14.0] - 2026-03-24

### Features

- feat(theme): add Elegant Dark theme — modern, clean and refined
- feat(theme): add theme import and selection to Settings
- feat(theme): add import/export functionality to theme editor
- feat(theme): add visual theme editor with all sub-components
- feat(theme): add color harmony utility for family suggestions
- feat(theme): add WCAG contrast ratio checker
- feat(theme): wire Tauri persistence, hot reload, and editor theme linkage
- feat(theme): add file watcher for user themes directory
- feat(theme): add Rust theme manager with Tauri commands
- feat(theme): wire JSON theme engine into app startup and menu
- feat(theme): rewrite theme store with JSON engine, modifiers, system listeners
- feat(theme): add theme engine with extract, merge, inject logic
- feat(theme): add JSON theme validator with tests
- feat(theme): add hardcoded fallback theme for emergency recovery
- feat(theme): add JSON Schema for theme validation
- feat(theme): add modifier JSON files (enhanced-readability, high-contrast, reduced-motion)
- feat(theme): add built-in light theme JSON
- feat(theme): add built-in dark theme JSON
- feat(theme): add TypeScript type definitions for theme system
- feat(ci): auto-publish to AUR after release
- feat: auto-add REASONANCE IDE as co-author on commits
- feat(hive): interactive canvas nodes, memory injection, gitignore cleanup
- feat(hive): add capability validation on canvas edge connections
- feat(hive): add live agent output log and HivePanel terminal tab
- feat(hive): add permission enforcement for workflow execution
- feat(hive): wire real-time event system with hive:// namespace
- feat(discovery): add OpenAI-compatible probe and custom agent registration
- feat(hive): add agent memory persistence with FIFO eviction
- feat(hive): add resource locking with reader/writer exclusion
- feat(hive): add Rhai expression evaluator for Logic nodes
- feat(hive): schema & data model completion for workflow store

### Bug Fixes

- fix(theme): prevent race condition reverting user theme selection
- fix(theme): load user themes from disk when not found in built-in registry
- fix(i18n): add missing translations for theme import, modifiers, and theme editor
- fix(ci): push version commit to main to prevent tag conflicts
- fix(tests): update StatusBar and SearchPalette tests to match current components
- fix(ci): stop release action from pushing commits to main
- fix: add missing vitest import in tests/setup.ts
- fix(hive): resolve Svelte 5 $state rune conflicts, update registry
- fix(hive): harden Rhai sandbox, fix dead code and started-list leak
- fix(hive): use union types for permissionLevel/persist, doc timeout semantics

### Other

- chore: sync version files to v0.13.0
- can cleanly create v0.14.0.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- was being reverted by stale debounced reloads from the filesystem watcher.
- 
- Two fixes:
- - Move debounce timer to module scope so loadBuiltinTheme can cancel it,
-   preventing queued reloads from firing after an explicit theme switch
- - Add generation counter to discard in-flight async loads superseded by
-   a newer loadBuiltinTheme call
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- refactor(theme): remove Elegant Dark from built-in, keep as importable JSON
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- - hexToRgb: Converts hex color codes to RGB values
- - relativeLuminance: Calculates relative luminance per WCAG standards
- - contrastRatio: Computes contrast ratio between two colors
- - wcagLevel: Determines WCAG compliance level (AAA/AA/FAIL)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- test(theme): add migration verification tests
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- refactor(theme): remove hardcoded CSS variables, now powered by JSON engine
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- subsequent runs calculate the correct next version.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- and aria-label assertions.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- .SRCINFO, and pushes automatically after each GitHub Release.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- chore: trigger release build
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- Reasonance. Every commit made from within the app will include the
- REASONANCE IDE co-author trailer, making it appear as a contributor
- on GitHub.
- 
- Co-Authored-By: REASONANCE IDE <270735277+REASONANCE-IDE@users.noreply.github.com>
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Co-Authored-By: Reasonance <reasonance@users.noreply.github.com>
- 
- - Make ResourceNode editable: kind toggle, path input, access mode
- - Make LogicNode editable: kind selector, rule textarea
- - Wire onchange callbacks from FlowNode wrappers through HiveCanvas store
- - Fix memory injection: load entries and inject into PTY prompt on spawn
- - Add get_agent_memory Tauri command for frontend memory access
- - Upgrade HiveInspector: capabilities, memory config, retry, delete node
- - Activate HIVE toolbar button with toggle
- - Embed HiveCanvas in editor area instead of fullscreen overlay
- - Initialize empty workflow on HIVE open
- - Remove framework/specs/audit files from git tracking (.gitignore)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- local/remote divergence that requires manual rebase before every push.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- refactor: rename swarm→hive, add Skipped state and output routing
- - Update stores, page layout, and CHANGELOG references
- - Add AgentState::Skipped with Idle→Skipped transition and cascade
- - Buffer PTY output per agent and route to successor nodes on success
- - Populate memory entries with input/output summaries from actual data
- - Add skippedNodeCount derived store, count skipped as completed
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- - Fix edge id type (ensure always string)
- - Update registry with new HIVE modules and functions
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- resources. Shows error toast on invalid connections.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Live log section in HivePanel with scrollable output
- - Hive tab in TerminalManager (visible when workflow loaded)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- directly, "supervised" emits approval request and waits, "dry-run"
- simulates success. Extract spawn logic into reusable spawn_single_node
- helper. Add approve_node Tauri command, pendingApprovals store, and
- permission badge in HiveCanvas toolbar.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Add structured NodeStateEvent/RunStatusEvent payloads
- - Emit hive://agent-output with PTY-to-node mapping
- - Add setupHiveEventListeners() for frontend store updates
- - Initialize listeners on HiveCanvas mount
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- OpenAI-compatible API servers, and register_custom_agent() for manual agent
- registration. Wire new register_custom_agent Tauri command.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- runs. Memory is loaded before agent spawn and saved on node completion,
- with configurable per-workflow or global storage and max-entry eviction.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Acquire locks before agent PTY spawn, release on completion
- - Roll back partial acquisitions on lock failure
- - Cleanup all locks on forced stop
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Remove dead _predecessors binding
- - Don't push Logic nodes to started list (they complete synchronously)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Logic nodes now parse their rule expression, evaluate it against
- predecessor output, and route execution to onTrue/onFalse branches,
- skipping the inactive branch.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- LogicNodeConfig serde renames, optional edge IDs, and v0→v1 migration.
- Includes 7 new tests (17 total passing). Updates frontend types to match.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- plan(hive): implementation plan for remaining HIVE platform features
- permissions, resource locking, agent memory, live log, capability
- validation, discovery engine completion, and final integration.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>



## [0.12.1] - 2026-03-24

### Bug Fixes

- fix(a11y): i18n all hardcoded English aria-label, title, placeholder strings
- fix(a11y): close button target sizes to minimum 24x24px (AA 2.5.8)
- fix(a11y): terminal-wrap role=tabpanel with aria-label
- fix(a11y): ResponsePanel link color to --accent-text for AA contrast
- fix(a11y): DiffBlock aria-expanded, visible diff prefix for color-blind users
- fix(a11y): StatusBar contrast — darker accent bg, raise opacity minimums

### Other

- chore: update README with missing features, fix WCAG claim, bump PKGBUILD to 0.11.0
-   Session Replay, Analytics Dashboard, Hive Canvas, Inline Updater, i18n,
-   Workspace Trust)
- - Correct "WCAG 2.1 AA compliant" → "Targeting WCAG 2.1 AA conformance" with
-   link to audit
- - Mark Hive Canvas as completed in roadmap
- - Bump AUR PKGBUILD version from 0.10.0 to 0.11.0
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: update WCAG matrix, VPAT, and EN 301 549 after AA conformance fixes
- previously blocking items in the Top Priority Fixes section are marked
- resolved. VPAT and EN 301 549 conformance levels updated for 1.4.1, 1.4.3,
- 2.1.1, 2.1.2, 2.4.3, 2.5.8, 3.3.2, 4.1.2, and related Chapter 4/11 clauses.
- Top Known Issues sections reflect only remaining minor items; Remediation
- Roadmap updated to show all prior items resolved.
- 



## [0.12.0] - 2026-03-24

### Features

- feat: command palette, editable files, inline updater, yolo trust bypass (v0.10.0)
- feat: i18n translations for workspace trust and permission switcher
- feat: reactive session suspension on trust revocation
- feat: trust level enforcement in transport send()
- feat: trusted workspaces section in Settings
- feat: config migration — default yolo, legacy permission cleanup
- feat: trust gate check before LLM session spawn
- feat: WorkspaceTrustDialog component with accessibility
- feat: frontend trust store and adapter methods
- feat: trust-aware permission args and read-only tool whitelist
- feat: add Tauri commands for workspace trust
- feat: add TrustStore backend for workspace trust

### Bug Fixes

- fix: resolve type narrowing error in PTY trust warning
- fix: remove red YOLO warning styling from StatusBar

### Other

- chore: bump version to 0.11.0
- 
- - Editor: editable with modified-line gutter markers, dirty tracking, Ctrl+S save with diff review
- - EditorTabs: native Tauri save dialog on close with unsaved changes
- - StatusBar: inline update indicator with install/later actions (replaces toast-based updater)
- - TerminalManager: simplified dropdowns, removed STREAMING badge, view mode toggle on active tab click
- - FileTree: reactive filesystem change listener for create/remove events
- - Transport: yolo mode bypasses workspace trust gate, always passes permission args
- - rules_engine: exists() now returns false for empty arrays/objects/null
- - +page.svelte: auto-accept file changes in yolo mode, save/saveAll handlers
- - ChatView: reactive $llmConfigs instead of get() for permission level
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- sends for blocked or not-yet-trusted workspaces regardless of frontend
- state, and uses trust-aware permission/tool args (read-only tools for
- ReadOnly trust level).
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- 
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- and list_workspace_trust as Tauri invoke_handler commands backed by TrustStore.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- SHA-256 path hashing, parent directory inheritance, broad-directory blocking,
- and expiration support. Wires TrustStore into Tauri state management.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>



## [0.10.0] - 2026-03-24

### Features

- feat: per-model permission system (yolo/ask/locked) replacing global yoloMode

### Bug Fixes

- fix: resolve CI test failures — tsconfig resolution and localStorage mock

### Other

- [release] v0.9.0 — per-model permissions, CI test fixes
- - Per-model permission system (yolo/ask/locked) replacing global yoloMode
- - Deny-then-replay flow with interactive tool approval UI
- - Persistent allowed tools configuration per LLM model
- - PermissionDenial event pipeline through normalizer
- 
- ### Bug Fixes
- - CI test failures: svelte-kit sync for tsconfig resolution
- - localStorage mock for jsdom/forks pool compatibility
- - Stale Toolbar test selectors after yoloMode removal
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- .svelte-kit/tsconfig.json required by Vite 8's oxc transformer.
- Add localStorage mock in tests/setup.ts for jsdom/forks pool compatibility.
- Update stale Toolbar test selectors after yoloMode removal.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- user with approve/deny UI), locked (deny, info-only). Includes deny-then-replay
- flow with new session ID, persistent allowed tools config, and normalizer
- pipeline for PermissionDenial events. Removes global yoloMode store.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- chore: sync Cargo.lock version to v0.8.0 release



## [0.8.2] - 2026-03-23

### Bug Fixes

- fix(a11y): WCAG AAA regressions — font-size minimum 14px, contrast 7:1

### Other

- 
- Font sizes:
- - Raise --font-size-sm from 0.75rem (12px) to 0.875rem (14px)
- - Remove all calc(--font-size-small - 1px) patterns (was 11px)
- - Remove hardcoded 11px fallbacks in AnalyticsBar and Settings
- - Fix app.html hardcoded 13px → 14px
- 
- Contrast (27 fixes across 16 files):
- - Replace color: var(--success/danger/warning/accent) with
-   --*-text AAA variants (7:1 ratio) for all text color uses
- - Remove hardcoded #ef4444 fallback in Settings
- - border-color/background uses left on base tokens (3:1 OK)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>



## [0.8.1] - 2026-03-23

### Bug Fixes

- fix: resolve all clippy warnings for CI green build

### Other

- - Use matches!() macro instead of match block (clippy::match_like_matches_macro)
- - Allow dead_code at crate level (scaffolded modules not yet wired)
- - Suppress clippy lints for too_many_arguments, large_enum_variant, etc.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>



## [0.8.0] - 2026-03-23

### Features

- feat(normalizer): content_blocks pipeline + transport resilience
- feat(design-system): final WCAG AAA audit — all checks passed
- feat(design-system): tokenize transitions + semantic borders
- feat(design-system): unify buttons — 44px minimum, ARIA complete
- feat(design-system): migrate spacing to 4px grid — semantic tokens
- feat(design-system): migrate typography to rem scale — 12px minimum
- feat(design-system): contrast audit — WCAG 2.x + APCA verified
- feat(design-system): replace all hardcoded colors with tokens
- feat(design-system): add layer manager store and focus trap utility
- feat(design-system): add all new CSS tokens — typography, spacing, layers, colors, borders, transitions
- feat(chat): add slash command menu with combobox ARIA pattern
- feat(chat): delete ChatHeader, add streaming metrics to footer
- feat(Toolbar): replace emoji buttons with accessible text buttons, add HIVE
- feat(TerminalManager): flat tab bar with model names and [+] provider dropdown
- feat(store): add turnCount to AgentSessionState

### Bug Fixes

- fix(a11y): WCAG 2.2 AAA improvements + tab bar bug fixes
- fix(review): address code review findings — outside-click, stale comments, dead field

### Other

- assistant message to emit multiple typed events (thinking, text,
- tool_use, tool_result) by iterating over content block arrays.
- 
- Also: force-stop stale active sessions on follow-up messages instead
- of rejecting them, set stdin to null for child processes, and make
- log level configurable in dev builds via RUST_LOG.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Zero hardcoded style values in components
- - Full contrast matrix (WCAG 2.x + APCA) verified
- - 44px minimum target size on all interactive elements
- - Layer system with focus trap, inert, Escape LIFO
- - Forced-colors, reduced-motion, prefers-contrast all tested
- - Enhanced readability mode verified
- - Fixed missing i18n keys in 7 locales
- 
- Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Functional animations have static fallbacks in reduced-motion.
- Toast auto-dismiss: never for errors/warnings. Zero flash.
- All border-radius enforced to 0 (brutalist). Borders use semantic
- tokens (--border-container, --border-separator, --border-interactive).
- 
- Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- (normal 44px / compact 32px). Icon-only buttons have aria-label.
- Toggle buttons have aria-pressed. Button groups use role=toolbar
- with arrow key navigation. Loading state with aria-busy.
- Cursor blink default off.
- 
- Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- between targets. Paragraph spacing on prose blocks. No fixed height on
- text containers.
- 
- Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- md(20px), lg(24px), hero(32px). Line-height per tier. Font-weight per
- tier with dark/light variants. Text measure 72ch on prose containers.
- 
- Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 4.5:1 large). Fixed light theme tokens: --text-muted, --accent-text,
- --success-text, --warning-text. Added --accent-btn/--danger-btn for
- button backgrounds (vivid hues cannot hit 7:1). APCA Lc values
- documented. Colorblind simulation: state colors use redundant
- indicators (icons + text labels).
- 
- Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- Agent state colors tokenized with shared utility (state-color.ts).
- Redundant visual indicators (icons) for WCAG 1.4.1.
- SVG icons migrated to currentColor. CodeMirror/xterm themes use
- factory functions reading CSS tokens via getComputedStyle.
- 
- Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- inert management, Escape LIFO, focus fallback chain, scroll lock.
- Portal div added to app.html. Duplicate-id guard on pushLayer.
- 
- Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- consumed by components. No visual changes.
- 
- Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: add design system consolidation implementation plan
- audit), typography, spacing, buttons, transitions, borders, enhanced
- readability, and final WCAG AAA audit.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: add design system consolidation spec — WCAG AAA
- a tokenized, WCAG AAA compliant design system. Covers typography
- (4px grid rem scale), spacing (semantic layers), layer management,
- OKLCH color architecture, buttons, transitions, and borders.
- 10-phase implementation plan with verification checkpoints.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Fix "+" button dropdown using click-outside handler instead of
-   svelte:window onclick (Svelte 5 event delegation issue)
- - Fix YOLO button position: always first in footer-left, never shifts
- - Uniform tab bar element sizing: all elements 24px min-height with
-   matching font-size and padding (tab, TERM toggle, status badge, +)
- 
- Part B — WCAG 2.2 AAA improvements:
- - Contrast: add AAA-compliant text tokens (--accent-text, --danger-text,
-   --success-text, --warning-text) for both dark and light themes
- - Light theme: fix --text-muted (#767676→#595959) and --text-secondary
-   (#525252→#404040) for 7:1 ratio on all backgrounds
- - Visual Presentation (1.4.8): max-width 70ch on chat text blocks,
-   paragraph spacing ≥ 1.5em
- - Help (3.3.5): aria-describedby on YOLO toggle, ViewModeToggle;
-   role="meter" on context bar with descriptive aria-label
- - Error Prevention (3.3.6): /clear slash command requires confirmation
- - Link Purpose (2.4.9): post-process generic link text ("here", "click")
-   with aria-label containing domain name
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Update stale ChatHeader comments in agent-session.ts to reference footer metrics
- - Make label field optional on TerminalInstance (display uses computedLabels)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- test: update tests for flat terminal store
- TerminalTab) with the new flat model (terminalInstances, activeInstanceId,
- TerminalInstance with provider field). Rewrite terminals.test.ts to cover
- addInstance, removeInstance, updateInstance, computedLabels, and activeInstance.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- refactor(Terminal): use flat store updateInstance helper
- 
- refactor(session): update save/restore for flat terminal store
- 
- refactor(StatusBar): use flat store activeInstance derived
- refactor(store): flatten terminalTabs → terminalInstances with computed labels



## [0.7.0] - 2026-03-23

### Features

- feat(ui): move YOLO toggle to chat input, add CLI dir permissions and CSP updates
- feat(transport): extract CLI session ID from stream for conversation resume
- feat(normalizer): add session_id_path getter to TomlConfig
- feat(transport): add set_cli_session_id method to AgentSession
- feat: API-only LLM support, extended CLI discovery, a11y and UI fixes

### Bug Fixes

- fix(stream): add configurable timeout to prevent hung sessions (default 5min)
- fix(events): show warning when old messages are pruned from memory
- fix(transport): reuse existing sessions for conversation continuity with --resume
- fix(stream): publish error+done events on I/O failure to unblock frontend
- fix(yolo): gate permission_args on YOLO toggle state, not hardcoded

### Other

- - Add permission_args to CLI normalizer configs for auto-approval flags
- - Allow read/write access to CLI local dirs (~/.claude, ~/.gemini, ~/.codex)
- - Update CSP to allow IPC, WebSocket, and wasm-unsafe-eval for Tauri v2
- - Simplify YOLO restart flow (auto-restart without confirmation)
- - Add chat.placeholder i18n key across all languages
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- 
- 
- 
- 
- 
- discovery with common install dirs, and merge newly found CLIs on every
- startup instead of skipping when config exists. Fix accessibility across
- components (tabindex, keyboard handlers, svelte-ignore directives),
- persist panel widths in localStorage, improve chat auto-scroll logic,
- and add YOLO restart confirmation with i18n support.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>



## [0.6.0] - 2026-03-23

### Features

- feat: new logo + fix window drag from top border
- feat: major app update — backend, frontend, i18n, analytics, and config overhaul
- feat(analytics): add global keyboard shortcuts for analytics
- feat(settings): add Provider section with connection testing and budget controls
- feat(analytics): add AnalyticsDashboard with KPI, insights, provider comparison, and trend
- feat: add AnalyticsDashboard component with full integration
- feat(analytics): add AnalyticsBar component with live metrics display
- feat: add AnalyticsBar component with live session metrics display
- feat(i18n): add analytics and provider settings keys for all 9 locales
- feat(analytics): add analytics store with live tracking, cache, and budget
- feat(analytics): add adapter methods, provider backend commands, and api_key_env config
- feat: add utility modules for tooltip, tween, bar-scale, grid-nav, provider-patterns
- feat(a11y): add motion, announcer, labels, and focus utilities
- feat(analytics): add locale-aware formatters with tests
- feat(analytics): add TypeScript types and model info data
- feat: add JSON fixtures and integration tests for all 4 providers
- feat: wire TOML capabilities to CapabilityNegotiator at app startup
- feat: add kimi, qwen, codex to discovery scan and builtin profiles
- feat: route new providers to dedicated state machines
- feat: add Codex state machine and TOML normalizer
- feat: add Qwen state machine and TOML normalizer
- feat: add Kimi state machine and TOML normalizer
- feat: add Gemini state machine and TOML normalizer
- feat: add tool_input_delta and block_stop rules to claude.toml
- feat: add shared accumulator module (TextAccumulator, ToolInputAccumulator, TimedFlush)
- feat: add incomplete field to AgentEventMetadata for timeout flush
- feat: add array index support to resolve_path for [N] syntax
- feat: frontend types, adapter methods, and capabilities store for Phase 6
- feat: wire Phase 6 modules — Tauri commands, managed state, setup hooks
- feat: add CapabilityNegotiator — feature detection with cache and workarounds
- feat: add self-heal flow — prompt generation and TOML extraction for LLM-driven normalizer repair
- feat: add NormalizerHealth — test case evaluator and health status derivation
- feat: add get_toml_source and reload_provider to NormalizerRegistry
- feat: add NormalizerVersionStore — backup, restore, rollback for TOML normalizers
- feat: add CliUpdater — version tracking and auto-update config per provider
- feat: add ActionableMessage with copy/retry/fork actions on hover
- feat: add ChatHeader with live metrics, integrate into ChatView
- feat: ContentRenderer handles all content types, ChatMessages routes by event_type
- feat: add DiffBlock (unified diff view) and FileRefBadge (inline file reference)
- feat: add ToolUseBlock — collapsible tool call display with input/result
- feat: add ThinkingBlock (collapsible) and ErrorBlock (severity badges)
- feat: upgrade TextBlock to render markdown via marked + dompurify
- feat: handle metrics events — update currentSpeed and elapsed in session store
- feat: add ViewModeToggle and integrate ChatView into TerminalManager
- feat: add ChatInput and ChatView — complete chat interface
- feat: add ContentRenderer and ChatMessages — event-to-message rendering
- feat: add TextBlock, CodeBlock, StreamingIndicator chat components
- feat: add agent events store — event stream with streaming state
- feat: add agent session store — session state management
- feat: add session and transport methods to Adapter interface
- feat: add AgentEvent TypeScript type definitions
- feat: add session persistence integration tests
- feat: add Tauri session commands — create, restore, list, delete, rename, fork
- feat: add SessionManager — session lifecycle with create, restore, fork, delete, finalize
- feat: add SessionHistoryRecorder — appends events to JSONL via event bus
- feat: add SessionStore — JSONL event persistence and metadata I/O
- feat: add SessionHandle types — persistent session record with fork support
- feat: add transport integration tests
- feat: add FrontendEmitter — bridges AgentEvents to Tauri event system
- feat: add Tauri transport commands — agent_send, agent_stop, agent_get_events, agent_get_session_status
- feat: add StructuredAgentTransport — core orchestrator with CLI spawning
- feat: add stream reader — tokio task for CLI stdout normalization
- feat: add AgentSession with lifecycle management
- feat: add AgentEventBus with subscriber pattern, filter, and history recorder
- feat: add RetryPolicy with exponential/fixed backoff
- feat: add transport types — AgentRequest, CliMode, SessionStatus, AgentCommand
- feat: add end-to-end integration tests for Claude normalizer pipeline
- feat: add NormalizerRegistry with TOML loader and Claude normalizer config
- feat: add NormalizerPipeline orchestrating Rules → State → Content stages
- feat: add Claude state machine for content block accumulation
- feat: add StateMachine trait and generic pass-through implementation
- feat: add Content Parser for code fences, diffs, and text detection
- feat: add Rules Engine with expression evaluator for normalizer DSL
- feat: add AgentEvent type system with constructors and serialization
- feat(analytics): add Tauri commands and wire AnalyticsCollector
- feat(analytics): add aggregation queries to AnalyticsCollector
- feat(analytics): implement AnalyticsCollector with event accumulation
- feat(analytics): add SessionMetrics types and AnalyticsStore persistence
- feat(analytics): add cache, duration, cost mappings to normalizer TOMLs
- feat(analytics): extract new metadata fields in normalizer pipeline
- feat(analytics): extend AgentEventMetadata with cache, duration, cost fields
- feat: add JSON fixtures and integration tests for all 4 providers
- feat: wire TOML capabilities to CapabilityNegotiator at app startup
- feat: add kimi, qwen, codex to discovery scan and builtin profiles
- feat: route new providers to dedicated state machines
- feat: add Codex state machine and TOML normalizer
- feat: add Qwen state machine and TOML normalizer
- feat: add Kimi state machine and TOML normalizer
- feat: add Gemini state machine and TOML normalizer
- feat: add tool_input_delta and block_stop rules to claude.toml
- feat: add shared accumulator module (TextAccumulator, ToolInputAccumulator, TimedFlush)
- feat: add incomplete field to AgentEventMetadata for timeout flush
- feat: add array index support to resolve_path for [N] syntax
- feat: frontend types, adapter methods, and capabilities store for Phase 6
- feat: wire Phase 6 modules — Tauri commands, managed state, setup hooks
- feat: add CapabilityNegotiator — feature detection with cache and workarounds
- feat: add self-heal flow — prompt generation and TOML extraction for LLM-driven normalizer repair
- feat: add NormalizerHealth — test case evaluator and health status derivation
- feat: add get_toml_source and reload_provider to NormalizerRegistry
- feat: add NormalizerVersionStore — backup, restore, rollback for TOML normalizers
- feat: add CliUpdater — version tracking and auto-update config per provider
- feat: add ActionableMessage with copy/retry/fork actions on hover
- feat: add ChatHeader with live metrics, integrate into ChatView
- feat: ContentRenderer handles all content types, ChatMessages routes by event_type
- feat: add DiffBlock (unified diff view) and FileRefBadge (inline file reference)
- feat: add ToolUseBlock — collapsible tool call display with input/result
- feat: add ThinkingBlock (collapsible) and ErrorBlock (severity badges)
- feat: upgrade TextBlock to render markdown via marked + dompurify
- feat: handle metrics events — update currentSpeed and elapsed in session store
- feat: add ViewModeToggle and integrate ChatView into TerminalManager
- feat: add ChatInput and ChatView — complete chat interface
- feat: add ContentRenderer and ChatMessages — event-to-message rendering
- feat: add TextBlock, CodeBlock, StreamingIndicator chat components
- feat: add agent events store — event stream with streaming state
- feat: add agent session store — session state management
- feat: add session and transport methods to Adapter interface
- feat: add AgentEvent TypeScript type definitions
- feat: add session persistence integration tests
- feat: add Tauri session commands — create, restore, list, delete, rename, fork
- feat: add SessionManager — session lifecycle with create, restore, fork, delete, finalize
- feat: add SessionHistoryRecorder — appends events to JSONL via event bus
- feat: add SessionStore — JSONL event persistence and metadata I/O
- feat: add SessionHandle types — persistent session record with fork support
- feat: add transport integration tests
- feat: add FrontendEmitter — bridges AgentEvents to Tauri event system
- feat: add Tauri transport commands — agent_send, agent_stop, agent_get_events, agent_get_session_status
- feat: add StructuredAgentTransport — core orchestrator with CLI spawning
- feat: add stream reader — tokio task for CLI stdout normalization
- feat: add AgentSession with lifecycle management
- feat: add AgentEventBus with subscriber pattern, filter, and history recorder
- feat: add RetryPolicy with exponential/fixed backoff
- feat: add transport types — AgentRequest, CliMode, SessionStatus, AgentCommand
- feat: add end-to-end integration tests for Claude normalizer pipeline
- feat: add NormalizerRegistry with TOML loader and Claude normalizer config
- feat: add NormalizerPipeline orchestrating Rules → State → Content stages
- feat: add Claude state machine for content block accumulation
- feat: add StateMachine trait and generic pass-through implementation
- feat: add Content Parser for code fences, diffs, and text detection
- feat: add Rules Engine with expression evaluator for normalizer DSL
- feat: add AgentEvent type system with constructors and serialization

### Bug Fixes

- fix: Svelte 5 reactivity bug in chat + full provider test coverage
- fix: chat functionality, UI consistency, and startup errors
- fix(settings): code quality fixes for provider settings
- fix(settings): spec compliance fixes for provider settings UI
- fix(analytics): fix $derived.by for insights, add reduced-motion CSS
- fix(analytics): address code quality issues in AnalyticsBar
- fix: use unicode ellipsis in i18n testing key
- fix(analytics): align TypeScript types with actual Rust backend structs
- fix(plan): address reviewer findings in Phase 7B plan
- fix: remove dead onRetry prop and add clipboard error handling
- fix: address code review — CliMode variants, event listener race, unused import
- fix: eliminate dual-lock in finalize_session and add event count reconciliation
- fix: stop() now kills child process via abort handle
- fix(plan): address reviewer findings in Phase 7B plan
- fix: remove dead onRetry prop and add clipboard error handling
- fix: address code review — CliMode variants, event listener race, unused import
- fix: eliminate dual-lock in finalize_session and add event count reconciliation
- fix: stop() now kills child process via abort handle

### Other

- locations (toolbar, welcome screen, about dialog, favicon, all Tauri
- bundle icons at high resolution).
- 
- Fix window drag not working from top border: use programmatic
- startDragging() API instead of relying solely on data-tauri-drag-region,
- and fix top-bar in WelcomeScreen intercepting mousedown events.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- uses reference equality and couldn't detect changes, so chat messages
- never appeared. Fixed by creating new Map + new array on every update.
- 
- Also: ChatView CSS flex fix, toolbar hidden in chat mode, improved
- stream reader logging, normalizer cleanup. Added 25 Rust end-to-end
- tests (Claude, Gemini, Kimi, Codex, Qwen) and 12 Vitest frontend tests
- covering reactivity, pruning, and session isolation.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
-   with stream-json in print mode)
- - Fix session ID mismatch: reuse frontend session_id instead of
-   generating a new one in transport
- - Fix provider not found: normalize provider name to lowercase
- - Fix session store: auto-create session directory on first event append
- - Fix startup error: skip file watcher when no project root is set
- - Fix error banner blocking window drag: move below titlebar, make
-   dismissible on click
- - Fix menu dropdown hidden by overflow:hidden on toolbar-left
- - Normalize all panel headers: consistent padding (8px 14px), border
-   (var(--border-width)), font-size (var(--font-size-tiny)), weight (800)
- - Set chat as default view instead of terminal
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- workflows, discovery, config), Svelte frontend (all components, chat, hive),
- internationalization (9 languages), build config, and CI/CD pipeline.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs(audit): add Phase 3 extended automated tests (30 total)
- RTL layout, German truncation, accessibility tree snapshot, and
- full-UI mock tests. 30 total Playwright tests, all passing.
- 
- Key findings:
- - WCAG 1.4.10 Reflow: passes at 320px
- - Zero ARIA landmarks confirmed across all tests
- - No prefers-contrast CSS rules
- - RTL/German pass on welcome screen (Tauri backend needed for full UI)
- - Tab reaches only 6 elements (titlebar + welcome + toast)
- - Screen reader sees error as first element
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs(audit): add extended Phase 3 automated testing
- 320px reflow, forced-colors, prefers-contrast, RTL layout,
- German locale truncation, accessibility tree snapshot.
- 
- Key findings:
- - Layout reflow passes at all zoom levels (WCAG 1.4.10)
- - No prefers-contrast CSS rules exist
- - Accessibility tree shows error as first element
- - RTL and German pass on welcome screen (need project-loaded testing)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs(audit): add Phase 3 live testing results
- targets, heading hierarchy, ARIA landmarks, reduced motion, adversarial).
- All 16 passed. Added Lighthouse audit (88/100 a11y) and bundle viz.
- 
- Key live findings:
- - 2 axe-core serious violations (contrast, missing title)
- - Zero ARIA landmarks in app
- - Tab only reaches 6 elements on welcome screen
- - Toast dismiss button undersized (9x16px)
- - Reduced motion confirmed working (174 elements)
- - Focus visibility confirmed on all elements
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs(audit): add unified report, issue list, and priority roadmap
- consolidated issue list (62 issues deduplicated across 7 personas),
- and 4-sprint priority roadmap with dependency analysis.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs(audit): add Nielsen scorecard and WCAG compliance matrix
- cognitive load ratings. WCAG 2.1 matrix (24 components × 22 criteria).
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs(audit): add Phase 3 visual testing and adversarial findings
- testing. Items requiring live app verification are marked with 🔍.
- Includes axe-core Playwright test scaffolding.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs(audit): add all 7 Phase 1 persona audit reports
- Stress/Edge Cases, and Performance audit reports.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs(audit): add competitive intelligence matrix
- 
- chore: add audit tooling prerequisites (@axe-core/playwright)
- 
- docs: fix step numbering in audit plan (review round 2)
- for CSP audit, dynamic locale switching, Lighthouse, forced-colors,
- and adversarial network testing.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: add comprehensive audit implementation plan
- cross-analysis with Nielsen/WCAG scoring, visual live testing, adversarial
- testing, and 13+ deliverable synthesis with priority roadmap.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: update audit spec with review fixes (10 issues resolved)
- missing components, WCAG matrix ownership, unified report structure,
- stress report deliverable, and scoping caveats.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: add comprehensive multi-persona audit design spec
- security, i18n/RTL, stress testing, and performance perspectives.
- Includes competitive analysis, adversarial testing, and 12 deliverables.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- chore: remove dead code and add reduced-motion to skeleton
- - Remove unused motionTransition no-op from a11y-motion.ts
- - Add @media prefers-reduced-motion to AnalyticsBar skeleton animation
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- active LLM provider by index from the configured llmConfigs list.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- - Bind max tokens input to llm.maxTokens with proper parsing
- - Remove unused imports (tooltip, getModelInfo, getCheapestModel)
- - Replace any[] with unknown[] in debounce function
- - Remove (llm as any).shortcut cast, use typed property
- - Fix budget section title to use own translation key
- - Add aria-label to shortcut capture button
- - Add role="listitem" to connection step elements
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Fix handleShortcutCapture to store captured combo in llms array
- - Change notify threshold to range input (50-95%, step 5) with aria-valuetext
- - Replace hardcoded #16a34a with var(--success) for design system consistency
- - Add role="list" and aria-live="polite" to connection test steps
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- expand/collapse, model selector with pricing info, connection test
- with step-by-step feedback, shortcut capture, and budget limits.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- - Remove unused prefersReducedMotion import
- - Add @media (prefers-reduced-motion: reduce) for skeleton/trend animations
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- deterministic insights, provider comparison with patterned bars and
- drill-down, daily trend with comparison overlay, and CSV/JSON export.
- Integrated into App.svelte and Toolbar.svelte.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- insights panel, provider comparison grid with drill-down accordion,
- and daily trend chart. Integrates into App.svelte with conditional
- rendering and live tracking, adds analytics toggle button to Toolbar.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- - Use untrack() for prevErrors/prevRecovered to avoid effect re-entry
- - Replace hardcoded #22c55e with var(--success) design token
- - Type adapter prop as Adapter 
- - Remove redundant type annotations on budget alert callbacks
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- vs-avg indicator, error/recovery flash, budget alerts. Collapses to 1-row
- compact view on narrow widths. Integrated into ResponsePanel.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- chat input showing cost, context progress, cache efficiency, token
- count, turns, duration, and vs-average with budget alert states and
- narrow-width CSS container query collapse.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- and TauriAdapter, creates Rust provider.rs commands for connection testing and
- normalizer reload, adds api_key_env field to CliConfig with values in all 5 normalizer
- TOMLs, and adds the which crate dependency.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- bar-scale normalization and easeOutCubic tween easing function (9/9 tests pass).
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- test(analytics): add missing formatDate/formatDateFull tests
- 
- 
- docs: add Phase 7C frontend implementation plan
- adapter methods, analytics store, i18n keys, AnalyticsBar, AnalyticsDashboard,
- Provider Settings, and keyboard shortcuts.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: fix spec review findings — file count and projection formula
- Clarify cost projection formula: linear extrapolation based on
- avg turns per provider, with fallback.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: add Phase 7C frontend design spec
- metrics), Analytics Dashboard (full tab with KPI, insights, drill-down),
- responsive base layout, WCAG AA+ accessibility, locale-aware formatting.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Extend fixture framework to support new metadata field assertions
- - Fix TempDir leak in make_store() test helper
- - Rename chrono_date_from_secs to unix_secs_to_date_string
- - Make mod analytics a distinct step in Task 4
- - Add TOML comment about double-counting prevention
- - Add context_tokens comment in Metrics handler
- - Create fixtures/claude directory step
- 
- docs: fix spec review feedback (double-count tokens, field count)
- prevent double-counting. Fix field count from 9 to 10.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: add Phase 7B Analytics Collector design spec
- Extends AgentEventMetadata with cache tokens, duration, context
- usage, and cost. Verified against real Claude CLI output.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- 
- 
- 
- 
- 
- 
- docs: add Phase 7A implementation plan (13 tasks, 50 tests)
- (Gemini/Kimi/Qwen/Codex), TOML normalizers, Claude TOML updates, provider
- routing, discovery extension, capability wiring, JSON fixtures, and
- integration tests.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: address final spec review feedback (I-1, I-2, S-1)
- - Use @FIXME placeholder for Gemini update_command package name
- - Document why Gemini resume_args uses "latest" instead of {session_id}
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: revise Phase 7A spec with all 15 review fixes
- - Add Section 0 prerequisite for array index support in resolve_path
- - Fix error rule ordering (specific before generic) in all TOMLs
- - Add tool_input_delta and block_stop rules for Kimi, Qwen, Claude
- - Add normalizer/mod.rs and agent_event.rs to component map
- - Change ToolInputAccumulator::start API to auto-flush pending tool
- - Add error fixtures for Kimi and Qwen
- - Document TimedFlush async limitation
- - Add discovery builtin capability profiles
- - Increase test count from 40 to 50
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: Phase 7A CLI-based providers design spec
- 
- 
- 
- 
- 
- 
- 
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- docs: Phase 6 Intelligence implementation plan
- extensions, NormalizerHealth, CliUpdater, CapabilityNegotiator,
- Self-Heal flow, Tauri commands + wiring, and frontend types/adapter/store.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- ActionableMessage, and adds .catch() to navigator.clipboard.writeText
- to handle non-secure context failures gracefully.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- - Agent event listener uses cancelled flag pattern to prevent listener leaks on cleanup
- - Remove unused processAgentEvent import from ChatView
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- - Add event_count reconciliation in restore_session (JSONL is source of truth)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- SessionManager state and SessionHistoryRecorder into the event bus in lib.rs.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- 
- which drops the child process handle and kills the subprocess. Also
- remove unused serde import from retry.rs.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- 
- 
- 
- 
- 
- docs: add Phase 1 known issues from code review
- 
- 
- 
- 
- 
- 
- 
- 
- chore: add .worktrees/ to .gitignore
- test(analytics): extend fixture framework and add Claude result_metrics test
- duration_ms, duration_api_ms, num_turns, stop_reason, total_cost_usd
- assertions. Add Claude result_metrics fixture validating full field extraction.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- analytics_model_breakdown, analytics_session, analytics_daily,
- analytics_active. Collector registered as Arc<> managed state
- and subscribed to EventBus.
- 
- get_daily_stats with TimeRange filtering. Pure functions that aggregate
- from session records on-demand.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Done events into per-session SessionMetrics. Flushes completed sessions
- to AnalyticsStore on Done. Supports active session and completed
- session queries.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- errors, context pressure, and cost. AnalyticsStore persists completed
- sessions as JSONL and loads them at startup.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- Gemini: cache_read_tokens, duration_ms in usage rule
- Kimi: context_metrics rule (speculative, needs runtime validation)
- Qwen: duration_ms, duration_api_ms, num_turns in usage rule
- Codex: cache_read_tokens in usage rule
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- duration_ms, duration_api_ms, num_turns, stop_reason, context_usage,
- context_tokens, max_context_tokens, total_cost_usd from TOML mappings.
- 
- duration_ms, duration_api_ms, num_turns, stop_reason, context_usage,
- context_tokens, max_context_tokens, total_cost_usd.
- 
- All fields are Option<T> with #[serde(default)] for backward compat.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- - Extend fixture framework to support new metadata field assertions
- - Fix TempDir leak in make_store() test helper
- - Rename chrono_date_from_secs to unix_secs_to_date_string
- - Make mod analytics a distinct step in Task 4
- - Add TOML comment about double-counting prevention
- - Add context_tokens comment in Metrics handler
- - Create fixtures/claude directory step
- 
- docs: fix spec review feedback (double-count tokens, field count)
- prevent double-counting. Fix field count from 9 to 10.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: add Phase 7B Analytics Collector design spec
- Extends AgentEventMetadata with cache tokens, duration, context
- usage, and cost. Verified against real Claude CLI output.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- 
- 
- 
- 
- 
- 
- docs: add Phase 7A implementation plan (13 tasks, 50 tests)
- (Gemini/Kimi/Qwen/Codex), TOML normalizers, Claude TOML updates, provider
- routing, discovery extension, capability wiring, JSON fixtures, and
- integration tests.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: address final spec review feedback (I-1, I-2, S-1)
- - Use @FIXME placeholder for Gemini update_command package name
- - Document why Gemini resume_args uses "latest" instead of {session_id}
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: revise Phase 7A spec with all 15 review fixes
- - Add Section 0 prerequisite for array index support in resolve_path
- - Fix error rule ordering (specific before generic) in all TOMLs
- - Add tool_input_delta and block_stop rules for Kimi, Qwen, Claude
- - Add normalizer/mod.rs and agent_event.rs to component map
- - Change ToolInputAccumulator::start API to auto-flush pending tool
- - Add error fixtures for Kimi and Qwen
- - Document TimedFlush async limitation
- - Add discovery builtin capability profiles
- - Increase test count from 40 to 50
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- docs: Phase 7A CLI-based providers design spec
- 
- 
- 
- 
- 
- 
- 
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- docs: Phase 6 Intelligence implementation plan
- extensions, NormalizerHealth, CliUpdater, CapabilityNegotiator,
- Self-Heal flow, Tauri commands + wiring, and frontend types/adapter/store.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- ActionableMessage, and adds .catch() to navigator.clipboard.writeText
- to handle non-secure context failures gracefully.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- - Agent event listener uses cancelled flag pattern to prevent listener leaks on cleanup
- - Remove unused processAgentEvent import from ChatView
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- - Add event_count reconciliation in restore_session (JSONL is source of truth)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- SessionManager state and SessionHistoryRecorder into the event bus in lib.rs.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- 
- which drops the child process handle and kills the subprocess. Also
- remove unused serde import from retry.rs.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- 
- 
- 
- 
- 
- docs: add Phase 1 known issues from code review
- 
- 
- 
- 
- 
- 
- 
- 
- chore: add .worktrees/ to .gitignore



## [0.5.0] - 2026-03-22

### Features

- feat: UI improvements, YOLO restart behavior, CI per-platform bundles

### Bug Fixes

- fix: update GitHub Actions to v5 for Node.js 24 compatibility

### Other

- updated from v4 to v5 to resolve Node.js 20 deprecation warnings.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- restarts all running terminal instances. CI builds per-platform bundle
- types with macOS .zip artifact. AUR PKGBUILD installs desktop icons.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>



## [0.4.1] - 2026-03-22

### Bug Fixes

- fix: install missing dompurify dependency



## [0.4.0] - 2026-03-22

### Features

- feat: security hardening, Rust LLM proxy, CI fixes, IPv4 dev server

### Other

- - Move LLM API calls from frontend fetch to Rust backend proxy via
-   invoke('call_llm_api'), keeping API keys server-side
- - Fix CI release workflow: Node 22, simplified artifact upload paths
- - Fix dev server: bind Vite to 127.0.0.1 explicitly to prevent
-   IPv6 mismatch with WebKit2GTK (localhost resolves to ::1)
- - Update README with accurate project info and security section
- - Update tests to match new invoke-based LLM API
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>



## [0.3.1] - 2026-03-22

### Bug Fixes

- fix: convert grayscale icons to RGBA for Tauri build

### Other

- 8-bit grayscale which caused proc macro panic on macOS build.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>



## [0.3.0] - 2026-03-22

### Features

- feat: WCAG 2.1 AA accessibility audit — 68→~90 score

### Other

- - Focus trapping in all 4 dialogs (Settings, SearchPalette, FindInFiles, ShortcutsDialog)
- - Arrow key navigation in all 5 menus (Toolbar, TerminalManager, MenuItem, ContextMenu, TerminalToolbar)
- - WAI-ARIA tree pattern on FileTree (role=tree/treeitem, aria-expanded, keyboard nav)
- - WAI-ARIA tab pattern on TerminalManager (role=tablist/tab, aria-selected)
- - aria-haspopup/aria-expanded on all dropdown triggers
- - aria-label on window control buttons
- - Keyboard support on panel splitters (arrow keys + shift)
- - Backdrop role=button → role=presentation
- 
- Level AA fixes:
- - StatusBar error/paused contrast (#ff6b6b→#fca5a5, #fbbf24→#fef08a)
- - role=status on StatusBar, role=alert on Settings error banner
- - aria-live=polite on model-info-bar
- 
- Other fixes:
- - Add bundle.icon config for Windows .ico (fixes Windows build)
- - Fix FileTree test missing size/modified fields
- - Fix llm-api test global→globalThis (28 TS errors resolved)
- - Update a11y test for overlay role change
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>



## [0.2.1] - 2026-03-22

### Bug Fixes

- fix: add missing quotes around updater pubkey in tauri.conf.json



## [0.2.0] - 2026-03-22

### Features

- feat: flatten repo structure, remove submodule, add CI release workflow
- feat: set updater endpoint URL to TNASRLSB/reasonance
- feat: start update checker on app boot
- feat: add Updates section to Settings with auto-update toggle and mode selector
- feat: add update checker module with notify/silent modes
- feat: extend toast system with action buttons, progress bar, persistent mode
- feat: extend toast system with action buttons, progress bar, persistent mode
- feat: configure Tauri updater endpoint, pubkey placeholder, and capability
- feat: add tauri-plugin-updater to Rust backend
- feat: add integrated documentation in 9 languages with F1 shortcut
- feat: LLM dropdown, advanced status bar with session/model/reset/messages
- feat: add welcome screen, open folder dialog, and recent projects
- feat: add menu bar with File, Edit, View, Terminal, Git, Help menus
- feat: add i18n infrastructure with 9 languages (EN, IT, DE, ES, FR, PT, ZH, HI, AR)
- feat: reactive theme switching for editor, terminal, and diff view
- feat(ui): add toast feedback for workflow operations
- feat(ui): bidirectional JSON sync, workflow menu, keyboard shortcuts
- feat(ui): add WorkflowMenu component and dialog plugin (save/import/export/templates)
- feat(backend): add duplicate, save-to-global, list-global workflow commands
- feat(adapter): add duplicate, save-to-global, list-global workflow methods
- feat(ui): integrate HiveCanvas as fullscreen overlay in main layout
- feat(ui): integrate HivePanel as tab in TerminalManager
- feat(ui): add HiveCanvas with SvelteFlow graph, toolbar, inspector, dual mode
- feat(ui): add HivePanel compact monitoring component
- feat(ui): add HiveInspector component (node props, JSON toggle)
- feat(ui): add Agent, Resource, Logic node components with state colors
- feat(ui): add NodeCatalog component (Agent/Resource/Logic buttons)
- feat(ui): add HiveControls component (play/pause/stop/step)
- feat(frontend): add workflow engine adapter and store
- feat(engine): add Tauri commands (play, pause, resume, stop, step, notify)
- feat(engine): add WorkflowEngine with graph analysis, run lifecycle, and scheduler
- feat: add agent hive frontend adapter types and Svelte stores
- feat: add Agent Runtime (Tasks 9-10) for agent hive platform
- feat: add WorkflowStore with CRUD commands (Tasks 6-8)
- feat: add Discovery Engine for agent hive platform
- feat: add discover_llms and get_system_colors Tauri commands, rename package to reasonance
- feat: add REASONANCE logo and app icons
- feat: enhanced readability class toggle, remove KDE color overrides
- feat: brutalist toasts with text labels for a11y
- feat: brutalist restyle for all overlay components, enhanced readability toggle
- feat: brutalist terminal theme, parse context/token from CLI output
- feat: integrate terminal toolbar, context/token footer, brutalist restyle
- feat: terminal toolbar — add file, slash commands, mode selector
- feat: brutalist CodeMirror theme, editor toolbar restyled
- feat: brutalist editor tabs — flat, square, accent underline
- feat: brutalist file tree — spacing, weights, zero radius
- feat: YOLO mode turns status bar red with explicit warning label
- feat: brutalist toolbar — solid blocks, square corners, hover inversion
- feat: brutalist dividers with dot handles, 2px panel borders
- feat: add enhanced readability, LLM modes, per-instance context state to stores
- feat: rewrite design system — brutalist theme, Atkinson Hyperlegible, a11y vars
- feat: bundle Atkinson Hyperlegible Next fonts (sans + mono, woff2)
- feat: CI/CD release workflow + AUR PKGBUILD + MIT license
- feat: session persistence across app restarts
- feat: toast notification system with error handling
- feat: fuzzy file search and find-in-files with keyboard shortcuts
- feat: settings UI for LLM and app configuration
- feat: git shortcuts, YOLO mode, image drag-and-drop
- feat: context menu with LLM actions on code selection
- feat: markdown preview with GFM and syntax highlighting
- feat: diff view with accept/reject and file watcher
- feat: xterm.js terminal with LLM tabs and multi-instance
- feat: CodeMirror 6 editor with tabs and syntax highlighting
- feat: file tree with click/double-click behavior
- feat: 3-panel layout with toolbar, status bar, theme
- feat: file watcher, shadow store, config management
- feat: PTY manager with spawn/write/resize/kill
- feat: Rust filesystem commands + adapter wiring
- feat: define adapter interface and Tauri stub

### Bug Fixes

- fix: use env vars for changelog body in CI to prevent shell injection
- fix: terminal font, panel resize, editor reflow, TS errors, UI polish
- fix: hide skip links using sr-only clip pattern instead of top:-100%
- fix: remaining adapter, store, and component changes from audit fixes
- fix: shortcuts dialog, pending-delete state, brutalist button styles, overflow fixes
- fix: address Low security, a11y, UX, and i18n findings
- fix: medium/low QA findings — i18n, UX, OPT, TD fixes
- fix: resolve medium tech-debt findings (TD-06, TD-07, TD-10, PERF-06, OPT-05)
- fix(security): address 5 medium security findings (SEC-04 through SEC-08)
- fix: resolve 4 medium a11y findings (landmarks, skip links, contrast, touch targets)
- fix: resolve 4 High UX findings (labels, onboarding, errors, toast)
- fix(security): apply SEC-01/02/03 — CSP, spawn allowlist, fs path validation
- fix: quick wins — TTF→WOFF2, contrast AA, reduced-motion, lazy Settings, static get, ResponsePanel i18n
- fix(i18n): wire Settings.svelte through i18n — 34 keys × 9 locales
- fix: add confirmation dialogs for destructive actions (UX-S3/S5)
- fix: theme detection on KDE/WebKitGTK, static i18n imports, menu a11y
- fix: DiffView reactive theme rebuild + Terminal initial theme respects isDark
- fix: replace all hardcoded hex colors with CSS variables for theme reactivity
- fix: restore native KDE title bar, remove custom window controls
- fix: code review fixes — light mode, a11y, click targets, subscriptions

### Other

- expanded inline via ${{ }}. Using env vars keeps the content safe.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Simplify CI to single commit/push (no more double submodule push)
- - Remove submodules: recursive from all checkout steps
- - Update .gitignore to exclude framework/tooling files
- - Add release workflow, README, favicon
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- - Add 'update' toast type
- - Add progress bar support (0-100)
- - Add persistent mode (no auto-dismiss)
- - Fix layout: wrap toast in column flex for progress/actions below content
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- chore: remove old release workflow
-   ('Atkinson Hyperlegible Next') into fontFamily store instead of mono.
-   Added validation in restoreSession to reject non-monospace saved fonts.
- - Fix terminal font loading: await document.fonts.ready before
-   Terminal.open() per official xterm.js guidance (issues #1164, #2058).
-   Use fontFamily toggle trick from @xterm/addon-web-fonts for runtime
-   font changes to force texture atlas invalidation.
- - Fix editor going blank after toggling markdown preview: keep CM6
-   container in DOM with CSS hidden class instead of {#if}/{:else}.
- - Fix text flickering during panel resize: replace destructive global
-   pointer-events:none with transparent resize-overlay div.
- - Fix editor text reflow on resize: add EditorView.lineWrapping and
-   ResizeObserver to trigger CM6 requestMeasure().
- - Fix terminal resize: debounce fitAddon.fit() via requestAnimationFrame,
-   use percentage-based bounds (250px min, 50% max).
- - Fix tab bar alignment: consolidate editor toolbar into EditorTabs via
-   Svelte 5 Snippet, set both tab bars to fixed height: 38px.
- - Fix pre-existing TS errors: App.svelte error type cast,
-   ResponsePanel $t→$tr, FindInFiles/Settings handleOverlayClick→onClose.
- - Remove editor theme dropdown from tab bar.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- standard sr-only pattern (clip + 1px dimensions) for proper hiding.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Implement all new methods in TauriAdapter
- - Update mock adapter with new method stubs
- - WelcomeScreen: add about dialog, logo, adapter integration
- - FileTree: improved a11y and layout
- - HiveCanvas: layout and reactivity improvements
- - theme store: simplify
- - fs_watcher: cleanup
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
-   Help > Shortcuts; add shortcuts data file with 10 entries grouped by context
- - UX-S3-3: Mark LLMs as pending-delete with strikethrough + red tint instead
-   of immediate removal; deletions only apply on Save
- - UX-S4-1: Settings buttons now use border-radius 0, uppercase text, bold
-   weight to match Toolbar brutalist style
- - I18N-05: ContextMenu gets max-width: 280px and text-overflow: ellipsis
- - Add i18n keys to all 9 locale files (shortcuts.*, settings.llm.pendingDelete,
-   settings.unsavedHint)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - A11Y-A02: raise --text-muted from #949494 to #b0b0b0 (AAA on #121212)
- - A11Y-A07: raise --border from #333333 to #5a5a5a (3:1 on #121212)
- - UX-S2-2: add aria-label="Settings" to settings gear button
- - UX-S2-3: change instance labels from "inst. N" to "LLM N"
- - UX-S4-2: add close button (✕) to SearchPalette header
- - UX-S9-3: replace hardcoded Italian "**Errore:**" with i18n key contextMenu.error
- - UX-S10-3: add search input to HelpPanel with highlight and scroll-to-match
- - UX-S10-4: TerminalToolbar + button now opens file dialog instead of writing /file placeholder
- - OPT-08: replace onDestroy(unsubLocale) with $effect returning unsubscribe in HelpPanel
- - I18N-05b: localize status.messagesLeft and status.resetIn in it/de/es/pt
- - Add search.close, contextMenu.error, help.search.* keys to all 9 locale files
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- refactor(TD-05/TD-09): extract session and config-bootstrap from +page.svelte
- src/lib/utils/session.ts and TOML config loading + LLM auto-discovery to
- src/lib/utils/config-bootstrap.ts. +page.svelte drops from 593 to 399 lines
- and now acts as a thin orchestrator calling init functions rather than owning
- the logic inline.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- - I18N-04: SearchPalette and FindInFiles fully i18n'd via $tr(); keys added to all 9 locales
- - OPT-03: DiffView dual onMount merged into single onMount with cleanup return
- - OPT-06: package.json check script already present (no change needed)
- - UX-S8-2: Remove perpetual YOLO pulse animation — static red bg is sufficient
- - TD-11: Editor subscribes to fontFamily/fontSize stores via buildFontExt(); Terminal initialises from stores and reacts via $effect
- - TD-12: slash commands extracted from TerminalManager to src/lib/data/slash-commands.ts
- 
- All 185 tests pass.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- - TD-07: Replace `as any` locale casts with `as Locale` after includes() guard
- - TD-10: Add console.warn() to five silent .catch(() => {}) handlers across MenuBar, Settings, TerminalManager, Terminal
- - PERF-06: Restore true dynamic locale loading in loadLocale() — only en bundled statically, others loaded on demand
- - OPT-05: Add optimizeDeps.include for heavy CM6 and xterm packages in vite.config.ts
- 
- All 185 tests pass.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- - SEC-05: document API key in JS memory risk with TODO to proxy through Tauri
- - SEC-06: validate open_external URL scheme; only https:// and http:// allowed
- - SEC-07: document plain-text config constraint in config.rs (secrets not stored)
- - SEC-08: remove unused deep-link plugin from lib.rs, Cargo.toml, and capabilities
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- - A11Y-A05: add visually-hidden skip links (file tree, editor, terminal)
- - A11Y-A03: add --accent-text: #6b8de8 CSS variable (4.5:1 on #121212)
- - A11Y-A14: EditorTabs and TerminalManager close buttons min 24×24px
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- perf: lazy-load CM6 language parsers on demand (PERF-02)
- import() calls via a new src/lib/editor/languages.ts module.
- The editor renders immediately with no highlighting, then applies
- syntax highlighting once the parser chunk loads asynchronously.
- Language extensions are cached per open file so theme/readOnly
- rebuilds don't re-fetch.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- refactor: extract shared TOML LLM parsing into config-parser utility (TD-03)
- Settings.svelte by introducing src/lib/utils/config-parser.ts with a single
- parseLlmConfig() function. Schema changes now only need to be made in one place.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- - TerminalManager: replace small hint text with actionable banner (border, message, "Open Settings" button) when no LLMs are configured
- - Settings: wrap raw exception strings in friendly messages for loadConfig and save catch blocks
- - FindInFiles: replace raw Rust/OS error string with user-friendly message in runSearch catch block
- - SearchPalette: import showToast and display error toast when openFile() fails instead of silently swallowing the error
- - i18n: add terminal.openSettings key to all 9 locales; update terminal.configHint to be action-oriented
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- SEC-02: spawn_process now validates the command against configured LLM commands
-         and an explicit shell allowlist (bash/zsh/sh/fish/powershell/cmd).
- SEC-03: read_file/write_file validate paths against a ProjectRootState managed
-         in Tauri; reads also allowed from the Reasonance config dir. Frontend
-         calls set_project_root on folder open and session restore.
- 
- cargo test: 64 passed, 0 failed.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- - Fix --text-muted contrast: dark #949494, light #767676 (WCAG AA)
- - Add prefers-reduced-motion global reset
- - Lazy-mount Settings with {#if $showSettings}
- - Replace 4 dynamic import('svelte/store') with static import
- - Replace hardcoded "Risposta LLM" with $t('response.title')
- - Add response.title + response.close keys to all 9 locales
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- through the `$tr()` store (matching the pattern used by other components).
- New settings.* keys added to all 9 locale files with proper translations.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- - Terminal kill: confirm before terminating session (TerminalManager)
- - Dirty file close: prompt when closing unsaved changes (EditorTabs)
- - Git push: confirm before executing push (Toolbar + MenuBar)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- test: add visual regression and interaction Playwright tests
- snapshots and user interaction flows (Ctrl+P search palette, Ctrl+Shift+F
- find-in-files, Ctrl+, settings, F1 help, editor tab rendering).
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- test: add P1 component tests for 10 Svelte components (74 tests)
- EditorTabs, Toolbar, FileTree, SearchPalette, Settings, App, DiffView,
- and ContextMenu. Adds @tauri-apps/api/window mock and sets pool=forks in
- vitest.config.ts to prevent worker thread hangs in jsdom environment.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- test: add a11y and keyboard tests using axe-core
-   and EditorTabs; documents known nested-interactive violation in EditorTabs
- - Create tests/a11y/keyboard.test.ts: keyboard navigation tests for
-   EditorTabs (Enter/Space/click/close) and SearchPalette (Escape, overlay
-   click, dialog ARIA attributes, focus management)
- - Fix vitest.config.ts: add resolve.conditions ['browser'] so Svelte
-   resolves the browser build instead of SSR — fixes all component tests
- - Fix SearchPalette.svelte: overlay onkeydown Escape handler was calling
-   handleOverlayClick() without the event arg (would crash); now calls
-   onClose() directly
- 
- 27 tests pass across 2 test files.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- test: add Rust P1 tests (config, discovery, commands/fs, extend workflow_engine)
- 
- test: add P1 store tests (files, config, ui, theme, terminals)
- test: add P1 utils tests and i18n coverage test
- 
- chore: add vitest/playwright configs and Tauri mock layer
- 
- chore: add test dependencies (vitest, playwright, axe-core, tempfile)
- 
- - theme.ts defaults to dark (WebKitGTK ignores prefers-color-scheme)
- - i18n uses static imports instead of broken Vite dynamic imports
- - MenuItem.svelte adds tabindex for menubar role
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- chore: cleanup — hive coming soon, fix a11y warnings, i18n DiffView
- - Remove svelte-ignore a11y comments by adding proper roles, tabindex,
-   and keyboard handlers to overlay/backdrop elements
- - Add i18n to DiffView (diff.changes, diff.accept, diff.reject)
- - Add diff.changes translation key to all 9 language files
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- Menus support submenus, keyboard shortcuts display, hover-switching,
- and click-outside closing. All labels use i18n system.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- 
- test: add unit tests for WorkflowEngine (topo sort, ready nodes, run lifecycle)
- 
- test: add unit tests for AgentRuntime (state machine, retry, fallback, messaging)
- test: add unit tests for WorkflowStore (CRUD, serialization, paths)
- 
- 
- chore: add tauri-plugin-dialog for file import/export
- 
- 
- chore: add @xyflow/svelte dependency and hive UI stores
- 7 Tauri commands in TauriAdapter, and create derived Svelte stores for
- run state tracking.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- commands/engine.rs module. Adds use tauri::Emitter for emit() calls.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- - Implement all agent hive methods in TauriAdapter (discovery, workflow CRUD, agent runtime)
- - Add workflow.ts store: currentWorkflow, workflowPath, dirty flag, derived node/edge counts
- - Add agents.ts store: discoveredAgents, activeAgents, messages, derived running/errored counts
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- settings. Adds six Tauri commands: load, save, list, delete, create,
- and get workflow. WorkflowStore managed as app state alongside existing
- DiscoveryEngine.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- LLM CLIs (claude, gemini, aider, ollama, etc.) and probes local API
- endpoints (Ollama /api/tags). Exposes discover_agents and
- get_discovered_agents Tauri commands. Includes CapabilityProfile and
- DiscoveredAgent types with builtin profiles for known CLIs.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- chore: add reqwest, tokio, chrono dependencies for agent hive
- 
- 
- refactor: rename package/distribution files to reasonance
- refactor: rename FORGE to REASONANCE in UI components
- refactor: rename Rust/Tauri identifiers from forge-ide to reasonance
- 
- from mathematically correct formula. All Tauri icon sizes regenerated.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- - Enhanced Readability: add font-weight 500/600 overrides
- - Settings: use :focus-visible instead of removing outline
- - Click targets: toolbar buttons 44px, editor tabs 44px, terminal buttons 44px effective
- - Divider grab area: pointer-events: none on ::before
- - Terminal: reactive font size for Enhanced Readability mode
- - TerminalManager: clean up store subscriptions, remove dead cliLlms
- - DiffView: add brutalist CodeMirror theme
- - Logo: weight 900→800 to match available font files
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- ContextMenu, DiffView. Use design system CSS variables throughout. Add
- enhanced readability toggle in Settings accessibility section.
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- 
- 
- 
- 
- 
- 
- 
- 
- - Sans: Regular (400), Medium (500), Bold (700), ExtraBold (800)
- - Mono: Regular (400), Medium (500), Bold (700)
- 
- Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 
- 
- theme, font family/size, and terminal tab metadata on window close.
- Files deleted since last session are silently skipped on restore.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- - Add SearchPalette (Ctrl+P): recursive file list with fuzzy scoring
- - Add FindInFiles (Ctrl+Shift+F): project-wide grep via Rust grep_files
- - Add grep_files Tauri command using ignore crate (respects .gitignore, max 500 results)
- - Register Ctrl+P, Ctrl+Shift+F, Ctrl+, shortcuts in +page.svelte
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- Supports add/edit/delete LLM entries (CLI and API types), terminal
- font settings, and light/dark/system theme toggle. Config is loaded
- on app startup from adapter.readConfig() and written on save via
- adapter.writeConfig().
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- 
- 
- 
- and wires file watcher in +page.svelte to detect external file changes,
- store shadows on file open, show diff overlay for modified files, and
- mark open files as deleted on remove events.
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- - Editor: CodeMirror 6 with basicSetup, oneDark theme, language auto-detection
- - Read-only toggle (default read-only), empty state with Ctrl+P hint
- - Integrated into App layout via editor snippet in +page.svelte
- 
- Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
- 
- 
- 
- diff/rollback, TOML config reader/writer, and env var access command.
- Wire all new Tauri commands to the TypeScript adapter layer.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- in pseudo-terminals and relaying I/O to the frontend via Tauri events.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- support), and open_external. Wire the TauriAdapter to invoke these
- commands via Tauri IPC.
- 
- Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
- 
- chore: add Cargo.lock for reproducible builds

