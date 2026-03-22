# Priority Roadmap

**Date:** 2026-03-22
**Based on:** Comprehensive audit of 62 issues

## Scoring Method

- **Impact (1-5):** How many users affected? How severely?
- **Effort (1-5):** How many files? How complex? How risky? (1 = easy, 5 = hard)
- **Priority Score = Impact / Effort** (higher = do first)

---

## Sprint 1: Urgent
**Focus:** Security blockers, crash prevention, and high-value quick wins
**Estimated scope:** ~15 issues

| # | Issue | Impact | Effort | Score | Rationale |
|---|-------|--------|--------|-------|-----------|
| ISSUE-001 | Workflow commands bypass path validation (arbitrary file R/W/D) | 5 | 2 | 2.50 | Sandbox escape. Add `validate_read_path`/`validate_write_path` to 6 commands — pattern already exists in codebase. |
| ISSUE-004 | `list_dir` and `grep_files` skip path validation | 5 | 1 | 5.00 | Same pattern as ISSUE-001. Two lines each. Must ship together. |
| ISSUE-016 | `start_watching` accepts arbitrary paths | 4 | 1 | 4.00 | Same pattern. One function, one validation call. |
| ISSUE-009 | Google Gemini API key exposed in URL query parameter | 5 | 1 | 5.00 | Move key from query string to `x-goog-api-key` header. Single file, ~3 lines changed. |
| ISSUE-010 | `write_config` enables privilege escalation via PTY allowlist | 5 | 2 | 2.50 | Validate TOML content and restrict command values. Two files. |
| ISSUE-003 | No file size guard on `read_file` — crashes app on large files | 5 | 1 | 5.00 | Add `metadata.len()` check before `read_to_string`. One file, ~5 lines. |
| ISSUE-002 | Mutex `lock().unwrap()` cascading panics (408 occurrences) | 5 | 4 | 1.25 | Start with top-4 critical modules: `workflow_engine.rs`, `agent_runtime.rs`, `session_manager.rs`, `event_bus.rs`. Full sweep deferred to Sprint 2. |
| ISSUE-015 | Stderr silenced for CLI processes — errors lost | 4 | 1 | 4.00 | Change `Stdio::null()` to `Stdio::piped()` and emit as warning events. One file. |
| ISSUE-011 | No tests in CI pipeline | 4 | 1 | 4.00 | Add `cargo test`, `npm test`, `cargo clippy` steps to `release.yml`. One file. |
| ISSUE-021 | DOMPurify default config allows forms/SVG in LLM responses | 4 | 1 | 4.00 | Add explicit `ALLOWED_TAGS` whitelist to 3 call sites. |
| ISSUE-005 | Monolithic 1.89 MB bundle — no code splitting | 5 | 3 | 1.67 | Start with `manualChunks` in vite config for the biggest wins (CodeMirror, xterm, xyflow). |
| ISSUE-012 | `agentEvents` Map grows unboundedly — memory leak | 4 | 2 | 2.00 | Add max event cap with pruning. Two files. |
| ISSUE-006 | No onboarding flow — users cannot discover LLM config | 5 | 3 | 1.67 | MVP: auto-detect CLIs on first launch, show setup prompt. Critical for first-time usability. |
| ISSUE-014 | Accent color fails WCAG AA contrast | 4 | 2 | 2.00 | Change CSS custom property and audit usages. Broad impact but simple change. |
| ISSUE-030 | Symlink loop crashes SearchPalette | 4 | 1 | 4.00 | Add visited-path tracking. One file, ~10 lines. |

---

## Sprint 2: Critical
**Focus:** Remaining P1s, core UX gaps, and high-impact P2s
**Estimated scope:** ~14 issues

| # | Issue | Impact | Effort | Score | Rationale |
|---|-------|--------|--------|-------|-----------|
| ISSUE-002 | Mutex `lock().unwrap()` — remaining 30 files | 5 | 4 | 1.25 | Complete the sweep started in Sprint 1. Systematic but repetitive. |
| ISSUE-008 | Chat diffs/code blocks have no apply/reject actions | 5 | 3 | 1.67 | Core UX gap. Requires wiring DiffBlock actions to file system commands. |
| ISSUE-007 | No session management UI | 4 | 4 | 1.00 | New component needed. Session sidebar with list, search, rename, delete. |
| ISSUE-013 | FileTree keyboard inaccessible | 4 | 2 | 2.00 | Roving tabindex + Enter/Space handlers. One file. |
| ISSUE-017 | MenuItem submenus keyboard-inaccessible | 4 | 2 | 2.00 | Arrow key navigation + correct ARIA structure. Two files. |
| ISSUE-023 | Editor defaults to read-only with no indicator | 4 | 2 | 2.00 | Add lock icon/banner and ensure correct prop passing. |
| ISSUE-024 | SessionHistoryRecorder does sync I/O inside event bus lock | 4 | 3 | 1.33 | Refactor to async channel-based writer. One file but architectural change. |
| ISSUE-022 | No chat message virtualization | 4 | 3 | 1.33 | Add svelte-virtual-list or pagination. One file but integration work. |
| ISSUE-034 | Multiple simultaneous agent sends — no guard | 4 | 1 | 4.00 | Add send-in-progress boolean guard + debounce. Two files. |
| ISSUE-036 | Concurrent file editing — no conflict detection | 4 | 3 | 1.33 | Implement hash-based optimistic locking + atomic writes. |
| ISSUE-031 | Rapid file switching causes editor teardown without debounce | 3 | 1 | 3.00 | Add 100ms debounce on file switch effect. One file, ~5 lines. |
| ISSUE-026 | CSP allows `unsafe-inline` for styles | 3 | 2 | 1.50 | Switch to nonce-based style CSP. Requires Svelte compatibility check. |
| ISSUE-025 | Toast notifications keyboard-inaccessible | 3 | 2 | 1.50 | Add tabindex, pause-on-focus, keyboard reachability. One file. |
| ISSUE-028 | EditorTabs keyboard navigation incorrect | 3 | 2 | 1.50 | Roving tabindex + ArrowLeft/Right handlers. One file. |

---

## Sprint 3: Important
**Focus:** i18n/a11y systematic fixes, remaining P2s, and structural improvements
**Estimated scope:** ~17 issues

| # | Issue | Impact | Effort | Score | Rationale |
|---|-------|--------|--------|-------|-----------|
| ISSUE-020 | RTL (Arabic) completely broken — 80+ physical CSS properties | 4 | 5 | 0.80 | Systematic migration of 80+ properties across 20+ files. High effort but critical for Arabic users. |
| ISSUE-018 | 67-70 i18n keys untranslated per locale | 3 | 4 | 0.75 | ~518 translations across 7 locale files. Bulk translation work. |
| ISSUE-019 | 68 hardcoded English strings in templates | 3 | 3 | 1.00 | Extract 68 strings across 25+ components to i18n keys. |
| ISSUE-027 | No font fallback for CJK and Devanagari | 3 | 1 | 3.00 | Add fonts to CSS fallback chain. One file, one line. |
| ISSUE-029 | Terminal container lacks ARIA role and label | 3 | 1 | 3.00 | Add role and aria-label attributes. One file. |
| ISSUE-032 | ResponsePanel no focus trap and no Escape close | 3 | 2 | 1.50 | Add focus trap + Escape handler + ARIA attributes. One file. |
| ISSUE-033 | Analytics store clones entire metrics vector on every query | 3 | 3 | 1.00 | Refactor to indexed queries or SQLite. Backend change. |
| ISSUE-035 | DiffBlock relies on color alone for add/remove | 3 | 1 | 3.00 | Add border markers or background patterns. One file. |
| ISSUE-037 | TerminalManager close uses span instead of button | 3 | 1 | 3.00 | Replace `<span role="button">` with `<button>`. Add aria-label. One file. |
| ISSUE-038 | Small touch targets across multiple components | 3 | 3 | 1.00 | Increase padding on close/dismiss buttons across 7 components. |
| ISSUE-039 | No visible save button | 2 | 1 | 2.00 | Add save icon in EditorTabs. One file. |
| ISSUE-040 | Mode switching dropdown does nothing (TODO) | 2 | 1 | 2.00 | Hide the dropdown until implemented. One file, ~3 lines. |
| ISSUE-041 | "SWARM" tab shows placeholder | 2 | 1 | 2.00 | Add disabled styling and brief explanation, or hide tab. One file. |
| ISSUE-048 | File read errors silently swallowed | 3 | 2 | 1.50 | Show user-visible error + detect binary files. Two files. |
| ISSUE-051 | Error display exposes stack traces in production | 3 | 1 | 3.00 | Conditional display based on build mode. One file. |
| ISSUE-054 | No forced-colors / high-contrast support | 3 | 3 | 1.00 | Add `@media (forced-colors: active)` rules across components. |
| ISSUE-047 | No locale-aware number and date formatting | 2 | 2 | 1.00 | Add `Intl.NumberFormat`/`Intl.DateTimeFormat` to analytics. |

---

## Sprint 4: Polish
**Focus:** P3 minor issues, dead code cleanup, and quality-of-life improvements
**Estimated scope:** ~10 issues

| # | Issue | Impact | Effort | Score | Rationale |
|---|-------|--------|--------|-------|-----------|
| ISSUE-042 | Font family hardcoded on save | 2 | 2 | 1.00 | Either make font configurable or remove non-functional UI. |
| ISSUE-043 | Find in Files does not jump to matching line | 2 | 2 | 1.00 | Wire up editor scrollToLine API. |
| ISSUE-044 | No input validation in Settings for command paths | 2 | 2 | 1.00 | Add binary existence check, URL validation. |
| ISSUE-045 | RetryPolicy defined but never invoked — dead code | 2 | 2 | 1.00 | Implement retry loop or remove dead code. |
| ISSUE-046 | YOLO mode toggle race condition | 2 | 1 | 2.00 | Add boolean guard to prevent concurrent restart loops. |
| ISSUE-049 | `{@html}` in ResourceNode with dictionary lookup | 2 | 1 | 2.00 | Replace HTML entities with Unicode characters. |
| ISSUE-050 | Budget cost uses crude estimate instead of actual data | 2 | 1 | 2.00 | Use `total_cost_usd` from backend metrics. One file. |
| ISSUE-052 | `get_env_var` allowlist includes PATH and HOME | 2 | 1 | 2.00 | Remove PATH from allowlist if not needed by frontend. |
| ISSUE-053 | `session_rename` accepts unbounded title length | 1 | 1 | 1.00 | Add max-length validation. One file, ~2 lines. |

---

## Backlog
**Focus:** Nice-to-have features and long-term improvements

| # | Issue | Impact | Effort | Score | Rationale |
|---|-------|--------|--------|-------|-----------|
| ISSUE-055 | No file/folder creation or deletion from FileTree | 2 | 3 | 0.67 | New feature: right-click context menu. Nice-to-have. |
| ISSUE-056 | No cursor line/column display in status bar | 2 | 2 | 1.00 | Wire CodeMirror cursor position to StatusBar. |
| ISSUE-057 | Markdown preview toggle not exposed in editor UI | 1 | 2 | 0.50 | Add toggle button for .md files. |
| ISSUE-058 | No terminal output export | 1 | 2 | 0.50 | "Save output" button on terminal toolbar. |
| ISSUE-059 | No version info on welcome screen | 1 | 1 | 1.00 | Add version to welcome screen footer. |
| ISSUE-060 | No drag-and-drop to open folder | 1 | 2 | 0.50 | Add drop handler on welcome screen. |
| ISSUE-061 | No empty state for ChatMessages | 1 | 1 | 1.00 | Add placeholder when events list is empty. |
| ISSUE-062 | No CSS containment on major panels | 1 | 1 | 1.00 | Add `contain: layout style` to panel containers. |

---

## Dependencies

The following issues have ordering dependencies that must be respected:

1. **ISSUE-001 before ISSUE-010:** Path validation patterns (ISSUE-001, ISSUE-004, ISSUE-016) establish the security baseline that ISSUE-010 (config write validation) builds upon. Fix the traversal holes first.

2. **ISSUE-002 (Sprint 1 top-4) before ISSUE-024:** The event bus mutex fix must land before refactoring SessionHistoryRecorder to async I/O, since both touch `event_bus.rs`.

3. **ISSUE-005 before ISSUE-022:** Code splitting (especially lazy-loading CodeMirror and xterm) should land before adding virtualization libraries to ChatMessages, to avoid inflating the bundle further.

4. **ISSUE-011 before all other Sprints:** CI test gating must be the very first merge so that every subsequent fix is validated by the pipeline.

5. **ISSUE-014 before ISSUE-020:** Fix the accent color contrast before the RTL CSS migration, since the RTL work touches many of the same files.

6. **ISSUE-006 before ISSUE-007:** Onboarding flow should exist before session management UI, since onboarding establishes the initial session context.

7. **ISSUE-019 before ISSUE-018:** Extract hardcoded strings to i18n keys before translating the missing keys, to avoid translating a stale key set.

---

## Recommended Order of Attack

### Phase 1: Seal the Security Perimeter (Days 1-3)

Start with the five path-traversal and credential issues (ISSUE-001, ISSUE-004, ISSUE-016, ISSUE-009, ISSUE-010). These are all low-effort, high-impact fixes that follow patterns already established in the codebase. The path validation functions `validate_read_path()` and `validate_write_path()` already exist and are used by `read_file` and `write_file` — the fix is literally applying the same pattern to the 8 commands that were missed. ISSUE-009 (Gemini API key in URL) is a 3-line change. These five fixes eliminate all known sandbox escapes and credential leakage vectors.

### Phase 2: Stop the Crashes (Days 3-5)

ISSUE-003 (file size guard) and ISSUE-030 (symlink loop) are both single-file, sub-10-line fixes that prevent guaranteed crashes. ISSUE-002 (mutex unwrap panics) requires more work but the top-4 worst offenders should be done immediately — these are the modules with the highest concurrent access (workflow engine, agent runtime, session manager, event bus). ISSUE-015 (stderr silenced) is a one-line fix that immediately improves debuggability for every user.

### Phase 3: Wire Up CI (Day 5)

ISSUE-011 is a single-file change to `release.yml` that gates all future merges. Every subsequent fix benefits from this. Do it early.

### Phase 4: Quick Security + Performance Wins (Days 5-7)

ISSUE-021 (DOMPurify hardening) is a copy-paste of a config object to 3 call sites. ISSUE-005 (code splitting) requires more thought but the `manualChunks` config in Vite is well-documented and the four chunks are clearly identified. ISSUE-012 (memory leak) is a bounded-map implementation with pruning.

### Phase 5: Core UX Gaps (Week 2)

ISSUE-006 (onboarding) and ISSUE-008 (chat apply/reject) are the two issues that most directly affect whether a new user can accomplish anything with the product. These are larger efforts but directly impact the product's reason for existing. ISSUE-014 (contrast) is a quick CSS fix that improves readability for everyone. ISSUE-034 (send guard) and ISSUE-031 (debounce) are small fixes that prevent common user-triggered bugs.

### Phase 6: Accessibility Debt (Weeks 2-3)

Group the keyboard accessibility fixes together: ISSUE-013 (FileTree), ISSUE-017 (MenuItem), ISSUE-028 (EditorTabs), ISSUE-025 (Toast), ISSUE-032 (ResponsePanel). These all follow the same roving-tabindex and ARIA patterns and can be done systematically. ISSUE-037 (span-to-button) and ISSUE-035 (color-only diffs) are quick additions to the same sweep.

### Phase 7: Internationalization (Weeks 3-4)

The i18n work (ISSUE-019, ISSUE-018, ISSUE-020, ISSUE-027, ISSUE-047) should be batched. Extract hardcoded strings first (ISSUE-019), then translate missing keys (ISSUE-018), then fix RTL layout (ISSUE-020). This ordering avoids rework. ISSUE-027 (font fallback) is a one-line fix that can land anytime.

### Phase 8: Structural Backend Improvements (Week 4)

ISSUE-024 (async I/O for event recording), ISSUE-022 (chat virtualization), ISSUE-036 (conflict detection), and ISSUE-033 (analytics store) are all deeper architectural changes that benefit from the stability established by earlier fixes. These improve scalability but are not blocking basic functionality.

### Phase 9: Polish and Backlog (Ongoing)

The remaining P3s and P4s can be picked up opportunistically. Many are sub-hour fixes (ISSUE-046, ISSUE-049, ISSUE-050, ISSUE-053, ISSUE-059, ISSUE-061, ISSUE-062) that make good first-contribution tasks or filler work between larger efforts.
