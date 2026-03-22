# Unified Audit Report — Reasonance

**Date:** 2026-03-22
**Audited by:** 7 personas (Vibecoder, CTO, UX/UI Designer, Security, i18n, Stress, Performance)
**Application:** Reasonance IDE v0.5.0 (Svelte 5 + Tauri 2 + Rust)
**Reports synthesized:** 12 (vibecoder-report, cto-report, uxui-report, security-report, i18n-report, stress-report, performance-report, nielsen-scorecard, wcag-matrix, competitive-matrix, visual-testing-findings, adversarial-findings)

---

## Executive Summary

Reasonance is a well-architected multi-provider AI IDE with a clean Rust backend, thoughtful security boundaries, and above-average accessibility awareness for the category. The three-stage normalizer pipeline, declarative TOML-driven provider system, production-grade analytics dashboard, and consistent i18n infrastructure are genuine engineering strengths. The codebase has 284 Rust tests and 46 frontend test files, demonstrating a testing culture.

**Biggest wins:**
- Native Tauri 2 + Rust architecture avoids Electron's overhead -- a shared advantage only with Zed
- Multi-provider AI with 5+ providers and a normalizer abstraction layer -- unique in market
- Analytics dashboard is the most polished component (Nielsen avg 4.1/5.0), with real-time cost tracking, insights engine, and data export
- Accessibility foundations (skip links, ARIA tree widget, focus traps, semantic landmarks, Atkinson Hyperlegible font, `prefers-reduced-motion`) are ahead of every AI-native competitor

**Biggest risks:**
- 408 `unwrap()` calls in Rust (89 on mutex locks) create cascading crash risk from a single thread panic -- this is the single most dangerous issue, flagged by CTO, Stress, Performance, and Nielsen reports
- Path validation gaps allow filesystem traversal via IPC commands (`list_dir`, `grep_files`, workflow CRUD) -- flagged by CTO, Security, and Adversarial reports
- 1.89 MB monolithic JS bundle with zero code splitting -- every user pays the full cost of every feature on first load
- RTL (Arabic) is structurally broken: 80+ physical CSS properties across 20+ components, zero logical properties
- 67-70 i18n keys untranslated per locale (except Italian), plus 60+ hardcoded English strings in component templates

**Overall verdict:** Strong beta quality with excellent architectural foundations. Three mandatory fixes (mutex handling, path validation, error boundaries) gate production readiness. The accessibility and i18n gaps must be resolved to deliver on the "IDE for every human" promise.

**Nielsen Heuristic Average: 3.2/5.0** -- lowest scores in Help/Documentation (2.4), Error Prevention (2.9), and Error Recovery (2.9).

**WCAG 2.1 AA Conformance: Partial -- not yet conformant.** Pass rate: 19/30 Level A criteria; 12/20 Level AA criteria.

---

## Systemic Patterns

Issues appearing across 3+ audit reports represent the real structural problems in the codebase. These are grouped by theme.

### 1. Mutex Unwrap Cascade Risk (CTO, Stress, Performance, Nielsen, Adversarial)

**408 `unwrap()` calls** on mutex locks across 34 Rust source files. A panic in any thread holding a mutex poisons it, causing every subsequent `.lock().unwrap()` to also panic -- cascading to full application crash with no recovery. Worst offenders: `workflow_engine.rs` (48), `agent_runtime.rs` (42), `session_manager.rs` (33), `commands/fs.rs` (34).

Every component communicating with the backend is affected. This is the single largest risk to production stability.

### 2. Path Validation Gaps (CTO, Security, Adversarial)

File I/O sandboxing is applied to `read_file`/`write_file` but **not** to:
- `list_dir` (arbitrary directory enumeration)
- `grep_files` (arbitrary content search)
- `start_watching` (arbitrary filesystem monitoring)
- `load_workflow` / `save_workflow` / `delete_workflow` (arbitrary file read/write/delete)

The workflow commands are the most dangerous: they enable full sandbox escape via crafted workflow paths. A compromised frontend or XSS could read `/etc/shadow`, write to `/etc/cron.d/`, or delete `~/.ssh/authorized_keys`.

### 3. Missing Code Splitting / Monolithic Bundle (CTO, Performance, Visual Testing)

The entire UI compiles into a **single 1.89 MB (590 KB gzip) JavaScript chunk** containing CodeMirror, xterm.js, xyflow, highlight.js, marked, DOMPurify, and all 30+ components. No `manualChunks` configuration exists in vite.config.ts. Of 6 heavy library groups, only CodeMirror language packs and xterm WebGL addon are lazy-loaded. Every user pays the full cost of every feature on first load.

### 4. Unbounded Memory Growth (CTO, Stress, Performance, Adversarial)

Multiple stores grow without limits:
- `agentEvents` Map grows unboundedly during sessions (no max event count, no pruning)
- `agentSessions` creates `new Map(map)` on every event during streaming -- O(n) copies causing GC pressure
- Analytics store clones the entire `Vec<SessionMetrics>` on every query
- Chat messages have no virtualization -- 10K messages = 50K+ DOM nodes
- No explicit terminal scrollback cap configured

### 5. RTL Layout Completely Broken (i18n, UX/UI, Nielsen, WCAG)

Arabic locale correctly sets `dir="rtl"` on the document root, but **zero CSS logical properties** are used anywhere. 80+ physical directional properties (`margin-left`, `padding-left`, `text-align: left`, `border-left`, `left:`, `right:`) across 20+ components will render incorrectly in RTL. No `[dir="rtl"]` overrides exist.

### 6. Incomplete Internationalization (i18n, Vibecoder, Nielsen)

- 67-70 keys untranslated across 7 of 8 non-English locales (analytics and provider settings blocks)
- 60+ hardcoded English strings in component templates (`title`, `aria-label`, `placeholder`)
- No `Intl.NumberFormat` or `Intl.DateTimeFormat` -- numbers and dates display in English format regardless of locale
- No CJK or Devanagari fonts in the fallback chain
- Non-reactive `t()` function vs reactive `$tr()` store -- stale translations possible after locale switch

### 7. Accent Color Fails WCAG AA Contrast (UX/UI, WCAG, Nielsen, Visual Testing)

`--accent` (`#1d4ed8`) as text on dark backgrounds yields ~3.2:1 ratio (needs 4.5:1 for AA). Affects: WelcomeScreen primary button, StatusBar text, MarkdownPreview/ResponsePanel links, ShortcutsDialog group labels. StatusBar white-on-accent is ~3.8:1 (also fails). StatusBar `.file-encoding` at `opacity: 0.5` creates critically low contrast.

### 8. Silent Error Swallowing (Vibecoder, CTO, Stress, Nielsen)

Multiple failure paths are caught but produce no user-visible feedback:
- File read errors: `console.error` only
- Binary file open: raw error string, no friendly message
- LLM process spawn failure: caught, logged, silent
- Agent message send failure: streaming indicator stops, no error message shown
- Session fork failure: silent
- CLI stderr permanently lost (`Stdio::null()`)

### 9. No Onboarding / Help for First-Time Users (Vibecoder, Nielsen, Visual Testing)

Welcome screen shows "Open Folder" with zero context about LLM configuration prerequisites. No setup wizard, no getting-started flow, no explanation of the three-panel layout, no version info. The critical first step (configuring an LLM provider) is completely undiscoverable from the welcome screen. Nielsen H10 (Help/Docs) scored the lowest at 2.4/5.0.

### 10. Keyboard Navigation Gaps (UX/UI, WCAG, Visual Testing)

- FileTree: all items have `tabindex="-1"` with no `tabindex="0"` entry point -- cannot Tab into the tree
- EditorTabs: all tabs have `tabindex="0"` (should be only active tab); no ArrowLeft/ArrowRight
- MenuBar: `role="menubar"` incorrectly placed on individual items, not the bar container; submenus hover-only
- TerminalManager tabs: no arrow key navigation
- Toast: actions unreachable by keyboard
- MenuItem submenus: hover-only, no keyboard access

---

## Component Health Summary

| Component | Vibecoder | CTO | UX/UI | Security | i18n | Stress | Perf | Overall |
|-----------|:---------:|:---:|:-----:|:--------:|:----:|:------:|:----:|:-------:|
| WelcomeScreen | :warning: No onboarding | -- | :warning: ARIA gaps, contrast fail | -- | :warning: Hardcoded EN | :white_check_mark: Empty state OK | -- | :x: Poor |
| Settings | :warning: Buried scan CLI, no validation | -- | :white_check_mark: Dialog OK | :warning: Config write chain | :warning: Hardcoded EN placeholders | -- | -- | :warning: Fair |
| FileTree | :white_check_mark: Functional | -- | :warning: No tab entry, no Enter/Space | -- | :warning: Hardcoded EN | :warning: No empty state, no size guard | -- | :warning: Fair |
| Editor | :warning: Read-only default, no save btn | -- | :warning: `a11y_no_static_element_interactions` | -- | -- | :x: No file size guard, no debounce | :warning: Eager CM bundle | :warning: Fair |
| EditorTabs | :white_check_mark: | -- | :warning: tabindex, no arrows | -- | -- | :warning: No tab limit | -- | :white_check_mark: OK |
| ChatInput | :white_check_mark: | -- | :white_check_mark: | -- | :warning: Hardcoded EN | -- | -- | :white_check_mark: OK |
| ChatMessages | :warning: No apply/reject | :warning: Map cloning | :white_check_mark: | -- | -- | :x: No virtualization | :x: Unbounded growth | :x: Poor |
| CodeBlock/DiffBlock | :x: No actions | -- | :x: No apply, contrast | -- | -- | -- | -- | :x: Poor |
| Terminal | :white_check_mark: | :warning: stderr null | :x: No role, ARIA | -- | :warning: Hardcoded EN | :white_check_mark: | :warning: Regex overhead | :warning: Fair |
| TerminalManager | :warning: Mode TODO, swarm placeholder | -- | :warning: span-as-button, no arrows | -- | :warning: Hardcoded EN | :warning: YOLO race | -- | :warning: Fair |
| AnalyticsDashboard | :white_check_mark: | :warning: Vec clone scaling | :white_check_mark: | -- | :x: 74 keys untranslated | :white_check_mark: | -- | :white_check_mark: Good |
| AnalyticsBar | :white_check_mark: | -- | :warning: Contrast | -- | :x: Untranslated | -- | -- | :warning: Fair |
| SearchPalette | :white_check_mark: | -- | :white_check_mark: | -- | -- | :warning: Symlink loop | -- | :white_check_mark: Good |
| Toolbar | :white_check_mark: | -- | :warning: YOLO no aria-pressed | -- | :warning: Hardcoded EN | -- | -- | :warning: Fair |
| MenuBar/MenuItem | -- | -- | :x: Wrong ARIA structure, hover-only subs | -- | -- | -- | -- | :x: Poor |
| StatusBar | :white_check_mark: | -- | :x: Contrast failures | -- | -- | -- | -- | :warning: Fair |
| Toast | -- | -- | :warning: Keyboard unreachable | -- | :warning: Hardcoded EN | -- | -- | :warning: Fair |
| ResponsePanel | -- | -- | :x: No focus trap, no role | :warning: DOMPurify default | -- | -- | -- | :x: Poor |
| DiffView | :white_check_mark: Accept/reject | -- | :warning: No ARIA | -- | -- | -- | -- | :warning: Fair |
| HelpPanel | :white_check_mark: | -- | :white_check_mark: | -- | :white_check_mark: Locale-aware | -- | -- | :white_check_mark: Good |
| ShortcutsDialog | :white_check_mark: | -- | :white_check_mark: (except contrast) | -- | -- | -- | -- | :white_check_mark: Good |
| Session Mgmt | :x: No UI at all | -- | -- | -- | -- | -- | -- | :x: Critical |
| Rust Backend | -- | :warning: 89 mutex unwraps | -- | :x: Path validation gaps | -- | :x: 408 unwraps | :warning: Event bus I/O under lock | :x: Poor |
| CI/CD Pipeline | -- | :x: No tests in CI | -- | -- | -- | -- | -- | :x: Poor |

---

## Top 20 Critical Issues

### ISSUE-001: Workflow commands bypass project-root path validation (arbitrary file R/W/D)
**Severity:** HIGH | **Personas:** Security, CTO, Adversarial
**Location:** `src-tauri/src/commands/workflow.rs`, `src-tauri/src/workflow_store.rs:143-198`
**Evidence:** `load_workflow`, `save_workflow`, `delete_workflow` accept arbitrary file paths and pass directly to `std::fs::read_to_string`, `std::fs::write`, `std::fs::remove_file` without calling `validate_read_path` / `validate_write_path`.
**Impact:** Full sandbox escape. A malicious frontend (via XSS or compromised dependency) can read, write, or delete any file the process user can access.
**Fix:** Apply `validate_write_path()` / `validate_read_path()` to all workflow commands or restrict to `.reasonance/workflows/` subdirectory.

### ISSUE-002: Mutex `unwrap()` cascade risk -- 408 sites across 34 Rust files
**Severity:** HIGH | **Personas:** CTO, Stress, Performance, Nielsen
**Location:** All `src-tauri/src/**/*.rs` files, worst: `workflow_engine.rs` (48), `agent_runtime.rs` (42), `session_manager.rs` (33)
**Evidence:** Every `lock().unwrap()` on a `Mutex` will panic if the mutex is poisoned by a prior thread panic. In a multi-threaded Tauri app with event bus fan-out, this creates cascading crash risk.
**Impact:** Single thread panic crashes the entire desktop application with no recovery.
**Fix:** Replace with `lock().unwrap_or_else(|e| e.into_inner())` or propagate errors via `Result`. Priority: event bus, transport, session manager.

### ISSUE-003: `list_dir` and `grep_files` lack project-root path validation
**Severity:** HIGH | **Personas:** CTO, Security, Adversarial
**Location:** `src-tauri/src/commands/fs.rs:135,188`
**Evidence:** Both commands accept arbitrary paths without calling `validate_read_path`, unlike `read_file`/`write_file`.
**Impact:** Frontend can enumerate directories and search file contents anywhere on disk.
**Fix:** Add `validate_read_path` check using `ProjectRootState`.

### ISSUE-004: Google Gemini API key exposed in URL query parameter
**Severity:** HIGH | **Personas:** Security, CTO
**Location:** `src-tauri/src/commands/llm.rs:127-129`
**Evidence:** API key embedded in URL as `?key={}`, visible in logs, proxies, crash reports.
**Impact:** API key leakage to network intermediaries and logging systems.
**Fix:** Use `x-goog-api-key` header authentication instead.

### ISSUE-005: Monolithic 1.89 MB JS bundle with zero code splitting
**Severity:** HIGH | **Personas:** Performance, CTO, Visual Testing
**Location:** `vite.config.ts` (no `manualChunks`), `nodes/2.DNw6E_Td.js`
**Evidence:** Single chunk contains CodeMirror, xterm.js, xyflow, highlight.js, marked, DOMPurify, all 30+ components. Vite warns: "chunks larger than 500 kB."
**Impact:** Every user pays the full cost of every feature on first load. 590 KB gzip for initial page.
**Fix:** Split into per-feature chunks: CodeMirror, xterm, xyflow, markdown group. Use `{#await import()}` for lazy loading. Target: no chunk > 200 KB gzip.

### ISSUE-006: No tests wired into CI pipeline
**Severity:** HIGH | **Personas:** CTO
**Location:** `.github/workflows/release.yml`
**Evidence:** 284 Rust tests and 46 frontend test files exist but `cargo test` and `npm test` are not called in the release pipeline. Code ships without test validation.
**Impact:** Regressions can reach production undetected.
**Fix:** Add `cargo test` and `npm test` steps before the build step.

### ISSUE-007: Config write + PTY allowlist = privilege escalation chain
**Severity:** MEDIUM-HIGH | **Personas:** Security
**Location:** `src-tauri/src/commands/config.rs:13-19`, `src-tauri/src/commands/pty.rs:9-51`
**Evidence:** `write_config` accepts arbitrary string content without TOML validation. Config is later parsed by `is_allowed_command()` to determine PTY spawn allowlist. Writing a config with `command = "rm"` adds arbitrary binaries to the allowlist.
**Impact:** Privilege escalation from config write to arbitrary command execution.
**Fix:** Validate TOML content before writing. Separate command allowlist from user-editable config.

### ISSUE-008: No onboarding flow for first-time users
**Severity:** HIGH (UX) | **Personas:** Vibecoder, Nielsen, Visual Testing
**Location:** `src/lib/components/WelcomeScreen.svelte`
**Evidence:** Welcome screen shows "Open Folder" with zero context. No mention of LLM configuration requirement. "Scan CLI" button buried in Provider section of Settings, not in LLM config section.
**Impact:** First 30 seconds of experience feel broken. Users cannot use the app without configuration knowledge it does not teach.
**Fix:** Add setup wizard: detect CLIs, prompt for API keys, show guided first-session flow.

### ISSUE-009: Chat diffs and code blocks have no apply/reject actions
**Severity:** HIGH (UX) | **Personas:** Vibecoder, Nielsen
**Location:** `src/lib/components/chat/DiffBlock.svelte`, `src/lib/components/chat/CodeBlock.svelte`
**Evidence:** `DiffBlock` in chat is display-only (no accept/reject). `CodeBlock` has copy button but no "Apply to file" or "Insert into editor" action. The separate `DiffView` component (with accept/reject) is only used for file-watcher diffs, not chat diffs.
**Impact:** Highest cognitive gap in the app. Users must mentally map chat content to file actions and manually copy/paste.
**Fix:** Add "Apply" and "Reject" buttons to DiffBlock in chat context. Add "Insert into [filename]" to CodeBlock.

### ISSUE-010: No session management UI
**Severity:** HIGH (UX) | **Personas:** Vibecoder
**Location:** `src/lib/stores/agent-session.ts` (stores exist), no component
**Evidence:** `agentSessions` Map tracks sessions in stores. No component renders a session list, history browser, search, rename, or delete UI. Fork button creates invisible orphan sessions.
**Impact:** Users cannot browse, switch, or manage chat sessions. Past conversations are invisible.
**Fix:** Build a session sidebar or drawer with list, search, rename, delete.

### ISSUE-011: Accent color fails WCAG AA contrast on dark backgrounds
**Severity:** MEDIUM | **Personas:** UX/UI, WCAG, Nielsen, Visual Testing
**Location:** `src/app.css` (`--accent: #1d4ed8`); WelcomeScreen, StatusBar, MarkdownPreview, ShortcutsDialog, ResponsePanel
**Evidence:** `#1d4ed8` on `#121212` = ~3.2:1 (needs 4.5:1). StatusBar white on `#1d4ed8` = ~3.8:1 (needs 4.5:1). StatusBar `.file-encoding` at `opacity: 0.5` = critically low.
**Impact:** Low-vision users cannot read accent-colored text. Fails WCAG 1.4.3.
**Fix:** Lighten accent to ~`#4f8ff7` or similar for 4.5:1+ ratio on dark backgrounds.

### ISSUE-012: RTL layout completely broken -- 80+ physical CSS properties
**Severity:** MEDIUM | **Personas:** i18n, UX/UI, WCAG, Nielsen
**Location:** 20+ component files (App.svelte, FileTree, Toolbar, MenuItem, TerminalManager, AnalyticsDashboard, Settings, ResponsePanel, Toast, etc.)
**Evidence:** `dir="rtl"` set on document root but zero CSS logical properties used. 80+ instances of `margin-left`, `padding-left`, `text-align: left`, `border-left`, `left:`, `right:`.
**Impact:** Arabic locale is unusable. All layout, indentation, and positioning renders incorrectly.
**Fix:** Convert all physical CSS directional properties to logical equivalents. Add `[dir="rtl"]` overrides for absolute positioning.

### ISSUE-013: Unbounded agent event memory growth
**Severity:** MEDIUM | **Personas:** CTO, Stress, Performance
**Location:** `src/lib/stores/agent-events.ts:6`, `agent-session.ts:71-83`
**Evidence:** `agentEvents` Map grows without limit. `processAgentEvent` creates `new Map(map)` + `[...events, event]` on every event. During streaming at 100+ events/sec, this produces O(n^2) total work and significant GC pressure.
**Impact:** Long sessions consume excessive memory. Performance degrades over time.
**Fix:** Cap event arrays at 5000 with oldest-event pruning. Use mutable Map updates or Svelte 5 `$state()`.

### ISSUE-014: 67-70 i18n keys untranslated per locale
**Severity:** MEDIUM | **Personas:** i18n
**Location:** All locale JSON files in `src/lib/i18n/` (except `it.json`)
**Evidence:** Analytics and provider settings key blocks (74 keys) are in English across ar, de, es, fr, hi, pt, zh. Italian is the only fully translated non-English locale.
**Impact:** Non-English users see mixed-language UI in analytics dashboard and provider settings.
**Fix:** Translate 74 keys x 7 locales = 518 translations needed.

### ISSUE-015: DOMPurify used with default configuration
**Severity:** MEDIUM | **Personas:** Security
**Location:** `MarkdownPreview.svelte:23`, `ResponsePanel.svelte:34`, `chat/TextBlock.svelte:10`
**Evidence:** Default DOMPurify allows `<form>`, `<input>`, `<details>`, `<summary>`, `<svg>` which could be used for UI redressing or phishing in LLM responses.
**Impact:** Crafted LLM responses could render phishing forms or misleading UI elements.
**Fix:** Use hardened allowlist: restrict to safe content tags only.

### ISSUE-016: FileTree keyboard inaccessible -- no tab entry point
**Severity:** MEDIUM | **Personas:** UX/UI, WCAG
**Location:** `src/lib/components/FileTree.svelte:153`
**Evidence:** All tree items have `tabindex="-1"`. No item has `tabindex="0"` for initial Tab focus. Enter/Space to activate missing -- only `onclick` handler. Violates WCAG 2.1.1.
**Impact:** Keyboard-only users cannot access the file tree at all.
**Fix:** Add roving tabindex pattern: first visible item gets `tabindex="0"`, add Enter/Space handlers.

### ISSUE-017: Event bus holds lock during synchronous file I/O
**Severity:** MEDIUM | **Personas:** CTO, Performance
**Location:** `src-tauri/src/transport/event_bus.rs:49-59,165`
**Evidence:** `publish()` holds the subscriber Mutex during all `on_event()` callbacks. `SessionHistoryRecorder` does synchronous JSONL append inside the lock. Disk I/O serializes all event processing.
**Impact:** Under high throughput, file I/O latency blocks frontend emission causing visible UI freezes.
**Fix:** Buffer events in memory, flush to disk via a channel to a dedicated writer task.

### ISSUE-018: Stderr silenced for CLI processes
**Severity:** MEDIUM | **Personas:** CTO, Stress, Nielsen
**Location:** `src-tauri/src/transport/mod.rs:99`
**Evidence:** `cmd.stderr(Stdio::null())` permanently discards all CLI error output. Most LLM CLI tools print diagnostics to stderr.
**Impact:** When a provider CLI errors, the user gets no feedback -- session hangs or terminates silently.
**Fix:** Capture stderr, surface as warning events or log entries.

### ISSUE-019: No file size guard on `read_file`
**Severity:** MEDIUM | **Personas:** Stress, Adversarial
**Location:** `src-tauri/src/commands/fs.rs:121`
**Evidence:** `fs::read_to_string` with no `metadata.len()` check. Entire content loaded into JS string then into CodeMirror.
**Impact:** Opening a 50MB file freezes or crashes the application via memory exhaustion.
**Fix:** Check file size before reading. Reject or warn for files > 10 MB.

### ISSUE-020: `start_watching` accepts arbitrary paths without validation
**Severity:** MEDIUM | **Personas:** Security
**Location:** `src-tauri/src/commands/fs.rs:228-234`
**Evidence:** Path passed directly to filesystem watcher without project-root validation.
**Impact:** Monitor sensitive directories for changes outside project scope.
**Fix:** Validate path is within project root before starting watcher.

---

## Promise vs Reality

| Reasonance Claims | Evidence | Verdict |
|-------------------|----------|---------|
| **"Built for every human"** | WCAG 2.1 AA: Partial (not conformant). FileTree keyboard-inaccessible. Accent color fails contrast. RTL completely broken (80+ physical CSS properties). No high contrast mode. No forced-colors support. No screen reader dedicated mode. Toast actions unreachable by keyboard. MenuBar submenus hover-only. 60+ hardcoded English strings. No CJK/Devanagari fonts. | :x: **Not yet.** Strong foundations (skip links, ARIA tree, focus traps, landmarks, Atkinson font) but systemic gaps in keyboard nav, contrast, and RTL prevent conformance. Better than Cursor/Zed/Windsurf on a11y, but far behind VS Code. |
| **"Secure by design"** | API keys never reach frontend (good). PTY command allowlist (good). File I/O sandboxing on `read_file`/`write_file` (good). BUT: workflow commands bypass all path validation (sandbox escape). `list_dir`/`grep_files` accept arbitrary paths. Config write enables privilege escalation to arbitrary command execution. Google API key in URL. DOMPurify uses default config. | :warning: **Partially.** Security architecture is thoughtful with multiple SEC-annotated mitigations, but critical gaps in workflow commands and path validation undermine the sandbox. The foundations are solid; the coverage is incomplete. |
| **"Native speed"** | Tauri 2 + Rust avoids Electron overhead (genuine advantage). 0 cargo audit vulnerabilities. BUT: 1.89 MB monolithic JS bundle with no code splitting. `agentEvents` store creates O(n^2) copies during streaming. Analytics store clones entire Vec on every query. Event bus does synchronous file I/O under lock. No chat message virtualization (10K messages = 50K+ DOM nodes). | :warning: **Partially.** The native backend is genuinely fast. The frontend bundle and reactivity patterns undermine perceived performance. The architecture supports speed, but the implementation has bottlenecks. |
| **"AI-native"** | Multi-provider support (5+ providers) with normalizer abstraction layer (unique). Workflow/swarm builder (early). Analytics with cost tracking and insights. BUT: no onboarding wizard. Chat diffs cannot be applied. Code blocks have no "insert into file" action. Session management is invisible. Mode switching is a TODO. Swarm is a placeholder. Stderr silenced. No chat history persistence UI. | :warning: **Partially.** The AI infrastructure is strong and differentiated. The AI interaction UX has critical gaps (no code apply, no diff accept in chat, no sessions). The backend is AI-native; the frontend experience is not yet. |

---

## Competitive Position

### Where Reasonance Leads
1. **Multi-provider AI native** -- Only IDE with built-in support for 5+ AI providers simultaneously, with a normalizer abstraction layer. No competitor offers this.
2. **Native + AI** -- Tauri 2 + Rust gives Electron-free performance. Shared advantage with Zed only, but Zed lacks multi-provider AI.
3. **Built-in analytics** -- Cost tracking, usage analytics, insights engine, data export. No competitor has this built-in.
4. **Workflow orchestration** -- Visual swarm/workflow builder for multi-agent pipelines. Early but unique.
5. **Accessibility awareness** -- More ARIA attributes, focus traps, and semantic landmarks than any AI-native competitor (Cursor, Zed, Windsurf all have worse a11y).

### Where Reasonance Trails
1. **Accessibility maturity vs VS Code** -- VS Code has dedicated screen reader mode, high contrast themes, years of a11y investment. Reasonance has foundations but not conformance.
2. **Extension ecosystem** -- VS Code's marketplace is unmatched. Reasonance has no extension system.
3. **i18n depth** -- VS Code supports 50+ locales. Reasonance has 9 with incomplete translations.
4. **Community size** -- Early-stage product vs massive installed bases.
5. **Chat UX maturity** -- Cursor's inline code application is ahead. Reasonance's chat diffs are display-only.

### Strategic Opportunity
**First AI-native IDE with genuine accessibility.** The competition is weak here:
- Cursor/Windsurf broke VS Code's accessibility with AI overlays
- Zed chose native rendering, sacrificing OS a11y hooks
- No one has solved "accessible AI features"

If Reasonance achieves WCAG AA with accessible AI interactions, it occupies a unique market position no competitor claims.

---

## Positive Findings — What NOT to Break

These features work well across multiple audits and represent the product's core strengths:

1. **Analytics Dashboard** (Nielsen 4.1/5.0) -- Period selector, KPI cards with sparklines, provider comparison, insights engine, CSV/JSON export, skeleton loading, ARIA roles, `role="progressbar"`, budget alerts. The most polished component in the application. (Vibecoder, CTO, UX/UI, Nielsen)

2. **Three-panel layout with resizable dividers** -- Skip links for keyboard navigation, `role="separator"` on dividers, keyboard resize with Arrow keys + Shift modifier, resize overlay prevents selection during drag, error boundaries on each panel with retry buttons. (Vibecoder, UX/UI, Nielsen)

3. **Security architecture foundations** -- API keys stored as env var names (never in config/frontend), PTY command allowlist, `open_external` URL scheme validation, env var access allowlist, CSP without `unsafe-eval`, `withGlobalTauri: false`, updater signature verification. (Security, CTO)

4. **Declarative normalizer pipeline** -- Three-stage pipeline (Rules Engine -> State Machine -> Content Parser) with TOML-driven configuration. Adding a new provider requires only a TOML file. Hot-reload via `reload_provider()`. `TimedFlush` mechanism prevents stuck tool events. Production-grade extensibility. (CTO)

5. **SearchPalette** (Nielsen 4.1/5.0) -- 4-tier fuzzy scoring, keyboard navigation, ARIA listbox/option pattern, loading/empty states, relative path display. (Vibecoder, UX/UI, Nielsen)

6. **Terminal empty state** -- When no LLMs configured: banner with Settings button. When LLMs exist: selectable cards with start button. Gold standard for empty states in the app. (Vibecoder, Stress)

7. **Internationalization infrastructure** -- Every user-facing string through `$tr()`, 9 locales, lazy-loaded locale files, locale-aware help docs, `dir="rtl"` wiring for Arabic. The system is well-designed even if translations are incomplete. (i18n, Vibecoder)

8. **HelpPanel and ShortcutsDialog** (Nielsen 4.1/5.0 each) -- Locale-aware markdown docs with in-doc search and highlighting, grouped shortcuts with `<kbd>` rendering, F1 shortcut. (Vibecoder, UX/UI, Nielsen)

9. **Robust Rust test coverage** -- 284 `#[test]` functions across 42 modules covering all backend layers. (CTO)

10. **CSS design system** -- 35 custom properties covering colors, typography, spacing, borders, focus styles. `font-display: swap` on font faces. WOFF2-only strategy. Minimal reset. `prefers-reduced-motion` blanket override. (Performance, UX/UI)

11. **Session persistence** -- Atomic writes (write-to-tmp-then-rename) for metadata. Append-only JSONL for events. Fork support with event slicing. `SessionHistoryRecorder` batches metadata writes every 10 events. (CTO)

12. **Error boundaries on all panels** -- `<svelte:boundary>` wraps file tree, editor, and terminal with retry buttons. Crashes in one panel do not take down the entire app. (Vibecoder, Stress)
