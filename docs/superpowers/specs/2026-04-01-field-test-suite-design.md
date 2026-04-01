# Reasonance Field Test Suite — Design Spec

**Created:** 2026-04-01
**Purpose:** Comprehensive field testing of all Reasonance features using AI-driven visual testing on the real desktop environment (KDE Wayland, Arch Linux).
**Approach:** Claude Code acts as the test engine — reads scenarios, takes screenshots, analyzes UI via vision, executes actions via dotool, verifies results, generates reports and bug reports.

---

## Architecture

### Philosophy

Tests are defined as natural language scenarios, not brittle coordinate-based scripts. Claude reads each scenario, observes the actual UI state via screenshots, reasons about what to do, and acts. This makes tests resilient to UI changes and able to catch unexpected issues.

The Python helper library handles mechanical tasks (screenshots, input, window management, app lifecycle). Claude handles judgment tasks (is this UI correct? did the action succeed? is there something wrong here?).

### Infrastructure

| Function | Tool | Notes |
|----------|------|-------|
| Screenshots | `spectacle -b -n -f -o <file>` | KDE Wayland native |
| Mouse/keyboard | `dotool` via pipe | `echo "mouseto 0.5 0.5" \| dotool` |
| Window control | `qdbus6` + KWin scripting | Focus, minimize, geometry |
| Backend logs | `RUST_LOG=info` to file | Parsed for errors |
| UI analysis | Claude Code `Read` tool on PNG | Vision-based assertions |
| App lifecycle | `npx tauri dev` | Kill/relaunch between test groups |

### File Structure

```
tests/field/
├── scenarios/                    # Test definitions (YAML)
│   ├── smoke.yaml                # 1-7: quick pass/fail
│   ├── e2e.yaml                  # 8-25: full user workflows
│   ├── stress.yaml               # 26-33: performance and limits
│   ├── edge.yaml                 # 34-44: edge cases
│   ├── cross.yaml                # 45-52: cross-feature interactions
│   ├── security.yaml             # 53-57: security verification
│   ├── visual.yaml               # 58-62: visual regression
│   └── integrity.yaml            # 63-67: data integrity
├── lib/
│   ├── screen.py                 # Screenshot capture and comparison
│   ├── input.py                  # dotool wrappers (click, type, key, mouseto)
│   ├── window.py                 # KWin D-Bus window management
│   ├── app.py                    # Reasonance launch, wait, kill, log parsing
│   └── report.py                 # JSON + HTML report generation
├── runner.py                     # CLI: launch app, invoke Claude per scenario
├── screenshots/
│   ├── baseline/                 # Reference screenshots per test
│   └── runs/                     # Timestamped run results
├── reports/                      # Generated reports per run
└── bugs/                         # Auto-generated bug reports
```

### Helper Library API

**screen.py:**
```python
def screenshot(name: str, directory: str = None) -> str:
    """Take full-screen screenshot, return path."""

def screenshot_active(name: str) -> str:
    """Screenshot active window only."""

def compare_images(current: str, baseline: str) -> float:
    """Perceptual similarity score 0.0-1.0."""
```

**input.py:**
```python
def click(x: int, y: int):
    """Click at absolute pixel coordinates."""

def click_pct(x_pct: float, y_pct: float):
    """Click at percentage coordinates (0.0-1.0)."""

def type_text(text: str, delay_ms: int = 50):
    """Type text with inter-key delay."""

def key(combo: str):
    """Press key combo, e.g. 'ctrl+p', 'Return', 'Escape'."""

def wait(ms: int):
    """Pause between actions."""
```

**window.py:**
```python
def focus(resource_class: str):
    """Bring window to front via KWin D-Bus scripting."""

def minimize_others(keep: str):
    """Minimize all windows except the specified one."""

def get_geometry(resource_class: str) -> dict:
    """Return {x, y, width, height} of window."""

def maximize(resource_class: str):
    """Maximize window for consistent screenshots."""
```

**app.py:**
```python
class ReasonanceApp:
    def launch(self, env: dict = None):
        """Start app with npx tauri dev, RUST_LOG=info."""

    def wait_ready(self, timeout: int = 60):
        """Block until 'setup complete' appears in logs."""

    def kill(self):
        """Terminate app and all child processes."""

    def logs(self) -> str:
        """Return full backend log content."""

    def get_errors(self) -> list[str]:
        """Extract ERROR/WARN/panic lines from logs."""

    def get_frontend_errors(self) -> list[str]:
        """Extract 'Unhandled rejection' from Vite output."""

    def startup_time_ms(self) -> int:
        """Milliseconds from launch to setup complete."""
```

**report.py:**
```python
def generate_report(results: list[dict], output_dir: str):
    """Generate JSON + HTML report with embedded screenshots."""

def generate_bug_report(test_result: dict, output_dir: str):
    """Generate detailed bug report markdown for failed tests."""
```

---

## Test Execution Model

### How Claude Runs a Test

1. Read the scenario from YAML (steps + pass criteria)
2. Ensure app is in the required state (launch if needed, navigate to starting point)
3. For each step:
   a. Take screenshot
   b. Analyze UI — find the element to interact with
   c. Execute the action (click, type, key)
   d. Wait for UI to settle (screenshot loop or log check)
   e. Verify the step's expected outcome
   f. If something unexpected appears, document it
4. Check all pass criteria
5. Record result: pass/fail, screenshots, errors, duration, notes
6. If fail: generate bug report with steps to reproduce, screenshots, log excerpts, likely cause

### Isolation Between Tests

- **Smoke tests (1-7):** Sequential, share one app instance. Quick pass/fail.
- **E2E tests (8-25):** Each gets a fresh app launch OR a known clean state.
- **Stress tests (26-33):** Fresh app per test. Monitor memory/timing.
- **Edge cases (34-44):** Fresh app or clean state per test.
- **Cross-feature (45-52):** Fresh app, specific setup per test.
- **Security (53-57):** Fresh app with specific trust/permission config.
- **Visual (58-62):** Fresh app, fixed window size (1280x800), both themes.
- **Integrity (63-67):** Fresh app, specific data setup.

### Fuzzing Mode

Beyond the 67 deterministic tests, a free-exploration mode:
- Claude navigates the app without a script
- Randomly opens files, clicks buttons, resizes panels, toggles settings
- Tries unusual combinations (open 20 tabs then switch theme, resize to minimum, etc.)
- Documents anything that breaks, looks wrong, or behaves unexpectedly
- Runs for a configurable duration (default: 30 minutes)
- Generates a fuzzing report with all findings

---

## Test Scenarios

### Suite 1: Smoke Tests (1-7)

```yaml
- id: smoke_01
  name: App startup
  steps:
    - Launch Reasonance with RUST_LOG=info
    - Wait for "setup complete" in logs
  pass_criteria:
    - No panic in logs
    - No "Unhandled rejection" in frontend
    - Startup completes within 30 seconds
    - Window appears and is visible

- id: smoke_02
  name: Open project
  steps:
    - Click "APRI CARTELLA" button
    - Select /home/uh1/VIBEPROJECTS/REASONANCE
  pass_criteria:
    - File tree populates with project files
    - src/, src-tauri/, package.json visible in tree
    - No red error banner
    - No errors in backend logs

- id: smoke_03
  name: Open file
  steps:
    - Click on a .ts file in the file tree (e.g. src/lib/adapter/tauri.ts)
  pass_criteria:
    - Editor tab opens with file name
    - Content visible with syntax highlighting
    - No errors

- id: smoke_04
  name: Open terminal
  steps:
    - Click "Terminale" in menu bar or use keyboard shortcut
    - Or click on an LLM provider in the terminal panel
  pass_criteria:
    - Terminal panel shows PTY output or LLM selection
    - No crash or error

- id: smoke_05
  name: Open each panel
  steps:
    - Click SETTINGS button in toolbar
    - Click ANALYTICS button
    - Click MEMORY button
    - Click HIVE button
    - Click GIT button
  pass_criteria:
    - Each panel opens without crash
    - Each panel shows relevant content
    - Can return to editor view

- id: smoke_06
  name: Theme switch
  steps:
    - Open Settings
    - Find theme selector
    - Switch between dark and light theme
  pass_criteria:
    - Theme changes visually
    - No flash of unstyled content
    - All components update consistently
    - No errors

- id: smoke_07
  name: Close and reopen (state persistence)
  steps:
    - Note current state (open files, active tab, panel sizes)
    - Kill the app
    - Relaunch the app
    - Wait for startup
  pass_criteria:
    - Last project auto-restored
    - Previously open files restored in tabs
    - Active file tab correct
    - Panel sizes approximately restored
```

### Suite 2: E2E Workflows (8-25)

```yaml
- id: e2e_08
  name: File tree navigation
  steps:
    - Expand/collapse directories in file tree
    - Verify git status icons appear on modified files
    - Check auto-fold for single-child directories
    - Verify directory aggregation for git status
    - Scroll through the tree
  pass_criteria:
    - Directories expand/collapse smoothly
    - Git status icons visible (modified, untracked, etc.)
    - Single-child dirs auto-folded (e.g. src/routes/+page.svelte)
    - Scroll performance smooth
    - Icons match actual git status

- id: e2e_09
  name: Editor full workflow
  steps:
    - Open a file from file tree
    - Verify syntax highlighting
    - Edit the file (type some text)
    - Ctrl+Z to undo
    - Ctrl+S to save
    - Open a second file (multi-tab)
    - Switch between tabs
    - Ctrl+P to open file search
    - Type a filename, select result
    - Close a tab
  pass_criteria:
    - Syntax highlighting correct for file type
    - Edits appear in real-time
    - Undo reverts the edit
    - Save writes to disk (verify with cat)
    - Multi-tab works, switching preserves content
    - Ctrl+P opens fuzzy search, results appear
    - Tab closes cleanly

- id: e2e_10
  name: Chat with real LLM
  requires_llm: true
  providers: [claude, qwen]
  steps:
    - Click on a provider (e.g. CLAUDE) in the terminal panel
    - Wait for session to initialize
    - Type a simple prompt ("What is 2+2?")
    - Press Enter
    - Wait for streaming response
    - Verify response appears
    - Check session history
  pass_criteria:
    - Session starts without error
    - Prompt appears in chat
    - Response streams in (text appears incrementally)
    - Response is coherent (actually answers the question)
    - Session history shows the exchange
    - No errors in backend logs

- id: e2e_11
  name: Terminal PTY
  steps:
    - Open a terminal (not LLM — a shell PTY)
    - Type "echo hello" + Enter
    - Verify output
    - Type "ls" + Enter
    - Verify file listing
    - Resize terminal panel
    - Verify output reflows or stays correct
    - Open second terminal
    - Close first terminal
  pass_criteria:
    - Shell prompt visible
    - Commands execute and show output
    - Resize doesn't break display
    - Multiple terminals work independently
    - Closing terminal cleans up PTY

- id: e2e_12
  name: Workflow HIVE
  steps:
    - Click HIVE button in toolbar
    - Create new workflow
    - Add 2-3 nodes from the node palette
    - Connect nodes with edges
    - Press play to execute workflow
    - Observe node state changes
    - Pause workflow
    - Stop workflow
  pass_criteria:
    - HIVE canvas renders
    - Nodes can be added and positioned
    - Edges connect between nodes
    - Play starts execution (node states change)
    - Pause/stop work correctly
    - No errors throughout

- id: e2e_13
  name: File operations with undo
  steps:
    - Create a new file via File menu or context menu
    - Verify it appears in file tree
    - Rename the file
    - Verify tree updates
    - Delete the file (should go to trash)
    - Ctrl+Z to undo delete
    - Verify file is restored
  pass_criteria:
    - Create works, file appears in tree
    - Rename updates tree and any open tab
    - Delete removes from tree
    - Undo restores the file
    - Undo stack works correctly

- id: e2e_14
  name: Git integration
  steps:
    - View git status icons in file tree
    - Open Git panel (GIT button)
    - Check status display
    - Modify a file
    - Verify git icon changes to "modified"
  pass_criteria:
    - Git status icons show for tracked files
    - Git panel shows branch, status
    - Modified files get modified icon
    - Directory-level aggregation works

- id: e2e_15
  name: Permissions
  steps:
    - Trigger a permission-requiring action
    - Verify permission prompt appears
    - Deny the permission
    - Verify action was blocked
    - Re-trigger, approve this time
    - Verify action executes
  pass_criteria:
    - Permission prompt renders correctly
    - Deny blocks the action
    - Approve allows the action
    - Decision recorded in session
    - Timeout auto-denies (if testable)

- id: e2e_16
  name: Agent memory
  steps:
    - Click MEMORY button in toolbar
    - View memory panel
    - Add a memory entry (if UI allows)
    - Search for an entry
    - List all entries
  pass_criteria:
    - Memory panel opens
    - Entries displayed (if any exist)
    - Search works
    - No errors

- id: e2e_17
  name: Settings full
  steps:
    - Open Settings panel
    - Change editor font size
    - Change terminal font size
    - Toggle analytics
    - Change model slot assignments
    - Close settings
    - Verify changes took effect
  pass_criteria:
    - Each setting changes without error
    - Editor reflects new font size
    - Terminal reflects new font size
    - Settings persist (check after re-opening settings)

- id: e2e_18
  name: Analytics
  steps:
    - Open Analytics panel
    - View API value banner
    - Check pace delta display
    - Look for sparkline visualization
    - Check model breakdown
  pass_criteria:
    - Analytics panel renders
    - Data displayed (or "no data" message if no sessions yet)
    - After running a chat session, analytics update

- id: e2e_19
  name: Search (Ctrl+P and grep)
  steps:
    - Press Ctrl+P
    - Type a filename (e.g. "tauri")
    - Select a result
    - Verify file opens
    - Use grep/search feature if available
    - Verify search results with anchors
  pass_criteria:
    - Ctrl+P opens search overlay
    - Results filter as you type
    - Selecting result opens the file
    - Search results anchor to correct positions

- id: e2e_20
  name: i18n
  steps:
    - Observe all UI text is in Italian
    - Check menu items, buttons, status bar, panels
    - Look for untranslated strings
  pass_criteria:
    - All major UI elements in Italian
    - No mixed language strings
    - Pluralization correct where applicable

- id: e2e_21
  name: Accessibility
  steps:
    - Navigate UI using only keyboard (Tab, Enter, Escape, arrows)
    - Check focus indicators are visible
    - Verify dialog focus trapping
    - Check ARIA attributes in key components
  pass_criteria:
    - All interactive elements reachable via keyboard
    - Focus indicators clearly visible
    - Dialogs trap focus correctly
    - Screen reader announcements work (check via ARIA live regions)

- id: e2e_22
  name: PTY resilience
  steps:
    - Open a terminal
    - Kill the PTY process externally (kill -9)
    - Observe reconnection behavior
    - Check reconnection overlay/message
  pass_criteria:
    - PTY loss detected
    - Reconnection attempted with backoff
    - UI shows reconnection status
    - Eventually reconnects or shows clear error

- id: e2e_23
  name: Circuit breaker
  steps:
    - Configure a broken/unreachable provider
    - Attempt to use it multiple times
    - Observe circuit breaker state transitions
  pass_criteria:
    - First failures show error
    - After threshold, circuit opens
    - Subsequent calls fail fast
    - Circuit eventually resets

- id: e2e_24
  name: CLI updater
  steps:
    - Check backend logs for CLI update messages
    - Verify update notifications appear (if updates available)
  pass_criteria:
    - Update check runs on startup
    - Version information logged
    - No crash from update check

- id: e2e_25
  name: State persistence full
  steps:
    - Open project, open 3 files, position cursors, resize panels
    - Open a terminal
    - Set specific layout (sidebar narrow, terminal wide)
    - Kill the app
    - Relaunch
    - Verify everything restored
  pass_criteria:
    - Project auto-opens
    - All 3 files restored in tabs
    - Cursor positions restored
    - Panel sizes restored
    - Terminal session info restored
```

### Suite 3: Stress Tests (26-33)

```yaml
- id: stress_26
  name: 50+ files open
  steps:
    - Programmatically open 50+ files via Ctrl+P
    - Switch between tabs
    - Monitor memory (RSS)
  pass_criteria:
    - All files open without crash
    - Tab switching responsive (<200ms)
    - Memory stays under 500MB
    - No leaked file handles

- id: stress_27
  name: Large file tree
  steps:
    - Open a project with 5000+ files (e.g. node_modules included)
    - Scroll through file tree rapidly
    - Expand/collapse directories
  pass_criteria:
    - Tree renders within 2 seconds
    - Scroll at 60fps (smooth)
    - No UI freeze
    - Virtualization kicks in

- id: stress_28
  name: Large file in editor
  steps:
    - Open a file >10MB (generate one if needed)
    - Scroll through it
    - Search within it
  pass_criteria:
    - File opens within 5 seconds
    - Scroll responsive
    - Editor doesn't freeze
    - Memory reasonable

- id: stress_29
  name: High-volume PTY output
  steps:
    - Run "find / 2>&1" or "seq 1 100000" in terminal
    - Observe output streaming
    - Check for dropped data
  pass_criteria:
    - Terminal doesn't freeze
    - Output streams smoothly
    - No kernel-level errors in logs
    - Can still interact with other parts of UI

- id: stress_30
  name: Multiple concurrent agent sessions
  requires_llm: true
  steps:
    - Start 3 chat sessions with different providers
    - Send prompts to all 3
    - Wait for all responses
  pass_criteria:
    - All 3 sessions work independently
    - No cross-talk between sessions
    - All responses complete
    - No errors

- id: stress_31
  name: Memory leak check
  steps:
    - Record initial RSS
    - Loop 100 times: open file, edit, close
    - Record final RSS
    - Compare
  pass_criteria:
    - RSS growth < 50MB over 100 cycles
    - No monotonic increase (levels off)
    - GC/cleanup works

- id: stress_32
  name: Rapid UI interaction
  steps:
    - Rapid-fire: switch tabs 50 times, toggle theme 10 times, resize panels 20 times
    - Do it fast (~100ms between actions)
  pass_criteria:
    - No crash
    - No UI corruption
    - App recovers to stable state
    - No zombie processes

- id: stress_33
  name: Startup benchmark
  steps:
    - Launch app 5 times, measure startup time each
    - Record: time to "setup complete", time to UI interactive
  pass_criteria:
    - Average startup < 5 seconds (dev mode)
    - No startup time regression >20% across runs
    - Consistent timing (low variance)
```

### Suite 4: Edge Cases (34-44)

```yaml
- id: edge_34
  name: Binary file in editor
  steps:
    - Open a binary file (PNG, compiled binary)
  pass_criteria:
    - Editor shows something reasonable (hex, "binary file" message, or prevents opening)
    - No crash

- id: edge_35
  name: Unicode and emoji
  steps:
    - Create file with unicode name (e.g. "test-emoji-file.ts")
    - Write content with CJK characters, emoji, RTL text
    - Save and reopen
  pass_criteria:
    - Filename renders correctly in tree
    - Content renders correctly in editor
    - Save/load preserves all characters

- id: edge_36
  name: Very long lines
  steps:
    - Open/create file with lines >10K characters
    - Scroll horizontally
    - Edit within long line
  pass_criteria:
    - Editor renders without freeze
    - Horizontal scroll works
    - Editing responsive

- id: edge_37
  name: Empty project
  steps:
    - Open an empty directory as project
  pass_criteria:
    - File tree shows empty state (not crash)
    - App remains functional
    - Can create files

- id: edge_38
  name: Deep directory nesting
  steps:
    - Navigate to a deeply nested directory (20+ levels)
  pass_criteria:
    - File tree renders all levels
    - Path doesn't overflow UI
    - Can open files at deep levels

- id: edge_39
  name: Symlinks in tree
  steps:
    - If symlinks exist in project, verify they display correctly
    - Navigate through symlinked directories
  pass_criteria:
    - Symlinks visible in tree
    - Navigation works
    - No infinite loops from circular symlinks

- id: edge_40
  name: External file modification
  steps:
    - Open a file in editor
    - Modify same file from terminal (echo "new line" >> file)
    - Check if editor detects change
  pass_criteria:
    - fs_watcher fires event
    - Editor updates or prompts to reload
    - No data loss

- id: edge_41
  name: Corrupt config files
  steps:
    - Backup llms.toml, write malformed TOML
    - Launch app
    - Restore backup
  pass_criteria:
    - App starts despite corrupt config
    - Error shown but not crash
    - Graceful degradation (no LLMs available)

- id: edge_42
  name: Network disconnect during chat
  requires_llm: true
  steps:
    - Start a chat session
    - Send a long prompt
    - Simulate network issue (if possible) or use unreachable endpoint
  pass_criteria:
    - Error handled gracefully
    - Circuit breaker may activate
    - UI shows clear error message
    - Can retry or start new session

- id: edge_43
  name: Permission denied files
  steps:
    - Try to open a file without read permission
    - Try to save to a read-only location
  pass_criteria:
    - Clear error message (not crash)
    - Editor handles gracefully

- id: edge_44
  name: Project without .git
  steps:
    - Open a directory that is not a git repository
  pass_criteria:
    - Git features degrade gracefully
    - No git status icons (correct)
    - Git panel shows "not a repository" or similar
    - All other features work
```

### Suite 5: Cross-Feature Interactions (45-52)

```yaml
- id: cross_45
  name: Edit triggers git status update
  steps:
    - Open a tracked file, modify it, save
    - Check git status icon changes in file tree
  pass_criteria:
    - Icon changes to "modified" within 2 seconds
    - Directory aggregation updates

- id: cross_46
  name: Delete file closes editor tab
  steps:
    - Open a file in editor
    - Delete the file via file operations
    - Check editor tab
  pass_criteria:
    - Editor tab closes or shows "file deleted" state
    - Undo restores file AND tab

- id: cross_47
  name: Rename updates editor tab
  steps:
    - Open a file, then rename it via file operations
  pass_criteria:
    - Editor tab name updates
    - Content preserved
    - File tree updates

- id: cross_48
  name: Settings changes apply live
  steps:
    - Open a file in editor, open terminal
    - Change font size in settings
    - Check editor and terminal
  pass_criteria:
    - Font size changes immediately in both
    - No restart needed

- id: cross_49
  name: Agent writes file, tree updates
  requires_llm: true
  steps:
    - Ask LLM agent to create a file
    - Check file tree for new file
    - Open it in editor
  pass_criteria:
    - File appears in tree after creation
    - Content is what the agent wrote
    - fs_watcher detected the change

- id: cross_50
  name: Workflow drives terminal
  steps:
    - Create workflow with a node that runs a shell command
    - Execute workflow
    - Check terminal for output
  pass_criteria:
    - Node executes command
    - Terminal shows output
    - Workflow state reflects completion

- id: cross_51
  name: Theme consistency across all components
  steps:
    - Switch to dark theme
    - Visit every panel (editor, terminal, settings, analytics, memory, HIVE, git)
    - Take screenshot of each
    - Switch to light theme
    - Repeat
  pass_criteria:
    - No component has mismatched colors
    - Text readable in both themes
    - Icons visible in both themes
    - No "unstyled" flashes

- id: cross_52
  name: Multi-provider normalizer
  requires_llm: true
  providers: [claude, qwen]
  steps:
    - Send same prompt to Claude and Qwen
    - Compare response rendering
  pass_criteria:
    - Both responses parse and render correctly
    - Event types (text, thinking, tool_use) displayed appropriately
    - No normalizer errors in logs
```

### Suite 6: Security (53-57)

```yaml
- id: security_53
  name: Workspace trust
  steps:
    - Open an untrusted project directory
    - Attempt to use tools that should be restricted
  pass_criteria:
    - Trust prompt appears
    - Restricted tools blocked in untrusted mode
    - Trust can be granted
    - After trust, tools work

- id: security_54
  name: Permission deny
  steps:
    - Trigger a tool that requires permission
    - Deny the permission
  pass_criteria:
    - Permission prompt appears with clear description
    - Deny blocks the tool
    - No workaround possible
    - Audit event emitted (check logs)

- id: security_55
  name: Policy file regex
  steps:
    - Add a deny pattern to permissions.toml
    - Trigger the matching tool
  pass_criteria:
    - Pattern matched
    - Tool denied
    - Policy file respected

- id: security_56
  name: Symlink escape
  steps:
    - Create symlink pointing outside project
    - Attempt to access it
  pass_criteria:
    - Access blocked or warning shown
    - Cannot read files outside project bounds

- id: security_57
  name: Prompt injection resistance
  steps:
    - Send a prompt containing injection attempts (e.g. "ignore previous instructions")
    - Check that the agent doesn't execute unintended actions
  pass_criteria:
    - Agent responds normally
    - No unintended tool execution
    - No sensitive data leaked
```

### Suite 7: Visual Regression (58-62)

```yaml
- id: visual_58
  name: Baseline screenshots
  steps:
    - Capture reference screenshots of every major view:
      welcome screen, editor with file, terminal, settings,
      analytics, memory, HIVE, git panel, file tree, search overlay
  pass_criteria:
    - All screenshots captured and saved to baseline/
    - Used as reference for future regression checks

- id: visual_59
  name: Dark theme completeness
  steps:
    - Set dark theme
    - Visit every component and panel
    - Screenshot each
  pass_criteria:
    - All components have dark backgrounds
    - Text is light and readable
    - No white/bright rectangles from unstyled components
    - Icons and borders appropriate for dark theme

- id: visual_60
  name: Light theme completeness
  steps:
    - Set light theme
    - Visit every component and panel
    - Screenshot each
  pass_criteria:
    - All components have light backgrounds
    - Text is dark and readable
    - No dark rectangles from unstyled components
    - Consistent light palette

- id: visual_61
  name: Responsive layout
  steps:
    - Resize window to 1024x600 (small)
    - Screenshot
    - Resize to 1920x1080 (large)
    - Screenshot
    - Resize to 800x400 (tiny)
    - Screenshot
  pass_criteria:
    - Layout adapts at each size
    - No overlapping elements
    - No hidden critical UI elements
    - Minimum usable size handled gracefully

- id: visual_62
  name: Error state rendering
  steps:
    - Trigger each known error state:
      file not found, network error, permission denied, corrupt config
    - Screenshot each error display
  pass_criteria:
    - Errors displayed clearly and readably
    - Error messages helpful (not "[object Object]")
    - Dismiss/retry actions available
    - No cascading UI corruption
```

### Suite 8: Data Integrity (63-67)

```yaml
- id: integrity_63
  name: Kill during save
  steps:
    - Start editing a file
    - Begin save (Ctrl+S)
    - Immediately SIGKILL the app
    - Relaunch
    - Check file state
  pass_criteria:
    - File is either fully saved or fully original (no partial write)
    - App recovers on next launch
    - No corrupt state files

- id: integrity_64
  name: Transaction semantics
  steps:
    - Perform an action that writes session + event log atomically
    - Kill mid-operation
    - Verify consistency
  pass_criteria:
    - Both records present or both absent
    - No orphaned records

- id: integrity_65
  name: Full state restore
  steps:
    - Set up complex state: 5 files open with specific cursors,
      custom panel layout, specific theme, active terminal
    - Kill and relaunch
    - Verify every detail
  pass_criteria:
    - File tabs restored in order
    - Cursor positions correct (line + column)
    - Scroll positions approximately correct
    - Panel widths restored
    - Theme persisted

- id: integrity_66
  name: Config migration
  steps:
    - Modify settings format to simulate old version
    - Launch app
  pass_criteria:
    - Old config loaded without crash
    - Migrated to new format
    - No data loss

- id: integrity_67
  name: Concurrent writes
  steps:
    - Open same file in two ways (editor + external edit)
    - Both modify the file
    - Check for data corruption
  pass_criteria:
    - Conflict detected
    - User prompted or last-write-wins with notification
    - No silent data loss
```

### Fuzzing Mode

```yaml
- id: fuzz_01
  name: Free exploration
  duration_minutes: 30
  steps:
    - Navigate app freely without script
    - Click random UI elements
    - Try unusual combinations
    - Open/close/resize/toggle rapidly
    - Try invalid inputs in all text fields
    - Document anything unexpected
  pass_criteria:
    - No crashes during entire session
    - No data corruption
    - All errors handled gracefully
    - Report lists all findings with screenshots
```

---

## Bug Report Format

When a test fails, auto-generate:

```markdown
# BUG-NNN: [Short description]

**Severity:** Critical / High / Medium / Low
**Found by:** Test [id] — [name]
**Date:** YYYY-MM-DD

## Steps to Reproduce
1. ...
2. ...

## Expected Behavior
...

## Actual Behavior
...

## Screenshots
![before](screenshots/before.png)
![after](screenshots/after.png)

## Relevant Logs
```
[timestamp] ERROR ...
```

## Probable Cause
File: `src/path/to/file.rs:123`
Reason: ...

## Suggested Fix
...
```

---

## Runner CLI

```bash
# Run everything
python tests/field/runner.py --all

# Run specific suite
python tests/field/runner.py --suite smoke
python tests/field/runner.py --suite e2e

# Run single test
python tests/field/runner.py --test smoke_01

# Update visual baselines
python tests/field/runner.py --suite visual --update-baseline

# Fuzzing mode
python tests/field/runner.py --fuzz --duration 30

# Generate report only (from existing run data)
python tests/field/runner.py --report runs/2026-04-01-120000
```

---

## Known Issues to Verify First

These bugs were found during the initial field test and should be verified/fixed before the full suite runs:

1. **FIXED (49ffae4):** `tokio::runtime::Handle::current()` panic on Linux — verify fix works
2. **OPEN:** `Unhandled rejection: [object Object]` on project load — likely Zod schema mismatch
3. **OPEN:** File tree empty after opening project — linked to bug #2
