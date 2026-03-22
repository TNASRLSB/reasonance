# Nielsen Heuristic Scorecard -- Reasonance IDE

**Date:** 2026-03-22
**Source data:** Phase 1 audit reports (vibecoder, CTO, UX/UI, security, i18n, stress, performance)
**Scale:** 1 (critical failures) -- 5 (excellent, no issues found)

---

## Scoring Matrix

| Component | H1 Visibility | H2 Real World | H3 Control/Freedom | H4 Consistency | H5 Error Prevention | H6 Recognition | H7 Flexibility | H8 Aesthetic | H9 Error Recovery | H10 Help/Docs | AVG |
|-----------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **App (shell)** | 4 | 4 | 4 | 4 | 3 | 4 | 4 | 4 | 4 | 3 | 3.8 |
| **WelcomeScreen** | 2 | 3 | 3 | 3 | 2 | 2 | 2 | 4 | 2 | 1 | 2.4 |
| **FileTree** | 3 | 4 | 2 | 4 | 2 | 4 | 3 | 4 | 2 | 2 | 3.0 |
| **Editor** | 3 | 4 | 2 | 3 | 2 | 3 | 4 | 4 | 2 | 3 | 3.0 |
| **EditorTabs** | 3 | 4 | 3 | 3 | 3 | 4 | 2 | 4 | 3 | 2 | 3.1 |
| **DiffView** | 3 | 3 | 4 | 3 | 3 | 3 | 2 | 4 | 2 | 1 | 2.8 |
| **ChatInput** | 4 | 4 | 4 | 4 | 3 | 4 | 3 | 4 | 3 | 2 | 3.5 |
| **ChatMessages** | 4 | 4 | 3 | 4 | 2 | 4 | 2 | 4 | 3 | 2 | 3.2 |
| **CodeBlock/DiffBlock** | 3 | 4 | 1 | 3 | 3 | 3 | 1 | 4 | 2 | 1 | 2.5 |
| **Terminal** | 4 | 3 | 3 | 3 | 2 | 3 | 4 | 3 | 3 | 2 | 3.0 |
| **TerminalManager** | 4 | 3 | 3 | 3 | 3 | 4 | 3 | 3 | 3 | 2 | 3.1 |
| **Settings** | 3 | 3 | 4 | 3 | 2 | 3 | 3 | 3 | 4 | 2 | 3.0 |
| **AnalyticsDashboard** | 5 | 4 | 4 | 4 | 4 | 5 | 4 | 4 | 4 | 3 | 4.1 |
| **SearchPalette** | 4 | 4 | 5 | 4 | 4 | 5 | 4 | 4 | 4 | 3 | 4.1 |
| **FindInFiles** | 3 | 4 | 4 | 4 | 3 | 4 | 3 | 4 | 3 | 2 | 3.4 |
| **ShortcutsDialog** | 4 | 4 | 5 | 4 | 4 | 5 | 3 | 4 | 3 | 5 | 4.1 |
| **HelpPanel** | 4 | 4 | 4 | 4 | 4 | 5 | 4 | 4 | 3 | 5 | 4.1 |
| **HEURISTIC AVG** | **3.5** | **3.7** | **3.4** | **3.5** | **2.9** | **3.8** | **3.0** | **3.8** | **2.9** | **2.4** | **3.2** |

---

## Heuristic-Level Summary

### H1 -- Visibility of System Status (avg 3.5)

**Strengths:** ChatHeader shows live metrics (provider, model, tokens, tok/s, elapsed). AnalyticsBar provides always-visible session metrics (context%, cost, velocity, projection). StreamingIndicator shows when agent is responding. YOLO mode activates red status bar + warning banner. Skeleton loading states on analytics components.

**Weaknesses:** Editor has no loading indicator while file content is being fetched or language highlighting loads. FileTree shows no loading state during `listDir`. Editor gives no visual indication of read-only mode. No progress indicator for file watcher diff detection. No cursor line/column display in status bar.

### H2 -- Match Between System and Real World (avg 3.7)

**Strengths:** File tree uses standard folder/file metaphor with expand/collapse. Editor tabs match browser tab conventions. Git dropdown uses standard Git terminology. Analytics uses familiar KPI card + chart patterns.

**Weaknesses:** "TERM" / "CHAT" toggle labels are abbreviated and may confuse non-technical users. "YOLO" is jargon. "Swarm" tab uses domain-specific terminology without explanation. Session concept is invisible in the UI despite existing in the data model.

### H3 -- User Control and Freedom (avg 3.4)

**Strengths:** Settings has pending-delete pattern with undo before save. SearchPalette and dialogs close with Escape. App panels are resizable with keyboard support. Chat auto-scroll only when near bottom (does not hijack scroll).

**Weaknesses:** No undo/redo beyond CodeMirror's built-in. No session history navigation. Fork button creates invisible orphan sessions. Cannot create/delete files from FileTree. No way to revert applied diffs from chat. Editor defaults to read-only with no toggle mechanism. CodeBlock/DiffBlock in chat have no apply/reject actions.

### H4 -- Consistency and Standards (avg 3.5)

**Strengths:** CSS custom properties used consistently across all components (35 tokens). ARIA roles correctly applied on trees, tabs, dialogs, menus in most places. i18n system covers all user-facing strings (226 keys). Dark/light theme consistent via CSS variables.

**Weaknesses:** MenuBar has incorrect `role="menubar"` placement (on items, not container). EditorTabs has `tabindex="0"` on all tabs instead of only active tab. Physical CSS properties used everywhere (80+ instances) breaking RTL. `confirm()` browser dialogs mixed with custom modals. Some dropdowns use `menuKeyHandler` while others do not. Instance close is `<span role="button">` instead of `<button>`.

### H5 -- Error Prevention (avg 2.9)

**Strengths:** Tab close with unsaved changes shows confirmation. `grep_files` caps results at 500. PTY write rejects payloads > 64KB. Open-same-file-twice is deduplicated. Destructive git actions require confirmation.

**Weaknesses:** No file size guard on `read_file` (50MB file will freeze UI). No input validation on Settings fields (command paths, API endpoints, key env vars). No debounce on rapid file switching (full editor teardown/rebuild each time). No symlink loop detection in SearchPalette file indexing. No max tab count limit. No content validation on `write_config` (enables privilege escalation chain via PTY allowlist). No atomic file writes from editor.

### H6 -- Recognition Rather Than Recall (avg 3.8)

**Strengths:** TerminalManager empty state shows selectable LLM cards. Editor empty state hints at Ctrl+P. Toolbar provides visual icons for all major actions. Analytics insights surface actionable warnings automatically. SearchPalette provides fuzzy matching. FileTree uses gitignored file dimming as visual cue.

**Weaknesses:** LLM configuration requires knowing binary names, args, env var names. No preset templates or auto-detection in the config form (Scan CLI button is buried in a separate section). Keyboard shortcuts are not discoverable inline. No breadcrumb or path indicator in editor for current file context.

### H7 -- Flexibility and Efficiency of Use (avg 3.0)

**Strengths:** Keyboard shortcuts for major actions (Ctrl+P search, F1 help, Ctrl+Shift+A analytics). Context menu on selected editor text for LLM actions. CodeMirror language extensions lazy-loaded. Terminal Ctrl+F search. Slash commands in terminal toolbar. Multiple LLM instances with tab management.

**Weaknesses:** Mode switching dropdown renders but does nothing (TODO). No session management (browse, switch, search history). No file creation from FileTree. No "Apply to file" action on code blocks in chat. No export of terminal output. Chat diffs cannot be accepted/rejected inline. No bulk operations on tabs. No customizable keyboard shortcuts (beyond LLM switching). Monolithic 1.89MB JS bundle means slow first load for all users regardless of feature usage.

### H8 -- Aesthetic and Minimalist Design (avg 3.8)

**Strengths:** Clean three-panel layout with resizable dividers. Atkinson Hyperlegible font for readability. Consistent color theming via CSS custom properties. Minimal reset (no bloated normalize.css). WOFF2-only font strategy. Sparklines and compact KPI cards in analytics.

**Weaknesses:** Accent color (`#1d4ed8`) fails WCAG AA contrast as text on dark backgrounds (3.2:1, needs 4.5:1). StatusBar white-on-accent fails AA for normal text (3.8:1). StatusBar opacity reductions on `.file-lang` (0.7) and `.file-encoding` (0.5) create critically low contrast. German translations are 1.4-2.1x longer than English, causing truncation. No CJK/Devanagari fonts in fallback chain.

### H9 -- Help Users Recognize, Diagnose, and Recover from Errors (avg 2.9)

**Strengths:** `<svelte:boundary>` on all three main panels with RETRY buttons. ErrorBlock in chat shows severity and error code. Settings shows specific error on save/load failure. PTY exit writes exit message to terminal. Analytics error flash with recovery pulse. Toast notifications with `role="alert"`.

**Weaknesses:** 408 Rust `unwrap()` calls -- any mutex poison cascades to full app crash with no recovery. File read failures are silently swallowed (`console.error` only). Binary file open produces raw error string, no friendly message. LLM process spawn failure is silent. Agent message send failure stops streaming indicator but shows no user-visible error. Session fork failure is silent. Stderr from CLI processes is `Stdio::null()` -- all CLI error output permanently lost.

### H10 -- Help and Documentation (avg 2.4)

**Strengths:** HelpPanel loads locale-aware markdown docs with in-doc search and highlighting. ShortcutsDialog groups shortcuts by context with `<kbd>` rendering. F1 opens help. Shortcuts accessible from Help menu.

**Weaknesses:** No onboarding wizard or getting-started flow. No contextual help or tooltips explaining features on first use. No explanation of what the app does or how its three-panel layout works on the welcome screen. No documentation of LLM configuration requirements. No inline help in Settings forms. "Swarm" feature shows placeholder with no explanation. No version info visible outside Settings.

---

## Cognitive Load Assessment

| Component | Rating | Justification |
|-----------|--------|---------------|
| **App (shell)** | Medium | Three-panel layout is standard for IDEs; skip links and landmarks help navigation. Resizable dividers add minor cognitive overhead. The lack of onboarding means users must self-discover the layout purpose. |
| **WelcomeScreen** | Low | Simple screen with one primary action ("Open Folder"). However, the absence of guidance about LLM configuration creates a hidden prerequisite that raises effective cognitive load when users hit a wall later. |
| **FileTree** | Low | Standard tree widget with familiar expand/collapse. Gitignored file dimming provides useful visual hierarchy. Single-click open is intuitive. Lack of file creation options reduces available actions (lower load but lower capability). |
| **Editor** | Medium | CodeMirror provides familiar editing experience. However, invisible read-only state, absence of save button, hidden markdown preview toggle, and no loading indicators mean users must recall rather than recognize actions. |
| **EditorTabs** | Low | Standard tab bar metaphor. Dirty indicator (dot) is subtle but conventional. Unsaved-close confirmation prevents data loss. Truncation via ellipsis is clean. |
| **DiffView** | Medium | Accept/Reject buttons are clear when present. However, the diff view only appears for file-watcher changes; users cannot trigger it manually. The relationship between chat diffs and editor diffs is unclear. |
| **ChatInput** | Low | Simple textarea + send button. Enter-to-send, Shift+Enter-for-newline is a common chat convention. Disabled state during streaming is clear. |
| **ChatMessages** | Medium | Message grouping by role with content blocks (text, code, diff, thinking, tool use, error) creates a complex visual hierarchy. No virtualization means performance degrades with message count, adding scroll friction. Auto-scroll behavior is correct but users reading history may be confused when it re-engages. |
| **CodeBlock/DiffBlock** | High | Code blocks display code but offer no actionable path (no "apply to file", no "insert into editor"). Diff blocks in chat lack accept/reject, creating a dead-end view. Users must mentally map chat content to file actions, then manually copy/paste. This is the highest cognitive gap in the app. |
| **Terminal** | Medium | xterm.js provides familiar terminal experience. PTY context parsing (context%, tokens, model) surfaces useful data automatically. The TERM/CHAT toggle uses abbreviated labels. Search (Ctrl+F) is standard. Clipboard operations work as expected. |
| **TerminalManager** | Medium | Two-level tab system (LLM tabs + instance tabs) adds structural complexity. The empty state with LLM selector cards reduces initial confusion. "Swarm" tab placeholder adds unnecessary noise. Mode dropdown is non-functional, creating false affordance. |
| **Settings** | High | Comprehensive but dense. LLM configuration requires prior knowledge (binary names, args, env var names, yolo flags). No presets, no auto-detection in the form itself (Scan CLI is in a different section). 9 sections to navigate. Font hardcoded on save despite appearing editable. No input validation gives false confidence. |
| **AnalyticsDashboard** | Medium | Feature-rich but well-organized: period selector, KPI cards, sparklines, provider comparison, insights. The insights engine reduces cognitive load by surfacing actionable warnings. Skeleton loading provides progress feedback. Export options are discoverable. |
| **SearchPalette** | Low | Single input with fuzzy matching and scored results. Keyboard navigation (arrows + enter) is standard. Relative paths reduce visual noise. Loading and empty states are clear. |
| **FindInFiles** | Low | Standard grep-style search with grouped results. File + line number display is conventional. Click-to-open is intuitive. However, not jumping to the matching line after opening adds a small recall burden. |
| **ShortcutsDialog** | Low | Clean grouped display of all keyboard shortcuts. `<kbd>` rendering is clear. Read-only reference with no interactions beyond close. |
| **HelpPanel** | Low | Markdown documentation with search and highlighting. Locale-aware content loading. Standard help panel pattern. |

---

## Error State Coverage

### Legend
- **Handled** = user-visible feedback with recovery path
- **Partial** = error caught but feedback is missing, silent, or incomplete
- **Missing** = no error handling; crash, blank state, or silent failure

| Component | Error States Handled | Error States Partial | Error States Missing |
|-----------|---------------------|---------------------|---------------------|
| **App (shell)** | Panel crash (boundary + retry), window resize during drag (overlay + bounds) | -- | -- |
| **WelcomeScreen** | Empty recent projects list | Theme toggle failure, window control failure | No LLM configured (deferred to TerminalManager) |
| **FileTree** | -- | File read error (caught, console.error only) | Empty directory (no message), listDir loading failure (silent), binary file click (raw error) |
| **Editor** | Empty state (no file open) | -- | File content loading failure, language load failure, read-only state (no indicator), large file (no size guard) |
| **EditorTabs** | Unsaved close (confirm dialog), deleted file (italic + "(deleted)" label) | -- | Tab overflow (no count limit/warning) |
| **DiffView** | Accept/Reject with visual feedback | -- | Diff generation failure, merge conflict |
| **ChatInput** | Disabled during streaming | -- | -- |
| **ChatMessages** | Error events via ErrorBlock (severity + code) | Agent send failure (streaming stops, no message) | Empty state (no "start a conversation" prompt), memory exhaustion from unbounded events |
| **CodeBlock/DiffBlock** | Copy button with "COPIED" feedback | -- | Copy failure |
| **Terminal** | PTY exit message, WebGL fallback to DOM renderer, clipboard paste failure | -- | PTY spawn failure (silent), large paste > 64KB (error caught but only console.warn) |
| **TerminalManager** | No LLMs configured (banner + Settings link), LLM selector cards | LLM spawn failure (caught, console.log) | YOLO toggle race condition (concurrent restarts), process orphaning on task abort |
| **Settings** | Save/load error (banner with message), pending delete with undo | Config parse failure (partial catch) | Invalid command paths, invalid API endpoints, invalid env var names, font hardcoded on save |
| **AnalyticsDashboard** | Loading skeleton, error state from store, budget warnings (color change) | -- | Analytics data corruption, very large dataset (OOM from full Vec clone) |
| **SearchPalette** | Loading state, empty results, hint text | -- | Symlink loop in file indexing (infinite recursion) |
| **FindInFiles** | Match/file count summary, empty results | -- | Grep execution failure |
| **ShortcutsDialog** | -- | -- | -- (read-only, minimal error surface) |
| **HelpPanel** | English fallback for missing locale docs | -- | Doc file missing entirely, search in empty doc |

---

## Cross-Cutting Issues (affecting multiple components)

### 1. Rust Backend Crash Risk
408 `unwrap()` calls on mutex locks across 34 Rust source files. A single thread panic poisons the mutex, cascading panics across all subsequent lock attempts. This affects every component that communicates with the backend (FileTree, Editor, Terminal, TerminalManager, Settings, AnalyticsDashboard, SearchPalette, FindInFiles). **Every heuristic is degraded by this systemic risk.**

### 2. RTL Layout Breakage
80+ physical CSS directional properties (`margin-left`, `padding-left`, `text-align: left`, `border-left`, `left:`, `right:`) across 20+ components. Arabic locale sets `dir="rtl"` but no CSS uses logical properties. All 17 components with styled layouts are affected. **Impacts H2 (real world match), H4 (consistency), H8 (aesthetics) for Arabic users.**

### 3. Missing Internationalization
67-70 i18n keys untranslated per locale (except Italian). 60+ hardcoded English strings in component templates (`title`, `aria-label`, `placeholder`). No locale-aware number/date formatting. **Impacts H2, H4, H10 for non-English users.**

### 4. Monolithic Bundle
1.89 MB single JS chunk contains all components and libraries. No code splitting. Every user pays the full cost of every feature on first load. **Impacts H1 (perceived system status during load), H7 (efficiency), H8 (performance as aesthetic).**

### 5. Accent Color Contrast
`--accent` (`#1d4ed8`) fails WCAG AA when used as text on dark backgrounds (3.2:1 ratio, needs 4.5:1). Used in WelcomeScreen, StatusBar, MarkdownPreview links, ShortcutsDialog group labels. **Impacts H8 (aesthetics), H4 (consistency) for low-vision users.**

### 6. Unbounded Memory Growth
`agentEvents` Map grows without limit during sessions. No chat message virtualization. No explicit terminal scrollback cap configured. Analytics store clones entire Vec on every query. **Impacts H1 (system degrades over time), H5 (no prevention), H9 (no recovery from OOM).**

---

## Priority Improvement Targets

| Priority | Heuristic | Current Avg | Target | Key Actions |
|----------|-----------|:-----------:|:------:|-------------|
| P0 | H10 Help/Docs | 2.4 | 3.5 | Add onboarding wizard, contextual tooltips, LLM config guidance |
| P0 | H5 Error Prevention | 2.9 | 3.5 | File size guards, input validation, debounce rapid switching, mutex error handling |
| P0 | H9 Error Recovery | 2.9 | 3.5 | Surface file read errors, binary file messages, LLM spawn failures, capture stderr |
| P1 | H7 Flexibility | 3.0 | 3.5 | Code apply actions in chat, session management, file creation in tree, code splitting |
| P1 | H3 Control/Freedom | 3.4 | 4.0 | Accept/reject diffs in chat, session history, read-only toggle, undo for applied changes |
| P2 | H4 Consistency | 3.5 | 4.0 | Fix ARIA structure, logical CSS properties, standardize button elements, fix tabindex |
| P2 | H1 Visibility | 3.5 | 4.0 | Loading indicators in editor/filetree, read-only indicator, cursor position display |
