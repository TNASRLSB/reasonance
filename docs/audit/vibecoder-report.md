# Vibecoder Audit Report

**Date:** 2026-03-22
**Persona:** Developer relying exclusively on LLM tools (no terminal comfort, no config file editing, everything through the UI)
**Judgment:** Can I accomplish every task without reading docs or source code?

## Executive Summary

Reasonance delivers a competent three-panel IDE layout with strong accessibility foundations (skip links, ARIA roles, focus trapping, keyboard navigation). The first-launch-to-first-prompt path is achievable but requires configuration knowledge the UI does not teach. The biggest systemic issue is that the app is built around CLI-spawned LLM processes, which demands that the user already have `claude`, `codex`, or similar binaries installed and configured -- a prerequisite invisible to the vibecoder persona. Once configured, core flows (file browsing, editing, chat, analytics) work logically, though several intermediate states lack feedback or guidance.

---

## Flow-by-Flow Findings

### 1. First Launch Experience

**Verdict:** :warning: Issues

**What the user sees:** `WelcomeScreen.svelte` shows the REASONANCE logo, subtitle "IDE for Vibecoders", a single "Open Folder" button, and a recent-projects list (empty on first run). There is no onboarding wizard, no "getting started" steps, no explanation of what the app does or how its three-panel layout works.

**Specific findings:**

- **No onboarding flow.** The welcome screen (`WelcomeScreen.svelte:30-54`) jumps straight to "Open Folder" with zero context. A vibecoder who just installed the app has no idea that they need to configure an LLM provider before the terminal panel will do anything useful.
- **No mention of LLM configuration.** The critical first step -- adding an LLM in Settings -- is completely undiscoverable from the welcome screen. The user must know to click the gear icon in the toolbar after opening a project.
- **Empty recent projects handled well.** Line 40-41 shows a clean "No recent projects" message via i18n key `welcome.noRecent`. This is fine.
- **Theme toggle present but unlabeled.** The sun/moon toggle (`WelcomeScreen.svelte:21-23`) uses unicode characters without a text label. The `title` attribute ("Toggle theme") helps on hover but is invisible on touch.
- **No version info on welcome screen.** Version is buried in Settings (`Settings.svelte:592`). A vibecoder has no idea what version they are running.

**After opening a project:** The app transitions to `App.svelte`, which renders a three-panel layout (file tree / editor / terminal). If no LLMs are configured, the terminal panel shows an empty state with "Start LLM" text and a banner pointing to Settings (`TerminalManager.svelte:312-318`). This is good -- but it comes AFTER the user has already opened a project, which means the first 30 seconds of the experience feel broken.

---

### 2. Project Opening

**Verdict:** :white_check_mark: Pass

**How it works:** The "Open Folder" button on the welcome screen triggers `onOpenFolder` (a prop from the parent). Recent projects are clickable buttons in a list (`WelcomeScreen.svelte:44-49`). After opening, `projectRoot` store is set, and `FileTree.svelte` loads entries via `adapter.listDir()`.

**Specific findings:**

- **Open folder is discoverable.** Large, centered, primary-styled button. Hard to miss.
- **Recent projects are clear.** Full paths shown, clickable. No icons but adequate for the persona.
- **Empty directory handled silently.** `FileTree.svelte:17-22` calls `adapter.listDir(root)` and renders whatever comes back. An empty directory simply shows an empty tree with the header label. There is no "This folder is empty" message or suggestion to open a different folder. The tree header shows the folder name (`FileTree.svelte:141`), which at least confirms a folder is open.
- **File tree is functional.** Click to open file (single click opens in editor, double click opens externally -- `FileTree.svelte:44-74`). The single-click delay of 250ms (`FileTree.svelte:55-69`) could feel sluggish.
- **Keyboard navigation is thorough.** Arrow keys, Home, End all work via `handleTreeKeydown` (`FileTree.svelte:76-123`). ARIA `treeitem` roles and `aria-expanded` attributes are correct.
- **Gitignored files are dimmed.** `FileTree.svelte:148` applies `opacity: 0.5` class. Subtle but appropriate.
- **No file creation/deletion from file tree.** A vibecoder cannot create new files or folders from the UI. They must use the terminal or ask the LLM. This is a gap for the persona but may be intentional.
- **No drag-and-drop to open a folder.** The welcome screen and file tree do not support dropping a folder to open it.

---

### 3. Chat Flow (End-to-End)

**Verdict:** :warning: Issues

**Architecture:** Each terminal instance can toggle between terminal and chat view via `ViewModeToggle.svelte`. The chat view (`ChatView.svelte`) composes `ChatHeader`, `ChatMessages`, and `ChatInput`. Events flow through `agent-events.ts` -> `processAgentEvent()` -> store updates -> reactive rendering.

**Specific findings:**

- **Send prompt works.** `ChatInput.svelte` has a textarea and SEND button. Enter submits (Shift+Enter for newline, line 14). The button disables during streaming (`disabled={streaming}`, line 33). Clear and discoverable.
- **User messages appear immediately.** `ChatView.svelte:39-63` creates a synthetic `AgentEvent` with `provider: 'user'` so it renders inline. Good UX.
- **Response rendering is comprehensive.** `ContentRenderer.svelte` handles text, code, diff, file_ref, and JSON content types. `ChatMessages.svelte` groups consecutive events by role and renders appropriate block components (ThinkingBlock, ToolUseBlock, ErrorBlock).
- **Diff display in chat is read-only.** `DiffBlock.svelte` shows diffs with old/new lines color-coded, collapsible per file. However, there are NO accept/reject buttons in the chat diff view. The `DiffView.svelte` component (which has accept/reject) is a separate component used in the editor panel for file-watcher diffs, not in chat. A vibecoder seeing a diff in chat has no way to apply or reject it from the chat UI.
- **No inline code-apply action.** When the agent returns a code block in chat, there is no "Apply to file" or "Copy to editor" button. The user must manually copy code. `CodeBlock.svelte` is purely display.
- **Fork button exists but is incomplete.** `ActionableMessage.svelte:53-56` shows a FORK button on agent messages. The handler (`ChatView.svelte:73-80`) calls `adapter.sessionFork()` but the comment says "Phase 6/7: navigate to forked session tab" -- meaning it forks but does not navigate. The user clicks FORK and nothing visible happens.
- **Copy button works.** `ActionableMessage.svelte:28-35` copies plain text of all events in a message group to clipboard. Feedback changes button text to "COPIED" for 2 seconds. Good.
- **Streaming indicator present.** `StreamingIndicator.svelte` renders at the bottom of the message list during streaming (`ChatMessages.svelte:106-108`).
- **Auto-scroll behavior is correct.** `ChatMessages.svelte:21-30` only auto-scrolls when user is near the bottom (within 100px). This avoids hijacking scroll position when the user is reading earlier messages.
- **ChatHeader shows live metrics.** Provider, model, token count, tok/s, elapsed time, and status badge (`ChatHeader.svelte:32-48`). Useful real-time feedback for the vibecoder.
- **No chat history persistence UI.** There is no visible way to see previous chat sessions, rename them, or search through history. Session state lives in stores but there is no session list/picker component.
- **Error display is clear.** `ErrorBlock.svelte` renders with severity and error code. Fatal errors update session status via `agent-events.ts:82-83`.

---

### 4. Editor Flow

**Verdict:** :warning: Issues

**Specific findings:**

- **CodeMirror 6 editor works.** `Editor.svelte` uses CM6 with async language loading, theme support (dark/light/one-dark), line wrapping, fold gutters. The editor rebuilds state on theme/font changes correctly.
- **Empty state is helpful.** When no file is open, `Editor.svelte:241-244` shows a message (`editor.openFile` i18n key) and a hint about search (`editor.searchHint`). This guides the user toward Ctrl+P.
- **Editor is read-only by default.** `Editor.svelte:52` has `readOnly = true` as default. The parent must pass `readOnly={false}` to enable editing. If the parent forgets this or the wiring is wrong, the vibecoder cannot type. This is a potential silent failure point. The user gets no indication that the editor is in read-only mode.
- **No save button in the UI.** Saving is only available via Ctrl+S (referenced in `MenuBar.svelte:62`). There is no visible save button in the editor tabs or toolbar. The menu bar has File > Save, but a vibecoder might not think to look there. The dirty indicator (`EditorTabs.svelte:49`) shows a dot but no save affordance.
- **Tab close with unsaved changes uses browser confirm dialog.** `EditorTabs.svelte:17-19` uses `confirm()` with a clear message. Adequate but not beautiful.
- **Markdown preview exists but toggle is not obvious.** `Editor.svelte:52` accepts `showMarkdownPreview` prop. The toggle mechanism is not visible in the editor UI itself -- it must be controlled by the parent component. A vibecoder opening a `.md` file has no button to switch to preview mode.
- **DiffView (file-watcher) has accept/reject.** `DiffView.svelte:83-93` provides Accept and Reject buttons. Accept writes shadow, Reject reverts file. Clear color coding (green accept, red reject). However, it is not clear how the user triggers this view -- it appears to be activated by file change detection, not user action.
- **Context menu on selected text.** `Editor.svelte:73-84` shows a context menu on right-click when text is selected. This likely offers LLM actions (explain, refactor, etc.). The `ContextMenu.svelte` component and `ResponsePanel.svelte` handle the response. This is a strong feature for vibecoders.
- **No line number display for cursor position.** The status bar shows file name and language (`StatusBar.svelte:70-74`) but not the current cursor line/column. A minor gap.

---

### 5. Terminal Flow

**Verdict:** :warning: Issues

**Specific findings:**

- **Empty state is well-designed.** When no terminals exist, `TerminalManager.svelte:308-338` shows "TERMINAL" header, "Start LLM" text, and either a config hint pointing to Settings (if no LLMs configured) or a card selector with a start button (if LLMs exist). This is the best empty state in the app.
- **LLM selection is visual and clear.** Cards show name and command (`TerminalManager.svelte:322-334`). The start button is prominent with accent color.
- **Tab system works.** Two levels: LLM tabs (top bar) and instance tabs (sub-bar). Adding instances via "+" button. Close with "x" button that confirms via `confirm()` dialog.
- **Terminal/Chat toggle is present but cryptic.** `ViewModeToggle.svelte:16` shows either "TERM" or "CHAT" with arrow indicators. The labels are abbreviated and might confuse a vibecoder who does not know what "TERM" means. No tooltip explains the difference beyond the `title` attribute.
- **Terminal search via Ctrl+F.** `Terminal.svelte:113-116` toggles a search bar. Previous/Next/Close buttons with keyboard support (Enter, Shift+Enter, Escape). Good.
- **Clipboard paste works.** `Terminal.svelte:100-106` handles Ctrl+V by reading from clipboard adapter. Ctrl+C copies selection. These are essential for the vibecoder who pastes code.
- **PTY output parsing extracts context/token info.** `Terminal.svelte:136-158` regex-parses context%, token count, model name, messages left, and reset timer from terminal output. This data flows to `StatusBar.svelte` and is displayed live. Clever but fragile -- depends on specific CLI output formats.
- **Swarm tab is placeholder.** `TerminalManager.svelte:301-307` shows "SWARM" tab with "Coming soon" text. This could confuse a vibecoder who clicks it expecting functionality.
- **YOLO mode restart is transparent.** Toggling YOLO restarts all instances (`TerminalManager.svelte:176-232`). The toolbar button shows clear state (`Toolbar.svelte:92-98`), and the status bar turns red with a warning banner. Well communicated.
- **TerminalToolbar adds file context.** `TerminalToolbar.svelte:40-45` opens a file dialog and writes `/file <path>` to the PTY. The toolbar also has slash command picker and mode selector. These are powerful features for vibecoders.
- **Mode switching is incomplete.** `TerminalToolbar.svelte:52-54` has a TODO comment: "Wire mode switching via adapter." The mode dropdown renders but does nothing when clicked.
- **No terminal output export.** There is no way to save terminal output to a file.

---

### 6. Settings Flow

**Verdict:** :warning: Issues

**Specific findings:**

- **Settings modal is comprehensive.** Sections for LLM configuration, language, terminal font, accessibility, theme, updates, provider, and budget. Modal traps focus correctly (`Settings.svelte:39-42`).
- **LLM configuration form is functional but requires knowledge.** Adding a CLI LLM requires knowing the binary name (e.g., "claude"), args, and yolo flag (`Settings.svelte:439-458`). Adding an API LLM requires knowing the provider type, API key env var name, and model ID. There are no preset templates or auto-detection hints in the form itself.
- **"Scan CLI" button exists.** `Settings.svelte:601-603` calls `adapter.discoverLlms()`. This is the auto-detection escape hatch, but it is buried in the "Provider" section near the bottom of settings, not in the LLM config section at the top. A vibecoder who cannot find CLIs on their system will not know this exists.
- **Connection test is present.** `Settings.svelte:294-313` calls `adapter.testProviderConnection()` with step-by-step progress display. Good feedback mechanism.
- **Model selector shows pricing.** `Settings.svelte:648-650` displays per-1M-token costs next to model names. Helpful for cost-conscious vibecoders.
- **Budget configuration exists.** Daily/weekly limits in USD with notification threshold percentage. Wired to `analytics.ts` budget alerts.
- **Error handling on save is clear.** `Settings.svelte:286-288` shows a specific error message if save fails. Load errors (`Settings.svelte:131-133`) also display a clear message.
- **Pending delete pattern is good UX.** `Settings.svelte:217-224` marks LLMs for deletion but does not actually remove until Save. Undo is possible. Visual indicator with strikethrough style.
- **Shortcut capture works.** `Settings.svelte:315-333` captures key combos for LLM switching. The UI for this (pressing keys while a capture is active) is functional but the entry point is not visible in the code shown.
- **Font family is hardcoded on save.** `Settings.svelte:277` always saves `'Atkinson Hyperlegible Mono', monospace` regardless of what the user might have wanted. The font display in settings (`Settings.svelte:511`) is a static text span, not a selector.
- **9 language options.** `Settings.svelte:45-55` supports en, it, de, es, fr, pt, zh, hi, ar. Good international coverage.
- **No input validation beyond name required.** `Settings.svelte:179` checks `name.trim()` but does not validate command paths, API key env var existence, or endpoint URLs.

---

### 7. Analytics Flow

**Verdict:** :white_check_mark: Pass

**Specific findings:**

- **AnalyticsBar (always visible) provides live session metrics.** `AnalyticsBar.svelte` shows: context%, cost, tokens, cost velocity, cost projection, model name, cache hit rate, turns, duration, and vs-average ratio. Two-row layout that collapses gracefully at narrow widths (`AnalyticsBar.svelte:472-500`).
- **AnalyticsDashboard is feature-rich.** Period selector (1d/7d/14d/30d/all), KPI cards with sparklines, provider comparison bars, daily trend bars, insights engine, model breakdown drill-down, and export (CSV/JSON). This is a standout feature.
- **Dashboard access is discoverable.** The chart emoji button in the toolbar (`Toolbar.svelte:100-106`), the chart emoji button in the analytics bar (`AnalyticsBar.svelte:153-160`), and the keyboard shortcut Ctrl+Shift+A (`App.svelte:49-52`) all open it.
- **Insights system surfaces actionable warnings.** Error spikes (>5% rate), cost anomalies (>2x average), and cache drops (<30% hit rate) generate dismissible insight cards (`AnalyticsDashboard.svelte:119-172`). Good for the vibecoder who does not monitor costs.
- **Budget alerts change status bar color.** `AnalyticsBar.svelte:103-107` applies `budget-warning` and `budget-danger` classes that change the border color. Visual but could be more prominent.
- **Error flash animation.** `AnalyticsBar.svelte:59-78` briefly pulses the bar red on new errors and blue on recoveries. Respects `prefers-reduced-motion`.
- **Skeleton loading states.** Both the bar (`AnalyticsBar.svelte:209-221`) and dashboard KPI cards (`AnalyticsDashboard.svelte:340-348`) show skeleton placeholders during loading. Good perceived performance.
- **Data caching with 30-second TTL.** `analytics.ts:11` caches historical data for 30 seconds. Force refresh available via dashboard button.
- **Accessibility is strong.** ARIA roles, live regions, `role="progressbar"` with value attributes, screen reader announcements via `analyticsAnnouncer`, and `role="radiogroup"` for period selector.

---

### 8. Search, Shortcuts, and Help

**Verdict:** :white_check_mark: Pass

**SearchPalette:**
- **Fuzzy file search with scoring.** `SearchPalette.svelte:81-99` implements a 4-tier scoring: starts-with filename > contains filename > contains path > fuzzy char-in-order. Fast and effective.
- **Keyboard navigation.** Arrow up/down to select, Enter to open, Escape to close. Auto-focus on input when opened.
- **Relative path display.** `SearchPalette.svelte:159-165` strips the project root for cleaner display.
- **Discoverable via Ctrl+P.** Referenced in editor empty state hint. Also available via menu bar (not shown in files but implied by events).

**FindInFiles:**
- **Grep-based full-text search.** `FindInFiles.svelte:49-63` calls `adapter.grepFiles()`. Results grouped by file with line numbers.
- **Click to open result file.** `FindInFiles.svelte:66-76` opens the file in the editor. Note: does not jump to the matching line (comment at line 71: "jumping to line would require editor line API").
- **Summary shows match count and file count.** `FindInFiles.svelte:141-143`. Helpful context.

**ShortcutsDialog:**
- **Grouped by context.** `ShortcutsDialog.svelte:32-41` groups shortcuts by their context key. Each shortcut shows description and key combo rendered as `<kbd>` elements.
- **Accessible.** Dialog with `aria-modal`, focus trapping, Escape to close.
- **Discoverable via Help menu.** `MenuBar.svelte:137` dispatches `reasonance:shortcuts` event.

**HelpPanel:**
- **Locale-aware markdown documentation.** `HelpPanel.svelte:9-20` loads docs from `$lib/docs/{locale}/index.md` with English fallback.
- **Search within docs.** `HelpPanel.svelte:30-75` performs text-node-level highlighting with scroll-to-first-match. This is a polished feature.
- **Accessible via F1.** `MenuBar.svelte:136` references F1 shortcut.

---

### 9. Session Management

**Verdict:** :x: Fail

**Specific findings:**

- **No session list UI.** The `agent-session.ts` store tracks sessions with `agentSessions` (a Map of session state) and `activeAgentSessionId`, but there is no component that renders a list of sessions. The user cannot browse, switch between, or manage chat sessions.
- **"Session" is invisible terminology.** Nothing in the UI explains what a session is. The closest concept visible to the user is "terminal instance" (e.g., "Claude 1", "Claude 2" tabs in `TerminalManager.svelte`). The session abstraction exists only in stores.
- **No session renaming.** `AgentSessionState.title` exists (`agent-session.ts:11`) but there is no UI to set or change it.
- **No session history browser.** `adapter.sessionGetEvents()` loads events for a session (`ChatView.svelte:30-34`), but there is no way to discover past sessions.
- **Fork creates orphan sessions.** The FORK button (`ActionableMessage.svelte:53-56`) calls `adapter.sessionFork()` which returns a new session ID, but the comment at `ChatView.svelte:77` says "Phase 6/7: navigate to forked session tab" -- this is not implemented. Forked sessions are invisible.
- **No session export or sharing.** There is no way to export a conversation or share it.
- **Session lifecycle is tied to terminal instance.** Closing a terminal instance (`TerminalManager.svelte:118-153`) kills the process. There is no concept of pausing/resuming sessions. The session state in stores is not cleaned up on close (no call to `removeSession()`), which could lead to stale data.

---

## Friction Points (ranked by severity)

| # | Severity | Flow | Issue | Suggested Fix |
|---|----------|------|-------|---------------|
| 1 | Critical | First Launch | No onboarding -- user does not know they need to configure an LLM before the app is useful | Add a setup wizard: detect CLIs, prompt for API keys, show a "Your first session" guided flow |
| 2 | Critical | Session Mgmt | No session list, no history, no way to see past conversations | Build a session sidebar or drawer with list, search, rename, delete |
| 3 | High | Chat | Diffs in chat have no accept/reject -- code cannot be applied from chat view | Add "Apply" and "Reject" buttons to `DiffBlock.svelte` when rendered inside `ChatMessages` |
| 4 | High | Chat | Code blocks in chat have no "Apply to file" action | Add a "Copy" and "Insert into [filename]" button to `CodeBlock.svelte` |
| 5 | High | Editor | Read-only by default with no visible indicator or toggle | Show a lock icon when read-only; add an "Edit" button to unlock |
| 6 | High | Settings | "Scan CLI" button buried in Provider section, not in LLM config section | Move it to the top of LLM config section, or run it automatically on first launch |
| 7 | Medium | Editor | No visible save button -- only Ctrl+S or menu | Add a save icon/button in the editor tabs actions area |
| 8 | Medium | Editor | Markdown preview toggle not exposed in UI | Add a preview toggle button in `EditorTabs` actions when active file is `.md` |
| 9 | Medium | Terminal | Mode switching dropdown does nothing (TODO in code) | Implement mode switching or hide the dropdown until implemented |
| 10 | Medium | Chat | FORK button creates invisible orphan sessions | Either implement session navigation or hide the button |
| 11 | Medium | Terminal | "SWARM" tab shows placeholder -- confusing | Hide the tab or mark it clearly as "Coming Soon" with disabled styling |
| 12 | Medium | File Tree | No "empty folder" message | Show "No files found" or suggestion text when directory is empty |
| 13 | Medium | File Tree | No file/folder creation or deletion from UI | Add right-click context menu with New File, New Folder, Delete options |
| 14 | Low | Settings | Font family hardcoded on save (always Atkinson Hyperlegible Mono) | Respect user selection or remove the non-functional font display |
| 15 | Low | Editor | Find in Files does not jump to matching line | Wire up editor line scrolling when a search result is clicked |
| 16 | Low | Terminal | Single-click file open has 250ms delay (double-click detection) | Consider using single-click immediate open, double-click for external |
| 17 | Low | First Launch | No version info on welcome screen | Add version number to welcome screen footer |
| 18 | Low | Settings | No input validation for command paths or API endpoints | Validate that binary exists on save, check URL format for endpoints |

---

## What Works Well

- **Three-panel layout with resizable dividers.** The drag handles include keyboard support (Arrow keys with Shift for larger steps, `App.svelte:66-87`), and a resize overlay prevents selection during drag. Skip links for keyboard navigation are present.

- **Accessibility throughout.** Focus trapping in modals (`trapFocus` utility), ARIA roles on trees/tabs/dialogs/menus, `aria-live="polite"` on chat messages, `role="progressbar"` on analytics, screen reader announcements via dedicated announcer utility. Reduced-motion support for animations.

- **Analytics system is production-grade.** Real-time cost tracking, historical comparison, per-provider breakdown, cache hit rate monitoring, cost velocity and projection, budget alerts, and data export. The insights engine surfaces actionable warnings without requiring the user to interpret raw data.

- **Internationalization.** Every user-facing string goes through `$tr()`. Nine languages supported with locale-aware help docs. This is rare in early-stage tools.

- **Terminal empty state guides the user.** When no LLMs are configured, the terminal panel shows a clear banner with a button to open Settings. When LLMs exist, it shows selectable cards with a start button. This is the gold standard for the rest of the app to follow.

- **Error boundaries on all panels.** `App.svelte:109-122, 130-145, 153-166` wraps file tree, editor, and terminal in `<svelte:boundary>` with retry buttons. Crashes in one panel do not take down the entire app.

- **Git integration via toolbar and menu bar.** Quick-access dropdown with common git commands (`Toolbar.svelte:47-58`) plus destructive action confirmation (push requires confirm). Menu bar provides the same commands in a standard File/Edit/View/Terminal/Git/Help structure.

- **Dark/light theme support with system detection.** Three-way toggle (light/dark/system) in settings and welcome screen. All components use CSS custom properties consistently.

- **Context menu on selected editor text.** Right-click selected code to send it to the LLM with a response panel. This is a natural interaction for vibecoders who want to ask "explain this" or "refactor this."

- **StatusBar provides live session context.** Context window usage bar, token count, reset timer, messages remaining, and progress indicators are all parsed from terminal output and displayed in a compact footer. When YOLO mode is active, the entire bar turns red with a clear warning.
