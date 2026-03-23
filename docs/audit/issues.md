# Audit Issue List

**Date:** 2026-03-23 (updated with live test results)
**Total issues:** 62 (code-level) + 2 newly confirmed via live testing

> **Live Testing Update (2026-03-23):** Playwright + axe-core + Lighthouse tests confirmed several code-level findings and surfaced 2 new issues: missing `<title>` element (WCAG 2.4.2) and zero ARIA landmarks in the app. See `visual-testing-findings.md` for full live test data. Lighthouse accessibility score: **88/100**.

## Summary
| Severity | Count |
|----------|-------|
| P0 Blocker | 5 |
| P1 Critical | 12 |
| P2 Major | 21 |
| P3 Minor | 16 |
| P4 Enhancement | 8 |

## Issues

### P0 — Blocker

#### ISSUE-001: Workflow commands bypass project-root path validation (arbitrary file read/write/delete)
- **Severity:** P0
- **Component:** Workflow Store / IPC Commands
- **Found by:** Security, Adversarial
- **WCAG criterion:** N/A
- **Description:** `load_workflow`, `save_workflow`, `delete_workflow`, `duplicate_workflow`, and `save_to_global` accept arbitrary `file_path` strings and pass them directly to `std::fs::read_to_string`, `std::fs::write`, and `std::fs::remove_file` without any path validation. A malicious frontend script (via XSS or compromised dependency) could read, write, or delete any file the process user has permissions for.
- **Impact:** Complete sandbox escape. Arbitrary file read/write/delete on the host filesystem. A crafted `.reasonance/workflows/*.json` file or XSS could exploit this.
- **Suggested fix:** Apply `validate_read_path()` / `validate_write_path()` to all workflow commands, or restrict workflow paths to the project `.reasonance/workflows/` subdirectory with canonicalization checks. Also apply to `list_workflows` which accepts arbitrary `dir` parameter.
- **Files:** `src-tauri/src/commands/workflow.rs` (all commands), `src-tauri/src/workflow_store.rs:143-198`

#### ISSUE-002: Mutex `lock().unwrap()` causes cascading panics (408 occurrences)
- **Severity:** P0
- **Component:** Rust Backend (all modules)
- **Found by:** CTO, Stress, Nielsen
- **WCAG criterion:** N/A
- **Description:** 408 `unwrap()` calls on mutex locks across 34 Rust source files. A single thread panic poisons the associated mutex, causing ALL subsequent `.lock().unwrap()` calls on that mutex to panic, creating a cascade that crashes the entire desktop application with no recovery path. Worst offenders: `workflow_engine.rs` (48), `agent_runtime.rs` (42), `session_manager.rs` (33), `commands/fs.rs` (34).
- **Impact:** Any single panic in any Rust thread takes down the entire application. Users lose all unsaved work across all sessions.
- **Suggested fix:** Replace all `lock().unwrap()` with `lock().unwrap_or_else(|e| e.into_inner())` (if poisoned state is recoverable) or propagate errors via `Result`. Priority: event bus, transport session map, and workflow engine where concurrent access is highest.
- **Files:** `src-tauri/src/**/*.rs` — 34 files, top offenders: `workflow_engine.rs`, `agent_runtime.rs`, `session_manager.rs`, `transport/mod.rs`, `analytics/store.rs`, `normalizer/mod.rs`, `transport/event_bus.rs`

#### ISSUE-003: No file size guard on `read_file` — 50MB file freezes/crashes app
- **Severity:** P0
- **Component:** File System / Editor
- **Found by:** Stress, Adversarial, Nielsen
- **WCAG criterion:** N/A
- **Description:** `read_file` calls `fs::read_to_string` with no size check (`commands/fs.rs:121`). The entire file content is loaded into memory, transferred via IPC, and loaded into CodeMirror. Opening a large file (50MB+) will exhaust memory and freeze or crash the application.
- **Impact:** Denial of service. Any user accidentally opening a large file loses the entire application state.
- **Suggested fix:** Check `metadata.len()` before `read_to_string`. Reject or warn for files > 10MB. Show a user-friendly message in the editor area.
- **Files:** `src-tauri/src/commands/fs.rs:121`

#### ISSUE-004: `list_dir` and `grep_files` skip path validation — directory traversal via IPC
- **Severity:** P0
- **Component:** File System / IPC Commands
- **Found by:** CTO, Security, Adversarial
- **WCAG criterion:** N/A
- **Description:** While `read_file` and `write_file` enforce project-root sandboxing via `validate_read_path()`, the `list_dir` (`fs.rs:135`) and `grep_files` (`fs.rs:188`) commands accept arbitrary paths without any validation. This allows the frontend to enumerate directory contents and search file contents anywhere on the filesystem.
- **Impact:** Information disclosure — full filesystem directory listing and content search from the frontend. Bypasses the otherwise solid project-root sandboxing.
- **Suggested fix:** Add `validate_read_path` check or directory-level equivalent to both commands. Require paths to be within the project root or the Reasonance config directory.
- **Files:** `src-tauri/src/commands/fs.rs:135,188`

#### ISSUE-005: Monolithic 1.89 MB JavaScript bundle — no code splitting
- **Severity:** P0
- **Component:** Build / Bundle
- **Found by:** Performance, CTO, Nielsen
- **WCAG criterion:** N/A
- **Description:** The entire UI compiles into a single 1.89 MB (590 KB gzip) JavaScript chunk (`nodes/2.DNw6E_Td.js`) containing CodeMirror, xterm.js, xyflow, highlight.js, marked, DOMPurify, and all 30+ Svelte components. Vite config has zero `manualChunks` configuration. Only CodeMirror language packs and the xterm WebGL addon are properly lazy-loaded.
- **Impact:** Every user pays the full load cost of every feature on first launch. 4x over Vite's recommended chunk size threshold. Slow startup on all platforms.
- **Suggested fix:** Split into lazy-loaded chunks: (a) CodeMirror core (~150 KB gzip, load on first file open), (b) xterm.js (~120 KB gzip, load on first terminal spawn), (c) xyflow/svelte (~50 KB gzip, load on swarm canvas open), (d) highlight.js + marked + DOMPurify (~105 KB gzip, load on first markdown render). Use `{#await import()}` pattern or `manualChunks` in vite config. Also use `highlight.js/lib/core` instead of full bundle.
- **Files:** `vite.config.ts`, `src/lib/components/Editor.svelte:3-5`, `src/lib/components/Terminal.svelte:3-9`, `src/lib/components/swarm/SwarmCanvas.svelte:2-11`, `src/lib/components/MarkdownPreview.svelte:2-6`

---

### P1 — Critical

#### ISSUE-006: No onboarding flow — user cannot discover LLM configuration requirement
- **Severity:** P1
- **Component:** WelcomeScreen
- **Found by:** Vibecoder, Nielsen
- **WCAG criterion:** N/A
- **Description:** The welcome screen shows only "Open Folder" with zero context. There is no onboarding wizard, no getting-started steps, no explanation of the three-panel layout, and no mention that an LLM provider must be configured before the app is useful. The "Scan CLI" auto-detection button is buried in the Provider section of Settings, not in the LLM config section.
- **Impact:** First-time users cannot accomplish anything. The app appears broken until they independently discover Settings and configure an LLM provider.
- **Suggested fix:** Add a setup wizard: detect installed CLIs automatically on first launch, prompt for API keys, show a "Your first session" guided flow. Move "Scan CLI" to the top of LLM config section.
- **Files:** `src/lib/components/WelcomeScreen.svelte:30-54`, `src/lib/components/Settings.svelte:601-603`

#### ISSUE-007: No session management UI — no history, no browsing, no switching
- **Severity:** P1
- **Component:** Session Management
- **Found by:** Vibecoder, Nielsen
- **WCAG criterion:** N/A
- **Description:** `agent-session.ts` tracks sessions with a Map and `activeAgentSessionId`, but there is no component to render a session list. Users cannot browse, switch between, rename, search, or manage chat sessions. The session abstraction is invisible. Fork creates orphan sessions with no navigation.
- **Impact:** Users lose all conversation context when they close a terminal tab. No way to reference past interactions. Fork button appears functional but produces no visible result.
- **Suggested fix:** Build a session sidebar or drawer with list, search, rename, and delete. Wire the Fork button to navigate to the forked session tab.
- **Files:** `src/lib/stores/agent-session.ts`, `src/lib/components/chat/ActionableMessage.svelte:53-56`, `src/lib/components/chat/ChatView.svelte:73-80`

#### ISSUE-008: Chat diffs and code blocks have no apply/reject actions
- **Severity:** P1
- **Component:** Chat / DiffBlock / CodeBlock
- **Found by:** Vibecoder, Nielsen
- **WCAG criterion:** N/A
- **Description:** `DiffBlock.svelte` in chat is purely display — no accept/reject buttons (unlike `DiffView.svelte` in the editor which has them). `CodeBlock.svelte` has only a copy button — no "Apply to file" or "Insert into editor" action. Users must manually copy code from chat and paste it into files.
- **Impact:** The highest cognitive gap in the app. Users see diffs and code in chat but cannot act on them, requiring mental mapping between chat content and file actions.
- **Suggested fix:** Add "Apply" and "Reject" buttons to `DiffBlock.svelte` when rendered inside `ChatMessages`. Add "Copy" and "Insert into [filename]" button to `CodeBlock.svelte`.
- **Files:** `src/lib/components/chat/DiffBlock.svelte`, `src/lib/components/chat/CodeBlock.svelte`, `src/lib/components/DiffView.svelte:83-93`

#### ISSUE-009: Google Gemini API key exposed in URL query parameter
- **Severity:** P1
- **Component:** LLM API / Network
- **Found by:** Security, CTO
- **WCAG criterion:** N/A
- **Description:** The `call_google` function constructs the API URL with the key in the query string: `?key={api_key}`. The key appears in server logs, proxy/CDN logs, crash reports, and Referer headers.
- **Impact:** API key leakage through server logs, network intermediaries, and error messages.
- **Suggested fix:** Switch to header-based authentication using `x-goog-api-key` header and remove the `key=` query parameter.
- **Files:** `src-tauri/src/commands/llm.rs:127-129`

#### ISSUE-010: `write_config` enables privilege escalation via PTY command allowlist
- **Severity:** P1
- **Component:** Config / PTY / Security
- **Found by:** Security
- **WCAG criterion:** N/A
- **Description:** `write_config` accepts arbitrary string content and writes to the Reasonance config file. No TOML validation, no content validation. The config is later parsed by `is_allowed_command()` to determine PTY spawn permissions. An attacker can add arbitrary binaries (e.g., `rm`) to the allowlist by writing a crafted config, then execute arbitrary commands via `spawn_process`.
- **Impact:** Privilege escalation from config write to arbitrary command execution.
- **Suggested fix:** Validate TOML content parses correctly before writing. Validate that `command` values match known LLM binary patterns. Separate the command allowlist from user-editable config.
- **Files:** `src-tauri/src/commands/config.rs:13-19`, `src-tauri/src/commands/pty.rs:9-51`

#### ISSUE-011: No tests wired into CI pipeline
- **Severity:** P1
- **Component:** CI/CD
- **Found by:** CTO
- **WCAG criterion:** N/A
- **Description:** The release workflow builds but never runs tests. 284 Rust test functions across 42 modules and 46 frontend test files exist but are not gated in `release.yml`. No `cargo clippy` or linting step either.
- **Impact:** Regressions ship to production uncaught. Tests exist but provide no value if never run.
- **Suggested fix:** Add `cargo test`, `npm test`, and `cargo clippy -- -D warnings` steps before the build step in `.github/workflows/release.yml`.
- **Files:** `.github/workflows/release.yml`

#### ISSUE-012: `agentEvents` Map grows unboundedly — memory leak
- **Severity:** P1
- **Component:** Frontend Stores
- **Found by:** CTO, Performance, Stress
- **WCAG criterion:** N/A
- **Description:** `agentEvents` (`agent-events.ts:6`) is a `Map<string, AgentEvent[]>` that grows without limit. Events are never evicted for active sessions. `clearSessionEvents` exists but is never called automatically. Additionally, `new Map(map)` is created on every event during streaming, causing O(n) GC pressure.
- **Impact:** Memory grows linearly with session duration. Long sessions (thousands of events) will consume significant memory and degrade performance. GC pressure causes frame drops during streaming.
- **Suggested fix:** Add a max event count (e.g., 5000) with oldest-event pruning. Use mutable Map with Svelte 5 `$state` and direct `.set()` mutations instead of copy-on-write.
- **Files:** `src/lib/stores/agent-events.ts:6,60-65`, `src/lib/stores/agent-session.ts:71-83`

#### ISSUE-013: FileTree keyboard inaccessible — cannot Tab into tree
- **Severity:** P1
- **Component:** FileTree
- **Found by:** UX/UI, WCAG, Visual Testing
- **WCAG criterion:** 2.1.1 Keyboard, 2.4.3 Focus Order
- **Description:** All tree items have `tabindex="-1"` (`FileTree.svelte:153`). There is no `tabindex="0"` entry point on the first visible item, so keyboard users cannot Tab into the file tree at all. Additionally, Enter/Space to open files is missing — only `onclick` handler exists.
- **Impact:** Keyboard-only users and screen reader users cannot access the file explorer, a core feature of the IDE.
- **Suggested fix:** Set `tabindex="0"` on the first visible tree item (roving tabindex pattern). Add Enter/Space key handlers to open files and toggle directories.
- **Files:** `src/lib/components/FileTree.svelte:76-123,151,153`

#### ISSUE-014: Accent color `--accent` (#1d4ed8) fails WCAG AA contrast
- **Severity:** P1
- **Component:** Global theme / Multiple components
- **Found by:** UX/UI, WCAG, Nielsen, Visual Testing
- **WCAG criterion:** 1.4.3 Contrast (Minimum)
- **Description:** `--accent` (`#1d4ed8`) on dark background `#121212` yields approximately 3.2:1 contrast ratio (needs 4.5:1 for normal text, 3:1 for large text). Used as text color in WelcomeScreen primary button, ShortcutsDialog group labels (10px font), ResponsePanel/MarkdownPreview links. StatusBar white `#fff` on `--accent` background yields 3.8:1, also failing AA. `.file-encoding` at `opacity: 0.5` creates critically low contrast.
- **Impact:** Low-vision users cannot read accent-colored text. Affects multiple core components.
- **Suggested fix:** Change `--accent` to a lighter blue (e.g., `#60a5fa` or `#93c5fd`) that achieves 4.5:1+ on dark backgrounds. Audit all uses of `--accent` as foreground text color. Remove opacity reductions on informational text.
- **Files:** `src/app.css` (CSS custom property definition), `src/lib/components/WelcomeScreen.svelte`, `src/lib/components/ShortcutsDialog.svelte`, `src/lib/components/StatusBar.svelte`, `src/lib/components/ResponsePanel.svelte`, `src/lib/components/MarkdownPreview.svelte`

#### ISSUE-015: Stderr silenced for CLI processes — errors permanently lost
- **Severity:** P1
- **Component:** Transport Layer
- **Found by:** CTO, Nielsen
- **WCAG criterion:** N/A
- **Description:** `cmd.stderr(Stdio::null())` (`transport/mod.rs:99`) means all CLI error output is permanently discarded. If a provider CLI prints errors to stderr (most do), the user gets no feedback — the session just hangs or terminates silently.
- **Impact:** Users cannot diagnose LLM CLI failures. Sessions appear to hang with no error information.
- **Suggested fix:** Capture stderr output and surface it as warning events in the UI, or at minimum log it for debugging.
- **Files:** `src-tauri/src/transport/mod.rs:99`

#### ISSUE-016: `start_watching` accepts arbitrary paths without validation
- **Severity:** P1
- **Component:** File System / IPC Commands
- **Found by:** Security
- **WCAG criterion:** N/A
- **Description:** The `start_watching` command passes the user-supplied path directly to the filesystem watcher without project-root validation. This could be used to monitor sensitive directories for changes.
- **Impact:** Information disclosure via filesystem event monitoring outside project scope.
- **Suggested fix:** Validate the path is within the project root before starting the watcher.
- **Files:** `src-tauri/src/commands/fs.rs:228-234`

#### ISSUE-017: MenuItem submenus are keyboard-inaccessible
- **Severity:** P1
- **Component:** MenuItem / MenuBar
- **Found by:** UX/UI, WCAG, Visual Testing
- **WCAG criterion:** 2.1.1 Keyboard, 1.4.13 Content on Hover or Focus
- **Description:** Submenus open only on `onmouseenter` (mouse hover), not on keyboard ArrowRight. No Left/Right arrow navigation between top-level menu items. `role="menubar"` is placed on individual menu items instead of the parent bar container — incorrect ARIA structure.
- **Impact:** Keyboard-only users cannot access any submenu functionality. Incorrect ARIA confuses screen readers.
- **Suggested fix:** Add ArrowRight to open submenus, ArrowLeft to close. Add Left/Right navigation between top-level menu items. Move `role="menubar"` to the parent `div.menu-bar` container.
- **Files:** `src/lib/components/MenuItem.svelte:44,63-67`, `src/lib/components/MenuBar.svelte`

---

### P2 — Major

#### ISSUE-018: 67-70 i18n keys untranslated per locale (7 of 8 non-English locales)
- **Severity:** P2
- **Component:** i18n / Localization
- **Found by:** i18n
- **WCAG criterion:** N/A
- **Description:** Analytics dashboard keys (18), analytics bar keys (7), analytics insights keys (7), analytics budget keys (6), analytics a11y keys (4), and settings provider keys (31) are entirely in English across Arabic, German, Spanish, French, Hindi, Portuguese, and Chinese. Only Italian is fully translated.
- **Impact:** Non-English users see a mix of translated and English text in analytics and provider settings areas. ~30% of keys are untranslated.
- **Suggested fix:** Translate the 74 key blocks for the 7 remaining locales (~518 translations total).
- **Files:** `src/lib/i18n/locales/ar.json`, `de.json`, `es.json`, `fr.json`, `hi.json`, `pt.json`, `zh.json`

#### ISSUE-019: 68 hardcoded English strings in component templates
- **Severity:** P2
- **Component:** i18n / Multiple Components
- **Found by:** i18n
- **WCAG criterion:** N/A
- **Description:** 68 hardcoded English strings in `title=`, `aria-label=`, and `placeholder=` attributes across 25+ components. Examples: "File explorer", "Git commands", "Toggle theme", "Send a message...", "Close search", "Analytics Dashboard", etc.
- **Impact:** Non-English users see untranslated labels, tooltips, and ARIA labels throughout the interface. Screen reader users in non-English locales get English announcements.
- **Suggested fix:** Move all 68 strings to i18n keys and use `$tr()` in templates. See the i18n report for the complete list with file/line references.
- **Files:** Multiple — see `docs/audit/i18n-report.md` for full listing of all 68 instances across 25+ component files under `src/lib/components/`

#### ISSUE-020: RTL (Arabic) completely broken — 80+ physical CSS directional properties
- **Severity:** P2
- **Component:** CSS / Layout / All Components
- **Found by:** i18n, Nielsen, WCAG
- **WCAG criterion:** 1.3.2 Meaningful Sequence
- **Description:** While `dir="rtl"` is correctly set on `<html>` for Arabic locale, no component CSS uses logical properties. 80+ instances of `margin-left`, `padding-left`, `text-align: left`, `border-left`, `left:`, `right:` across 20+ components. No `[dir="rtl"]` overrides exist anywhere.
- **Impact:** Arabic users see a completely broken layout — panels, menus, text alignment, borders, and indentation are all mirrored incorrectly.
- **Suggested fix:** Convert all physical CSS properties to logical equivalents: `margin-left` to `margin-inline-start`, `padding-left` to `padding-inline-start`, `text-align: left` to `text-align: start`, `border-left` to `border-inline-start`, `left:` to `inset-inline-start`, etc. Add `[dir="rtl"]` overrides for absolute positioning in dropdown menus.
- **Files:** `src/lib/components/App.svelte`, `FileTree.svelte`, `Toolbar.svelte`, `MenuItem.svelte`, `TerminalManager.svelte`, `AnalyticsDashboard.svelte`, `Settings.svelte`, `ResponsePanel.svelte`, `Toast.svelte`, and 12+ more components

#### ISSUE-021: DOMPurify used with default config — allows form/input/SVG in LLM responses
- **Severity:** P2
- **Component:** Markdown Rendering / Security
- **Found by:** Security, Adversarial
- **WCAG criterion:** N/A
- **Description:** All three markdown rendering components (`MarkdownPreview.svelte:23`, `ResponsePanel.svelte:34`, `chat/TextBlock.svelte:10`) use `DOMPurify.sanitize()` with default settings. Defaults allow `<form>`, `<input>`, `<details>`, `<summary>`, and `<svg>` elements which could enable UI redressing or phishing via crafted LLM responses.
- **Impact:** Potential UI redressing or phishing attacks through malicious LLM responses rendered as HTML.
- **Suggested fix:** Use a hardened DOMPurify config with explicit `ALLOWED_TAGS` whitelist (p, br, strong, em, a, code, pre, ul, ol, li, h1-h6, blockquote, table elements, hr, img, span, del) and `ALLOW_DATA_ATTR: false`.
- **Files:** `src/lib/components/MarkdownPreview.svelte:23`, `src/lib/components/ResponsePanel.svelte:34`, `src/lib/components/chat/TextBlock.svelte:10`

#### ISSUE-022: No chat message virtualization — DOM grows unbounded
- **Severity:** P2
- **Component:** ChatMessages
- **Found by:** Stress, Performance, Adversarial
- **WCAG criterion:** N/A
- **Description:** `ChatMessages.svelte` renders ALL events with `{#each}` (line 79-104). No virtualization, no windowing, no pagination. Each message creates multiple DOM nodes (role label, content blocks). At 10,000 messages, DOM will contain 50,000+ nodes.
- **Impact:** Progressive performance degradation. Long sessions become unusably slow as DOM grows.
- **Suggested fix:** Add virtualization (e.g., svelte-virtual-list) or at minimum pagination — show last 200 messages with a "Load more" button.
- **Files:** `src/lib/components/chat/ChatMessages.svelte:79-104`

#### ISSUE-023: Editor defaults to read-only with no visible indicator or toggle
- **Severity:** P2
- **Component:** Editor
- **Found by:** Vibecoder, Nielsen
- **WCAG criterion:** N/A
- **Description:** `Editor.svelte:52` has `readOnly = true` as default. If the parent does not pass `readOnly={false}`, the editor is silently read-only. There is no visual indicator (lock icon, banner, etc.) and no toggle for the user.
- **Impact:** Users attempt to type and nothing happens, with no explanation of why.
- **Suggested fix:** Show a lock icon or banner when the editor is in read-only mode. Add an "Edit" button to unlock. Ensure the parent always passes the correct readOnly state.
- **Files:** `src/lib/components/Editor.svelte:52`

#### ISSUE-024: SessionHistoryRecorder does synchronous file I/O inside event bus lock
- **Severity:** P2
- **Component:** Event Bus / Transport
- **Found by:** Performance, CTO
- **WCAG criterion:** N/A
- **Description:** `SessionHistoryRecorder::on_event()` performs JSONL file I/O while the event bus Mutex is held (`event_bus.rs:165`). Under high event throughput, this serializes all event processing behind disk I/O latency. All subscribers (including FrontendEmitter) are blocked.
- **Impact:** UI freezes during disk contention. Visible lag during high-frequency streaming.
- **Suggested fix:** Buffer events in memory and flush to disk asynchronously via a channel to a dedicated writer task, rather than doing synchronous I/O while holding the lock.
- **Files:** `src-tauri/src/transport/event_bus.rs:48-59,165`

#### ISSUE-025: Toast notifications are keyboard-inaccessible
- **Severity:** P2
- **Component:** Toast
- **Found by:** UX/UI, WCAG, Visual Testing
- **WCAG criterion:** 2.1.1 Keyboard
- **Description:** Toasts auto-dismiss with no mechanism for keyboard users to access them before they disappear. No `tabindex` on toasts. Action buttons inside toasts are unreachable via keyboard. Container has `pointer-events: none`.
- **Impact:** Keyboard-only users miss all toast notifications and cannot interact with toast action buttons.
- **Suggested fix:** Add `tabindex="0"` to toasts. Pause auto-dismiss timer on focus. Ensure Tab can reach action buttons within toasts.
- **Files:** `src/lib/components/Toast.svelte:29-50,80,93`

#### ISSUE-026: CSP allows `unsafe-inline` for styles
- **Severity:** P2
- **Component:** Security / CSP
- **Found by:** Security
- **WCAG criterion:** N/A
- **Description:** The CSP includes `style-src 'self' 'unsafe-inline'`. While less dangerous than script injection, CSS injection can enable CSS-based data exfiltration attacks via attribute selectors loading external resources.
- **Impact:** Low-medium. CSS injection could be used for limited data exfiltration if combined with another vulnerability.
- **Suggested fix:** Use CSP nonces or hashes instead of `unsafe-inline` for styles, if Svelte's runtime styling permits.
- **Files:** `src-tauri/tauri.conf.json:25`

#### ISSUE-027: No font fallback for CJK and Devanagari scripts
- **Severity:** P2
- **Component:** Typography / i18n
- **Found by:** i18n
- **WCAG criterion:** N/A
- **Description:** Font stack `--font-ui: 'Atkinson Hyperlegible Next', system-ui, -apple-system, sans-serif` has no CJK or Devanagari fonts. Atkinson Hyperlegible does not include these glyphs. Fallback to `system-ui` is not guaranteed on all platforms (especially Linux).
- **Impact:** Chinese and Hindi users may see incorrect or missing glyphs, broken rendering, or inconsistent typography.
- **Suggested fix:** Add explicit CJK and Devanagari fonts: `'Noto Sans SC', 'Noto Sans Devanagari', 'Microsoft YaHei', 'PingFang SC'` to the fallback chain.
- **Files:** `src/app.css:89-90`

#### ISSUE-028: EditorTabs keyboard navigation incorrect — no arrow keys, wrong tabindex
- **Severity:** P2
- **Component:** EditorTabs
- **Found by:** UX/UI, WCAG
- **WCAG criterion:** 2.1.1 Keyboard, 4.1.2 Name Role Value
- **Description:** No ArrowLeft/ArrowRight navigation between tabs (standard tablist pattern per WAI-ARIA Authoring Practices). All tabs have `tabindex="0"` — per ARIA Practices, only the active tab should have `tabindex="0"`, others should have `tabindex="-1"` (roving tabindex).
- **Impact:** Keyboard users must Tab through every tab instead of using arrow keys. Non-standard interaction pattern.
- **Suggested fix:** Implement roving tabindex: active tab gets `tabindex="0"`, others get `tabindex="-1"`. Add ArrowLeft/ArrowRight handlers to move between tabs.
- **Files:** `src/lib/components/EditorTabs.svelte:24-28,31,39-41`

#### ISSUE-029: Terminal container lacks ARIA role and label
- **Severity:** P2
- **Component:** Terminal
- **Found by:** UX/UI, WCAG
- **WCAG criterion:** 1.1.1 Non-text Content, 1.3.1 Info and Relationships, 4.1.2 Name Role Value
- **Description:** Terminal wrapper div has `svelte-ignore a11y_click_events_have_key_events` and `a11y_no_static_element_interactions` suppressed. No `role` attribute on the terminal container. Search input lacks `aria-label` (only has `placeholder`).
- **Impact:** Screen reader users have no context about the terminal's purpose. Suppressed a11y warnings indicate unresolved accessibility debt.
- **Suggested fix:** Add `role="application"` or `role="log"` to the terminal container. Add `aria-label` to the terminal wrapper and search input.
- **Files:** `src/lib/components/Terminal.svelte:265,269,286`

#### ISSUE-030: Symlink loop causes infinite recursion in SearchPalette
- **Severity:** P2
- **Component:** SearchPalette / File System
- **Found by:** Stress, Adversarial
- **WCAG criterion:** N/A
- **Description:** `SearchPalette.buildFileList` (line 35-51) recursively lists all directories with no cycle detection. A symlink loop would cause infinite recursion and stack overflow, crashing the file indexing.
- **Impact:** Application crash when opening a project that contains symlink loops.
- **Suggested fix:** Track visited paths (by inode or canonical path) and skip already-visited directories.
- **Files:** `src/lib/components/SearchPalette.svelte:35-51`

#### ISSUE-031: Rapid file switching causes full editor teardown without debounce
- **Severity:** P2
- **Component:** Editor
- **Found by:** Stress, Adversarial
- **WCAG criterion:** N/A
- **Description:** `Editor.svelte` `$effect` on `$activeFilePath` (line 160-171) calls `initEditor` which destroys and recreates the entire CodeMirror instance. No debounce. Each switch triggers full teardown + async language load. Rapid tab switching can produce flashing and race conditions with language loading.
- **Impact:** Visual flashing, potential stale language applied to wrong file, performance degradation.
- **Suggested fix:** Add 50-100ms debounce on file switch `$effect`. Consider reusing the CodeMirror instance and swapping document state instead of full teardown.
- **Files:** `src/lib/components/Editor.svelte:144-171`

#### ISSUE-032: ResponsePanel has no focus trap and no Escape close
- **Severity:** P2
- **Component:** ResponsePanel
- **Found by:** UX/UI, WCAG
- **WCAG criterion:** 2.1.2 No Keyboard Trap, 4.1.2 Name Role Value
- **Description:** ResponsePanel opens as an overlay but has no focus trap and no Escape key handler. Focus can move behind the panel. Panel also lacks `role` and `aria-label` attributes.
- **Impact:** Keyboard users can get focus trapped behind the panel or have to Tab through all elements to exit.
- **Suggested fix:** Add focus trap with Escape to close. Add `role="complementary"` or `role="dialog"` with `aria-label`.
- **Files:** `src/lib/components/ResponsePanel.svelte:59,63,168,179,194`

#### ISSUE-033: Analytics store clones entire metrics vector on every query
- **Severity:** P2
- **Component:** Analytics
- **Found by:** CTO, Performance
- **WCAG criterion:** N/A
- **Description:** `all_completed()` (`store.rs:43-45`) clones the entire `Vec<SessionMetrics>` on every call. All queries (provider analytics, model breakdown, daily stats) iterate the full cloned vector. Startup loads entire JSONL file into memory.
- **Impact:** Memory waste and latency scaling linearly with historical session count. Power users with 1000+ sessions will experience multi-second freezes or OOM.
- **Suggested fix:** Migrate from JSONL+Vec to SQLite for query performance at scale. At minimum, add indexed queries by provider/date.
- **Files:** `src-tauri/src/analytics/store.rs:43-45,47-70`

#### ISSUE-034: Multiple simultaneous agent sends possible — no guard
- **Severity:** P2
- **Component:** ChatView / Agent
- **Found by:** Adversarial
- **WCAG criterion:** N/A
- **Description:** No guard prevents multiple concurrent sends. If a user double-clicks send or sends before a response completes, multiple concurrent agent sends can produce interleaved events that corrupt chat state.
- **Impact:** Chat message ordering becomes incoherent. Potential for duplicated or lost messages.
- **Suggested fix:** Add a debounce on the send button and a guard that prevents new sends while a response is in progress.
- **Files:** `src/lib/components/chat/ChatInput.svelte`, `src/lib/components/chat/ChatView.svelte:37-71`

#### ISSUE-035: DiffBlock relies on color alone for add/remove distinction
- **Severity:** P2
- **Component:** DiffBlock
- **Found by:** WCAG
- **WCAG criterion:** 1.4.1 Use of Color
- **Description:** Added lines (green) and removed lines (red) are distinguished only by color and small `+/-` prefix characters. No pattern, icon, or other non-color differentiation.
- **Impact:** Color-blind users cannot reliably distinguish additions from removals in diffs.
- **Suggested fix:** Add distinct background patterns, left-border markers, or more prominent prefix indicators beyond just color and small `+/-` characters.
- **Files:** `src/lib/components/chat/DiffBlock.svelte`

#### ISSUE-036: Concurrent file editing — no conflict detection (last-write-wins)
- **Severity:** P2
- **Component:** Editor / File System
- **Found by:** Adversarial
- **WCAG criterion:** N/A
- **Description:** When both the user and an LLM agent edit the same file, there is no conflict detection. Backend uses `tokio::fs::write` which is not atomic (no temp+rename). Last write silently overwrites.
- **Impact:** Silent data loss when user and agent write to the same file simultaneously.
- **Suggested fix:** Implement optimistic locking (compare file hash before write) or use atomic writes (write to temp then rename). Show a conflict resolution UI when a collision is detected.
- **Files:** `src-tauri/src/commands/fs.rs:125-132`

#### ISSUE-037: TerminalManager instance close uses `<span role="button">` instead of `<button>`
- **Severity:** P2
- **Component:** TerminalManager
- **Found by:** UX/UI, WCAG, Nielsen
- **WCAG criterion:** 4.1.2 Name Role Value
- **Description:** Instance close button is a `<span>` with `role="button"` (line 351-358) instead of a native `<button>` element. Handles Enter key but missing Space key handler. Add instance button (line 373-375) lacks `aria-label` — just shows "+".
- **Impact:** Non-standard element does not get built-in keyboard handling. Space key does not activate. Missing label makes "+" button unclear for screen readers.
- **Suggested fix:** Replace `<span role="button">` with `<button>` element. Add `aria-label` to the "+" add instance button.
- **Files:** `src/lib/components/TerminalManager.svelte:351-358,373-375`

#### ISSUE-038: Small touch targets across multiple components
- **Severity:** P2
- **Component:** Multiple (SearchPalette, ShortcutsDialog, Settings, Terminal, Toast, Toolbar, MenuItem)
- **Found by:** UX/UI, WCAG
- **WCAG criterion:** 2.5.5/2.5.8 Target Size
- **Description:** Multiple close buttons and interactive elements fail WCAG 2.5.8 AA 24px minimum target size: SearchPalette close (~20x20px), ShortcutsDialog X (~18x18px), Settings close (~18x18px), Terminal search buttons (~16x16px), Toast dismiss (~16x16px), Toolbar buttons (min-height 26px but some below 24px width), Menu triggers (~20x20px).
- **Impact:** Users with motor impairments have difficulty hitting small targets. 11 components fail target size criteria.
- **Suggested fix:** Increase padding on all close/dismiss buttons to achieve minimum 24x24px clickable area (44x44px for AAA). Use `min-width` and `min-height` constraints.
- **Files:** `src/lib/components/SearchPalette.svelte`, `ShortcutsDialog.svelte`, `Settings.svelte`, `Terminal.svelte`, `Toast.svelte`, `Toolbar.svelte`, `MenuItem.svelte`

---

### P3 — Minor

#### ISSUE-039: No visible save button — only Ctrl+S or menu
- **Severity:** P3
- **Component:** Editor / EditorTabs
- **Found by:** Vibecoder
- **WCAG criterion:** N/A
- **Description:** Saving is only available via Ctrl+S or File > Save menu. No visible save button in the editor tabs or toolbar. The dirty indicator shows a dot but no save affordance.
- **Impact:** Users unfamiliar with keyboard shortcuts may not discover how to save.
- **Suggested fix:** Add a save icon/button in the editor tabs actions area, visible when a file has unsaved changes.
- **Files:** `src/lib/components/EditorTabs.svelte:49`, `src/lib/components/MenuBar.svelte:62`

#### ISSUE-040: Mode switching dropdown renders but does nothing (TODO in code)
- **Severity:** P3
- **Component:** TerminalToolbar
- **Found by:** Vibecoder, Nielsen
- **WCAG criterion:** N/A
- **Description:** `TerminalToolbar.svelte:52-54` has a TODO comment: "Wire mode switching via adapter." The mode dropdown renders but does nothing when clicked. Creates false affordance.
- **Impact:** Users click the dropdown expecting functionality, nothing happens.
- **Suggested fix:** Either implement mode switching or hide the dropdown until implemented.
- **Files:** `src/lib/components/TerminalToolbar.svelte:52-54`

#### ISSUE-041: "SWARM" tab shows placeholder — confusing for users
- **Severity:** P3
- **Component:** TerminalManager
- **Found by:** Vibecoder, Nielsen
- **WCAG criterion:** N/A
- **Description:** `TerminalManager.svelte:301-307` shows "SWARM" tab with "Coming soon" text. No explanation of what Swarm is.
- **Impact:** Users click expecting functionality and get a dead end with no context.
- **Suggested fix:** Hide the tab until the feature is ready, or clearly mark it as "Coming Soon" with disabled styling and a brief explanation.
- **Files:** `src/lib/components/TerminalManager.svelte:301-307`

#### ISSUE-042: Font family hardcoded on save (always Atkinson Hyperlegible Mono)
- **Severity:** P3
- **Component:** Settings
- **Found by:** Vibecoder
- **WCAG criterion:** N/A
- **Description:** `Settings.svelte:277` always saves `'Atkinson Hyperlegible Mono', monospace` regardless of user preference. The font display in settings is a static text span, not a selector.
- **Impact:** Font appears configurable but any change is ignored on save.
- **Suggested fix:** Either make the font truly configurable with a selector, or remove the non-functional font display from settings.
- **Files:** `src/lib/components/Settings.svelte:277,511`

#### ISSUE-043: Find in Files does not jump to matching line
- **Severity:** P3
- **Component:** FindInFiles / Editor
- **Found by:** Vibecoder
- **WCAG criterion:** N/A
- **Description:** `FindInFiles.svelte:66-76` opens the file in the editor when a search result is clicked, but does not jump to the matching line. Comment at line 71 notes: "jumping to line would require editor line API."
- **Impact:** Users must manually scroll to find the match after opening the file.
- **Suggested fix:** Wire up editor line scrolling API. Pass the line number from the search result to the editor's `scrollToLine` function.
- **Files:** `src/lib/components/FindInFiles.svelte:66-76`

#### ISSUE-044: No input validation in Settings for command paths, API endpoints, env vars
- **Severity:** P3
- **Component:** Settings
- **Found by:** Vibecoder, Nielsen
- **WCAG criterion:** N/A
- **Description:** `Settings.svelte:179` only checks `name.trim()`. No validation for binary existence, URL format, env var existence, or endpoint accessibility.
- **Impact:** Users can save invalid configurations with no feedback until they try to use them and encounter cryptic errors.
- **Suggested fix:** Validate that the binary exists on save, check URL format for endpoints, verify env var is set.
- **Files:** `src/lib/components/Settings.svelte:179`

#### ISSUE-045: RetryPolicy defined but never invoked — dead code
- **Severity:** P3
- **Component:** Transport
- **Found by:** CTO
- **WCAG criterion:** N/A
- **Description:** `RetryPolicy` is constructed and stored (`transport/mod.rs:37-41`) but `StructuredAgentTransport::send()` does not implement retry loops. The retry module is built but never called.
- **Impact:** No automatic retry on transient failures. Dead code increases maintenance burden.
- **Suggested fix:** Implement the retry loop in `send()` or remove the dead code.
- **Files:** `src-tauri/src/transport/mod.rs:37-41`, `src-tauri/src/retry.rs`

#### ISSUE-046: YOLO mode toggle race condition — concurrent restart loops possible
- **Severity:** P3
- **Component:** TerminalManager
- **Found by:** Stress
- **WCAG criterion:** N/A
- **Description:** `TerminalManager.svelte:176-232` iterates all instances, kills and respawns sequentially in async IIFE. If user toggles YOLO again mid-restart, both restart loops run concurrently, potentially duplicating or orphaning processes. No lock/guard.
- **Impact:** Orphaned LLM processes or duplicated terminal instances.
- **Suggested fix:** Add a boolean guard or debounce that prevents re-entry while a restart cycle is in progress.
- **Files:** `src/lib/components/TerminalManager.svelte:176-232`

#### ISSUE-047: No locale-aware number and date formatting
- **Severity:** P3
- **Component:** i18n / Analytics
- **Found by:** i18n
- **WCAG criterion:** N/A
- **Description:** No `Intl.NumberFormat` or `Intl.DateTimeFormat` usage. Numbers like "1,234" vs "1.234" display in English format regardless of locale. Cost values always show dollar sign and English decimal separator.
- **Impact:** Non-English users see numbers and dates in unfamiliar format.
- **Suggested fix:** Use `Intl.NumberFormat` and `Intl.DateTimeFormat` for locale-aware rendering.
- **Files:** `src/lib/i18n/index.ts`, `src/lib/components/AnalyticsDashboard.svelte`, `src/lib/components/AnalyticsBar.svelte`

#### ISSUE-048: File read errors silently swallowed — console.error only
- **Severity:** P3
- **Component:** FileTree / Editor
- **Found by:** Stress, Nielsen
- **WCAG criterion:** N/A
- **Description:** FileTree.svelte:66-68 catches file read errors but only logs to console. Binary file opens produce raw error strings with no friendly message. User sees nothing — file simply does not open.
- **Impact:** Users click files that fail to open with no explanation.
- **Suggested fix:** Show user-visible error in the editor area. Detect binary files (check for null bytes or non-UTF-8 error) and show "Cannot open binary file" message.
- **Files:** `src/lib/components/FileTree.svelte:66-68`, `src-tauri/src/commands/fs.rs:121`

#### ISSUE-049: `{@html}` in ResourceNode.svelte with dictionary lookup on user-controlled key
- **Severity:** P3
- **Component:** SwarmCanvas / ResourceNode
- **Found by:** Security
- **WCAG criterion:** N/A
- **Description:** `{@html kindIcons[kind] || '&#128196;'}` uses `{@html}` with dictionary lookup where `kind` comes from workflow definitions (potentially attacker-controlled). Currently safe because all dictionary values are hardcoded HTML entities and fallback is safe. Fragile pattern.
- **Impact:** No current exploit, but future expansion of the dictionary with dynamic values could introduce XSS.
- **Suggested fix:** Use Unicode characters directly instead of `{@html}` with HTML entities. Validate `kind` against the known set.
- **Files:** `src/lib/components/swarm/ResourceNode.svelte:27`

#### ISSUE-050: Budget cost uses crude estimate instead of actual backend data
- **Severity:** P3
- **Component:** Analytics
- **Found by:** CTO
- **WCAG criterion:** N/A
- **Description:** Frontend budget system (`analytics.ts:211`) uses `tokens * 0.00001` cost estimate instead of the actual `total_cost_usd` from backend metrics.
- **Impact:** Inaccurate cost tracking and budget alerts. Discrepancy between displayed cost and actual cost.
- **Suggested fix:** Use `total_cost_usd` from backend metrics for budget calculations.
- **Files:** `src/lib/stores/analytics.ts:211`

#### ISSUE-051: Error display in app.html exposes stack traces in production
- **Severity:** P3
- **Component:** Error Handling / Security
- **Found by:** Security
- **WCAG criterion:** N/A
- **Description:** Global error handler creates a `<pre>` element with full stack traces including file paths and line numbers. While safe against XSS (uses `textContent`), it leaks internal file structure.
- **Impact:** Information disclosure of internal paths in error conditions.
- **Suggested fix:** Conditionally disable detailed error display in production builds. Show a user-friendly error message instead.
- **Files:** `src/app.html:12-24`

#### ISSUE-052: `get_env_var` allowlist includes PATH and HOME
- **Severity:** P3
- **Component:** Security / IPC
- **Found by:** Security
- **WCAG criterion:** N/A
- **Description:** The env var allowlist (SEC-04) includes `PATH`, `HOME`, `USER`, `SHELL`, `TERM`, and `XDG_CONFIG_HOME`. Exposing `PATH` reveals installed software locations.
- **Impact:** Minor information disclosure of system paths and username.
- **Suggested fix:** Remove `PATH` from the allowlist if it is only needed for backend discovery operations.
- **Files:** `src-tauri/src/commands/system.rs:52-70`

#### ISSUE-053: `session_rename` accepts unbounded title length
- **Severity:** P3
- **Component:** Session Management
- **Found by:** CTO
- **WCAG criterion:** N/A
- **Description:** `session_rename` accepts arbitrary strings for `title` with no length limit. Extremely long titles could cause layout issues or storage bloat.
- **Impact:** Potential UI overflow and storage waste.
- **Suggested fix:** Add title length validation (e.g., max 500 characters).
- **Files:** `src-tauri/src/commands/session.rs:49-55`

#### ISSUE-054: No forced-colors or high-contrast media query support
- **Severity:** P3
- **Component:** CSS / Accessibility
- **Found by:** Visual Testing
- **WCAG criterion:** 1.4.11 Non-text Contrast
- **Description:** No `forced-colors` or `prefers-contrast: more` media query detected in the codebase. Icons relying on color alone will disappear. Focus indicators using box-shadow instead of outline become invisible.
- **Impact:** Users who rely on Windows High Contrast mode or forced-colors cannot use the application effectively.
- **Suggested fix:** Add `@media (forced-colors: active)` rules. Ensure focus indicators use `outline` (which is visible in forced-colors) rather than `box-shadow` alone.
- **Files:** `src/app.css`, all component styles

---

### P4 — Enhancement

#### ISSUE-055: No file/folder creation or deletion from FileTree UI
- **Severity:** P4
- **Component:** FileTree
- **Found by:** Vibecoder, Nielsen
- **WCAG criterion:** N/A
- **Description:** Users cannot create new files or folders from the file tree UI. They must use the terminal or ask the LLM.
- **Impact:** Reduced efficiency for basic file management tasks.
- **Suggested fix:** Add right-click context menu with New File, New Folder, Rename, Delete options.
- **Files:** `src/lib/components/FileTree.svelte`

#### ISSUE-056: No cursor line/column display in status bar
- **Severity:** P4
- **Component:** StatusBar / Editor
- **Found by:** Vibecoder, Nielsen
- **WCAG criterion:** N/A
- **Description:** StatusBar shows file name and language but not the current cursor position (line:column).
- **Impact:** Users cannot quickly see their position in the file.
- **Suggested fix:** Add cursor line/column display to the status bar, updating reactively from CodeMirror's cursor position.
- **Files:** `src/lib/components/StatusBar.svelte:70-74`

#### ISSUE-057: Markdown preview toggle not exposed in editor UI
- **Severity:** P4
- **Component:** Editor
- **Found by:** Vibecoder
- **WCAG criterion:** N/A
- **Description:** `Editor.svelte:52` accepts `showMarkdownPreview` prop but there is no button or toggle visible in the editor UI to switch between code and preview mode for markdown files.
- **Impact:** Users opening `.md` files cannot discover preview mode.
- **Suggested fix:** Add a preview toggle button in `EditorTabs` actions when the active file has a `.md` extension.
- **Files:** `src/lib/components/Editor.svelte:52`

#### ISSUE-058: No terminal output export
- **Severity:** P4
- **Component:** Terminal
- **Found by:** Vibecoder
- **WCAG criterion:** N/A
- **Description:** There is no way to save terminal output to a file for later reference.
- **Impact:** Users cannot preserve terminal session content.
- **Suggested fix:** Add a "Save output" button to the terminal toolbar that exports current terminal buffer to a text file.
- **Files:** `src/lib/components/TerminalManager.svelte`, `src/lib/components/Terminal.svelte`

#### ISSUE-059: No version info visible on welcome screen
- **Severity:** P4
- **Component:** WelcomeScreen
- **Found by:** Vibecoder
- **WCAG criterion:** N/A
- **Description:** Version number is only visible in Settings (`Settings.svelte:592`). Not shown on the welcome screen or anywhere easily discoverable.
- **Impact:** Users cannot quickly verify which version they are running.
- **Suggested fix:** Add version number to the welcome screen footer.
- **Files:** `src/lib/components/WelcomeScreen.svelte`

#### ISSUE-060: No drag-and-drop to open folder
- **Severity:** P4
- **Component:** WelcomeScreen / FileTree
- **Found by:** Vibecoder
- **WCAG criterion:** N/A
- **Description:** The welcome screen and file tree do not support dropping a folder to open it as a project.
- **Impact:** Users expecting drag-and-drop (common in other IDEs) cannot use this interaction pattern.
- **Suggested fix:** Add drag-and-drop handlers on the welcome screen and file tree for folder opening.
- **Files:** `src/lib/components/WelcomeScreen.svelte`

#### ISSUE-061: No empty state for ChatMessages
- **Severity:** P4
- **Component:** ChatMessages
- **Found by:** Stress, Nielsen
- **WCAG criterion:** N/A
- **Description:** Empty events array renders a blank `chat-messages` div. No "Start a conversation" prompt or guidance.
- **Impact:** New users see a blank area with no guidance on how to interact.
- **Suggested fix:** Add a placeholder message like "Send a message to start" when the event list is empty.
- **Files:** `src/lib/components/chat/ChatMessages.svelte`

#### ISSUE-062: No CSS containment on major panels
- **Severity:** P4
- **Component:** App Layout
- **Found by:** Performance
- **WCAG criterion:** N/A
- **Description:** No `contain: layout style` applied to major panels (file tree, editor, terminal). Layout recalculations during resize affect the entire DOM tree.
- **Impact:** Suboptimal resize performance, especially during panel dragging.
- **Suggested fix:** Add `contain: layout style` CSS to the file tree, editor, and terminal panel containers.
- **Files:** `src/lib/components/App.svelte`
