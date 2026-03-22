# Stress & Edge Cases Audit Report

**Date:** 2026-03-22
**Persona:** The chaos monkey
**Judgment:** Does the app degrade gracefully or crash?

## Executive Summary

Reasonance has **good structural error boundaries** (Svelte 5 `<svelte:boundary>` on all three main panels) but lacks **proactive limits** on resource-intensive operations. The app has no file size checks, no chat message virtualization, no terminal scrollback caps, and no tab count limits. The Rust backend contains **408 `unwrap()` calls** across 34 source files, any of which could panic and crash the entire desktop application. Binary file handling will produce an error but not a graceful one. The app will degrade under stress primarily through memory exhaustion rather than clean error messages.

**Overall verdict: FAIR.** The crash-recovery story (error boundaries, retry buttons) is decent. The crash-prevention story (limits, virtualization, guards) is weak.

---

## Stress Limits

| Scenario | Expected Behavior | Actual (from code) | Verdict |
|----------|-------------------|-------------------|---------|
| 50MB file in editor | Warn or refuse | `read_file` calls `fs::read_to_string` with no size check (fs.rs:121). Entire content loaded into JS string, then into CodeMirror doc. No lazy loading, no virtualization. | FAIL |
| 100 open tabs | Scroll tabs, maybe cap | `EditorTabs.svelte` uses `overflow-x: auto` on `.tabs-scroll` (line 82) with thin scrollbar. No tab count limit. All tabs rendered as DOM nodes. Tabs have `min-width: 100px` / `max-width: 200px` so 100 tabs = 10K-20K px scroll. | WARN |
| 10,000 chat messages | Virtualize list | `ChatMessages.svelte` renders ALL events with `{#each}` (line 79-104). No virtualization, no windowing, no pagination. Each message creates multiple DOM nodes (role label, content blocks). At 10K messages, DOM will contain 50K+ nodes. | FAIL |
| Terminal flood (e.g. `yes` command) | Cap scrollback | `Terminal.svelte` creates xterm.js with default `scrollback` (1000 lines by default in xterm.js). However, no explicit scrollback config is set (line 61-70). The PTY reader uses 4KB buffer (pty_manager.rs:57). xterm.js default is acceptable but not explicitly configured. | WARN |
| 1000+ files in FileTree directory | Lazy load or paginate | `list_dir` returns ALL entries at once (fs.rs:136-177). `FileTree.svelte` renders all entries in a flat `{#each}`. No virtualization. Deep nesting uses recursive Svelte snippets (line 144-167) creating nested DOM. | WARN |
| Rapid file switching (50 times/second) | Debounce | `Editor.svelte` `$effect` on `$activeFilePath` (line 160-171) calls `initEditor` which destroys and recreates the entire CodeMirror instance. No debounce. Each switch = full teardown + async language load. | FAIL |
| 500+ grep results | Paginate or cap | `grep_files` in fs.rs caps at 500 results (line 216-217). Good. | PASS |
| Very long lines (10K+ chars) | Wrap or horizontal scroll | CodeMirror uses `EditorView.lineWrapping` (Editor.svelte:127). Long lines will wrap. Rendering performance may degrade but won't crash. | PASS |

---

## Empty States

| Component | Has empty state? | Quality |
|-----------|-----------------|---------|
| `Editor.svelte` | Yes - "Open a file" message (line 241-244) | Good - shows hint with search shortcut |
| `EditorTabs.svelte` | Implicit - no tabs = empty bar | Acceptable - bar just appears empty, no guidance |
| `FileTree.svelte` | No explicit empty state | POOR - if `entries` is empty, tree-scroll div renders nothing. Blank area with just header. |
| `TerminalManager.svelte` | Yes - full empty state with LLM selector (line 308-338) | Good - guides user to configure/start LLMs |
| `ChatMessages.svelte` | No | POOR - empty events array = blank `chat-messages` div. No "start a conversation" prompt. |
| `ChatView.svelte` | No | Inherits ChatMessages gap |
| `WelcomeScreen.svelte` | Yes - handles empty recent projects (line 40-41) | Good - shows "no recent" message |
| `StatusBar.svelte` | Yes - shows idle hint when no session (line 65) | Good |
| `AnalyticsDashboard.svelte` | Partial - uses `StoreState` with loading/error/ready states | Acceptable - null checks on data with fallback to 0 |
| `SearchPalette.svelte` | Yes - loading state and empty results handled | Good |
| `Settings.svelte` | Unknown (file too large to read fully) | -- |
| `SwarmPanel.svelte` | Yes - "coming soon" placeholder | Acceptable |

---

## Error Recovery

| Error Scenario | Handled? | Recovery Path |
|----------------|----------|---------------|
| FileTree panel crash | YES | `<svelte:boundary>` in App.svelte:109-122 shows error + RETRY button |
| Editor panel crash | YES | `<svelte:boundary>` in App.svelte:130-145 shows error + RETRY button |
| Terminal panel crash | YES | `<svelte:boundary>` in App.svelte:153-166 shows error + RETRY button |
| File read fails | PARTIAL | FileTree.svelte:66-68 catches error, but only logs to console. User sees nothing. |
| Binary file opened | NO | Rust `fs::read_to_string` (fs.rs:121) returns `Err` for non-UTF-8. Error propagates to frontend as string. No friendly "cannot open binary file" message. |
| LLM process spawn fails | YES | TerminalManager.svelte:72-74 catches and logs. User sees nothing happen (silent failure). |
| Agent message send fails | PARTIAL | ChatView.svelte:67-70 catches, logs error, stops streaming indicator. No user-visible error message. |
| Session fork fails | PARTIAL | ChatView.svelte:78-80 catches and logs. No UI feedback. |
| Clipboard paste fails | YES | Terminal.svelte:104 catches and warns. Terminal still functional. |
| WebGL renderer unavailable | YES | Terminal.svelte:92-94 catches silently, falls back to DOM renderer. |
| PTY exit | YES | Terminal.svelte:187-190 writes exit message to terminal. |
| Mutex poisoned (Rust) | NO | 408 `unwrap()` calls on mutex locks across Rust codebase. A panic in any thread holding a mutex poisons it, causing ALL subsequent `unwrap()` calls to panic = cascading crash. See details below. |
| Network error on analytics fetch | YES | analytics.ts:75-77 catches, sets error state. |
| Config file missing/malformed | PARTIAL | config-bootstrap.ts has try/catch (3 occurrences). Error handling exists but may not cover all parse failures. |

### Critical: Rust `unwrap()` Analysis

**Total `unwrap()` calls in src-tauri/src: 408 across 34 files** (excluding test files).

Worst offenders in production code:
| File | Count | Risk |
|------|-------|------|
| `workflow_engine.rs` | 48 | HIGH - mutex unwraps on `self.runs.lock()` throughout |
| `agent_runtime.rs` | 42 | HIGH - mutex unwraps on `self.agents.lock()` throughout |
| `commands/fs.rs` | 34 | MEDIUM - mostly in path validation mutex locks |
| `workflow_store.rs` | 23 | HIGH - mutex unwraps |
| `analytics/store.rs` | 21 | HIGH - mutex unwraps |
| `normalizer/mod.rs` | 22 | HIGH - mutex unwraps |
| `transport/session_store.rs` | 19 | HIGH - mutex unwraps |
| `transport/session_manager.rs` | 33 | HIGH - mutex unwraps |
| `transport/mod.rs` | 18 | HIGH - mutex unwraps |
| `analytics/collector.rs` | 15 | HIGH - mutex unwraps |
| `normalizer_version.rs` | 16 | MEDIUM |
| `transport/event_bus.rs` | 12 | HIGH - mutex unwraps |
| `capability.rs` | 16 | MEDIUM |

**Impact:** A single panic in any Rust thread poisons the associated mutex. Every subsequent `.lock().unwrap()` on that mutex will also panic, creating a cascade. In a Tauri app, this crashes the entire application with no recovery. All mutex `.lock().unwrap()` calls should use `.lock().map_err()` or at minimum `.lock().expect("descriptive message")`.

---

## Race Conditions

| Scenario | Risk Level | Location |
|----------|------------|----------|
| Rapid tab switching during async language load | MEDIUM | Editor.svelte:144-157 - `initEditor` is async. If user switches tabs rapidly, multiple `getLangAsync` calls may be in-flight. The `if (!view) return` guard at line 153 partially mitigates but a stale closure could still apply wrong language to wrong file. |
| Multiple agent sends before response | LOW | ChatView.svelte:37-71 - `handleSend` sets streaming=true then awaits. Input is disabled during streaming (line 87). Race is gated by UI. |
| YOLO mode toggle restarts all instances | MEDIUM | TerminalManager.svelte:176-232 - Iterates all instances, kills and respawns sequentially in async IIFE. If user toggles YOLO again mid-restart, both restart loops run concurrently, potentially duplicating or orphaning processes. No lock/guard. |
| Store updates during streaming | LOW | agent-events.ts:60-65 - `processAgentEvent` creates new Map on each event. During high-frequency streaming, this creates many Map copies. No batching. Performance concern more than correctness. |
| FileTree reload on project root change | LOW | FileTree.svelte:17-22 - `$effect` on `$projectRoot` calls `adapter.listDir`. Rapid project root changes could interleave, but this is unlikely in practice. |
| Terminal resize during fit | LOW | Terminal.svelte:196-207 - Uses `requestAnimationFrame` coalescing. Well-handled. |
| Concurrent PTY writes | LOW | pty_manager.rs:84-99 - Protected by mutex lock. Safe. |
| Analytics live tracking vs historical fetch | LOW | analytics.ts - Separate stores. No conflict. |
| Multiple `processAgentEvent` in quick succession | MEDIUM | agent-events.ts:56-96 - Each call does `new Map(map)` and `[...events, event]`. With rapid streaming (100+ events/sec), this creates significant GC pressure. Array spreading means O(n^2) total work over n events. |

---

## Edge Cases

| Input | Component | Behavior |
|-------|-----------|----------|
| Binary file (image, .exe) | Editor via `read_file` | Rust `fs::read_to_string` fails on non-UTF-8 (fs.rs:121). Returns error string. File click in FileTree catches this (line 66-68) but only `console.error`. User sees no feedback -- file simply doesn't open. |
| File with emoji in name (e.g. `hello.rs`) | FileTree | `to_string_lossy()` in Rust (fs.rs:163) handles this. Svelte renders emoji in `.name` span. Should work. |
| File with spaces in path | FileTree/Editor | Path is passed as string throughout. No URL encoding issues. Should work. |
| Dot-prefix files (.env, .gitignore) | FileTree | Listed by `fs::read_dir`. Displayed normally. Gitignored files get `opacity: 0.5` styling. |
| Very long filename (200+ chars) | FileTree | `.name` span has `text-overflow: ellipsis` (FileTree.svelte:250-253). Tab in EditorTabs has `max-width: 200px` with `text-overflow: ellipsis` (line 143-147). Handled. |
| Symlink loops | FileTree | `adapter.listDir` calls `fs::read_dir` which doesn't follow symlinks. However, `SearchPalette.buildFileList` (line 35-51) recursively lists all dirs. A symlink loop would cause infinite recursion and stack overflow. No cycle detection. |
| Empty file (0 bytes) | Editor | `currentContent` defaults to `''` (Editor.svelte:96). CodeMirror handles empty doc. Works. |
| File deleted while open | Editor | `isDeleted` flag exists on `OpenFile` (files.ts:8). EditorTabs shows italic label with "(deleted)" (EditorTabs.svelte:46-48). Partial handling -- no auto-detection of external deletion. |
| Paste 1MB text into terminal | Terminal | PTY write has MAX_PAYLOAD of 64KB (pty_manager.rs:85-92). Paste > 64KB is rejected with error. But the clipboard read in Terminal.svelte:102-103 has no size check before calling `writePty`. The error propagates but is only caught with `.catch` console.warn. |
| Unicode/RTL text in editor | Editor | CodeMirror handles Unicode natively. Should work. |
| Deeply nested dirs (50+ levels) | FileTree | Recursive Svelte snippet at FileTree.svelte:144-167. Each level adds 16px padding-left (line 150). At 50 levels = 814px indent, likely pushing content off-screen. No max-depth guard. Performance degrades with recursive DOM nesting. |
| Opening same file twice | Editor | `addOpenFile` checks `files.some(f => f.path === file.path)` (files.ts:25). Deduplication works. Just switches to existing tab. |
| Null/undefined project root | FileTree | `currentRoot` derived defaults to `'.'` (FileTree.svelte:14). onMount uses `$projectRoot \|\| '.'` (line 25). Safe. |
| No LLM configs | TerminalManager | Shows "no LLMs configured" banner with Settings button (line 313-317). Good. |
| No providers in analytics | AnalyticsDashboard | `$providerAnalytics.data` null check with fallback to 0 (line 69-71). Handles gracefully. |
| Concurrent file writes from editor + agent | Rust fs commands | `write_file` (fs.rs:125-132) is not atomic -- uses `fs::write` which can produce partial writes on crash. No file locking between concurrent writes. |
| Window resize during panel drag | App.svelte | Resize overlay (line 94-95) prevents pointer events from reaching content during drag. `Math.max`/`Math.min` bounds enforce limits (line 32-38). Well-handled. |
| Close tab with unsaved changes | EditorTabs | `confirm()` dialog (line 18). Blocking browser confirm, works but uses `confirm()` which is not styleable and may behave differently in Tauri webview. |

---

## Recommendations (Priority Order)

### P0 -- Crash Prevention
1. **Replace all Mutex `unwrap()` in Rust with proper error handling.** 408 potential panic points. Use `.lock().map_err(|e| format!("Lock poisoned: {}", e))?` pattern. At minimum, the top 5 files (workflow_engine, agent_runtime, session_manager, session_store, transport/mod).
2. **Add file size guard in `read_file`.** Check `metadata.len()` before `read_to_string`. Reject or warn for files > 10MB.

### P1 -- Memory Protection
3. **Add chat message virtualization** or at minimum pagination (e.g., show last 200 messages, "load more" button).
4. **Configure xterm.js `scrollback` explicitly** (e.g., 5000-10000 lines) to prevent unbounded memory growth.
5. **Add symlink loop detection** in `SearchPalette.buildFileList` (track visited paths).

### P2 -- UX Degradation
6. **Debounce file switching** in Editor.svelte `$effect` (50-100ms) to prevent rapid teardown/rebuild cycles.
7. **Show user-visible error on file read failure** instead of silent `console.error`.
8. **Show binary file detection message** -- check for null bytes or non-UTF-8 error and display "Cannot open binary file" in editor area.
9. **Add empty state to ChatMessages** -- "Send a message to start" placeholder.
10. **Batch agent events** during streaming -- accumulate events for 16ms then flush, reducing Map/Array copy overhead.

### P3 -- Polish
11. **Add tab count limit or warning** at e.g. 50 tabs.
12. **Add max-depth indicator** in FileTree for deeply nested directories.
13. **Add FileTree empty state** when directory has no files.
14. **Replace `confirm()` dialogs** with custom modal for consistent Tauri webview behavior.
