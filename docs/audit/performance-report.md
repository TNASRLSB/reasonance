# Performance Audit Report

**Date:** 2026-03-22
**Persona:** Performance engineer
**Judgment:** Is the app fast and lean, or hiding bloat?

## Executive Summary

The app is **hiding significant bloat behind a monolithic page chunk**. The entire UI compiles into a single 1.89 MB (590 KB gzip) JavaScript chunk (`nodes/2.DNw6E_Td.js`) that includes CodeMirror, xterm.js, xyflow, highlight.js, marked, DOMPurify, and all 30+ Svelte components. There is no code splitting at the page level. The vite config has zero `manualChunks` configuration, and `optimizeDeps.include` eagerly pre-bundles all heavy libraries.

On the positive side: language extensions for CodeMirror are properly lazy-loaded, the WebGL addon is dynamically imported, stores are lean and well-structured, CSS uses custom properties throughout, and Rust hot paths are reasonably efficient. But the lack of route-level or component-level code splitting is the dominant performance problem.

**Verdict:** The foundation is solid, but the single-chunk architecture means every user pays the full cost of every feature on first load.

---

## Bundle Analysis

**Total client JS:** ~2.96 MB raw, ~930 KB gzip (estimated from build output)
**Total client CSS:** 117.25 KB raw, 18.27 KB gzip

| Chunk | Size (raw) | Size (gzip) | Contents | Lazy? |
|-------|-----------|-------------|----------|-------|
| `nodes/2.DNw6E_Td.js` | 1,887 KB | 590 KB | **ALL page components**: CodeMirror core, xterm.js, xyflow/svelte, highlight.js, marked, DOMPurify, all 30+ Svelte components | No -- eager, monolithic |
| `C6dmv-dA.js` | 304 KB | 98 KB | Likely Svelte runtime + SvelteKit internals | No |
| `tm6uBZu6.js` | 122 KB | 34 KB | Vendor chunk (unknown) | No |
| `BlnxQmLT.js` | 101 KB | 34 KB | Vendor chunk (unknown) | No |
| `MZrNszsx.js` | 85 KB | 34 KB | Vendor chunk (unknown) | No |
| `DS7jGy6v.js` | 71 KB | 26 KB | Vendor chunk (unknown) | No |
| `NniftOZN.js` | 57 KB | 22 KB | Vendor chunk (unknown) | No |
| `2.DwbSUwYR.css` | 117 KB | 18 KB | All CSS (includes xterm.css, xyflow/style.css, highlight.js theme) | No |

**Key problem:** Vite triggered a warning: "Some chunks are larger than 500 kB after minification." The 1.89 MB chunk is nearly 4x that threshold. No `build.rolldownOptions.output.codeSplitting` or `manualChunks` is configured in `vite.config.ts` (file: `/home/uh1/VIBEPROJECTS/REASONANCE/vite.config.ts`).

---

## Reactivity Issues

| Store/Component | Issue | Impact | Fix |
|----------------|-------|--------|-----|
| `agent-events.ts:60-64` | `processAgentEvent()` creates a **new Map copy on every event** (`new Map(map)`) then spreads the events array (`[...events, event]`). During active streaming, this fires many times per second. | **High** -- O(n) copy of all sessions' event lists on every text token. GC pressure grows linearly with session length. | Use a mutable update pattern or SvelteKit 5 `$state` with `Map.set()` for fine-grained reactivity. Consider ring buffer with max event count. |
| `agent-session.ts:71-83` | `updateTokens()` copies the entire `Map` on every usage event just to update two counters on one session. Same pattern in `updateMetrics()`, `updateSessionStatus()`, `updateViewMode()`. | **Medium** -- Repeated full Map clones for single-field updates during streaming. | Convert to `$state(new Map())` with direct `.set()` mutations, or use per-session writable stores. |
| `agent-events.ts:6` | `agentEvents` stores a `Map<string, AgentEvent[]>` where event arrays grow unbounded. No max length, no cleanup, no pruning. | **High** -- Memory grows linearly with session duration. Long sessions will accumulate thousands of events. | Add a max event count (e.g., 5000) with oldest-event pruning, or move historical events to IndexedDB. |
| `engine.ts:9-25` | Five `derived` stores chain off `currentRun` and then off each other: `nodeStates` -> `completedNodeCount`, `totalNodeCount`, `activeNodeCount`, `errorNodeCount` -> `statusSummary`. | **Low** -- Svelte derived stores are lazy and only recompute when subscribers exist. Acceptable for current scale. | No action needed unless node counts reach 1000+. |
| `Terminal.svelte:144-158` | `parseContextToken()` runs 5 regex matches on every PTY data chunk, then does a full `terminalTabs.update()` with nested `.map()` even when no match is found. | **Medium** -- Unnecessary store updates and regex on every output byte. | Short-circuit: only call `terminalTabs.update()` when at least one regex matches (the `if (ctxMatch || tokenMatch || ...)` gate exists but still triggers a full nested map). Cache previous values and skip update when unchanged. |

---

## Memory Leak Risks

| Component | Pattern | Risk Level |
|-----------|---------|------------|
| `Terminal.svelte:178-191` | `adapter.onPtyData()` and `adapter.onPtyExit()` push unlisten functions into `cleanups[]` **asynchronously**. If the component unmounts before the `.then()` resolves, the unlisten is never called. | **Medium** -- Race condition between unmount and async listener registration. The `onMount` return function cleans up `cleanups[]`, but late-resolving promises add to it after cleanup runs. |
| `agent-events.ts:6` | `agentEvents` Map grows unbounded. `clearSessionEvents()` exists but is only called explicitly -- there is no automatic cleanup when sessions end. | **High** -- Long-running app with multiple sessions will leak event arrays. |
| `analytics.ts:42` | Module-level `cache` object (`const cache: Record<string, CacheEntry<unknown>> = {}`) has no TTL-based eviction. `invalidateCache()` exists but must be called manually. | **Low** -- Cache entries are small analytics payloads, but stale entries persist forever. |
| `TerminalManager.svelte:235-248` | Agent event listener via `$effect` with cleanup -- **correctly implemented**. Uses `cancelled` flag + `unlisten` pattern. | **Safe** -- Good pattern. |
| `+page.svelte:129-234` | `onMount` registers event listeners on `window` and `document`, stores cleanup in `cleanups[]`. `onDestroy` calls all cleanups. | **Safe** -- Properly paired. |
| `theme.ts:17` | `themeMode.subscribe()` in `initTheme()` returns an unsubscribe function that is **never captured or called**. | **Low** -- Only one subscription created at app init, persists for app lifetime. Acceptable for a desktop app. |

---

## Lazy Loading Status

| Library | Approx Size | Currently | Should Be |
|---------|-------------|-----------|-----------|
| **CodeMirror core** (`codemirror`, `@codemirror/state`, `@codemirror/view`, `@codemirror/language`) | ~150 KB gzip | Eager (static import in `Editor.svelte:3-5`, `DiffView.svelte:3-5`) | **Lazy** -- Only needed when a file is opened. Dynamic import when first file tab activates. |
| **CodeMirror language packs** | ~5-15 KB each | **Lazy** (dynamic import in `languages.ts:8-55`) | Already correct. Good. |
| **xterm.js** (`@xterm/xterm` + 6 addons) | ~120 KB gzip | Eager (static imports in `Terminal.svelte:3-9`) | **Lazy** -- Only needed when terminal panel is active. Dynamic import on first terminal spawn. |
| **xterm WebGL addon** | ~30 KB gzip | **Lazy** (dynamic import in `Terminal.svelte:86`) | Already correct. Good. |
| **@xyflow/svelte** | ~50 KB gzip | Eager (static import in `SwarmCanvas.svelte:2-11`, `AgentFlowNode.svelte:2`, etc.) | **Lazy** -- Swarm canvas is a secondary feature. Should be behind `{#await import(...)}` or a lazy wrapper component. |
| **highlight.js** (full bundle) | ~80 KB gzip | Eager (static import in `MarkdownPreview.svelte:4`, `ResponsePanel.svelte:4`) | **Lazy** -- Only needed for markdown preview and response rendering. `import('highlight.js')` on demand. Also: importing the full bundle includes ALL languages. Use `highlight.js/lib/core` + register only needed languages. |
| **marked** | ~15 KB gzip | Eager (static import in `MarkdownPreview.svelte:2`, `ResponsePanel.svelte:2`) | **Lazy** -- Same components as highlight.js. Bundle together behind dynamic import. |
| **DOMPurify** | ~10 KB gzip | Eager (static import in `MarkdownPreview.svelte:6`, `ResponsePanel.svelte:6`) | **Lazy** -- Same as above. |

**Summary:** Of the 6 heavy library groups, only CodeMirror language packs and the xterm WebGL addon are properly lazy-loaded. The other 4 groups (CodeMirror core, xterm core, xyflow, highlight.js) are statically imported and bundled into the monolithic page chunk.

---

## CSS Performance

**File:** `/home/uh1/VIBEPROJECTS/REASONANCE/src/app.css` (239 lines)

**Custom properties:** 35 CSS custom properties defined in `:root`, covering colors, typography, spacing, borders, and focus styles. **Excellent** -- nearly all values are tokenized. Very few hard-coded values in component styles.

**Strengths:**
- All color values use CSS custom properties, enabling theme switching without JS
- Font sizes, weights, and spacing are all tokenized
- `font-display: swap` on all `@font-face` declarations prevents FOIT
- `prefers-reduced-motion` media query properly disables animations
- WOFF2-only font strategy (correct for Tauri/WebKitGTK)
- Minimal reset (no bloated normalize.css)

**Issues:**

| Issue | Location | Impact |
|-------|----------|--------|
| Universal selector `*` used twice | `app.css:161-168` (reset), `app.css:215-218` (scrollbar) | **Low** -- Universal selectors are fast in modern engines, and both rules are in the global stylesheet. Acceptable. |
| `:global()` selectors in component styles | `Terminal.svelte:348-354` (`.xterm`, `.xterm-viewport`) | **Low** -- Necessary for styling third-party library DOM. No alternative. |
| Duplicate `*` rule blocks | `app.css:161` and `app.css:215` | **Negligible** -- Could be merged but no measurable impact. |
| No CSS containment (`contain: layout style`) | All components | **Low** -- Adding `contain: layout style` to major panels (file tree, editor, terminal) would help the browser optimize layout recalculations during resize. |

**Verdict:** CSS is clean and well-architected. No significant performance issues.

---

## Rust Hot Path Analysis

### 1. Stream Reader (`src-tauri/src/transport/stream_reader.rs`)

**Pattern:** Async task reads lines from child process stdout, passes through normalizer pipeline, publishes events via event bus.

| Issue | Location | Impact |
|-------|----------|--------|
| `std::sync::Mutex` used for `NormalizerPipeline` in async context | `stream_reader.rs:15` (`Arc<Mutex<NormalizerPipeline>>`) | **Medium** -- Holding a `std::sync::Mutex` across an async boundary can block the tokio executor thread if contended. However, the lock is held only briefly (line 36-38), and there is likely only one reader per session, so contention is low in practice. |
| `raw_line.trim().to_string()` allocates a new String on every line | `stream_reader.rs:30` | **Low** -- One allocation per line is acceptable. Could use `Cow` but marginal benefit. |

### 2. Event Bus (`src-tauri/src/transport/event_bus.rs`)

**Pattern:** Pub/sub with subscriber list behind `Mutex`. Each `publish()` locks, iterates all subscribers, and calls `on_event()`.

| Issue | Location | Impact |
|-------|----------|--------|
| Lock held during all subscriber callbacks | `event_bus.rs:49-59` | **Medium** -- `publish()` holds the Mutex lock while iterating and calling every subscriber's `on_event()`. If any subscriber is slow (e.g., `FrontendEmitter` doing Tauri IPC, or `SessionHistoryRecorder` doing file I/O), it blocks all other publishers. |
| `event.clone()` for each subscriber in `FrontendEmitter` | `event_bus.rs:126` | **Medium** -- Every event is cloned for serialization to the frontend. For high-frequency text events during streaming, this is redundant cloning. Consider using `Arc<AgentEvent>` to share ownership. |
| `session_id.to_string()` allocation in `FrontendEmitter::on_event()` | `event_bus.rs:124` | **Low** -- String allocation per event. Could accept `&str` in the payload and let serde borrow. |
| `SessionHistoryRecorder` does synchronous file I/O inside the lock | `event_bus.rs:165` (`store.append_event`) | **High** -- File I/O (JSONL append) happens while the event bus Mutex is held. Under high event throughput, this serializes all event processing behind disk I/O latency. |
| No backpressure mechanism | Event bus design | **Medium** -- If the frontend cannot keep up with event emission (e.g., during very fast streaming), events queue up in the Tauri event system with no way to drop or coalesce them. |

### 3. Analytics Store (`src-tauri/src/analytics/store.rs`)

**Pattern:** JSONL append-only file with in-memory cache of all completed session metrics.

| Issue | Location | Impact |
|-------|----------|--------|
| `all_completed()` clones the entire Vec on every call | `store.rs:43-45` | **Medium** -- Returns `Vec<SessionMetrics>` clone. If called frequently from the frontend (e.g., on dashboard open), this copies all historical metrics. |
| `load()` reads entire JSONL file into memory at startup | `store.rs:47-70` | **Low** -- One-time cost at startup. Acceptable unless metrics file grows very large (10000+ sessions). |
| No index or pagination for queries | Store design | **Low** -- Currently acceptable but will not scale to thousands of sessions. Consider SQLite for analytics if growth is expected. |

---

## Recommendations (by impact)

| # | Impact | Effort | Description |
|---|--------|--------|-------------|
| 1 | **Critical** | Medium | **Split the monolithic page chunk.** The 1.89 MB `nodes/2.js` chunk must be broken up. Use SvelteKit's `{#await import()}` pattern or `manualChunks` in vite config to split: (a) CodeMirror into its own chunk, (b) xterm.js into its own chunk, (c) xyflow/svelte into its own chunk, (d) highlight.js + marked + DOMPurify into a "markdown" chunk. Target: no chunk > 200 KB gzip. |
| 2 | **High** | Low | **Cap agent event arrays.** Add a max length (e.g., 5000 events) to `agentEvents` Map in `agent-events.ts`. Prune oldest events when limit is reached. This prevents unbounded memory growth during long sessions. |
| 3 | **High** | Medium | **Move file I/O out of event bus lock.** In `SessionHistoryRecorder::on_event()`, buffer events in memory and flush to disk asynchronously (e.g., via a channel to a dedicated writer task), rather than doing synchronous file I/O while holding the event bus Mutex. |
| 4 | **High** | Low | **Use `highlight.js/lib/core` instead of the full bundle.** The full `highlight.js` import includes every language grammar (~80 KB gzip). Import `core` and register only the 10-15 languages the app actually supports (matching the CodeMirror language list in `languages.ts`). |
| 5 | **Medium** | Low | **Lazy-load SwarmCanvas.** The xyflow library (~50 KB gzip) is loaded even when the swarm feature is not used. Wrap `SwarmCanvas.svelte` in a dynamic import wrapper: `{#await import('./swarm/SwarmCanvas.svelte') then mod}<mod.default .../>{/await}`. |
| 6 | **Medium** | Low | **Avoid full Map recreation in agent stores.** Replace `writable<Map>` + `new Map(map)` pattern in `agent-events.ts` and `agent-session.ts` with Svelte 5 `$state(new Map())` and direct `.set()` mutations to avoid O(n) copies on every streaming event. |
| 7 | **Medium** | Low | **Add `contain: layout style` to major panels.** Apply CSS containment to the file tree, editor, and terminal panels to limit the scope of layout recalculations during resize operations. |
| 8 | **Medium** | Medium | **Use `Arc<AgentEvent>` in event bus.** Replace `event.clone()` per subscriber with `Arc<AgentEvent>` to share ownership and eliminate redundant deep copies during high-frequency streaming. |
| 9 | **Low** | Low | **Short-circuit terminal regex parsing.** In `Terminal.svelte:136-158`, skip the `terminalTabs.update()` call entirely when no regex matches. Currently the update runs even on non-matching data. Cache previous values and deduplicate. |
| 10 | **Low** | Low | **Add `optimizeDeps.include` entries for CodeMirror merge.** `@codemirror/merge` is missing from `vite.config.ts:8-24` pre-bundle list, which can cause slower dev server cold starts when the diff view is first opened. |
