# i18n / RTL Audit Report

**Date:** 2026-03-22
**Persona:** Non-English users across 9 locales (en, ar, de, es, fr, hi, it, pt, zh)
**Judgment:** Does a non-English user get a first-class experience?

## Executive Summary

**Verdict: No. Non-English users get a degraded experience.**

The core UI (menus, settings, editor, terminal) is well-translated for 8 locales, but a large block of keys added for the analytics dashboard and provider settings remain in English across 7 of 8 non-English locales. Italian is the only locale that was fully carried through the analytics/provider translation pass. Arabic RTL support is structurally wired (the `dir` attribute is set on `<html>`) but the CSS uses physical properties (`margin-left`, `padding-left`, `text-align: left`, `border-left`, `left:`, `right:`) throughout -- none have been converted to logical properties. The font stack has no CJK or Devanagari fallback. There are 60+ hardcoded English strings in component templates.

**Key numbers:**
- 226 total i18n keys
- 67-70 keys untranslated per locale (except Italian: 1)
- 60+ hardcoded English `title=`, `aria-label=`, `placeholder=` attributes
- 0 CSS logical properties used; 80+ physical directional properties
- 0 CJK/Devanagari fonts in the fallback chain

---

## Locale Completeness Matrix

All locale files have exactly 226 keys (no missing, no extra). However, many keys have values identical to English, meaning they were copied but never translated.

| Locale | Total Keys | Missing | Extra | Identical to EN | Genuinely Untranslated | Completion % |
|--------|-----------|---------|-------|-----------------|----------------------|-------------|
| en     | 226       | --      | --    | --              | --                   | 100%        |
| ar     | 226       | 0       | 0     | 75              | 67                   | 67%         |
| de     | 226       | 0       | 0     | 88              | 68                   | 61%         |
| es     | 226       | 0       | 0     | 87              | 68                   | 62%         |
| fr     | 226       | 0       | 0     | 88              | 70                   | 61%         |
| hi     | 226       | 0       | 0     | 75              | 67                   | 67%         |
| it     | 226       | 0       | 0     | 16              | 1                    | 93%         |
| pt     | 226       | 0       | 0     | 87              | 67                   | 62%         |
| zh     | 226       | 0       | 0     | 75              | 67                   | 67%         |

**Note:** "Genuinely untranslated" excludes technical terms that are legitimately the same across languages (Git, Commit, Push, Pull, Log, Terminal, System, Code, Cache, Analytics, Endpoint, Provider, Font, Editor, Global, Swarm, ON, OFF, ok, 7d, 14d, 30d).

### Untranslated key blocks (common to ar, de, es, fr, hi, pt, zh)

The following key families are entirely in English across 7 locales:

- `toolbar.analytics` (1 key)
- `analytics.bar.*` (7 keys)
- `analytics.dashboard.*` (18 keys)
- `analytics.insights.*` (7 keys)
- `analytics.budget.*` (6 keys)
- `analytics.a11y.*` (4 keys)
- `settings.provider.*` (31 keys)

**Total: 74 keys per locale left in English.**

Italian (`it.json`) is the only non-English locale where these blocks were translated.

---

## RTL (Arabic) Findings

### Infrastructure

The i18n system correctly:
- Derives `isRTL` from the locale store (`src/lib/i18n/index.ts:6`)
- Sets `document.documentElement.dir = 'rtl'` when Arabic is active (`src/lib/i18n/index.ts:74`)

### Components that mirror correctly

None can be confirmed as RTL-safe. While the `dir="rtl"` attribute is set on the document root, no component CSS uses logical properties, and no `[dir="rtl"]` overrides exist anywhere in the codebase.

### Components that break in RTL

Every component using physical directional CSS will break. The most impactful:

| Component | File | Issue |
|-----------|------|-------|
| App layout | `App.svelte:179,204,259,270,284-285,293-294` | `left:`, `right:`, `border-left`, `border-right` for panel positioning |
| FileTree | `FileTree.svelte:150,209,215` | `padding-left` for indent depth, `border-left`, `text-align: left` |
| Toolbar | `Toolbar.svelte:158,188,213,256,292` | `margin-right`, `margin-left`, `right:`, `text-align: left` |
| MenuItem | `MenuItem.svelte:137,159,173,180,195` | `left:`, `text-align: left`, `margin-left` for submenu positioning |
| TerminalManager | `TerminalManager.svelte:485,497,664` | `left:`, `text-align: left` |
| AnalyticsDashboard | `AnalyticsDashboard.svelte:756,796,806,1082,1088,1121,1158,1230,1288` | Multiple `margin-left`, `right:`, `text-align: left`, `border-right` |
| Settings | `Settings.svelte:1380` | `text-align: right` |
| ResponsePanel | `ResponsePanel.svelte:59,63,168,179,194` | `right:`, `border-left`, `padding-left`, `text-align: left` |
| Toast | `Toast.svelte:75,91` | `right:`, `border-left` |
| FindInFiles | `FindInFiles.svelte:336` | `text-align: right` |
| ContextMenu | `ContextMenu.svelte:110,169` | `left:` (dynamic positioning), `text-align: left` |
| EditorTabs | `EditorTabs.svelte:102,113` | `border-left`, `border-right` |
| WelcomeScreen | `WelcomeScreen.svelte:73-74,213` | `left:`, `right:`, `text-align: left` |
| MarkdownPreview | `MarkdownPreview.svelte:114,137,152` | `border-left`, `text-align: left`, `padding-left` |
| SwarmCanvas | `SwarmCanvas.svelte:300,324` | `border-left` |
| WorkflowMenu | `WorkflowMenu.svelte:179,199,214` | `left:`, `text-align: left`, `padding-left` |
| TerminalToolbar | `TerminalToolbar.svelte:230,242-243,258` | `left:`, `right:`, `text-align: left` |
| ChatInput | `chat/ChatInput.svelte` | Hardcoded English `placeholder` and `aria-label` |
| TextBlock | `chat/TextBlock.svelte:66,75,88` | `border-left`, `padding-left`, `text-align: left` |
| ErrorBlock | `chat/ErrorBlock.svelte:32` | `border-left` |
| AnalyticsBar | `AnalyticsBar.svelte:321,424` | `left:`, `margin-left: auto` |
| DiffBlock | `chat/DiffBlock.svelte:78` | `text-align: left` |

### Missing logical properties

**80+ instances** of physical directional properties need conversion:

| Physical Property | Logical Replacement | Instances |
|-------------------|---------------------|-----------|
| `margin-left` | `margin-inline-start` | ~8 |
| `margin-right` | `margin-inline-end` | ~3 |
| `padding-left` | `padding-inline-start` | ~8 |
| `padding-right` | `padding-inline-end` | ~0 |
| `text-align: left` | `text-align: start` | ~15 |
| `text-align: right` | `text-align: end` | ~2 |
| `left:` / `right:` | `inset-inline-start` / `inset-inline-end` | ~20 |
| `border-left` | `border-inline-start` | ~12 |
| `border-right` | `border-inline-end` | ~5 |

### Scrollbar position

Scrollbar CSS (`::-webkit-scrollbar` in `app.css:197-218`) does not account for RTL. In RTL, scrollbars should appear on the left side. WebKit-based engines generally handle this automatically when `dir="rtl"` is set, but this should be verified.

---

## Long Label (German) Findings

German translations are consistently 1.4x-2.1x longer than English equivalents. Key risk areas:

| Key | EN Length | DE Length | Ratio | Risk |
|-----|----------|----------|-------|------|
| `settings.llm.apiKeyEnv` | 15 | 31 | 2.1x | HIGH - likely overflows in Settings form labels |
| `search.errorOpen` | 19 | 34 | 1.8x | MEDIUM - toast/status text |
| `settings.unsavedHint` | 42 | 66 | 1.6x | MEDIUM - hint text below Settings |
| `terminal.configHint` | 46 | 71 | 1.5x | MEDIUM - empty state message |
| `search.noMatch` | 24 | 35 | 1.5x | LOW - search results area has space |

### Truncation handling

Many components use `text-overflow: ellipsis` with `overflow: hidden` and `white-space: nowrap`, which will silently truncate German labels:

- `EditorTabs.svelte:144-145` -- tab names truncated
- `FileTree.svelte:250-252` -- file names truncated (acceptable)
- `Toolbar.svelte:159-160` -- toolbar labels truncated
- `Settings.svelte:1016-1018,1246-1248,1335-1337` -- provider names/labels truncated
- `AnalyticsBar.svelte:362-363` -- analytics labels truncated
- `WelcomeScreen.svelte:220-222` -- recent project paths truncated (acceptable)
- `ContextMenu.svelte:170-172` -- context menu items truncated

**Concern:** Labels like "API-Schluessel Umgebungsvariable" (31 chars) in Settings form labels likely get cut off. The Settings component uses fixed-width label columns without `min-width` or wrapping.

---

## CJK / Devanagari Findings

### Encoding

All JSON files are valid UTF-8. Chinese (`zh.json`) uses proper CJK characters. Hindi (`hi.json`) uses proper Devanagari script. No encoding issues detected.

### Font-family fallback chain

**Critical issue.** The font stack in `app.css:89-90`:

```css
--font-ui: 'Atkinson Hyperlegible Next', system-ui, -apple-system, sans-serif;
--font-mono: 'Atkinson Hyperlegible Mono', monospace;
```

- **No CJK fonts** in the fallback chain. `system-ui` and `sans-serif` may resolve to a CJK-capable font on systems with CJK locale, but this is not guaranteed on all platforms (especially Linux).
- **No Devanagari fonts** specified. Same reliance on system defaults.
- **Atkinson Hyperlegible** does not include CJK or Devanagari glyphs, so the browser will always fall back.

**Recommendation:** Add explicit CJK and Devanagari fonts:
```css
--font-ui: 'Atkinson Hyperlegible Next', 'Noto Sans SC', 'Noto Sans Devanagari',
           'Microsoft YaHei', 'PingFang SC', system-ui, sans-serif;
```

### Line-breaking rules

No `word-break` or `overflow-wrap` rules target CJK text. CJK text naturally breaks between characters, but compound terms or mixed Latin/CJK content may not wrap correctly without:
```css
word-break: break-word;
overflow-wrap: break-word;
```

### Number formatting

The `t()` function in `index.ts` uses simple string interpolation for numbers (`{count}`, `{percent}`). There is no `Intl.NumberFormat` usage for locale-aware number formatting. Numbers like "1,234" vs "1.234" will display in English format regardless of locale.

---

## Dynamic Locale Switching

### Implementation (`src/lib/i18n/index.ts`)

- **Locale store:** `writable<Locale>('en')` (Svelte store)
- **Lazy loading:** Non-English locales are loaded on demand via dynamic `import()`
- **Derived reactive translator:** `tr` is a `derived` store that returns a translation function
- **Non-reactive `t()`:** The standalone `t()` function uses `get(locale)` which reads the current value but does NOT react to changes. Components using `t()` directly will NOT re-render on locale switch.
- **`dir` attribute:** Updated via `locale.subscribe()` in `initI18n()`

### Reactivity issues

| Pattern | Reactive? | Used where |
|---------|-----------|-----------|
| `$tr('key')` | Yes -- re-renders when locale changes | Components using the `tr` derived store |
| `t('key')` | No -- reads current value once | Any component calling `t()` outside a reactive context |

**Risk:** If any component uses `t()` in a non-reactive context (e.g., in a `<script>` block variable assignment), it will show stale translations after locale switch.

### Date/number formatting

- No `Intl.DateTimeFormat` or `Intl.NumberFormat` usage detected
- Dates and numbers are not locale-aware
- Cost values in analytics (`$0.42`) will always display with dollar sign and English decimal separator

---

## Hardcoded English Strings

| # | File | Line | String | Context |
|---|------|------|--------|---------|
| 1 | `ImageDrop.svelte` | 87 | `"Terminal — drop images here"` | `aria-label` |
| 2 | `ImageDrop.svelte` | 91 | `"Drop image to paste"` | Visible overlay text |
| 3 | `FileTree.svelte` | 143 | `"File explorer"` | `aria-label` |
| 4 | `Toolbar.svelte` | 77 | `"Git commands"` | `title` |
| 5 | `Toolbar.svelte` | 95 | `"YOLO mode — auto-approve permissions (restarts all instances)"` | `title` |
| 6 | `Toolbar.svelte` | 107 | `"Settings"` | `title` and `aria-label` |
| 7 | `Toolbar.svelte` | 109 | `"Minimize"` | `title` and `aria-label` |
| 8 | `Toolbar.svelte` | 110 | `"Maximize"` | `title` and `aria-label` |
| 9 | `Toolbar.svelte` | 111 | `"Close"` | `title` and `aria-label` |
| 10 | `FindInFiles.svelte` | 112 | `"Close"` | `aria-label` |
| 11 | `Settings.svelte` | 428 | `"e.g. Claude"` | `placeholder` |
| 12 | `Settings.svelte` | 442 | `"e.g. claude"` | `placeholder` |
| 13 | `Settings.svelte` | 446 | `"e.g. --no-update-notification"` | `placeholder` |
| 14 | `Settings.svelte` | 450 | `"e.g. --dangerously-skip-permissions"` | `placeholder` |
| 15 | `Settings.svelte` | 471 | `"e.g. ANTHROPIC_API_KEY"` | `placeholder` |
| 16 | `Settings.svelte` | 475 | `"e.g. claude-opus-4-5"` | `placeholder` |
| 17 | `Settings.svelte` | 480 | `"https://api.example.com/v1"` | `placeholder` |
| 18 | `Settings.svelte` | 685 | `"8192"` | `placeholder` |
| 19 | `Settings.svelte` | 702 | `"e.g. ANTHROPIC_API_KEY"` | `placeholder` |
| 20 | `Settings.svelte` | 784 | `"No limit"` | `placeholder` |
| 21 | `Settings.svelte` | 799 | `"No limit"` | `placeholder` |
| 22 | `SwarmPanel.svelte` | 41 | `"Open full canvas"` | `title` |
| 23 | `ViewModeToggle.svelte` | 13 | `"Toggle between chat and terminal view"` | `aria-label` |
| 24 | `NodeCatalog.svelte` | 15 | `"Add Agent Node"` | `title` |
| 25 | `NodeCatalog.svelte` | 18 | `"Add Resource Node"` | `title` |
| 26 | `NodeCatalog.svelte` | 21 | `"Add Logic Node"` | `title` |
| 27 | `WelcomeScreen.svelte` | 21 | `"Toggle theme"` | `title` |
| 28 | `WelcomeScreen.svelte` | 25 | `"Minimize"` | `title` |
| 29 | `WelcomeScreen.svelte` | 26 | `"Maximize"` | `title` |
| 30 | `WelcomeScreen.svelte` | 27 | `"Close"` | `title` |
| 31 | `Terminal.svelte` | 270 | `"Find in terminal..."` | `placeholder` |
| 32 | `Terminal.svelte` | 281 | `"Previous match"` | `aria-label` |
| 33 | `Terminal.svelte` | 282 | `"Next match"` | `aria-label` |
| 34 | `Terminal.svelte` | 283 | `"Close search"` | `aria-label` |
| 35 | `TerminalManager.svelte` | 255 | `"LLM sessions"` | `aria-label` |
| 36 | `TerminalManager.svelte` | 270 | `"Add LLM"` | `aria-label` |
| 37 | `TerminalManager.svelte` | 341 | `"Terminal instances"` | `aria-label` |
| 38 | `TerminalManager.svelte` | 355 | `"Close tab"` | `aria-label` |
| 39 | `TerminalToolbar.svelte` | 69 | `"Add file to context"` | `title` |
| 40 | `TerminalToolbar.svelte` | 76 | `"Slash commands"` | `title` |
| 41 | `AnalyticsDashboard.svelte` | 276 | `"Analytics Dashboard"` | `aria-label` |
| 42 | `AnalyticsDashboard.svelte` | 285 | `"Time period"` | `aria-label` |
| 43 | `AnalyticsDashboard.svelte` | 338 | `"Key metrics"` | `aria-label` |
| 44 | `AnalyticsDashboard.svelte` | 453 | `"Dismiss insight"` | `aria-label` |
| 45 | `AnalyticsDashboard.svelte` | 557 | `"Details for {provider.provider}"` | `aria-label` |
| 46 | `AnalyticsDashboard.svelte` | 589 | `"Model breakdown for {provider.provider}"` | `aria-label` |
| 47 | `AnalyticsDashboard.svelte` | 638 | `"Daily token trend"` | `aria-label` |
| 48 | `Toast.svelte` | 50 | `"Dismiss notification"` | `aria-label` |
| 49 | `SwarmControls.svelte` | 65 | `"Play"` | `title` |
| 50 | `SwarmControls.svelte` | 67 | `"Pause"` | `title` |
| 51 | `SwarmControls.svelte` | 69 | `"Resume"` | `title` |
| 52 | `SwarmControls.svelte` | 71 | `"Stop"` | `title` |
| 53 | `SwarmControls.svelte` | 72 | `"Step"` | `title` |
| 54 | `SwarmCanvas.svelte` | 188 | `"Close canvas"` | `title` |
| 55 | `WorkflowMenu.svelte` | 120 | `"Workflow actions"` | `title` |
| 56 | `App.svelte` | 108 | `"File explorer"` | `aria-label` |
| 57 | `App.svelte` | 125 | `"Resize file tree"` | `aria-label` |
| 58 | `App.svelte` | 148 | `"Resize terminal"` | `aria-label` |
| 59 | `App.svelte` | 152 | `"Terminal"` | `aria-label` |
| 60 | `StreamingIndicator.svelte` | 1 | `"Agent is responding"` | `aria-label` |
| 61 | `ChatInput.svelte` | 25 | `"Send a message..."` | `placeholder` |
| 62 | `ChatInput.svelte` | 28 | `"Message input"` | `aria-label` |
| 63 | `ChatInput.svelte` | 34 | `"Send message"` | `aria-label` |
| 64 | `CodeBlock.svelte` | 17 | `"Copy code"` | `aria-label` |
| 65 | `ActionableMessage.svelte` | 50 | `"Copy message"` | `aria-label` |
| 66 | `ActionableMessage.svelte` | 54 | `"Fork from here"` | `aria-label` |
| 67 | `FileRefBadge.svelte` | 30 | `"{action}: {path}"` | `title` (dynamic but untranslated prefix) |
| 68 | `SwarmPanel.svelte` | 50 | `"{ns.node_id}: {ns.state}"` | `title` |

**All files are under `src/lib/components/`.** Full paths: `src/lib/components/<filename>` or `src/lib/components/chat/<filename>` or `src/lib/components/swarm/<filename>`.

---

## Recommendations (Priority Order)

### P0 -- Blocking for Arabic users
1. **Convert all physical CSS properties to logical properties.** This is the single largest RTL issue. ~80 instances across ~20 components.
2. **Add `[dir="rtl"]` overrides** for any cases where logical properties alone are insufficient (e.g., absolute positioning of dropdown menus in `ContextMenu.svelte`, `MenuItem.svelte`).

### P1 -- Blocking for non-English users
3. **Translate the analytics and provider settings key blocks** for ar, de, es, fr, hi, pt, zh. ~74 keys x 7 locales = ~518 translations needed.
4. **Move all 68 hardcoded English strings** to i18n keys and use `$tr()` in templates.

### P2 -- Quality issues
5. **Add CJK and Devanagari fonts** to the `--font-ui` fallback chain.
6. **Add `Intl.NumberFormat` and `Intl.DateTimeFormat`** for locale-aware number and date rendering.
7. **Audit the `t()` vs `$tr()` usage** -- ensure all components use the reactive `$tr()` derived store, not the non-reactive `t()`.
8. **Test German label overflow** in Settings form, Toolbar, and AnalyticsBar components. Add `min-width` or allow wrapping for label elements.
9. **Add `word-break: break-word`** for CJK content in text-heavy areas (chat messages, markdown preview).
