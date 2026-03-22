# Changelog

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
- feat(ui): integrate SwarmCanvas as fullscreen overlay in main layout
- feat(ui): integrate SwarmPanel as tab in TerminalManager
- feat(ui): add SwarmCanvas with Svelvet graph, toolbar, inspector, dual mode
- feat(ui): add SwarmPanel compact monitoring component
- feat(ui): add SwarmInspector component (node props, JSON toggle)
- feat(ui): add Agent, Resource, Logic node components with state colors
- feat(ui): add NodeCatalog component (Agent/Resource/Logic buttons)
- feat(ui): add SwarmControls component (play/pause/stop/step)
- feat(frontend): add workflow engine adapter and store
- feat(engine): add Tauri commands (play, pause, resume, stop, step, notify)
- feat(engine): add WorkflowEngine with graph analysis, run lifecycle, and scheduler
- feat: add agent swarm frontend adapter types and Svelte stores
- feat: add Agent Runtime (Tasks 9-10) for agent swarm platform
- feat: add WorkflowStore with CRUD commands (Tasks 6-8)
- feat: add Discovery Engine for agent swarm platform
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
- - SwarmCanvas: layout and reactivity improvements
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
- chore: cleanup — swarm coming soon, fix a11y warnings, i18n DiffView
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
- chore: add svelvet dependency and swarm UI stores
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
- - Implement all agent swarm methods in TauriAdapter (discovery, workflow CRUD, agent runtime)
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
- chore: add reqwest, tokio, chrono dependencies for agent swarm
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

