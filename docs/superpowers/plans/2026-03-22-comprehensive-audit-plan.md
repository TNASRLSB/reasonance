# Comprehensive Multi-Persona Audit — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Audit every feature, component, and flow of Reasonance from 7 critical perspectives, producing 13 deliverables that map the path from current state to "accessible, reliable, production-ready."

**Architecture:** Phase 0 (competitive research) → Phase 1 (7 parallel code-level audits) → Phase 2 (cross-analysis + scoring) → Phase 3 (visual live testing) → Phase 3.5 (adversarial) → Phase 4 (report synthesis). Each phase feeds the next.

**Tech Stack:** Svelte 5, Tauri 2, Rust, CodeMirror 6, xterm.js, Playwright, axe-core, Lighthouse

**Spec:** `docs/superpowers/specs/2026-03-22-comprehensive-audit-design.md`

---

## File Map

All deliverables are written to `docs/audit/`:

| File | Created By | Purpose |
|------|-----------|---------|
| `docs/audit/competitive-matrix.md` | Task 1 | Phase 0 output |
| `docs/audit/vibecoder-report.md` | Task 2 | Agent 1 raw findings |
| `docs/audit/cto-report.md` | Task 3 | Agent 2 raw findings |
| `docs/audit/uxui-report.md` | Task 4 | Agent 3 raw findings |
| `docs/audit/security-report.md` | Task 5 | Agent 4 raw findings |
| `docs/audit/i18n-report.md` | Task 6 | Agent 5 raw findings |
| `docs/audit/stress-report.md` | Task 7 | Agent 6 raw findings |
| `docs/audit/performance-report.md` | Task 8 | Agent 7 raw findings |
| `docs/audit/nielsen-scorecard.md` | Task 9 | Phase 2 cross-analysis |
| `docs/audit/wcag-matrix.md` | Task 9 | Phase 2 WCAG evaluation |
| `docs/audit/visual-testing-findings.md` | Task 10-12 | Phase 3 live testing |
| `docs/audit/adversarial-findings.md` | Task 13 | Phase 3.5 output |
| `docs/audit/unified-report.md` | Task 14 | Synthesis of all findings |
| `docs/audit/issues.md` | Task 15 | Actionable issue list |
| `docs/audit/priority-roadmap.md` | Task 16 | Prioritized fix plan |

---

## Task 0: Prerequisites Check

**Files:** None created — validation only

- [ ] **Step 1: Verify Node.js and npm**

Run: `node --version && npm --version`
Expected: Node 18+ and npm 9+

- [ ] **Step 2: Verify project dependencies installed**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm ls --depth=0 2>/dev/null | head -5`
Expected: Dependencies listed. If `npm ERR!`, run `npm install` first.

- [ ] **Step 3: Install @axe-core/playwright**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm i -D @axe-core/playwright`
Expected: Package added to devDependencies

- [ ] **Step 4: Verify Playwright browsers**

Run: `npx playwright install --with-deps chromium`
Expected: Chromium installed

- [ ] **Step 5: Verify Rust/Tauri environment**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && cargo --version && cat src-tauri/Cargo.toml | head -5`
Expected: Cargo version shown, Cargo.toml readable

- [ ] **Step 6: Verify jq**

Run: `jq --version`
Expected: jq version shown

- [ ] **Step 7: Create audit output directory**

Run: `mkdir -p /home/uh1/VIBEPROJECTS/REASONANCE/docs/audit`

- [ ] **Step 8: Commit prerequisites setup**

```bash
git add -f docs/audit/.gitkeep package.json package-lock.json
git commit -m "chore: add audit tooling prerequisites (@axe-core/playwright)"
```

---

## Task 1: Phase 0 — Competitive Intelligence Matrix

**Files:**
- Create: `docs/audit/competitive-matrix.md`

**Parallelizable:** No (must complete before Phase 1 agents reference it)

- [ ] **Step 1: Research VS Code accessibility features**

Web search for: VS Code WCAG compliance, VS Code accessibility features, VS Code screen reader support, VS Code high contrast theme. Focus on public documentation at code.visualstudio.com/docs/editor/accessibility.

Document findings in a scratch section.

- [ ] **Step 2: Research Cursor accessibility and features**

Web search for: Cursor IDE accessibility, Cursor AI features, Cursor IDE keyboard navigation. Check cursor.com docs.

- [ ] **Step 3: Research Zed accessibility and features**

Web search for: Zed editor accessibility, Zed WCAG. Zed is open-source — check their GitHub repo for a11y issues/PRs.

- [ ] **Step 4: Research Windsurf accessibility and features**

Web search for: Windsurf IDE accessibility, Windsurf editor features. Check codeium.com/windsurf docs.

- [ ] **Step 5: Write competitive matrix**

Create `docs/audit/competitive-matrix.md` with this structure:

```markdown
# Competitive Matrix: Reasonance vs Industry

**Date:** 2026-03-22
**Status:** Best-effort — requires human validation

## Feature Comparison

| Feature | Reasonance | VS Code | Cursor | Zed | Windsurf |
|---------|-----------|---------|--------|-----|----------|
| Screen reader support | ? | ✅ | ? | ? | ? |
| Keyboard-only navigation | ? | ✅ | ? | ? | ? |
| High contrast mode | ? | ✅ | ? | ? | ? |
| Reduced motion | ? | ? | ? | ? | ? |
| RTL support | ? | ✅ | ? | ? | ? |
| WCAG 2.1 AA certified | ? | Partial | ? | ? | ? |
| Multi-AI support | ✅ | Via extensions | ✅ | ✅ | ✅ |
| Native performance | ✅ Tauri | ❌ Electron | ❌ Electron | ✅ Native | ❌ Electron |
| i18n (languages) | 9 | 50+ | ? | ? | ? |
| Built-in analytics | ✅ | ❌ | ❌ | ❌ | ❌ |
| Workflow orchestration | ✅ | Via extensions | ❌ | ❌ | ❌ |

## Accessibility Deep Dive

[Per-competitor section with specific findings]

## Where Reasonance Leads
[Unique advantages]

## Where Reasonance Trails
[Gaps vs competitors]

## Where Nobody Leads
[Industry-wide gaps that Reasonance could fill]
```

- [ ] **Step 6: Commit**

```bash
git add -f docs/audit/competitive-matrix.md
git commit -m "docs(audit): add competitive intelligence matrix"
```

---

## Task 2: Phase 1 — Agent 1: Vibecoder Audit

**Files:**
- Create: `docs/audit/vibecoder-report.md`
- Read (audit): All 51 Svelte components in `src/lib/components/`
- Read (audit): `src/routes/+page.svelte` (main layout)
- Read (audit): All 13 stores in `src/lib/stores/`
- Read (audit): `src/lib/adapter/` (frontend↔backend bridge)

**Parallelizable:** YES — can run simultaneously with Tasks 3-8

**Persona:** Developer who relies exclusively on LLM tools. No terminal comfort, no config file editing. Everything through the UI.

**Judgment criterion:** Can I accomplish every task without reading docs or source code?

- [ ] **Step 1: Audit first launch experience**

Read `src/lib/components/WelcomeScreen.svelte` and `src/lib/components/App.svelte`.
Document: What does a new user see? Is there onboarding? Is it obvious what to do first? Are there empty states?

- [ ] **Step 2: Audit project opening flow**

Read `src/lib/components/FileTree.svelte`, `src/lib/stores/files.ts`, adapter file open commands.
Document: How does a user open a project? Is it discoverable? What happens with an empty directory?

- [ ] **Step 3: Audit chat flow end-to-end**

Read all `src/lib/components/chat/*.svelte` (14 files), `src/lib/stores/agents.ts`, `src/lib/stores/agent-events.ts`, `src/lib/stores/agent-session.ts`.
Document: Send prompt → receive response → read text/code/diff → accept/reject diff. Any broken paths? Confusing UI? Missing feedback?

- [ ] **Step 4: Audit editor flow**

Read `src/lib/components/Editor.svelte`, `EditorTabs.svelte`, `DiffView.svelte`, `MarkdownPreview.svelte`, `src/lib/stores/files.ts`.
Document: Open file → edit → save → switch tabs → view diff → preview markdown. What's missing? What's confusing?

- [ ] **Step 5: Audit terminal flow**

Read `src/lib/components/TerminalManager.svelte`, `Terminal.svelte`, `TerminalToolbar.svelte`, `src/lib/stores/terminals.ts`.
Document: Open terminal → run command → multiple tabs → close. Would a vibecoder understand this?

- [ ] **Step 6: Audit settings flow**

Read `src/lib/components/Settings.svelte`, `src/lib/stores/config.ts`.
Document: Configure provider → enter API key → test connection → set budget. Is it intuitive? Clear error messages?

- [ ] **Step 7: Audit analytics flow**

Read `src/lib/components/AnalyticsDashboard.svelte`, `AnalyticsBar.svelte`, `src/lib/stores/analytics.ts`.
Document: View cost → understand usage → interpret metrics. Is the data meaningful to a non-technical user?

- [ ] **Step 8: Audit search, shortcuts, help**

Read `SearchPalette.svelte`, `FindInFiles.svelte`, `ShortcutsDialog.svelte`, `HelpPanel.svelte`.
Document: File search discoverable? Keyboard shortcuts learnable? Help system useful?

- [ ] **Step 9: Audit session management**

Read agent session stores and any session UI.
Document: Create session → switch → view history. Is it obvious what a "session" is?

- [ ] **Step 10: Write Vibecoder Report**

Create `docs/audit/vibecoder-report.md` with structure:

```markdown
# Vibecoder Audit Report

**Date:** 2026-03-22
**Persona:** Developer relying exclusively on LLM tools
**Judgment:** Can I accomplish every task without reading docs or source code?

## Executive Summary
[2-3 sentences: overall impression]

## Flow-by-Flow Findings

### 1. First Launch Experience
**Verdict:** ✅ Pass | ⚠️ Issues | ❌ Fail
[Findings with file:line references]

### 2. Project Opening
...
[Repeat for each flow]

## Friction Points (ranked by severity)
| # | Severity | Flow | Issue | Suggested Fix |
|---|----------|------|-------|---------------|

## What Works Well
[Positive findings — important for morale and knowing what NOT to break]
```

- [ ] **Step 11: Commit**

```bash
git add -f docs/audit/vibecoder-report.md
git commit -m "docs(audit): add Vibecoder persona report"
```

---

## Task 3: Phase 1 — Agent 2: CTO Audit

**Files:**
- Create: `docs/audit/cto-report.md`
- Read (audit): All 57 Rust files in `src-tauri/src/`
- Read (audit): `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`
- Read (audit): All 13 Svelte stores
- Read (audit): `package.json`, `.github/workflows/`
- Read (audit): `src/lib/adapter/`

**Parallelizable:** YES

**Persona:** Technical leader evaluating production-readiness.

**Judgment criterion:** Would I trust this in production for my team?

- [ ] **Step 1: Audit Rust architecture**

Read `src-tauri/src/lib.rs`, `main.rs`, `agent_runtime.rs`, `config.rs`, `discovery.rs`.
Evaluate: module boundaries, error handling patterns (Result vs unwrap/expect), panic safety.

- [ ] **Step 2: Audit transport layer**

Read all `src-tauri/src/transport/*.rs` (session_manager, event_bus, stream_reader, request, retry, session, session_handle, session_store).
Evaluate: reliability, ordering guarantees, backpressure, error recovery, resource cleanup.

- [ ] **Step 3: Audit normalizer layer**

Read all `src-tauri/src/normalizer/*.rs` (content_parser, pipeline, rules_engine, state machines for claude/gemini/kimi/qwen/codex/generic).
Evaluate: protocol correctness, extensibility, edge case handling, test coverage.

- [ ] **Step 4: Audit analytics backend**

Read `src-tauri/src/analytics/collector.rs`, `store.rs`, `mod.rs`.
Evaluate: data integrity, query performance, SQLite usage patterns.

- [ ] **Step 5: Audit commands layer**

Read all `src-tauri/src/commands/*.rs` (agent, pty, fs, config, discovery, engine, system, shadow, llm, transport, session, analytics, capability, provider, workflow).
Evaluate: input validation, IPC trust boundary, authorization.

- [ ] **Step 6: Audit Svelte state management**

Read all 13 stores in `src/lib/stores/`. Cross-reference with component usage.
Evaluate: reactivity correctness, race conditions, stale state, memory leaks, subscription cleanup.

- [ ] **Step 7: Audit build pipeline and dependencies**

Read `package.json`, `Cargo.toml`, `.github/workflows/release.yml`.
Run: `npm audit --audit-level=moderate` and `cd src-tauri && cargo audit 2>/dev/null || echo "cargo-audit not installed"`
Evaluate: outdated packages, known CVEs, CI/CD quality.

- [ ] **Step 8: Audit error recovery patterns**

Search codebase for: `unwrap()`, `expect(`, `.catch(`, `try {`, error boundaries, fallback UI.
Evaluate: what happens when things fail? Is there recovery or silent failure?

- [ ] **Step 9: Write CTO Report**

Create `docs/audit/cto-report.md`:

```markdown
# CTO Technical Audit Report

**Date:** 2026-03-22
**Persona:** Technical leader evaluating production adoption
**Judgment:** Would I trust this in production for my team?

## Executive Summary

## Architecture Assessment
### Strengths
### Concerns
### Risks

## Layer-by-Layer Findings
### Rust Backend
### Transport Layer
### Normalizer Layer
### Analytics
### Commands / IPC
### Svelte State Management
### Build Pipeline

## Code Quality Metrics
| Metric | Value | Assessment |
|--------|-------|------------|
| unwrap() count | ? | ? |
| Error handling coverage | ? | ? |
| Test coverage | ? | ? |
| Dead code | ? | ? |

## Production Readiness Verdict
[Clear yes/no/conditional with reasoning]

## Technical Debt Register
| # | Severity | Location | Issue | Suggested Fix |
```

- [ ] **Step 10: Commit**

```bash
git add -f docs/audit/cto-report.md
git commit -m "docs(audit): add CTO technical audit report"
```

---

## Task 4: Phase 1 — Agent 3: UX/UI Designer Audit

**Files:**
- Create: `docs/audit/uxui-report.md`
- Read (audit): All 51 Svelte components
- Read (audit): `src/app.css` (global styles)
- Read (audit): All existing a11y tests in `tests/a11y/`

**Parallelizable:** YES

**Persona:** Senior accessible design specialist.

**Judgment criterion:** Can every human use this effectively and comfortably?

- [ ] **Step 1: Audit ARIA usage across all components**

For each component, check:
- Correct `role` attributes (not decorative roles on interactive elements)
- `aria-label`, `aria-labelledby`, `aria-describedby` where needed
- `aria-expanded`, `aria-selected`, `aria-checked` state management
- `aria-live` regions for dynamic content (chat messages, streaming, toasts)
- `aria-hidden` on decorative elements

grep patterns: `role=`, `aria-`, `tabindex`, `sr-only`, `visually-hidden`

- [ ] **Step 2: Audit keyboard navigation**

For each component, check:
- Can every interactive element be reached via Tab?
- Is focus order logical (top-to-bottom, left-to-right)?
- Is `:focus-visible` styled (not just `:focus`)?
- Are focus traps implemented for modals/dialogs (SearchPalette, ShortcutsDialog, Settings)?
- Is roving tabindex used for lists/grids (FileTree, EditorTabs)?
- Can Escape close overlays?
- Are keyboard shortcuts documented and conflict-free?

- [ ] **Step 3: Audit color contrast**

Read `src/app.css` and extract all color values.
Check every text/background combination against WCAG 2.1:
- AA: 4.5:1 for normal text, 3:1 for large text
- AAA: 7:1 for normal text, 4.5:1 for large text
- UI components and graphical objects: 3:1

Pay special attention to: disabled states, placeholder text, status indicators, error text, links.

- [ ] **Step 4: Audit touch targets**

Check all clickable/tappable elements:
- Minimum 44x44px (WCAG 2.5.5 AAA) or 24x24px (WCAG 2.5.8 AA)
- Adequate spacing between targets
- Focus on: toolbar buttons, tab close buttons, tree items, menu items

- [ ] **Step 5: Audit reduced motion**

grep for: `@media (prefers-reduced-motion`, `transition`, `animation`, `transform`, `@keyframes`
Check: Every animation has a `prefers-reduced-motion: reduce` alternative?

- [ ] **Step 6: Audit typography and readability**

Check: font-family (Atkinson Hyperlegible?), font sizes, line-height, letter-spacing, max line width, text wrapping, truncation with ellipsis.

- [ ] **Step 7: Audit loading and error states**

For each component: What does it show when loading? When errored? When empty?
Check for: skeleton screens, spinners, error messages with recovery actions, empty state illustrations/text.

- [ ] **Step 8: Audit screen reader compatibility**

Check: semantic HTML (heading hierarchy, landmarks, lists), reading order matches visual order, live regions for dynamic updates, image alt text.

- [ ] **Step 9: Write UX/UI Designer Report**

Create `docs/audit/uxui-report.md`:

```markdown
# UX/UI Designer Accessibility Audit Report

**Date:** 2026-03-22
**Persona:** Senior accessible design specialist
**Judgment:** Can every human use this effectively and comfortably?

## Executive Summary

## WCAG 2.1 Compliance Overview
| Level | Criteria Checked | Pass | Fail | Partial | N/A |
|-------|-----------------|------|------|---------|-----|
| A | ? | ? | ? | ? | ? |
| AA | ? | ? | ? | ? | ? |
| AAA | ? | ? | ? | ? | ? |

## Component-by-Component Findings

### FileTree
**ARIA:** ✅/⚠️/❌
**Keyboard:** ✅/⚠️/❌
**Contrast:** ✅/⚠️/❌
**Touch targets:** ✅/⚠️/❌
[Details...]

[Repeat for each component]

## Systemic Patterns
[Issues that appear across multiple components]

## Positive Findings
[What's done well — don't break these]

## Issue Register
| # | Severity | Component | WCAG Criterion | Issue | Fix |
```

- [ ] **Step 10: Commit**

```bash
git add -f docs/audit/uxui-report.md
git commit -m "docs(audit): add UX/UI Designer accessibility report"
```

---

## Task 5: Phase 1 — Agent 4: Security Audit

**Files:**
- Create: `docs/audit/security-report.md`
- Read (audit): `src-tauri/tauri.conf.json` (permissions)
- Read (audit): `src-tauri/capabilities/` (capability files)
- Read (audit): Markdown rendering code (DOMPurify config)
- Read (audit): PTY manager, FS watcher, config handling
- Read (audit): `package.json` (dependency versions)

**Parallelizable:** YES

**Persona:** Security engineer performing penetration assessment.

**Judgment criterion:** Can a malicious input or extension compromise the user?

- [ ] **Step 1: Audit Tauri permission scope**

Read `src-tauri/tauri.conf.json` and all files in `src-tauri/capabilities/`.
Check: minimum privilege? Are any permissions broader than needed? Can frontend access file system unrestricted?

- [ ] **Step 2: Audit XSS vectors**

Read markdown rendering (TextBlock, MarkdownPreview — find DOMPurify config).
Read any innerHTML/dangerouslySetInnerHTML/`{@html}` usage.
Check: Is DOMPurify configured correctly? Any bypass vectors?

- [ ] **Step 3: Audit command injection**

Read PTY manager (file paths passed to shell), FS watcher (path handling), config.rs (TOML parsing of user input).
Check: Are file paths sanitized? Can project names inject commands? Can chat input reach shell execution?

- [ ] **Step 4: Audit credential exposure**

grep for: `api_key`, `apiKey`, `API_KEY`, `token`, `secret`, `password`, `localStorage`, `sessionStorage`.
Check: Do API keys ever reach frontend code? Are they logged? Visible in IPC messages? Stored in plaintext?

- [ ] **Step 5: Audit IPC trust boundary**

Read all `src-tauri/src/commands/*.rs` — these are the IPC surface.
Check: Does every command validate its input? Can frontend invoke dangerous operations (rm, exec, write arbitrary files)?

- [ ] **Step 6: Audit CSP and webview security**

Read `src-tauri/tauri.conf.json` for: CSP configuration, dangerousRemoteDomainIpcAccess, withGlobalTauri.
Read `src/app.html` for: meta CSP tags, script-src, style-src directives.
Check: Is `unsafe-inline` or `unsafe-eval` used? Are external domains whitelisted unnecessarily?
Map against OWASP Top 10 for desktop apps: injection, broken auth, sensitive data exposure, XXE, broken access control, security misconfiguration, XSS, insecure deserialization, using components with known vulns, insufficient logging.

- [ ] **Step 7: Audit supply chain**

Run: `npm audit --json | jq '.vulnerabilities | length'`
Run: `cd src-tauri && cargo audit 2>/dev/null || echo "install cargo-audit"`
Check `package.json` for known-vulnerable versions.

- [ ] **Step 7: Write Security Report**

Create `docs/audit/security-report.md`:

```markdown
# Security Audit Report

**Date:** 2026-03-22
**Persona:** Security engineer
**Judgment:** Can a malicious input or extension compromise the user?

## Executive Summary

## Threat Model
[Attack surface: IPC, file system, network, user input, dependencies]

## Findings

### Critical (P0)
### High (P1)
### Medium (P2)
### Low (P3)

## Vulnerability Register
| # | Severity | Vector | Location | Description | Remediation |
```

- [ ] **Step 8: Commit**

```bash
git add -f docs/audit/security-report.md
git commit -m "docs(audit): add security audit report"
```

---

## Task 6: Phase 1 — Agent 5: i18n/RTL Audit

**Files:**
- Create: `docs/audit/i18n-report.md`
- Read (audit): `src/lib/i18n/index.ts` (i18n system)
- Read (audit): All 9 locale JSON files in `src/lib/i18n/`
- Read (audit): All components for hardcoded strings
- Read (audit): `src/app.css` for RTL support

**Parallelizable:** YES

**Persona:** Users in all 9 supported locales, especially Arabic (RTL).

**Judgment criterion:** Does a non-English user get a first-class experience?

- [ ] **Step 1: Audit locale completeness**

Read `src/lib/i18n/en.json` — this is the reference.
Compare key count against all other 8 locale files.
Document: missing keys per locale, extra keys, empty values.

- [ ] **Step 2: Audit hardcoded English strings**

grep all `.svelte` files for string literals that should be translated.
Patterns: `"` strings in template sections that aren't variables, `placeholder="`, `title="`, `aria-label="`.
Document: every hardcoded English string with file:line.

- [ ] **Step 3: Audit RTL support for Arabic**

Read `src/app.css` for: `direction: rtl`, `[dir="rtl"]`, logical properties (`margin-inline-start`, `padding-inline-end`, `inset-inline`).
Check each component for: layout mirroring, scrollbar position, icon direction, text alignment.
Document: components that break in RTL.

- [ ] **Step 4: Audit German long labels**

Read `src/lib/i18n/de.json` — find longest values.
Check components for: text truncation with ellipsis, overflow handling, flexible widths.
Document: labels that would overflow their containers.

- [ ] **Step 5: Audit CJK and Hindi rendering**

Read `zh.json` and `hi.json`. Check: encoding (UTF-8), font-family fallback chain (does it include CJK and Devanagari fonts?), line-breaking rules, number formatting.

- [ ] **Step 6: Audit dynamic locale switching**

Read `src/lib/i18n/index.ts` — how is locale switching implemented? Is it reactive?
Check: when locale changes, do ALL components re-render with new strings? Or are some cached/stale?
Check: date/number formatting — does `Intl.DateTimeFormat` / `Intl.NumberFormat` use the active locale?
Document: components that don't update on locale switch, missing locale-aware formatting.

- [ ] **Step 7: Write i18n Report**

Create `docs/audit/i18n-report.md`:

```markdown
# i18n / RTL Audit Report

**Date:** 2026-03-22
**Persona:** Non-English users across 9 locales
**Judgment:** Does a non-English user get a first-class experience?

## Executive Summary

## Locale Completeness Matrix
| Locale | Total Keys | Missing | Extra | Completion % |
|--------|-----------|---------|-------|-------------|

## RTL (Arabic) Findings
### Components that mirror correctly
### Components that break in RTL
### Missing logical properties

## Long Label (German) Findings

## CJK / Devanagari Findings

## Hardcoded English Strings
| # | File | Line | String | Context |
```

- [ ] **Step 7: Commit**

```bash
git add -f docs/audit/i18n-report.md
git commit -m "docs(audit): add i18n/RTL audit report"
```

Note: Step numbering shifted — commit is now Step 8.

---

## Task 7: Phase 1 — Agent 6: Stress & Edge Cases Audit

**Files:**
- Create: `docs/audit/stress-report.md`
- Read (audit): Components that handle dynamic content (ChatMessages, Editor, Terminal, FileTree, EditorTabs)
- Read (audit): Error handling in stores and adapter
- Read (audit): Rust error handling and resource limits

**Parallelizable:** YES

**Persona:** The chaos monkey.

**Judgment criterion:** Does the app degrade gracefully or crash?

- [ ] **Step 1: Audit large content handling**

Read Editor.svelte — any file size limits? Virtualization? Lazy rendering?
Read ChatMessages.svelte — message list virtualization? DOM node limits?
Read Terminal.svelte — scrollback buffer limits? Memory caps?
Document: theoretical limits before degradation.

- [ ] **Step 2: Audit many-items handling**

Read EditorTabs.svelte — what happens with 100 tabs? Scroll? Overflow?
Read FileTree.svelte — deep nesting limits? Expand-all performance?
Document: UI behavior at scale.

- [ ] **Step 3: Audit empty states**

Read every component for: what renders when data is empty/null/undefined?
Check: empty project, no files, no sessions, no providers, no analytics data.
Document: components that crash or show blank white space instead of helpful empty states.

- [ ] **Step 4: Audit error states in code**

grep for: `catch`, `on:error`, `onerror`, `$effect` error handling, Rust `unwrap()`, `expect(`.
Count: how many error paths exist vs how many are handled?
Document: unhandled error paths that could crash the app.

- [ ] **Step 5: Audit concurrent operation safety**

Read stores for race conditions: multiple rapid updates, async operations that could interleave.
Check: rapid tab switching, multiple agent sends, resize during stream.
Document: potential race conditions.

- [ ] **Step 6: Audit binary/special file handling**

Read Editor.svelte — what happens when opening binary files? Images? Very long lines?
Read FileTree — special filenames (spaces, emoji, unicode, `.` prefix, very long names)?
Document: edge cases that produce unexpected behavior.

- [ ] **Step 7: Write Stress Report**

Create `docs/audit/stress-report.md`:

```markdown
# Stress & Edge Cases Audit Report

**Date:** 2026-03-22
**Persona:** The chaos monkey
**Judgment:** Does the app degrade gracefully or crash?

## Executive Summary

## Stress Limits
| Scenario | Expected Behavior | Actual (from code) | Verdict |
|----------|-------------------|-------------------|---------|
| 50MB file in editor | Warn or refuse | ? | ? |
| 100 open tabs | Scroll tabs | ? | ? |
| 10000 chat messages | Virtualize | ? | ? |
| Terminal flood | Cap scrollback | ? | ? |

## Empty States
| Component | Has empty state? | Quality |

## Error Recovery
| Error Scenario | Handled? | Recovery Path |

## Race Conditions
| Scenario | Risk Level | Location |

## Edge Cases
| Input | Component | Behavior |
```

- [ ] **Step 8: Commit**

```bash
git add -f docs/audit/stress-report.md
git commit -m "docs(audit): add stress and edge cases report"
```

---

## Task 8: Phase 1 — Agent 7: Performance Audit

**Files:**
- Create: `docs/audit/performance-report.md`
- Read (audit): `package.json` (bundle dependencies)
- Read (audit): `vite.config.ts` (build config, code splitting)
- Read (audit): All stores (reactivity patterns)
- Read (audit): Component imports (lazy loading)

**Parallelizable:** YES

**Persona:** Performance engineer with profiling tools.

**Judgment criterion:** Is the app fast and lean, or hiding bloat?

- [ ] **Step 1: Audit bundle composition**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -30`
Analyze: total bundle size, largest chunks, tree-shaking effectiveness.
Read `vite.config.ts` for: code splitting config, manual chunks, external deps.

- [ ] **Step 2: Audit Svelte reactivity patterns**

Read all 13 stores. Check for:
- `$derived` chains that could cause cascade re-renders
- `$effect` that write to stores (infinite loop risk)
- Large objects in stores that trigger full re-render on any property change
- Missing `$derived.by()` for expensive computations

- [ ] **Step 3: Audit memory leak patterns**

Check all components for:
- `$effect` without cleanup (event listeners, intervals, subscriptions)
- `onMount` without corresponding `onDestroy`
- Tauri `listen()` without `unlisten()`
- Growing arrays/maps in stores without cleanup

- [ ] **Step 4: Audit lazy loading**

Check: Which heavy libraries are loaded eagerly? (CodeMirror, xterm.js, xyflow, highlight.js)
Are they behind dynamic imports? Do they block initial render?

- [ ] **Step 5: Audit CSS performance**

Read `src/app.css`:
- Count CSS custom properties vs hard-coded values
- Check for expensive selectors (universal, deep nesting)
- Check for unused CSS rules
- Check for layout thrashing patterns (read-then-write in JS)

- [ ] **Step 6: Audit Rust performance patterns**

Read key hot paths: `stream_reader.rs` (parsing), `event_bus.rs` (pub/sub), `session_store.rs` (SQLite).
Check: async patterns, lock contention, serialization overhead, unnecessary cloning.

- [ ] **Step 7: Write Performance Report**

Create `docs/audit/performance-report.md`:

```markdown
# Performance Audit Report

**Date:** 2026-03-22
**Persona:** Performance engineer
**Judgment:** Is the app fast and lean, or hiding bloat?

## Executive Summary

## Bundle Analysis
| Chunk | Size (gzip) | Contents | Lazy? |

## Reactivity Issues
| Store | Issue | Impact | Fix |

## Memory Leak Risks
| Component | Pattern | Risk Level |

## Lazy Loading Status
| Library | Size | Currently | Should Be |

## CSS Performance

## Rust Hot Path Analysis

## Recommendations (by impact)
| # | Impact | Effort | Description |
```

- [ ] **Step 8: Commit**

```bash
git add -f docs/audit/performance-report.md
git commit -m "docs(audit): add performance audit report"
```

---

## Task 9: Phase 2 — Cross-Analysis, Nielsen Scorecard & WCAG Matrix

**Files:**
- Create: `docs/audit/nielsen-scorecard.md`
- Create: `docs/audit/wcag-matrix.md`
- Read: All 7 Phase 1 reports

**Parallelizable:** No — depends on Tasks 2-8

- [ ] **Step 1: Read all 7 Phase 1 reports**

Read vibecoder-report, cto-report, uxui-report, security-report, i18n-report, stress-report, performance-report.

- [ ] **Step 2: Build Nielsen Heuristic Scorecard**

For each of the 17 components in the spec's Phase 2 table, score against Nielsen's 10 heuristics (1-5):

1. Visibility of system status
2. Match between system and real world
3. User control and freedom
4. Consistency and standards
5. Error prevention
6. Recognition rather than recall
7. Flexibility and efficiency of use
8. Aesthetic and minimalist design
9. Help users recognize, diagnose, and recover from errors
10. Help and documentation

Write to `docs/audit/nielsen-scorecard.md`.

- [ ] **Step 3: Build WCAG 2.1 Compliance Matrix**

For each component, evaluate against every applicable WCAG 2.1 criterion at AA level (with AAA aspirational):

Key criteria to evaluate:
- 1.1.1 Non-text Content
- 1.3.1 Info and Relationships
- 1.3.2 Meaningful Sequence
- 1.4.1 Use of Color
- 1.4.3 Contrast (Minimum)
- 1.4.4 Resize Text
- 1.4.11 Non-text Contrast
- 1.4.12 Text Spacing
- 1.4.13 Content on Hover or Focus
- 2.1.1 Keyboard
- 2.1.2 No Keyboard Trap
- 2.4.3 Focus Order
- 2.4.6 Headings and Labels
- 2.4.7 Focus Visible
- 2.5.5 Target Size (AAA) / 2.5.8 Target Size Minimum (AA)
- 3.2.1 On Focus
- 3.2.2 On Input
- 3.3.1 Error Identification
- 3.3.2 Labels or Instructions
- 4.1.1 Parsing
- 4.1.2 Name, Role, Value
- 4.1.3 Status Messages

Write to `docs/audit/wcag-matrix.md` as a table: Component × Criterion → Pass/Fail/Partial/N-A.

- [ ] **Step 4: Add cognitive load and error state coverage per component**

For each component, add:
- Cognitive Load Rating (low/medium/high + justification)
- Error states handled vs missing

Append to nielsen-scorecard.md.

- [ ] **Step 5: Commit**

```bash
git add -f docs/audit/nielsen-scorecard.md docs/audit/wcag-matrix.md
git commit -m "docs(audit): add Nielsen scorecard and WCAG compliance matrix"
```

---

## Task 10: Phase 3A — Visual Testing: Vibecoder Flows

**Files:**
- Modify: `docs/audit/vibecoder-report.md` (append visual findings)
- Create: `docs/audit/visual-testing-findings.md`

**Parallelizable:** No — requires running app

**Prerequisites:** App must be running via `npm run dev`

- [ ] **Step 1: Launch the app**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run dev`
Wait for: `VITE vX.X.X ready` message.

- [ ] **Step 2: Walk through first launch flow**

Open browser to localhost URL. Document:
- What loads first? How long?
- Is the welcome screen shown?
- What's the first call to action?
- Screenshot key states.

- [ ] **Step 3: Walk through chat flow**

If a provider is configured, send a test prompt. Document:
- Is the input obvious?
- Does streaming feedback work?
- Are code blocks, diffs, tool uses rendered correctly?
- Any console errors during streaming?

- [ ] **Step 4: Walk through editor, terminal, settings, analytics, search**

Open files, switch tabs, use terminal, open settings, check analytics, use Cmd+K.
Document everything that feels wrong, broken, or confusing.

- [ ] **Step 5: Document visual findings**

Create/update `docs/audit/visual-testing-findings.md` with Phase 3A section.

- [ ] **Step 6: Commit**

```bash
git add -f docs/audit/visual-testing-findings.md docs/audit/vibecoder-report.md
git commit -m "docs(audit): add Phase 3A visual testing findings (Vibecoder)"
```

---

## Task 11: Phase 3B — Visual Testing: CTO Technical Inspection

**Files:**
- Modify: `docs/audit/cto-report.md` (append visual findings)
- Modify: `docs/audit/visual-testing-findings.md`

**Parallelizable:** No — requires running app

- [ ] **Step 1: Check browser console**

Open DevTools → Console. Document: errors, warnings, deprecation notices.

- [ ] **Step 2: Check network tab**

Document: failed requests, slow requests, unnecessary requests, request sizes.

- [ ] **Step 3: Check performance**

DevTools → Performance. Record during: page load, chat streaming, tab switching.
Document: frame drops, long tasks, memory growth.

- [ ] **Step 4: Analyze bundle**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npx vite-bundle-visualizer 2>/dev/null || npm run build -- --report`
Document: largest dependencies, code splitting effectiveness.

- [ ] **Step 5: Append findings to reports**

Update `docs/audit/visual-testing-findings.md` and `docs/audit/cto-report.md`.

- [ ] **Step 6: Commit**

```bash
git add -f docs/audit/visual-testing-findings.md docs/audit/cto-report.md
git commit -m "docs(audit): add Phase 3B visual testing findings (CTO)"
```

---

## Task 12: Phase 3C — Visual Testing: UX/UI Accessibility Live Testing

**Files:**
- Modify: `docs/audit/uxui-report.md`
- Modify: `docs/audit/wcag-matrix.md`
- Create: `tests/a11y/audit-axe-scan.test.ts`

**Parallelizable:** No — requires running app

- [ ] **Step 1: Write axe-core Playwright test**

Create `tests/a11y/audit-axe-scan.test.ts`:

```typescript
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('Accessibility Audit Scan', () => {
  test('main view has no critical a11y violations', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
      .analyze();
    console.log('Violations:', JSON.stringify(results.violations, null, 2));
    // Don't assert yet — we want the data
  });
});
```

- [ ] **Step 2: Run axe-core scan**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npx playwright test tests/a11y/audit-axe-scan.test.ts --reporter=json > docs/audit/axe-results.json 2>/dev/null`

- [ ] **Step 3: Keyboard-only walkthrough**

With app running, tab through every interactive element. Document:
- Elements that can't be reached
- Focus that disappears
- Focus rings that are invisible
- Traps that can't be escaped

- [ ] **Step 4: Test zoom levels**

Set browser zoom to 200% and 400%. Document:
- Layout overflow
- Text truncation
- Horizontal scrolling
- Overlapping elements

- [ ] **Step 5: Test reduced motion**

In DevTools → Rendering → Emulate CSS media feature → prefers-reduced-motion: reduce.
Document: animations that still play, transitions that should be instant.

- [ ] **Step 6: Test high contrast / forced-colors mode**

In DevTools → Rendering → Emulate CSS media feature → forced-colors: active.
Also test with `prefers-contrast: more`.
Document: elements that disappear, borders that vanish, icons that become invisible, focus indicators that rely on color alone.

- [ ] **Step 7: Run Lighthouse audit**

Run: `npx lighthouse http://localhost:1420 --output=json --output=html --output-path=docs/audit/lighthouse-report --chrome-flags="--no-sandbox" --only-categories=accessibility,performance,best-practices`
Document: accessibility score, performance score, specific failures.

- [ ] **Step 8: Update WCAG matrix with live findings**

Cross-reference axe-core results with Phase 1 code-level findings. Update `docs/audit/wcag-matrix.md`.

- [ ] **Step 7: Commit**

```bash
git add -f tests/a11y/audit-axe-scan.test.ts docs/audit/uxui-report.md docs/audit/wcag-matrix.md docs/audit/visual-testing-findings.md
git commit -m "docs(audit): add Phase 3C accessibility live testing findings"
```

---

## Task 13: Phase 3.5 — Adversarial Testing

**Files:**
- Create: `docs/audit/adversarial-findings.md`

**Parallelizable:** No — requires running app

- [ ] **Step 1: Test input extremes**

With app running:
- Paste very large text (generate 1MB string) into chat input
- Type extremely long file search query in SearchPalette
- Enter special characters in every text input: `<script>alert(1)</script>`, `'; DROP TABLE`, `../../etc/passwd`

Document: crashes, hangs, XSS, unexpected behavior.

- [ ] **Step 2: Test rapid interactions**

- Double-click every button rapidly
- Switch tabs during streaming
- Resize window while content loads
- Open/close settings rapidly
- Rapidly toggle between views

Document: race conditions, visual glitches, errors.

- [ ] **Step 3: Test file system edge cases**

- Create files with special names and try to open them
- Try to open very large files
- Try to open binary files (if testable)
- Create circular symlinks: `ln -s ./link-a ./link-b && ln -s ./link-b ./link-a`
- Create deeply nested directories (20+ levels)
- Test extremely long filenames (255 chars)

Document: behavior and error handling.

- [ ] **Step 4: Test network and provider edge cases**

- If a provider is configured: switch provider mid-response (click different model while streaming)
- Open 5+ chat sessions and send messages to all simultaneously
- Kill network during streaming: use DevTools → Network → Offline toggle
- Restore network and verify recovery behavior

Document: crashes, hangs, error messages, recovery.

- [ ] **Step 5: Write adversarial findings**

Create `docs/audit/adversarial-findings.md`:

```markdown
# Adversarial Testing Report

**Date:** 2026-03-22

## Input Extremes
| Test | Expected | Actual | Severity |

## Rapid Interactions
| Test | Expected | Actual | Severity |

## File System Edge Cases
| Test | Expected | Actual | Severity |

## Network Failures
[If testable]
```

- [ ] **Step 5: Commit**

```bash
git add -f docs/audit/adversarial-findings.md
git commit -m "docs(audit): add adversarial testing findings"
```

---

## Task 14: Phase 4A — Unified Report

**Files:**
- Create: `docs/audit/unified-report.md`
- Read: All previous reports

**Parallelizable:** No — depends on all previous tasks

- [ ] **Step 1: Read all reports and findings**

Read all 10+ documents produced so far.

- [ ] **Step 2: Identify systemic patterns**

Find issues that appear across 3+ agents. These are architectural, not cosmetic.
Examples: "ARIA is missing everywhere", "Error handling is inconsistent across all stores", "No component handles empty states well."

- [ ] **Step 3: Write unified report**

Create `docs/audit/unified-report.md`:

```markdown
# Unified Audit Report — Reasonance

**Date:** 2026-03-22
**Audited by:** 7 personas (Vibecoder, CTO, UX/UI Designer, Security, i18n, Stress, Performance)

## Executive Summary
[1-page overview: current state, biggest wins, biggest risks, overall verdict]

## Systemic Patterns
[Issues appearing across 3+ agents — these are the real problems]

## Component Health Summary
| Component | Vibecoder | CTO | UX/UI | Security | i18n | Stress | Perf | Overall |
[Ranked worst to best]

## Top 20 Critical Issues
[Cross-persona evidence, impact, fix]

## Promise vs Reality
| Reasonance Claims | Evidence | Verdict |
| "Built for every human" | [a11y findings] | ? |
| "Secure by design" | [security findings] | ? |
| "Native speed" | [performance findings] | ? |
| "AI-native" | [vibecoder findings] | ? |

## Competitive Position
[From competitive matrix: where Reasonance leads, trails, is unique]
```

- [ ] **Step 4: Commit**

```bash
git add -f docs/audit/unified-report.md
git commit -m "docs(audit): add unified cross-persona report"
```

---

## Task 15: Phase 4B — Actionable Issue List

**Files:**
- Create: `docs/audit/issues.md`

**Parallelizable:** Can run in parallel with Task 14

- [ ] **Step 1: Collect all issues from all reports**

Extract every finding with severity P0-P4 from all reports.
Deduplicate: same issue found by multiple agents → single entry with all personas listed.

- [ ] **Step 2: Write issue list**

Create `docs/audit/issues.md`:

```markdown
# Audit Issue List

**Date:** 2026-03-22
**Total issues:** ?

## Summary
| Severity | Count |
|----------|-------|
| P0 Blocker | ? |
| P1 Critical | ? |
| P2 Major | ? |
| P3 Minor | ? |
| P4 Enhancement | ? |

## Issues

### P0 — Blocker

#### ISSUE-001: [Title]
- **Severity:** P0
- **Component:** [which]
- **Found by:** [which personas]
- **WCAG criterion:** [if applicable]
- **Description:** [what's wrong]
- **Impact:** [who is affected and how]
- **Suggested fix:** [specific, actionable]
- **Files:** [exact paths]

[Repeat for all issues, grouped by severity]
```

- [ ] **Step 3: Commit**

```bash
git add -f docs/audit/issues.md
git commit -m "docs(audit): add actionable issue list"
```

---

## Task 16: Phase 4C — Priority Roadmap

**Files:**
- Create: `docs/audit/priority-roadmap.md`

**Parallelizable:** No — depends on Task 15

- [ ] **Step 1: Score each issue by impact × effort**

Impact (1-5): How many users affected? How severely?
Effort (1-5): How many files? How complex? How risky?
Priority = Impact / Effort (higher = do first)

- [ ] **Step 2: Group into sprints**

- **Sprint 1 (Urgent):** P0 blockers + high-impact/low-effort P1s
- **Sprint 2 (Critical):** Remaining P1s + high-impact P2s
- **Sprint 3 (Important):** Remaining P2s + systemic patterns
- **Sprint 4 (Polish):** P3s and P4s
- **Backlog:** Nice-to-haves, long-term improvements

- [ ] **Step 3: Write roadmap**

Create `docs/audit/priority-roadmap.md`:

```markdown
# Priority Roadmap

**Date:** 2026-03-22
**Based on:** Comprehensive audit of [total] issues

## Sprint 1: Urgent (estimated: X days)
**Focus:** Blockers and quick wins

| # | Issue | Impact | Effort | Priority Score |
[Issues ranked]

## Sprint 2: Critical
...

## Sprint 3: Important
...

## Sprint 4: Polish
...

## Backlog
...

## Dependencies
[Issues that must be fixed before others]

## Recommended Order of Attack
[Narrative: what to fix first and why]
```

- [ ] **Step 4: Commit**

```bash
git add -f docs/audit/priority-roadmap.md
git commit -m "docs(audit): add priority roadmap"
```

---

## Task 17: Final Commit & Summary

- [ ] **Step 1: Verify all 13+ deliverables exist**

Run: `ls -la docs/audit/`
Expected: competitive-matrix, vibecoder-report, cto-report, uxui-report, security-report, i18n-report, stress-report, performance-report, nielsen-scorecard, wcag-matrix, unified-report, issues, priority-roadmap + supporting files.

- [ ] **Step 2: Final commit**

```bash
git add -f docs/audit/
git commit -m "docs(audit): complete comprehensive multi-persona audit

7-perspective audit covering vibecoder UX, CTO technical review,
accessibility/WCAG compliance, security, i18n/RTL, stress testing,
and performance. Includes Nielsen scorecard, WCAG matrix,
competitive analysis, and prioritized fix roadmap."
```

---

## Parallelization Map

```
Task 0 (Prerequisites)
  │
  ▼
Task 1 (Competitive Matrix)
  │
  ▼
Tasks 2-8 (ALL 7 AGENTS IN PARALLEL)
  │ Vibecoder, CTO, UX/UI, Security, i18n, Stress, Performance
  │
  ├──────────────────────────┬───────────────────────────────┐
  ▼                          ▼                               ▼
Task 9 (Cross-Analysis)    Tasks 10-13 (Visual + Adversarial)
  Nielsen + WCAG matrix      SEQUENTIAL (app must be running)
  │                          10 → 11 → 12 → 13
  │                          │
  ├──────────────────────────┘
  ▼
Task 14 (Unified Report) ──┐
Task 15 (Issue List) ───────┤  PARALLEL
  │                         │
  ▼                         │
Task 16 (Priority Roadmap) ◄┘
  │
  ▼
Task 17 (Final verification)
```

**Key:** Tasks 9 and 10-13 can run in parallel after Phase 1 completes. Task 9 does code-level cross-analysis while Tasks 10-13 do live app testing. Both feed into Tasks 14-16.
