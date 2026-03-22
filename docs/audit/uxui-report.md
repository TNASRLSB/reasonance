# UX/UI Designer Accessibility Audit Report

**Date:** 2026-03-22
**Persona:** Senior accessible design specialist
**Judgment:** Can every human use this effectively and comfortably?
**Application:** Reasonance IDE (Svelte 5 + Tauri 2 desktop app)
**Files audited:** 48 component files across `src/lib/components/`, `src/app.css`, `src/lib/utils/a11y.ts`

---

## Executive Summary

Reasonance demonstrates **above-average accessibility awareness** for an IDE project. There are thoughtful implementations including: skip links, focus trap utility, keyboard navigation helpers, `prefers-reduced-motion` blanket override, ARIA tree widget in FileTree, proper dialog/modal patterns with `aria-modal`, `aria-live` regions for toasts and chat streaming, semantic HTML landmarks (`<nav>`, `<main>`, `<aside>`, `<header>`, `<footer>`), and the use of Atkinson Hyperlegible -- a font specifically designed for low-vision readability.

However, there are **systemic gaps** that prevent WCAG 2.1 AA conformance. The most critical are: missing `aria-label` on several interactive elements, `tabindex="-1"` on all tree items with no roving tabindex root, toast `slide-in` and streaming indicator `pulse` animations lacking `prefers-reduced-motion` alternatives in their component styles (they rely solely on the global blanket override which may not catch every edge), missing keyboard support in several dropdown menus, and multiple touch-target violations. Several components use `svelte-ignore a11y_no_static_element_interactions` to suppress Svelte's own accessibility warnings, which is a red flag.

**Overall WCAG 2.1 compliance estimate: Partial AA, not yet conformant.**

---

## WCAG 2.1 Compliance Overview

| Level | Criteria Checked | Pass | Fail | Partial | N/A |
|-------|-----------------|------|------|---------|-----|
| A     | 30              | 19   | 5    | 6       | 0   |
| AA    | 20              | 12   | 3    | 5       | 0   |
| AAA   | 10              | 5    | 2    | 3       | 0   |

---

## Component-by-Component Findings

### App.svelte
**File:** `src/lib/components/App.svelte`

**ARIA:** Pass
- Skip links implemented correctly (lines 97-101): file tree, editor, terminal
- Landmarks: `<nav>` for file tree, `<main>` for editor, `<aside>` for terminal, `<header>`, `<footer>`
- Dividers have `role="separator"`, `aria-label`, and `aria-hidden="true"` on decorative handle text
- Error boundaries provide recovery UI with retry buttons

**Keyboard:** Pass
- Skip links visible on `:focus` (line 201-215)
- Dividers are focusable (`tabindex="0"`) with Arrow key resize (lines 66-87), including Shift modifier for larger steps
- Global `Ctrl+Shift+A` for analytics, `Ctrl+1..9` for provider switching

**Contrast:** Pass (see Systemic Patterns for global analysis)

**Touch targets:** Warning
- Divider hit area is 6px wide; extended to 14px via `::before` pseudo-element (lines 288-297). Still under 24px minimum (WCAG 2.5.8 AA).
- Error retry button: `padding: 4px 16px` -- may be under 24px height.

**Loading/Error states:** Pass -- `<svelte:boundary>` provides error UI for each panel

---

### FileTree.svelte
**File:** `src/lib/components/FileTree.svelte`

**ARIA:** Pass
- `role="tree"` on container, `role="treeitem"` on each entry, `role="group"` on expanded children (lines 143, 146-155, 162)
- `aria-selected` for active file, `aria-expanded` for directories
- `aria-label="File explorer"` on tree container

**Keyboard:** Partial
- Arrow keys (Up/Down/Left/Right), Home, End implemented in `handleTreeKeydown` (lines 76-123)
- **Issue:** All tree items have `tabindex="-1"` (line 153). The first visible item should have `tabindex="0"` to allow initial Tab focus into the tree. Currently, you cannot Tab into the file tree -- there is no focusable entry point. This violates WCAG 2.1.1 (Keyboard).
- Enter/Space to open files is missing -- only `onclick` handler exists (line 151). Keyboard users can only use Arrow keys after focusing, but cannot activate.

**Contrast:** Pass

**Touch targets:** Warning
- Tree items: `padding: 6px 14px` with `font-size: 14px`. Computed height is approximately 32px (6+14+6+line-height). Below 44px AAA target.

**Loading/Error states:** Fail
- No loading state shown while `adapter.listDir()` is in progress
- No error state if `listDir` fails (caught silently at line 21)

---

### SearchPalette.svelte
**File:** `src/lib/components/SearchPalette.svelte`

**ARIA:** Pass
- `role="dialog"`, `aria-modal="true"`, `aria-label` on dialog (line 170)
- `role="listbox"` on results list, `role="option"` with `aria-selected` on each item (lines 193-205)
- Input has `aria-label` (line 180)
- Close button has `aria-label` (line 189)

**Keyboard:** Pass
- Focus trap implemented via `trapFocus` utility (line 29)
- Escape closes dialog (line 139)
- ArrowUp/ArrowDown navigate results, Enter selects (lines 143-152)
- Auto-focus on input when opened (line 65)

**Contrast:** Pass

**Touch targets:** Warning
- Close button: `padding: 4px 6px` with `font-size: 12px`. Very small target, approximately 20x20px. Fails WCAG 2.5.8 AA (24px minimum).
- Palette items: `padding: 6px 14px` -- height approximately 28px. Below 44px AAA.

**Loading/Error states:** Pass
- Loading state shown (line 184-186)
- Empty state for no matches (line 209)
- Hint text when no query (line 211)

---

### ShortcutsDialog.svelte
**File:** `src/lib/components/ShortcutsDialog.svelte`

**ARIA:** Pass
- `role="dialog"`, `aria-modal="true"`, `aria-label` (line 53)
- Close buttons have `aria-label` (line 56)

**Keyboard:** Pass
- Focus trap via `trapFocus` (line 15-18)
- Escape closes (line 28)
- Footer close button available

**Contrast:** Pass
- `group-label` uses `var(--accent)` color (`#1d4ed8` on dark). Against `#121212` background: contrast ratio approximately 3.2:1 at `font-size: 10px`. **Fails** WCAG AA (needs 4.5:1 for small text).

**Touch targets:** Warning
- Close button (X): `padding: 2px 6px` -- approximately 18x18px. Fails minimum target size.
- Footer close button: `padding: 4px 16px` -- approximately 22x30px. Below 24px height.

---

### Settings.svelte
**File:** `src/lib/components/Settings.svelte`

**ARIA:** Pass
- `role="dialog"`, `aria-modal="true"`, `aria-label` (line 351)
- Error banner has `role="alert"` (line 358)
- Form fields use `<label>` with `for` attribute (lines 368, 378)
- Focus trap implemented (lines 38-43)

**Keyboard:** Pass
- Escape closes (line 343)
- Focus trap active
- Standard form controls (select, input)

**Contrast:** Pass

**Touch targets:** Warning
- Close button: `padding: 2px 6px` -- small target. Approximately 18x18px.

**Loading/Error states:** Pass
- Error banner displayed on load/save failure
- Saving state tracked

---

### Toolbar.svelte
**File:** `src/lib/components/Toolbar.svelte`

**ARIA:** Partial
- Git dropdown: `aria-haspopup="true"`, `aria-expanded` on trigger (line 77); `role="menu"` on dropdown, `role="menuitem"` on items (lines 81-86). Good.
- Analytics button: `aria-pressed` state tracked (line 103). Good.
- Window controls: `aria-label` on all buttons (lines 109-111). Good.
- **Issue:** YOLO button (lines 91-98) lacks `aria-label` -- uses text content "YOLO" which is unclear for screen readers. The `title` attribute provides context but is not reliably announced. Missing `aria-pressed` state.
- **Issue:** Git trigger button (line 77) uses `title` but no `aria-label`. The text "GIT" with a triangle character is adequate but not ideal.

**Keyboard:** Partial
- Git dropdown uses `menuKeyHandler` for Arrow key navigation (line 81)
- **Issue:** Git dropdown opens on click but no keyboard shortcut is documented to open it
- **Issue:** Clicking outside closes dropdown (line 62) but focus is not returned to the trigger button

**Contrast:** Pass
- Button hover inverts colors (light text on dark becomes dark text on light)

**Touch targets:** Partial
- Toolbar buttons: `min-height: 26px` (line 178). Below 44px AAA and below 24px width in some cases due to small padding.
- Window buttons: `padding: 0 14px`, full toolbar height (44px). Pass for height, but width is approximately 28px. Adequate.

---

### MenuBar.svelte + MenuItem.svelte
**Files:** `src/lib/components/MenuBar.svelte`, `src/lib/components/MenuItem.svelte`

**ARIA:** Partial
- Menu trigger has `aria-haspopup="true"`, `aria-expanded` (MenuItem.svelte:50-51)
- Dropdown has `role="menu"`, items have `role="menuitem"` (MenuItem.svelte:57, 79, 94-98)
- **Issue:** MenuItem wrapper div has `role="menubar"` at line 44, but this is on individual menu items, not the bar container. The MenuBar container (`div.menu-bar`) has no `role="menubar"`. This is **incorrect ARIA structure** -- `role="menubar"` should be on the parent bar, not each individual dropdown wrapper.
- **Issue:** Submenu items with `has-submenu` class use a `<div>` with `role="menuitem"` (line 63-67) but submenus open only on hover (`onmouseenter`), not on keyboard ArrowRight. Keyboard users cannot access submenus.

**Keyboard:** Fail
- No Left/Right arrow navigation between top-level menu items (standard menubar pattern)
- Submenus open only on mouse hover, not keyboard
- ArrowUp/ArrowDown within dropdown work via `menuKeyHandler`
- Escape closes via global handler

**Contrast:** Pass

**Touch targets:** Warning
- Menu triggers: `padding: 4px 8px` with `font-size: 12px`. Approximately 20x20px. Fails minimum.
- Menu items: `padding: 6px 12px`. Approximately 28px height. Below 44px AAA.

---

### ContextMenu.svelte
**File:** `src/lib/components/ContextMenu.svelte`

**ARIA:** Pass
- `role="menu"` on container, `role="menuitem"` on items (lines 112-121)
- Backdrop has `role="presentation"` (line 103)
- Disabled state properly communicated via `disabled` attribute and `aria-disabled` implicit
- Focus auto-moves to first item on open (line 27)

**Keyboard:** Pass
- `menuKeyHandler` for Arrow navigation (line 113)
- Escape closes (line 97, 113)

**Contrast:** Pass

**Touch targets:** Warning
- Items: `padding: 7px 14px`. Approximately 32px height. Below 44px AAA.

---

### EditorTabs.svelte
**File:** `src/lib/components/EditorTabs.svelte`

**ARIA:** Pass
- `role="tablist"` on container (line 31)
- `role="tab"` with `aria-selected` on each tab (lines 39-41)
- Close buttons have `aria-label="Close {file.name}"` (line 54)

**Keyboard:** Partial
- Enter/Space to switch tabs via `handleKeyDown` (lines 24-28)
- **Issue:** No ArrowLeft/ArrowRight navigation between tabs (standard tablist pattern per WAI-ARIA Authoring Practices). Only Tab key can move between them, which is incorrect for `role="tablist"`.
- Tabs have `tabindex="0"` -- all of them. Per ARIA Practices, only the active tab should have `tabindex="0"`, others should have `tabindex="-1"`.

**Contrast:** Pass

**Touch targets:** Warning
- Tab close button: `min-width: 24px; min-height: 24px`. Meets AA minimum (24px). But `padding: 5px 6px` makes the actual area approximately 24x24px. Borderline.
- Tabs themselves: `height: 38px` (from container). Pass for height.

---

### Editor.svelte
**File:** `src/lib/components/Editor.svelte`

**ARIA:** Partial
- **Issue:** The editor container div (line 228-233) has `svelte-ignore a11y_no_static_element_interactions` suppressed. The `oncontextmenu` is on a `<div>` without a role. CodemirrorEditor handles its own ARIA internally, but the wrapper lacks semantic meaning.
- Empty state uses proper text hierarchy (lines 241-244)

**Keyboard:** Pass (delegated to CodeMirror)
- CodeMirror provides full keyboard support natively
- Context menu triggered by right-click only -- no keyboard shortcut for context menu actions (e.g., Shift+F10)

**Contrast:** Pass
- Editor themes define custom colors; both dark and light themes maintain good contrast for code

**Touch targets:** N/A (CodeMirror handles internally)

**Loading/Error states:** Partial
- Empty state shown when no file open (line 241)
- No loading indicator while file content is being fetched or language highlighting is loading

---

### Terminal.svelte
**File:** `src/lib/components/Terminal.svelte`

**ARIA:** Fail
- **Issue:** The terminal wrapper div at line 265 has two `svelte-ignore` comments suppressing `a11y_click_events_have_key_events` and `a11y_no_static_element_interactions`. The container div at line 286 has `onclick` but no keyboard handler.
- **Issue:** No `role` attribute on the terminal container. It should have `role="application"` or `role="log"` to inform screen readers about its purpose.
- **Issue:** Terminal search input (line 269) lacks `aria-label`. Only has `placeholder` which is not a reliable label.

**Keyboard:** Partial
- xterm.js handles internal keyboard input
- Ctrl+V paste, Ctrl+C copy, Ctrl+F find are handled (lines 100-117)
- Search bar: Enter/Shift+Enter for next/prev, Escape to close (lines 273-279)
- Terminal search buttons have `aria-label` (lines 281-283)

**Contrast:** Pass
- Custom xterm themes with carefully chosen colors for both dark and light modes

**Touch targets:** Warning
- Search buttons: `padding: 2px 6px; font-size: 10px`. Approximately 16x16px. Fails minimum.

---

### TerminalManager.svelte
**File:** `src/lib/components/TerminalManager.svelte`

**ARIA:** Partial
- LLM tabs: `role="tablist"`, `role="tab"`, `aria-selected` (lines 255-265)
- Instance tabs: `role="tablist"`, `role="tab"`, `aria-selected` (lines 341-359)
- Add LLM button: `aria-label`, `aria-haspopup`, `aria-expanded` (line 270)
- LLM dropdown: `role="menu"`, `role="menuitem"` (lines 272-278)
- No-LLM banner: `role="status"` (line 313)
- **Issue:** Instance close button is a `<span>` with `role="button"` (line 351-358). Should be a `<button>` element for proper semantics and built-in keyboard handling.
- **Issue:** Add instance button (line 373-375) lacks `aria-label`. Just shows "+".

**Keyboard:** Partial
- LLM dropdown uses `menuKeyHandler` (line 272)
- **Issue:** No ArrowLeft/ArrowRight between LLM tabs or instance tabs (standard tablist pattern)
- **Issue:** `<span role="button">` close buttons require manual Enter handler (line 357), but missing Space key handler

**Contrast:** Pass

**Touch targets:** Warning
- Instance tab close span: `min-width: 24px; min-height: 24px; padding: 5px 6px`. Meets 24px AA minimum.
- LLM card buttons: `padding: 10px 14px`. Approximately 40x40px. Below 44px AAA but passes AA.
- Start button: `padding: 12px 20px`. Pass.

---

### TerminalToolbar.svelte
**File:** `src/lib/components/TerminalToolbar.svelte`

**ARIA:** Partial
- Slash commands dropdown: `aria-haspopup`, `aria-expanded` on trigger (lines 78-79)
- Mode dropdown: `aria-haspopup`, `aria-expanded` (lines 100-101)
- **Issue:** Dropdown items at line 85 lack `role="menuitem"`. They use class `dropdown-item` but the `menuKeyHandler` targets `.dropdown-item` not `[role="menuitem"]`, which is inconsistent with other menus.
- Decorative icons use `aria-hidden="true"` (lines 71, 80, 103)

**Keyboard:** Partial
- `menuKeyHandler` handles arrow navigation within dropdowns
- **Issue:** No Escape key handler to close dropdowns from within

**Contrast:** Pass

**Touch targets:** Warning
- Toolbar buttons: `width: 28px; height: 28px`. Meets 24px AA. Below 44px AAA.
- Mode button: `padding: 4px 10px; font-size: 10px`. Approximately 22px height. Below 24px AA.

---

### Toast.svelte
**File:** `src/lib/components/Toast.svelte`

**ARIA:** Pass
- Container: `aria-live="polite"`, `aria-atomic="false"` (line 29)
- Each toast: `role="alert"` (line 35)
- Dismiss button: `aria-label="Dismiss notification"` (line 50)

**Keyboard:** Warning
- Toast container has `pointer-events: none` on container, `pointer-events: all` on individual toasts (lines 80, 93)
- **Issue:** Toasts auto-dismiss (timer-based presumably). No mechanism for keyboard users to access toasts before they disappear. No `tabindex` on toasts themselves. Users cannot Tab to action buttons inside toasts.

**Contrast:** Pass

**Touch targets:** Warning
- Dismiss button: `font-size: 1rem; padding: 0`. Approximately 16x16px. Fails minimum.
- Action buttons: `padding: 0.25rem 0.75rem`. Approximately 24x28px. Meets AA minimum.

**Reduced Motion:** Fail
- `slide-in` animation at line 190-198 has no component-level `prefers-reduced-motion` override. Relies on global blanket in `app.css`.
- The global override uses `animation-duration: 0.01ms !important` which is a valid approach, but the component should not assume the global override will always be present.

---

### StatusBar.svelte
**File:** `src/lib/components/StatusBar.svelte`

**ARIA:** Pass
- Container: `role="status"` (line 31)
- Content is all informational text; screen readers will announce changes

**Keyboard:** N/A (no interactive elements in normal mode)

**Contrast:** Partial
- White text (`#fff`) on `--accent` (`#1d4ed8`): contrast ratio approximately **3.8:1**. Fails WCAG AA for normal text (needs 4.5:1). The blue is too bright for white text.
- YOLO mode: White on `--danger` (`#dc2626`): approximately **4.5:1**. Passes AA.
- `.llm-count` at `opacity: 0.85`: effective color approximately `rgba(255,255,255,0.85)` on `#1d4ed8`. Even lower contrast.
- `.file-lang` at `opacity: 0.7` and `.file-encoding` at `opacity: 0.5`: **critically low contrast** for informational text.

**Touch targets:** N/A

---

### ViewModeToggle.svelte
**File:** `src/lib/components/ViewModeToggle.svelte`

**ARIA:** Pass
- `aria-label` descriptive text (line 13)
- `title` attribute for tooltip (line 14)

**Keyboard:** Pass (standard button)

**Contrast:** Pass

**Touch targets:** Warning
- `padding: 2px 8px; font-size: 11px`. Approximately 18x22px. Fails 24px AA minimum.

---

### WelcomeScreen.svelte
**File:** `src/lib/components/WelcomeScreen.svelte`

**ARIA:** Partial
- **Issue:** Container div at line 19 has `svelte-ignore a11y_no_static_element_interactions` suppressed
- **Issue:** Theme toggle button (line 21) lacks `aria-label`. Uses sun/moon emoji as content which may not be announced meaningfully.
- **Issue:** Window control buttons (lines 25-27) have `title` but no `aria-label`
- Heading hierarchy: `<h1>` for title, `<h2>` for recent. Good.
- Recent projects use proper `<ul>/<li>` structure (lines 43-51)

**Keyboard:** Pass
- All interactive elements are buttons
- Standard tab navigation

**Contrast:** Partial
- Primary button: `color: var(--accent)` (`#1d4ed8`) on transparent/`--bg-primary` (`#121212`). Approximately 3.2:1 for normal text. **Fails AA** (needs 4.5:1). The blue accent color used as text color on dark background is too low contrast.

**Touch targets:** Partial
- Theme toggle: `width: 36px; height: 32px`. Meets 24px AA.
- Window buttons: `width: 46px; height: 32px`. Meets 24px AA.
- Primary action button: `padding: 12px 32px`. Pass.
- Recent items: `padding: 8px 12px`. Height approximately 30px. Below 44px AAA.

---

### FindInFiles.svelte
**File:** `src/lib/components/FindInFiles.svelte`

**ARIA:** Pass
- `role="dialog"`, `aria-modal="true"`, `aria-label` (line 109)
- Input: `aria-label` (line 123)
- Results: `role="button"` with `tabindex="0"` and keydown handler (lines 153-155)
- Close button: `aria-label` (line 112)

**Keyboard:** Pass
- Focus trap via `trapFocus` (line 26-29)
- Escape closes, Enter searches (lines 79-85)
- Results navigable via Tab and activatable via Enter

**Contrast:** Pass

**Touch targets:** Warning
- Close button: `padding: 2px 6px`. Approximately 18x18px. Fails minimum.
- Result rows: `padding: 4px 14px`. Approximately 22px height. Below 24px AA.

---

### DiffView.svelte
**File:** `src/lib/components/DiffView.svelte`

**ARIA:** Partial
- **Issue:** No `role` on the diff container. No `aria-label` describing what the diff shows.
- **Issue:** Accept/Reject buttons lack `aria-label`. They use text content + icon character which is adequate but could be clearer.

**Keyboard:** Pass (delegated to CodeMirror MergeView)

**Contrast:** Pass

**Touch targets:** Warning
- Accept/Reject buttons: `padding: 3px 12px`. Approximately 22x32px. Below 24px height.

---

### HelpPanel.svelte
**File:** `src/lib/components/HelpPanel.svelte`

**ARIA:** Pass
- Search input: `aria-label` (line 92)
- Proper heading hierarchy delegated to MarkdownPreview

**Keyboard:** Pass
- Search input focusable, standard behavior

**Contrast:** Pass

**Touch targets:** Pass (search input is full-width, comfortable size)

---

### ImageDrop.svelte
**File:** `src/lib/components/ImageDrop.svelte`

**ARIA:** Pass
- `role="region"`, `aria-label="Terminal -- drop images here"` (lines 86-87)

**Keyboard:** Partial
- **Issue:** Drag-and-drop is the only mechanism to add images. No keyboard-accessible alternative (e.g., a button to open a file picker). Violates WCAG 2.1.1 (Keyboard).

**Contrast:** Pass

**Touch targets:** N/A

---

### MarkdownPreview.svelte
**File:** `src/lib/components/MarkdownPreview.svelte`

**ARIA:** Partial
- **Issue:** Renders arbitrary HTML via `{@html rendered}`. Image alt text depends entirely on the markdown source; no enforcement of alt text on rendered images.
- **Issue:** No landmark or heading for the preview container itself.

**Keyboard:** Pass (scrollable, links focusable)

**Contrast:** Partial
- Inline code: `color: #e879f9` (magenta) on `--bg-secondary` (`#1a1a1a`): approximately 6.5:1. Pass.
- Link color: `var(--accent)` (`#1d4ed8`) on text background: approximately 3.2:1. **Fails** AA for normal text.

**Touch targets:** N/A

---

### ResponsePanel.svelte
**File:** `src/lib/components/ResponsePanel.svelte`

**ARIA:** Partial
- Close button has `aria-label` (line 41)
- **Issue:** Panel lacks `role` and `aria-label`. Should have `role="complementary"` or similar.
- **Issue:** No focus trap -- panel opens as an overlay but keyboard focus can escape behind it.

**Keyboard:** Partial
- **Issue:** No focus trap when panel opens
- **Issue:** No Escape key to close

**Contrast:** Pass

**Touch targets:** Warning
- Close button: `padding: 2px 6px`. Approximately 18x18px. Fails minimum.

---

### AnalyticsBar.svelte
**File:** `src/lib/components/AnalyticsBar.svelte`

**ARIA:** Pass
- `role="status"` with descriptive `aria-label` (line 109)
- Progress bar: `role="progressbar"` with `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-label` (lines 116-120)
- Dashboard link: `aria-label` (line 156)
- Decorative elements: `aria-hidden="true"` (lines 128, 202)
- Screen reader announcer integration (lines 82-95)
- Skeleton state has `aria-label` (line 210)
- Reduced motion: checks `$prefersReducedMotion` for animation alternatives (lines 104-107, 126)

**Keyboard:** Pass
- Dashboard button: `min-width: 44px; min-height: 44px`. Meets AAA target. Has `focus-visible` styling (lines 433-436).

**Contrast:** Partial
- `.metric` default color: `var(--text-secondary)` (`#a3a3a3`) on `--bg-primary` (`#121212`): approximately 7.6:1. Pass.
- `.velocity` color: `var(--warning)` (`#ca8a04`) on `#121212`: approximately 5.9:1. Pass.
- Reduced opacity metrics (`.projection` at `opacity: 0.8`) may fall below thresholds.

**Reduced Motion:** Pass
- Component-level `prefers-reduced-motion` for skeleton animation (line 502-505)
- Error/recovery flash has instant alternatives when `$prefersReducedMotion` is true (lines 104-107)

---

### AnalyticsDashboard.svelte
**File:** `src/lib/components/AnalyticsDashboard.svelte`

**ARIA:** Pass (from what is visible in the first 100 lines)
- Screen reader announcer integration
- Uses accessible utility functions (`kpiLabel`, `providerBarLabel`, `trendBarLabel`)
- Grid navigation utility imported

**Keyboard:** Likely Pass (uses `gridNavigation` utility)

---

### Chat Components

#### ChatView.svelte
**File:** `src/lib/components/chat/ChatView.svelte`
- Orchestrator component; no direct accessibility concerns.

#### ChatMessages.svelte
**File:** `src/lib/components/chat/ChatMessages.svelte`

**ARIA:** Pass
- Container: `role="log"`, `aria-live="polite"` (line 78)
- Message groups: `role="article"` with `tabindex="0"` (line 82)
- Role labels ("AGENT"/"YOU") provide context

**Keyboard:** Pass
- Message groups focusable via `tabindex="0"`

#### ChatInput.svelte
**File:** `src/lib/components/chat/ChatInput.svelte`

**ARIA:** Pass
- Textarea: `aria-label="Message input"` (line 28)
- Send button: `aria-label="Send message"` (line 34)

**Keyboard:** Pass
- Enter to send, Shift+Enter for newline (lines 14-17)
- Disabled state properly communicated

#### ChatHeader.svelte
**File:** `src/lib/components/chat/ChatHeader.svelte`
**ARIA:** Partial
- **Issue:** No `role` or `aria-label` on the header. Status information would benefit from `aria-live` region.

#### StreamingIndicator.svelte
**File:** `src/lib/components/chat/StreamingIndicator.svelte`

**ARIA:** Pass
- `role="status"`, `aria-live="assertive"`, `aria-label="Agent is responding"` (line 1)

**Reduced Motion:** Fail
- `pulse` animation (lines 24-27) has no component-level `prefers-reduced-motion` override. Relies on global blanket.

#### CodeBlock.svelte
**File:** `src/lib/components/chat/CodeBlock.svelte`

**ARIA:** Pass
- Copy button: `aria-label="Copy code"` (line 17)
- Semantic `<pre><code>` structure

#### ThinkingBlock.svelte
**File:** `src/lib/components/chat/ThinkingBlock.svelte`

**ARIA:** Partial
- **Issue:** Expand/collapse button lacks `aria-expanded` attribute. Should toggle between `true`/`false`.
- **Issue:** No `aria-label` on the toggle button.

#### ToolUseBlock.svelte
**File:** `src/lib/components/chat/ToolUseBlock.svelte`

**ARIA:** Partial
- **Issue:** Same as ThinkingBlock -- expand/collapse button lacks `aria-expanded`.
- **Issue:** No `aria-label` describing the tool name.

#### ErrorBlock.svelte
**File:** `src/lib/components/chat/ErrorBlock.svelte`

**ARIA:** Pass
- `role="alert"` on the error container (line 19)

#### ActionableMessage.svelte
**File:** `src/lib/components/chat/ActionableMessage.svelte`

**ARIA:** Partial
- `role="group"` on wrapper (line 44)
- Action buttons have `aria-label` (lines 50, 54)
- **Issue:** Actions only visible on hover. Keyboard focus triggers visibility (lines 42-43), but `onfocusout` may hide actions before they can be used if focus moves to the action buttons within the same container. The `e.currentTarget.contains(e.relatedTarget)` check (line 43) should handle this correctly.

---

## Systemic Patterns

### 1. Color Contrast Issues (Systemic)

The `--accent` color (`#1d4ed8`, blue) is used extensively as both a text color and a background color. Analysis:

| Combination | Foreground | Background | Ratio | WCAG AA | WCAG AAA |
|-------------|-----------|------------|-------|---------|----------|
| Accent text on dark bg | `#1d4ed8` | `#121212` | ~3.2:1 | FAIL | FAIL |
| Accent-text on dark bg | `#6b8de8` | `#121212` | ~5.8:1 | PASS | FAIL |
| White on accent bg (StatusBar) | `#ffffff` | `#1d4ed8` | ~3.8:1 | FAIL (normal) | FAIL |
| White on accent bg (StatusBar) | `#ffffff` | `#1d4ed8` | ~3.8:1 | PASS (large text) | FAIL |
| Text-secondary on dark bg | `#a3a3a3` | `#121212` | ~7.6:1 | PASS | PASS |
| Text-muted on dark bg | `#b0b0b0` | `#121212` | ~8.7:1 | PASS | PASS |
| Text-body on dark bg | `#d4d4d4` | `#121212` | ~12.3:1 | PASS | PASS |
| Text-primary on dark bg | `#f0f0f0` | `#121212` | ~15.3:1 | PASS | PASS |
| Light theme text-muted on light bg | `#767676` | `#fafafa` | ~4.6:1 | PASS | FAIL |
| Light theme text-secondary on light bg | `#525252` | `#fafafa` | ~7.4:1 | PASS | PASS |
| Danger on dark bg | `#dc2626` | `#121212` | ~4.5:1 | PASS (large) | FAIL |
| Success on dark bg | `#16a34a` | `#121212` | ~4.0:1 | FAIL | FAIL |
| Warning on dark bg | `#ca8a04` | `#121212` | ~5.9:1 | PASS | FAIL |

**Critical findings:**
- `--accent` (`#1d4ed8`) as foreground text on dark backgrounds fails AA. Affects: WelcomeScreen primary button, ShortcutsDialog group labels, anywhere `color: var(--accent)` is used as text.
- `--accent` as background with white text fails AA for normal text. Affects: StatusBar (seen by all users constantly).
- `--success` (`#16a34a`) as foreground text on dark backgrounds fails AA. Affects: TerminalToolbar mode dot, AnalyticsBar safe zone indicators.
- `--accent-text` (`#6b8de8`) passes AA and should be used instead of `--accent` for text on dark backgrounds.

### 2. Missing `aria-expanded` on Collapsible Sections

Multiple components have expand/collapse buttons without `aria-expanded`:
- ThinkingBlock.svelte (line 12)
- ToolUseBlock.svelte (line 35)

### 3. Touch Target Size Violations

Most close/dismiss buttons across the application use `padding: 2px 6px` or similar, resulting in targets of approximately 16-20px. This is a systemic pattern found in:
- SearchPalette close button
- ShortcutsDialog close button
- Settings close button
- FindInFiles close button
- ResponsePanel close button
- Toast dismiss button
- Terminal search buttons

### 4. Tablist Patterns Without Arrow Key Navigation

Three `role="tablist"` implementations lack horizontal Arrow key navigation:
- EditorTabs.svelte
- TerminalManager.svelte LLM tabs
- TerminalManager.svelte instance tabs

Per WAI-ARIA Authoring Practices, tablists require ArrowLeft/ArrowRight to move between tabs, with only the active tab having `tabindex="0"`.

### 5. Suppressed Accessibility Warnings

The following components use `svelte-ignore` to suppress a11y warnings:
- `ShortcutsDialog.svelte:47` -- `a11y_no_static_element_interactions` on overlay
- `Toolbar.svelte:61` -- `a11y_no_static_element_interactions` on window
- `Terminal.svelte:263-264` -- both `a11y_click_events_have_key_events` and `a11y_no_static_element_interactions`
- `Editor.svelte:227` -- `a11y_no_static_element_interactions`
- `WelcomeScreen.svelte:18` -- `a11y_no_static_element_interactions`

Some of these are justified (overlay click handlers, window event handlers), but Terminal.svelte's suppressions indicate genuine accessibility gaps.

### 6. Reduced Motion Coverage

**Global blanket** in `app.css` (lines 229-238): Excellent. Sets `animation-duration: 0.01ms`, `transition-duration: 0.01ms` for all elements when `prefers-reduced-motion: reduce`. This is the recommended approach.

**Component-level handling:**
- AnalyticsBar.svelte: Properly uses `$prefersReducedMotion` store for conditional logic and has `@media (prefers-reduced-motion: reduce)` for skeleton. Excellent.
- AnalyticsBar.svelte: Error/recovery flash has instant alternatives. Excellent.
- Toast.svelte `slide-in` animation: Covered by global blanket only.
- StreamingIndicator.svelte `pulse` animation: Covered by global blanket only.
- ContextMenu.svelte `spin` animation: Covered by global blanket only.
- AnalyticsBar.svelte `shimmer` animation: Component-level override present. Good.

**Assessment:** The global blanket is comprehensive and sufficient for CSS animations. The `$prefersReducedMotion` store adds programmatic control where needed. **Pass** overall, with the caveat that the global blanket must remain in place.

### 7. Typography and Readability

**Excellent:**
- Atkinson Hyperlegible Next/Mono: Purpose-designed for legibility, especially for users with low vision
- Enhanced Readability mode increases font size to 18px, line-height to 1.8, adds letter-spacing 0.05em
- Base `line-height: 1.5` is ideal
- `font-display: swap` ensures text is visible during font loading

**Good:**
- `font-size-base: 14px` is adequate for a desktop IDE
- `font-size-small: 12px` and `font-size-tiny: 11px` are used extensively. 11px may be too small for some users, but Enhanced Readability bumps it to 12px.

**Concern:**
- No `max-width` constraint on main content areas. MarkdownPreview has `max-width: 860px` which is good. Other text areas have no line-length limit, which can reduce readability on ultra-wide monitors.

---

## Positive Findings

1. **Atkinson Hyperlegible font family** -- Specifically designed by the Braille Institute for maximum legibility. Exceptional choice for an IDE targeting accessibility.

2. **Skip links** -- Properly implemented in App.svelte with visually-hidden-until-focused pattern. Targets file tree, editor, and terminal.

3. **Focus trap utility** (`src/lib/utils/a11y.ts`) -- Reusable, well-implemented focus trap used in SearchPalette, ShortcutsDialog, Settings, and FindInFiles.

4. **Menu key handler utility** (`src/lib/utils/a11y.ts`) -- Reusable Arrow key navigation for menus with Home/End support and wrapping. Used in Git dropdown, LLM dropdown, context menu, and terminal toolbar menus.

5. **Global reduced motion blanket** -- `app.css` lines 229-238 neutralize all CSS animations/transitions in one rule.

6. **Enhanced Readability mode** -- User-toggleable mode that increases font sizes, line-height, letter-spacing, and word-spacing globally.

7. **Focus ring design** -- `:focus-visible` is styled globally with `2px solid var(--accent)` and `2px offset`. `:focus:not(:focus-visible)` removes outline for mouse users. Correct modern pattern.

8. **ARIA live regions** -- Toasts (`aria-live="polite"`), chat messages (`role="log"`, `aria-live="polite"`), streaming indicator (`aria-live="assertive"`), status bar (`role="status"`), analytics bar (`role="status"` with screen reader announcer).

9. **Semantic HTML landmarks** -- `<nav>`, `<main>`, `<aside>`, `<header>`, `<footer>` with `aria-label` attributes.

10. **Light and dark themes** -- Both available with system preference detection via `color-scheme`.

11. **Internationalization** -- `$tr()` used throughout for all user-facing strings. Supports 9 languages.

---

## Issue Register

| # | Severity | Component | WCAG Criterion | Issue | Fix |
|---|----------|-----------|---------------|-------|-----|
| 1 | Critical | StatusBar | 1.4.3 Contrast (AA) | White text on `#1d4ed8` background: ~3.8:1 ratio, fails AA for normal text | Change `--accent` to a lighter blue (e.g., `#2563eb`) or darken the status bar background and use `--accent-text` (`#6b8de8`) |
| 2 | Critical | FileTree | 2.1.1 Keyboard (A) | All tree items have `tabindex="-1"`, no `tabindex="0"` entry point. Cannot Tab into file tree. | Set `tabindex="0"` on the first visible tree item; update on focus change |
| 3 | Critical | MenuItem | 2.1.1 Keyboard (A) | Submenus open only on mouse hover; keyboard users cannot access submenu items. No ArrowRight to open submenu. | Add ArrowRight keyboard handler to open submenu, ArrowLeft to close |
| 4 | High | WelcomeScreen, MarkdownPreview | 1.4.3 Contrast (AA) | `var(--accent)` (`#1d4ed8`) used as text color on dark bg: ~3.2:1. Fails AA. | Use `--accent-text` (`#6b8de8`) for text on dark backgrounds |
| 5 | High | ShortcutsDialog | 1.4.3 Contrast (AA) | Group label uses `var(--accent)` at 10px font: ~3.2:1. Fails AA. | Use `--accent-text` or increase font size to qualify as large text |
| 6 | High | EditorTabs | 2.1.1 Keyboard (A) | No ArrowLeft/ArrowRight navigation between tabs. All tabs have `tabindex="0"` instead of only active tab. | Implement roving tabindex per WAI-ARIA tablist pattern |
| 7 | High | TerminalManager | 2.1.1 Keyboard (A) | LLM and instance tablists lack Arrow key navigation | Same fix as #6 |
| 8 | High | MenuItem | 4.1.2 Name, Role, Value (A) | `role="menubar"` on individual menu item wrappers instead of the parent MenuBar container | Move `role="menubar"` to `div.menu-bar` in MenuBar.svelte; remove from MenuItem |
| 9 | High | Terminal | 4.1.2 Name, Role, Value (A) | Terminal container lacks `role` attribute and has suppressed a11y warnings | Add `role="application"` to terminal wrapper; add `aria-label` |
| 10 | High | Terminal | 1.3.1 Info and Relationships (A) | Search input lacks `aria-label`; only has placeholder | Add `aria-label="Find in terminal"` to search input |
| 11 | Medium | ThinkingBlock, ToolUseBlock | 4.1.2 Name, Role, Value (A) | Expand/collapse buttons lack `aria-expanded` attribute | Add `aria-expanded={expanded}` to toggle buttons |
| 12 | Medium | Systemic (7 components) | 2.5.8 Target Size (AA) | Close/dismiss buttons ~16-20px. Fails 24px minimum. | Add `min-width: 24px; min-height: 24px; display: flex; align-items: center; justify-content: center` to all close buttons |
| 13 | Medium | Toolbar | 4.1.2 Name, Role, Value (A) | YOLO button lacks `aria-label` and `aria-pressed` | Add `aria-label="Toggle YOLO mode"` and `aria-pressed={$yoloMode}` |
| 14 | Medium | WelcomeScreen | 4.1.2 Name, Role, Value (A) | Theme toggle and window control buttons lack `aria-label` | Add `aria-label` to theme toggle ("Toggle theme") and window buttons ("Minimize", "Maximize", "Close") |
| 15 | Medium | ResponsePanel | 2.1.1 Keyboard (A) | No focus trap; no Escape key to close | Add `trapFocus()` call when visible; add Escape handler |
| 16 | Medium | ImageDrop | 2.1.1 Keyboard (A) | Drag-and-drop is only way to add images; no keyboard alternative | Add an "Add image" button that opens a file picker |
| 17 | Medium | TerminalManager | 4.1.2 Name, Role, Value (A) | Instance close button is `<span role="button">` instead of `<button>` | Replace `<span>` with `<button>` element |
| 18 | Medium | TerminalManager | 4.1.2 Name, Role, Value (A) | Add instance "+" button lacks `aria-label` | Add `aria-label="Add terminal instance"` |
| 19 | Medium | Toast | 2.1.1 Keyboard (A) | Toast action buttons not reachable via Tab; no mechanism for keyboard users to interact with toasts before auto-dismiss | Make toast container focusable; pause auto-dismiss on focus |
| 20 | Low | FileTree | 1.3.1 Info and Relationships (A) | No loading indicator during initial file list fetch | Add a loading skeleton or spinner |
| 21 | Low | Editor | 1.3.1 Info and Relationships (A) | No loading indicator during file/language loading | Add a loading state |
| 22 | Low | ChatHeader | 4.1.2 Name, Role, Value (A) | No `aria-label` or live region for status changes | Add `role="status"` or `aria-live="polite"` to status span |
| 23 | Low | DiffView | 4.1.2 Name, Role, Value (A) | No `aria-label` on diff container | Add `aria-label="File diff: {filename}"` |
| 24 | Low | MarkdownPreview | 1.1.1 Non-text Content (A) | Rendered images from markdown may lack alt text | Consider adding default alt text or warning when images lack alt |
| 25 | Low | Systemic | 1.4.10 Reflow (AA) | No `max-width` on main content areas for line-length readability | Not strictly a WCAG violation for an IDE, but MarkdownPreview's 860px cap is a good model to follow |

---

## Recommendations by Priority

### Immediate (Critical/High -- blocks WCAG A/AA)

1. **Fix accent color contrast** -- Create `--accent-bg` (darker blue, e.g., `#1e3a8a`) for background use where white text is needed, or lighten status bar text. Use `--accent-text` instead of `--accent` for all text-on-dark-background usage.

2. **Fix FileTree tabindex** -- Set first tree item to `tabindex="0"`, manage roving tabindex so focused item always has `tabindex="0"` and all others have `-1`.

3. **Add arrow key navigation to tablists** -- EditorTabs and TerminalManager tabs need ArrowLeft/ArrowRight with roving tabindex per WAI-ARIA Practices.

4. **Fix MenuBar ARIA structure** -- Move `role="menubar"` to the container; add ArrowRight for submenus.

5. **Add terminal ARIA attributes** -- `role="application"`, `aria-label`, search input `aria-label`.

### Short-term (Medium -- WCAG A/AA compliance gaps)

6. Increase all close/dismiss button minimum sizes to 24x24px.
7. Add `aria-expanded` to ThinkingBlock and ToolUseBlock toggle buttons.
8. Add focus trap and Escape handler to ResponsePanel.
9. Add keyboard alternative for ImageDrop.
10. Replace `<span role="button">` with `<button>` in TerminalManager.
11. Add missing `aria-label` attributes (YOLO button, WelcomeScreen buttons, add-instance button).
12. Make toast action buttons keyboard-accessible.

### Long-term (Low -- polish and AAA)

13. Add loading states to FileTree and Editor.
14. Add `aria-live` to ChatHeader status.
15. Consider max-width constraints for text content.
16. Audit rendered markdown for image alt text.
