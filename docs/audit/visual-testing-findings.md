# Visual & Live Testing Findings

**Date:** 2026-03-22
**Status:** Code-level analysis only — live GUI testing requires human validation

> **Note:** This document consolidates findings from code-level analysis for Phase 3A (Vibecoder flows), Phase 3B (CTO technical inspection), and Phase 3C (accessibility live testing). Full visual testing with a running app requires human walkthrough. Items marked 🔍 need live verification.

---

## Phase 3A: Vibecoder Flow Analysis

### First Launch Flow
- **WelcomeScreen** renders with app title, subtitle, and action buttons
- 🔍 No guided onboarding — user must discover Settings independently
- 🔍 Welcome screen has no visual indicator pointing to provider setup
- Empty states exist for terminal and editor but not for chat area

### Chat Flow
- `ChatInput` has a clear textarea with send button
- 🔍 During streaming, `StreamingIndicator` shows status but no cancel button is visible
- `DiffBlock` in chat is **display-only** — no accept/reject like `DiffView` in editor
- `CodeBlock` has copy button but no "insert into file" action
- 🔍 Long streaming responses: no virtualization, DOM grows unbounded

### Editor Flow
- EditorTabs support scrolling when many tabs open
- 🔍 Tab switching destroys/recreates CodeMirror — potential flash of empty content
- DiffView has accept/reject buttons with clear visual feedback
- 🔍 Binary files produce `console.error` only, no user-visible message

### Terminal Flow
- Terminal toolbar with clear controls
- 🔍 Multiple terminal tabs work but lack distinguishing labels
- 🔍 Kill button visibility during long-running commands

### Settings Flow
- Settings panel has provider configuration with API key inputs
- 🔍 No "Test Connection" immediate feedback — user has to try chatting
- Budget configuration present in analytics section

### Analytics Flow
- Dashboard with charts and metrics — comprehensive
- 🔍 Empty analytics state (fresh install) — does it show zeros or a helpful message?

### Search / Shortcuts / Help
- SearchPalette opens with keyboard shortcut (Cmd+K / Ctrl+K)
- ShortcutsDialog has organized shortcut list
- HelpPanel with documentation links

---

## Phase 3B: CTO Technical Inspection (Code-Level)

### Build Output
- Production build generates a monolithic page chunk (~590 KB gzip)
- No code splitting configured in vite.config.ts
- All heavy libraries (CodeMirror, xterm.js, xyflow, highlight.js) bundled together

### Dependency Health
- npm audit: vulnerabilities exist (see security report for count)
- Rust: 0 cargo audit vulnerabilities
- 284 Rust tests exist but not wired into CI release pipeline

### Error Handling Patterns
- 104 production `unwrap()` calls in Rust (89 mutex locks)
- Svelte has `<svelte:boundary>` on main panels but not on child components
- No global error boundary for unhandled promise rejections

### Console Output (Expected)
- 🔍 Console warnings from deprecated APIs need live verification
- 🔍 Network tab inspection for unnecessary requests needs live verification

---

## Phase 3C: Accessibility Testing (Code-Level + Automated)

### axe-core Test
- Test file created at `tests/a11y/audit-axe-scan.test.ts`
- 🔍 **Requires running app** — execute: `npx playwright test tests/a11y/audit-axe-scan.test.ts`

### Keyboard Navigation (Code Analysis)
| Area | Tab-Reachable? | Arrow Keys? | Escape Closes? |
|------|---------------|-------------|----------------|
| FileTree | ❌ No tabindex="0" entry | ❌ Not implemented | N/A |
| EditorTabs | ⚠️ Via tab buttons | ❌ No arrow navigation | N/A |
| ChatInput | ✅ Yes | N/A | N/A |
| SearchPalette | ✅ Yes | ✅ Results navigable | ✅ Yes |
| ShortcutsDialog | ✅ Yes | N/A | ✅ Yes |
| Settings | ✅ Yes (tabs) | ❌ No arrow navigation | ✅ Yes |
| ContextMenu | ⚠️ Opens on right-click | ✅ Arrow keys work | ✅ Yes |
| Terminal | ✅ Yes | N/A (xterm handles) | N/A |
| AnalyticsDashboard | ✅ Yes | N/A | N/A |
| Toast | ❌ Actions unreachable | N/A | ❌ No dismiss |
| MenuItem submenus | ❌ Hover-only | ❌ Not keyboard | N/A |

### Zoom Levels (Code Analysis)
- 🔍 **200% zoom:** Components use `rem` units and flexbox — likely OK but needs verification
- 🔍 **400% zoom:** Sidebar/panel layout may overflow — needs live check
- Several components use `text-overflow: ellipsis` — truncation at high zoom likely

### Reduced Motion
- Global `prefers-reduced-motion: reduce` blanket in `app.css` removes transitions/animations
- ✅ Comprehensive coverage — applied globally rather than per-component

### Forced Colors / High Contrast
- 🔍 No `forced-colors` media query detected in codebase
- 🔍 No `prefers-contrast: more` handling
- Icons that rely on color alone will disappear in forced-colors mode
- Focus indicators may become invisible if they rely on box-shadow rather than outline

### Color Contrast Issues (from code analysis)
| Element | Colors | Ratio | Verdict |
|---------|--------|-------|---------|
| StatusBar text | white on `--accent` (#4fc3f7) | ~3.8:1 | ❌ Fails AA |
| Accent text on dark bg | `--accent` on dark | ~3.2:1 | ❌ Fails AA |
| Muted text | `--text-muted` on dark | ~4.8:1 | ✅ Passes AA |
| Normal text | `--text` on `--bg` | ~15:1 | ✅ Passes AA |
| Links/active states | `--accent` on dark | ~3.2:1 | ❌ Fails AA |

### Lighthouse
- 🔍 **Requires running app** — execute: `npx lighthouse http://localhost:1420 --output=json --output=html --output-path=docs/audit/lighthouse-report --chrome-flags="--no-sandbox" --only-categories=accessibility,performance,best-practices`

---

## Items Requiring Human Live Testing

1. Launch the app with `npm run tauri dev` and walk through all flows visually
2. Run the axe-core Playwright test with app running
3. Run Lighthouse audit with app running
4. Test keyboard-only navigation end-to-end
5. Test at 200% and 400% zoom
6. Test with `prefers-reduced-motion: reduce` emulation
7. Test with `forced-colors: active` emulation
8. Test with screen reader (NVDA/Orca on Linux)
9. Test RTL locale (Arabic) layout
10. Test German locale for label truncation
