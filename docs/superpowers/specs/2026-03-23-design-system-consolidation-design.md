# Design System Consolidation — WCAG AAA

**Date:** 2026-03-23
**Type:** Refactoring + Accessibility
**Scope:** All 53 Svelte components + `app.css`
**Goal:** Consolidate 479+ hardcoded style values into a tokenized, WCAG AAA compliant design system

---

## Problem

The design system tokens exist in `app.css` but components ignore them. The audit found:

| Category | Hardcoded instances |
|----------|-------------------|
| Font sizes | 100+ (13 distinct px values) |
| Padding/margin/gap | 300+ (16 distinct values) |
| Colors | 20+ hex values |
| Border-radius | 11 (in a brutalist --radius: 0 system) |
| Heights | 20+ |
| Z-index | 12 ad-hoc values |
| Transitions | 4 durations, inconsistent |

This causes: visual inconsistency, theme switching bugs (`#fff` on light theme = invisible), inability to change design in one place, WCAG AAA violations.

---

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Typography scale | 4px grid, rem units, base 16px | Clean rem values (1rem = browser default), WCAG AAA readable |
| Minimum font size | 12px (0.75rem) | 8px dropped — inaccessible in practice |
| Spacing scale | 4px grid, rem, semantic layer on top | Consistent, enhanced-readability scales proportionally |
| Border-radius | 0 everywhere, 50% only for circles | Brutalist, no exceptions |
| Z-index | Layer manager with isolation: isolate | 6 semantic layers, focus trap, inert, portal |
| Buttons | Two sizes, both 44px min-height | WCAG 2.5.8 Target Size AAA, no tricks |
| Transitions | 2 tokens (fast/normal), functional vs decorative | Zero flash (2.3.2 AAA), reduced-motion respected |
| Colors | OKLCH architecture, zero hex in components | Contrast calculable by construction |
| Migration | Phased (10 phases), one spec, autonomous execution | Each phase = commit + verification |
| Widths | Not tokenized | Functional layout choices, not design system concern |

---

## 1. Typography

### Scale

| Token | Value | Px equiv. | Line-height | Role |
|-------|-------|-----------|-------------|------|
| `--font-size-sm` | 0.75rem | 12px | 1.6 | Minimum. Labels, meta, captions |
| `--font-size-base` | 1rem | 16px | 1.5 | UI text, code, terminal |
| `--font-size-md` | 1.25rem | 20px | 1.4 | Section headings |
| `--font-size-lg` | 1.5rem | 24px | 1.3 | Main headings (large text threshold) |
| `--font-size-hero` | 2rem | 32px | 1.2 | Hero titles |

### Line-height tokens

```css
--line-height-sm:   1.6;
--line-height-base: 1.5;
--line-height-md:   1.4;
--line-height-lg:   1.3;
--line-height-hero: 1.2;
```

### Font-weight per tier

| Tier | Dark theme | Light theme | Rationale |
|------|-----------|-------------|-----------|
| sm (12px) | 500 | 500 | Small text needs more weight |
| base (16px) — body text | 500 | 400 | Body text: medium weight on dark for legibility |
| base (16px) — UI labels | 400 | 400 | UI labels: regular weight sufficient at 16px |
| md (20px) | 700 | 700 | Section headings |
| lg (24px) | 700 | 700 | Main headings |
| hero (32px) | 800 | 700 | Hero titles, maximum impact |

### Semantic aliases

```css
--font-size-code:     var(--font-size-base);
--font-size-terminal: var(--font-size-base);
```

### Text measure

```css
--measure: 72ch;  /* max-width for prose blocks — WCAG 1.4.8: max 80 chars/line */
```

Applied to: MarkdownPreview, ResponsePanel, ChatMessages — anywhere continuous text is read.

### WCAG contrast thresholds

| Size | Weight | Classification | Required contrast |
|------|--------|---------------|-------------------|
| sm (12px) | any | Normal text | 7:1 (AAA) |
| base (16px) | any | Normal text | 7:1 (AAA) |
| md (20px) | 700+ | Large text | 4.5:1 (AAA) |
| md (20px) | <700 | Normal text | 7:1 (AAA) |
| lg (24px) | any | Large text | 4.5:1 (AAA) |
| hero (32px) | any | Large text | 4.5:1 (AAA) |

### Mapping current values

| Current value | Where used | Becomes |
|--------------|------------|---------|
| 8-9px | ChatInput badge, AnalyticsDashboard micro | `--font-size-sm` (12px) — bumped up |
| 10-11px | MenuItem, node labels, tiny text | `--font-size-sm` (12px) — bumped up |
| 12-13px | Toolbar, tabs, tree items, buttons | `--font-size-sm` (12px) |
| 14-15px | Main text, editor, terminal | `--font-size-base` (16px) |
| 16px | MarkdownPreview paragraphs | `--font-size-base` (16px) |
| 18px | TextBlock h1 | `--font-size-md` (20px) |
| 20-22px | WelcomeScreen titles | `--font-size-lg` (24px) |
| 32px | WelcomeScreen hero | `--font-size-hero` (32px) |

### Enhanced readability

```css
:root.enhanced-readability {
  --font-size-sm:   0.875rem;  /* 14px vs 12px */
  --font-size-base: 1.125rem;  /* 18px vs 16px */
  --font-size-md:   1.375rem;  /* 22px vs 20px */
  --font-size-lg:   1.75rem;   /* 28px vs 24px */
  --font-size-hero: 2.5rem;    /* 40px vs 32px */
  --line-height-base: 1.8;
  --letter-spacing: 0.05em;
  --word-spacing: 0.1em;
}
```

---

## 2. Spacing

### Primitive scale (4px grid, rem)

```css
--space-1: 0.25rem;   /* 4px */
--space-2: 0.5rem;    /* 8px */
--space-3: 0.75rem;   /* 12px */
--space-4: 1rem;      /* 16px */
--space-5: 1.5rem;    /* 24px */
--space-6: 2rem;      /* 32px */
```

### Semantic layer

```css
--inset-component:    var(--space-3);    /* padding inside a component (card, panel) */
--inset-section:      var(--space-4);    /* padding inside a section */
--stack-tight:        var(--space-1);    /* vertical gap between related elements */
--stack-normal:       var(--space-2);    /* standard vertical gap */
--stack-loose:        var(--space-4);    /* vertical gap between groups */
--interactive-gap:    var(--space-2);    /* minimum gap between clickable targets */
--paragraph-spacing:  calc(1.5 * 1.5em); /* WCAG 1.4.8: >= 1.5x line-height */
```

### Button/input aliases

```css
--btn-padding:    var(--space-2) var(--space-3);   /* 8px 12px */
--btn-padding-sm: var(--space-1) var(--space-2);   /* 4px 8px */
--panel-padding:  var(--space-4);                   /* 16px */
--input-padding:  var(--space-2) var(--space-3);   /* 8px 12px */
```

### Mapping current values

| Current | Becomes |
|---------|---------|
| 1-2px | `--space-1` (4px) |
| 3-6px | `--space-1` (4px) or `--space-2` (8px) by context |
| 7-10px | `--space-2` (8px) |
| 12-14px | `--space-3` (12px) |
| 16-18px | `--space-4` (16px) |
| 20-24px | `--space-5` (24px) |
| 32px | `--space-6` (32px) |

### WCAG 1.4.12 — text spacing override resilience

Rules for all components:
- Never use fixed `height` on text containers — only `min-height`
- Never use `overflow: hidden` on text-containing elements
- Vertical padding in `em` where content is text (scales with user override)
- Layout must not break when user applies: line-height 1.5x, letter-spacing 0.12em, word-spacing 0.16em, paragraph spacing 2x font-size

### Enhanced readability — proportional spacing

```css
:root.enhanced-readability {
  --space-1: 0.375rem;   /* 6px vs 4px */
  --space-2: 0.625rem;   /* 10px vs 8px */
  --space-3: 1rem;       /* 16px vs 12px */
  --space-4: 1.25rem;    /* 20px vs 16px */
  --space-5: 1.75rem;    /* 28px vs 24px */
  --space-6: 2.5rem;     /* 40px vs 32px */
}
```

Semantic tokens inherit automatically since they're aliases.

---

## 3. Layer System

### CSS layers

```css
--layer-base:      0;
--layer-dropdown:  1;
--layer-sticky:    2;
--layer-overlay:   3;
--layer-modal:     4;
--layer-toast:     5;
```

Each layer element uses `isolation: isolate` to create explicit stacking contexts. Low numbers because isolated contexts don't compete.

### Component mapping

| Component | Current z-index | New layer |
|-----------|----------------|-----------|
| App layout | 10-11 | `--layer-base` |
| ResponsePanel | 50 | `--layer-base` |
| TerminalToolbar menu, WorkflowMenu | 100 | `--layer-dropdown` |
| AnalyticsBar dropdown, Toolbar menu | 200 | `--layer-dropdown` |
| ContextMenu, FileTree context | 999-1001 | `--layer-dropdown` |
| MenuItem, submenu | 1000-1001 | `--layer-dropdown` |
| WelcomeScreen | 10 | `--layer-overlay` |
| App overlay/backdrop | 9998 | `--layer-overlay` |
| SessionPanel, SearchPalette, ShortcutsDialog, FindInFiles | 2000 | `--layer-modal` |
| Settings | 2000 | `--layer-modal` |
| Toast | 9999 | `--layer-toast` |

### Layer manager (Svelte store)

New file: `src/lib/stores/layerManager.ts`

Centralized store managing the layer stack:

```typescript
// Conceptual API:
interface LayerEntry {
  id: string;
  type: 'dropdown' | 'overlay' | 'modal' | 'toast';
  returnFocus: HTMLElement | null;
}

// Stack operations:
layerManager.push(entry)   // open layer, apply inert to layers below
layerManager.pop()         // close top layer, restore inert, return focus
layerManager.getTop()      // current top layer
```

Responsibilities:
- **Escape LIFO**: single global listener, pops from stack
- **`inert` automatic**: manager applies/removes inert on all layers below top
- **Focus return**: each layer registers where to return focus on close
- **Nesting**: supports N depth without components knowing each other

### Focus fallback chain

When the element that opened a layer no longer exists on close:

```
1. Original element (if still in DOM and visible)
2. Nearest focusable element in same container
3. Container itself if focusable
4. document.body (last resort)
```

### Scroll lock without layout shift

```css
body.layer-modal-open {
  overflow: hidden;
  scrollbar-gutter: stable;
}
```

Layer manager adds/removes this class automatically.

### Portal pattern

```html
<!-- In app.html -->
<div id="layer-portal"></div>
```

All modal/overlay layers render into the portal, not nested inside their parent component. Ensures:
- Screen reader reads layers in correct order (1.3.2 Meaningful Sequence)
- Stacking context isolation works correctly
- z-index doesn't get trapped by parent isolation

### ARIA requirements per layer type

| Layer type | Required attributes |
|-----------|-------------------|
| `modal` | `role="dialog"` + `aria-modal="true"` + `aria-labelledby` |
| `dropdown` | `role="menu"` or `role="listbox"` + `aria-expanded` on trigger |
| `toast` | `role="status"` + `aria-live="polite"` (info) or `role="alert"` (error) |
| `overlay` | `role="dialog"` + `aria-labelledby` (non-modal) |

### Focus trap

Every `modal` and interactive `overlay` layer implements:
- Tab/Shift+Tab cycle within the layer
- Initial focus on first interactive element
- Focus return to opening element on close (via fallback chain)

### Backdrop

- `aria-hidden="true"` — not read by screen reader
- Click closes the modal, but NOT during text selection that extends outside
- No `tabindex` — never receives focus
- In forced-colors mode: `background: Canvas; opacity: 0.8;`

### Forced-colors mode

```css
@media (forced-colors: active) {
  [data-layer="modal"],
  [data-layer="overlay"] {
    border: 2px solid CanvasText;
  }
  [data-layer="dropdown"] {
    border: 1px solid CanvasText;
  }
  .layer-backdrop {
    background: Canvas;
    opacity: 0.8;
  }
}
```

### Focus ring contrast per layer background

The focus ring (`--accent` colored) must have 3:1 contrast against every possible layer background. Verify in the contrast matrix (Phase 10).

---

## 4. Colors

### Architecture: OKLCH

Colors defined by hue and chroma, with lightness derived from contrast requirements:

```css
/* Base hues */
--hue-accent:       220;   /* blue */
--hue-danger:       25;    /* red */
--hue-success:      145;   /* green */
--hue-warning:      85;    /* yellow */
--hue-state-retry:  45;    /* orange */

/* Dark theme: high lightness for text on dark backgrounds */
--accent-text:     oklch(L C var(--hue-accent));
--danger-text:     oklch(L C var(--hue-danger));
--success-text:    oklch(L C var(--hue-success));
--warning-text:    oklch(L C var(--hue-warning));

/* Light theme: low lightness for text on light backgrounds */
/* Exact L values calculated during implementation to meet 7:1 on each background */
```

Advantage: new color = choose hue, calculate L for required contrast. Not trial-and-error.

### New tokens to add

**Code colors:**
```css
--code-bg:       /* dark code block background, OKLCH derived */
--code-accent:   /* syntax highlight accent, OKLCH derived */
```

**State colors (swarm/agent):**
```css
--state-idle:        /* neutral gray */
--state-idle-text:   /* text variant, 7:1 on bg-primary */
--state-queued:      /* from --hue-warning */
--state-queued-text:
--state-running:     /* from --hue-accent */
--state-running-text:
--state-success:     /* from --hue-success */
--state-success-text:
--state-failed:      /* from --hue-danger */
--state-failed-text:
--state-retrying:    /* from --hue-state-retry */
--state-retrying-text:
```

Each with light theme variant. Exact OKLCH values calculated during implementation.

**UI state colors:**
```css
--text-on-accent:    /* text on accent background, 7:1 verified */
--text-disabled:     /* gray, exempt from contrast but distinguishable */
--bg-disabled:
--border-disabled:
--bg-selected:       /* accent-tinted background */
--text-selected:     var(--text-primary);
--text-placeholder:  /* distinguishable from disabled by context */
--focus-ring-color:  /* OKLCH derived, 3:1 on all layer backgrounds */
```

### Rule: zero hex in components

After migration, no `.svelte` file may contain a hardcoded color value. Zero exceptions. If a color is needed, create a token.

### #fff / #000 elimination

All ~20 `color: #fff` and `color: #000` replaced with semantic tokens:
- On accent background button → `--text-on-accent`
- On any theme background → `--text-primary`

### Contrast matrix

Every text color token verified against every background where it's used:

| Text token | On bg-primary | On bg-secondary | On bg-tertiary | On bg-surface |
|-----------|:---:|:---:|:---:|:---:|
| --text-primary | ratio | ratio | ratio | ratio |
| --text-body | ratio | ratio | ratio | ratio |
| --text-secondary | ratio | ratio | ratio | ratio |
| --text-muted | ratio | ratio | ratio | ratio |
| --accent-text | ratio | ratio | — | ratio |
| --danger-text | ratio | ratio | — | — |
| --success-text | ratio | ratio | — | — |
| --warning-text | ratio | ratio | — | — |
| --state-*-text | ratio | ratio | — | — |
| --text-on-accent | on --accent | on --accent-hover | — | — |
| --code-accent | on --code-bg | — | — | — |

Duplicated for light theme. All ratios calculated with both WCAG 2.x and APCA algorithms.

### Alpha color effective contrast

`--bg-hover: rgba(255,255,255,0.06)` — contrast depends on what's behind. Document effective color on each background.

**Method:** For each alpha color, compute the effective opaque color when composited on each background used in the theme. Then calculate contrast of text tokens against that effective color. Example format (actual values computed with OKLCH-derived backgrounds during implementation):

```
bg-hover on bg-primary → effective [computed] → contrast with --text-body: X:1
bg-hover on bg-secondary → effective [computed] → contrast with --text-body: Y:1
```

### Forbidden combinations

Pairs that must never be used together:

| Text | Background | Why |
|------|-----------|-----|
| `--text-muted` | `--bg-tertiary` | Insufficient contrast |
| `--warning-text` | `--bg-tertiary` | Ambiguous hue contrast |
| `--success-text` + `--danger-text` | As sole discriminant | Colorblind indistinguishable |
| Any `--state-*` | Without redundant indicator | Violates 1.4.1 |

Complete list generated from contrast matrix: every pair with ratio < 7:1 (normal text) or < 4.5:1 (large text).

### Redundant indicators (1.4.1 Use of Color)

Wherever color conveys information, a second channel is required:

| Context | Color | Redundant indicator |
|---------|-------|-------------------|
| Agent state: idle | gray | icon pause + label "Idle" |
| Agent state: running | blue | icon spinner + label "Running" |
| Agent state: success | green | icon checkmark + label "Success" |
| Agent state: failed | red | icon X + label "Failed" |
| Agent state: retrying | orange | icon retry + label "Retrying" |
| Diff: addition | green | prefix `+` (already present) |
| Diff: deletion | red | prefix `-` (already present) |
| StatusBar errors | red | text prefix "Error:" |
| StatusBar warnings | yellow | text prefix "Warning:" |

### Colorblind verification

For every group of colors that must be distinguishable from each other, verify distinguishability in **luminosity** (not just hue):

| Group | Colors | Verification |
|-------|--------|-------------|
| Agent states | idle, running, success, failed, retrying | 5 distinct luminosities in grayscale |
| Semantics | danger, warning, success | 3 distinct luminosities |
| Diff | addition, deletion | 2 distinct luminosities + prefix |

Simulations: deuteranopia, protanopia, tritanopia, achromatopsia. If two colors in the same group have similar grayscale luminosity, one gets corrected.

### SVG/icon rule

All icons use `currentColor`. No hardcoded `fill` or `stroke`. This ensures:
- Automatic theme switching
- Forced-colors compliance
- Contrast managed by parent text token

### prefers-contrast: more

```css
@media (prefers-contrast: more) {
  :root {
    --text-body:      #ffffff;
    --text-secondary: #e0e0e0;
    --bg-primary:     #000000;
    --border-color:   #ffffff;
    --border-width:   3px;
  }
}
```

---

## 5. Buttons

### Two variants, one system

| Variant | Min-height | Padding | Font-size | Use |
|---------|-----------|---------|-----------|-----|
| **Normal** | 2.75rem (44px) | `var(--space-2) var(--space-3)` (8px 12px) | `var(--font-size-sm)` (12px) | Primary actions, toolbar, dialogs |
| **Compact** | 2.75rem (44px) | `var(--space-1) var(--space-2)` (4px 8px) | `var(--font-size-sm)` (12px) | Same height, reduced horizontal density |

Both 44px — WCAG 2.5.8 Target Size Enhanced AAA. No visual tricks.

### Common properties

```css
font-family: var(--font-ui);
font-size: var(--font-size-sm);
font-weight: 700;
min-height: 2.75rem;
border: var(--border-interactive);
background: var(--bg-surface);
color: var(--text-primary);
text-transform: uppercase;
letter-spacing: 0.05em;
cursor: pointer;
transition: background var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
```

### Semantic color variants

| Variant | Background | Border | Text |
|---------|-----------|--------|------|
| Default | `--bg-surface` | `--border-color` | `--text-primary` |
| Primary | `--accent` | `--accent` | `--text-on-accent` |
| Danger | `--danger` | `--danger` | `--text-on-accent` |
| Ghost | `transparent` | `transparent` | `--text-secondary` |

### States

| State | Change | WCAG |
|-------|--------|------|
| Hover | Background changes | 3:1 change vs default (1.4.11) |
| Active/Pressed | Darker background, no transform | Visible feedback |
| Focus-visible | `outline: var(--focus-ring)` | 2.4.13 Focus Appearance AAA |
| Disabled | `--bg-disabled`, `--text-disabled`, `--border-disabled`, `cursor: not-allowed` | Exempt from contrast but distinguishable |

### Focus Appearance AAA (2.4.13)

| Requirement | Implementation |
|------------|---------------|
| Indicator area >= 2px perimeter | `outline-offset: 2px` + `outline: 2px solid` = 4px total perimeter |
| 3:1 contrast focused vs unfocused | Focus ring vs background verified per variant |
| Not obscured | Layer system guarantees focus not covered |

Primary button override (accent-on-accent invisible):
```css
button.primary:focus-visible {
  outline-color: var(--text-on-accent);
  outline-offset: 3px;
}
```

### Icon-only buttons

| Rule | Detail | WCAG |
|------|--------|------|
| `aria-label` mandatory | No icon-only button without label | 4.1.2 |
| 44x44px unchanged | Same min-height as text buttons | 2.5.8 |
| Icon 3:1 contrast | Icon itself (not just text) must have 3:1 on background | 1.4.11 |
| `currentColor` | Icon uses currentColor, never hardcoded fill/stroke | Forced-colors |

### Toggle buttons

```html
<button aria-pressed="true|false">
```

Visual distinction not color-only (1.4.1):

| State | Color | Redundant indicator |
|-------|-------|-------------------|
| Pressed | `--bg-selected` | Thicker border (`calc(var(--border-width) * 2)`) + icon change |
| Unpressed | `--bg-surface` | Standard border |

### Loading state

```css
button[aria-busy="true"] {
  cursor: wait;
  opacity: 0.8;
  pointer-events: none;
}
```

- `aria-busy="true"` — screen reader knows action is in progress
- `aria-disabled="true"` during loading (not native `disabled` which removes from tab order)
- Animated spinner only if `prefers-reduced-motion: no-preference`, otherwise static "Loading..." text
- `aria-live="polite"` on result container to announce completion

### Button groups = `role="toolbar"`

```html
<div role="toolbar" aria-label="Main toolbar">
  <button tabindex="0">File</button>
  <button tabindex="-1">Edit</button>
  <button tabindex="-1">View</button>
</div>
```

- Tab enters toolbar (single tab-stop)
- Left/Right arrows navigate between buttons
- Tab exits toolbar to next element
- `aria-label` on toolbar for context

Applies to: Toolbar, EditorTabs, TerminalToolbar, SwarmControls, any horizontal button group.

### Native `<button>` mandatory

All buttons must be `<button>`, never `<div role="button">`. Native gives free: Enter+Space activation, focus, disabled state, form submission, screen reader recognition.

Any `<div role="button">` or `<span @click>` must be migrated.

### Mapping 9 current patterns to 2

| Current pattern | Component | Becomes |
|----------------|-----------|---------|
| Send button (8px 16px, 40px) | ChatInput | Normal + Primary |
| Toolbar button (3px 10px, 26px) | Toolbar | Normal + Default |
| Git trigger (3px 10px, 26px) | Toolbar | Normal + Default |
| Window control (3px 10px, 26px) | App | Normal + Ghost |
| Menu trigger (4px 8px) | MenuBar | Compact + Ghost |
| Menu row (6px 12px) | MenuItem | Compact + Default |
| Tree item (6px 14px) | FileTree | Compact + Ghost |
| Git dropdown (6px 12px) | Toolbar | Compact + Default |
| Small action (4px 6px) | Various | Compact + Ghost |

### Forced-colors

```css
@media (forced-colors: active) {
  button {
    border: 2px solid ButtonText;
  }
  button:hover {
    border-color: Highlight;
    color: Highlight;
  }
  button:focus-visible {
    outline: 2px solid Highlight;
  }
  button[aria-pressed="true"] {
    border-width: 4px;
    background: Highlight;
    color: HighlightText;
  }
  button:disabled,
  button[aria-disabled="true"] {
    border-color: GrayText;
    color: GrayText;
  }
}
```

---

## 6. Transitions

### Tokens

```css
--transition-fast:   0.1s ease;
--transition-normal: 0.2s ease;
```

Current `0.15s` and `0.3s` values rounded to nearest token.

### Usage mapping

| What | Token | Properties |
|------|-------|-----------|
| Hover color/background | `--transition-fast` | `background, color, border-color` |
| Hover list items | `--transition-fast` | `background, color` |
| Opacity fades | `--transition-normal` | `opacity` |
| Panel expand/collapse | `--transition-normal` | `max-height` |

### Rule: no transition on layout properties

Never `transition` on `width`, `margin`, `padding`, `top/left` — causes layout thrashing. Only composite-friendly: `opacity`, `transform`, `background`, `color`, `border-color`, `box-shadow`.

`max-height` is the only exception, for expandable panels only.

### Functional vs decorative motion

| Type | Example | In prefers-reduced-motion |
|------|---------|--------------------------|
| Decorative | Hover fade, panel slide-in | Eliminated (instant) |
| Functional | Loading spinner, progress bar | Replaced with static alternative |

```css
@media (prefers-reduced-motion: reduce) {
  /* Decorative: kill */
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }

  /* Functional: static alternative */
  .spinner {
    animation: none;
    /* show static icon or "Loading..." text */
  }
}
```

### Zero flash — 2.3.2 AAA

AAA = no flashing content, period. Banned:

| Banned | Alternative |
|--------|------------|
| `animation: blink` | Never use |
| Rapid opacity toggle | Single smooth fade |
| Blinking cursor | Default OFF, opt-in via Settings |
| Rapid border color change | Single smooth transition |
| Any `@keyframes` with >1 luminosity change >10% | Rewrite as monotonic transition |

### Cursor blink — default off

```css
/* Default: solid cursor (AAA safe) */
.cursor { opacity: 1; }

/* User opts in via Settings */
:root.cursor-blink .cursor {
  animation: cursor-blink 1s step-end infinite;
}

@media (prefers-reduced-motion: reduce) {
  .cursor { animation: none !important; }
}
```

Setting in Settings.svelte: "Blinking cursor: on/off", default **off**.

### Toast auto-dismiss — 2.2.1

| Toast type | Behavior |
|-----------|----------|
| Informational ("File saved") | Auto-dismiss OK, 5s minimum, hover/focus pauses timer |
| Warning/Error | NEVER auto-dismiss. Stays until user closes |
| With action ("Undo delete") | NEVER auto-dismiss. User needs time to act |

```css
--toast-duration: 5s;  /* informational only */
```

- `aria-live="polite"` for informational
- `role="alert"` for errors (immediate screen reader announcement)
- Hover pauses timer
- Focus pauses timer (keyboard user tabs to toast)

### State change visibility without motion

Every state change visible via transition must have a non-motion indicator in reduced-motion mode:

- Panel appearing: instant appearance + border highlight or focus moved into panel
- Toast appearing: instant + `aria-live` announcement

### Enhanced readability: slower transitions

```css
:root.enhanced-readability {
  --transition-fast:   0.15s ease;
  --transition-normal: 0.3s ease;
}
```

---

## 7. Borders

### Semantic border tokens

```css
/* Structural */
--border-container:   var(--border-width) var(--border-style) var(--border-color);
--border-separator:   1px solid var(--border-color);
--border-interactive: var(--border-width) var(--border-style) var(--border-color);

/* State */
--border-focus:       var(--border-width) solid var(--focus-ring-color);
--border-error:       var(--border-width) solid var(--danger);
--border-selected:    calc(var(--border-width) * 2) solid var(--accent);
```

### Base tokens

```css
--border-width: 2px;
--border-style: solid;
--border-color: var(--border);  /* existing token */
```

### Border-radius

```css
--radius: 0;         /* brutalist, no exceptions */
--radius-full: 50%;  /* circles only (state indicators) */
```

All 11 hardcoded border-radius values (2px, 4px, 6px, 8px) removed.

### Border contrast — 1.4.11

Border must have 3:1 contrast against BOTH adjacent colors (inside and outside):

| Border | Inner bg | Outer bg | Inner contrast | Outer contrast |
|--------|---------|---------|---------------|----------------|
| `--border-color` dark | `--bg-surface` | `--bg-primary` | to verify | to verify |
| `--border-color` dark | `--bg-surface` | `--bg-secondary` | to verify | to verify |
| `--border-color` light | `--bg-surface` | `--bg-primary` | to verify | to verify |

If border doesn't have 3:1 on both sides, buttons on that background are invisible as interactive elements. Corrected during implementation.

### Input borders

| State | Border | Additional indicator | WCAG |
|-------|--------|---------------------|------|
| Default | `--border-interactive` | — | 1.4.11: visible boundary 3:1 |
| Focus | `--border-focus` + outline | Border color change + focus ring | 2.4.13: double indicator |
| Error | `--border-error` | Icon + error text + `aria-invalid="true"` | 1.4.1: not color-only |
| Disabled | `--border-disabled` | `aria-disabled`, cursor | Exempt but distinguishable |

### Semantic separators

```html
<!-- Decorative separator: hidden from screen reader -->
<hr aria-hidden="true" />

<!-- Semantic separator: grouping -->
<li role="separator"></li>
```

`role="separator"` tells screen reader "boundary between groups". `aria-hidden` on decorative `<hr>` prevents reading "separator" 50 times.

### Forced-colors: state via thickness and style

In forced-colors mode, color is system-controlled. Communicate state via border **thickness** and **style**:

```css
@media (forced-colors: active) {
  input, textarea, select,
  [role="listbox"], [role="combobox"] {
    border: 2px solid ButtonText;
  }

  [role="separator"], hr {
    border-color: CanvasText;
  }

  input:focus, textarea:focus {
    border-width: 3px;
  }

  [aria-invalid="true"] {
    border-style: dashed;  /* dashed = error, distinguishable without color */
  }

  [aria-selected="true"],
  [aria-pressed="true"] {
    border-width: 4px;
  }
}
```

### Circular indicators (--radius-full)

The only case of radius. Requirements:
- 3:1 contrast on background (1.4.11)
- Minimum size 24x24px (perceivable as shape)
- In forced-colors: `border: 2px solid CanvasText`

### Enhanced readability + prefers-contrast

```css
:root.enhanced-readability {
  --border-width: 3px;
}

@media (prefers-contrast: more) {
  :root {
    --border-width: 3px;
    --border-color: /* max contrast value */;
  }
}
```

---

## 8. Implementation Phases

One spec, autonomous execution phase by phase. Each phase = commit + verification.

### Phase 1: Tokens in app.css

**What:** Add all new CSS custom properties to `app.css` without touching any component.

**Files:** `app.css` only

**Changes:**
- New typography tokens (scale, line-height per tier)
- New spacing tokens (primitive + semantic)
- New layer tokens
- New color tokens (OKLCH base hues, state colors, UI state colors)
- New button tokens
- New transition tokens
- New border tokens (semantic)
- Enhanced readability overrides for all new tokens
- `prefers-contrast: more` media query
- Remove old tokens that are replaced (after adding new ones)

**Verification:** Build succeeds, app looks identical (new variables exist but aren't consumed yet).

### Phase 2: Layer system

**What:** Build the layer infrastructure.

**Files:**
- New: `src/lib/stores/layerManager.ts`
- New: `src/lib/utils/focusTrap.ts`
- Modified: `src/app.html` (add `<div id="layer-portal">`)
- Modified: `src/app.css` (add `.layer-modal-open` scroll lock)

**Changes:**
- Layer manager store with push/pop/getTop
- Focus trap utility
- Focus fallback chain
- Portal target element
- Scroll lock CSS
- Global Escape listener

**Verification:** Layer manager works in isolation (can push/pop, inert toggling works, focus trap cycles correctly).

### Phase 3: Colors

**What:** Remove all hardcoded hex values, migrate to OKLCH, apply currentColor to icons.

**Files:** All 53 `.svelte` files + `app.css`

**Changes:**
- Calculate exact OKLCH L values for all color tokens to meet 7:1/4.5:1 on target backgrounds
- Replace all `#fff`, `#000`, `#e879f9`, `#0d1117`, etc. with tokens
- Replace all hardcoded state colors in AgentNode with tokens
- Add redundant indicators (icons + labels) for state colors
- Migrate all SVG fill/stroke to currentColor
- Add `--text-on-accent` and verify
- Build contrast matrix (WCAG 2.x + APCA) for dark and light themes
- Verify alpha color effective contrast
- Document forbidden combinations

**Verification (two checkpoints):**

1. **Checkpoint 3a — Token replacement:** Zero hex in `.svelte` files (grep verification). All hardcoded colors replaced with tokens. currentColor applied to all SVG icons. Commit.
2. **Checkpoint 3b — Contrast audit:** Contrast matrix complete (WCAG 2.x + APCA) for dark and light themes. All ratios pass. Alpha color effective contrast documented. Colorblind simulation for state groups. Forbidden combinations documented. Commit.

### Phase 4: Typography

**What:** Migrate all font sizes to new rem scale, apply line-height per tier, font-weight per tier, text measure.

**Files:** `app.css` + all `.svelte` files with font-size declarations (~40 files)

**Changes:**
- Replace all hardcoded font-size px values with tokens per mapping table
- Apply `--line-height-*` per tier
- Apply font-weight per tier per theme
- Add `max-width: var(--measure)` to prose containers
- Verify no text below 12px
- Update enhanced readability mode

**Verification:** Build OK, no text below 12px (visual check), prose containers respect `--measure`.

### Phase 5: Spacing

**What:** Migrate all padding/margin/gap to spacing scale.

**Files:** All `.svelte` files with padding/margin/gap (~50 files, 300+ changes)

**Changes:**
- Replace all hardcoded spacing values with primitive or semantic tokens
- Ensure no fixed `height` on text containers (only `min-height`)
- Ensure no `overflow: hidden` on text elements
- Apply `--interactive-gap` between adjacent clickable elements
- Apply `--paragraph-spacing` to prose blocks

**Verification:** 4px grid respected (no values outside scale). WCAG 1.4.12 test: apply user overrides (line-height 1.5x, letter-spacing 0.12em, word-spacing 0.16em) and verify layout doesn't break.

### Phase 6: Buttons

**What:** Unify all button patterns to normal/compact at 44px.

**Files:** All components with buttons

**Changes:**
- Apply common button properties
- Map 9 current patterns to 2 variants + 4 color variants
- Migrate `<div role="button">` and `<span @click>` to native `<button>`
- Add `aria-label` to all icon-only buttons
- Add `aria-pressed` to toggle buttons with non-color indicator
- Add `aria-busy` pattern for async buttons
- Implement `role="toolbar"` with arrow key navigation for button groups
- Apply focus appearance AAA per variant
- Add forced-colors button styles

**Verification:** All buttons 44px min-height. All icon-only buttons have aria-label. Toolbar keyboard navigation works. Focus ring visible on every variant.

### Phase 7: Transitions

**What:** Tokenize all transitions, implement motion policies.

**Files:** `app.css` + all components with transitions/animations

**Changes:**
- Replace all hardcoded durations with `--transition-fast` / `--transition-normal`
- Classify each animation as functional vs decorative
- Ensure functional animations have static alternative in reduced-motion
- Add cursor blink setting (default off)
- Implement toast auto-dismiss policy (never for errors/actions, hover pauses)
- Add non-motion state change indicators
- Apply enhanced-readability slower transitions

**Verification:** `prefers-reduced-motion` tested (all decorative motion gone, functional indicators still present). Zero flash (no blinking/flashing animations). Toast behavior correct per type.

### Phase 8: Borders

**What:** Apply semantic border tokens, remove all border-radius except 50%, implement forced-colors state communication.

**Files:** All `.svelte` files with border declarations

**Changes:**
- Replace all `border: Npx solid var(--border)` with semantic tokens
- Remove all border-radius values (2px, 4px, 6px, 8px)
- Apply `--border-separator` vs `--border-container` vs `--border-interactive` appropriately
- Add `role="separator"` and `aria-hidden` to decorative `<hr>`
- Apply input border states (focus, error, disabled)
- Implement forced-colors state via thickness and style
- Verify circular indicators have 24x24px minimum and 3:1 contrast
- Apply enhanced-readability and prefers-contrast border-width increase

**Verification:** Border contrast 3:1 verified on both sides (1.4.11). Forced-colors mode tested (states distinguishable). Zero non-zero border-radius except 50%.

### Phase 9: Enhanced readability + prefers-contrast

**What:** Verify and complete enhanced-readability mode and prefers-contrast: more support.

**Files:** `app.css`

**Changes:**
- Verify all token overrides in enhanced-readability are proportionally correct
- Verify prefers-contrast: more provides max contrast
- Test combination: enhanced-readability + prefers-contrast + dark/light theme (4 combinations)
- Ensure all enhanced-readability spacing scales proportionally with new semantic tokens

**Verification:** All 4 theme combinations tested. No broken layouts. Contrast boosted.

### Phase 10: Final audit

**What:** Comprehensive WCAG AAA verification.

**Files:** No new files.

**Checks:**
- [ ] Complete contrast matrix (WCAG 2.x + APCA) for all tokens on all backgrounds, both themes
- [ ] Colorblind simulation for all state color groups (deuteranopia, protanopia, tritanopia, achromatopsia)
- [ ] WCAG 1.4.12 text spacing override test (apply line-height 1.5x, letter-spacing 0.12em, word-spacing 0.16em, paragraph spacing 2x — layout must not break)
- [ ] All interactive elements 44x44px minimum (2.5.8)
- [ ] All interactive elements have 8px minimum gap (--interactive-gap)
- [ ] Focus never obscured (2.4.12) — test with all layer combinations
- [ ] Focus appearance AAA (2.4.13) — focus ring visible and contrasted on every component variant
- [ ] No flashing content (2.3.2)
- [ ] All icon-only buttons have aria-label (4.1.2)
- [ ] All toggle buttons have aria-pressed (4.1.2)
- [ ] All state colors have redundant non-color indicator (1.4.1)
- [ ] All modals have role="dialog" + aria-modal + aria-labelledby
- [ ] Keyboard navigation works for all toolbars (role="toolbar")
- [ ] Toast auto-dismiss policy enforced
- [ ] Zero hardcoded hex values in .svelte files
- [ ] Zero hardcoded px font-sizes in .svelte files
- [ ] Zero border-radius values except 50%
- [ ] Forced-colors mode tested: all states distinguishable via border thickness/style
- [ ] prefers-reduced-motion tested: all decorative motion eliminated, functional indicators present
- [ ] prefers-contrast: more tested: contrast boosted
- [ ] Enhanced readability tested: all scales proportional

**Verification:** Full checklist passed. Any failures are fixed and re-verified.

---

## WCAG AAA Criteria Covered

| Criterion | Level | How addressed |
|-----------|-------|--------------|
| 1.3.1 Info and Relationships | A | Semantic separators, ARIA roles on layers |
| 1.3.2 Meaningful Sequence | A | Portal pattern for layers |
| 1.4.1 Use of Color | A | Redundant indicators for all color-coded info |
| 1.4.3 Contrast Minimum | AA | Superseded by 1.4.6 |
| 1.4.6 Contrast Enhanced | AAA | 7:1 normal, 4.5:1 large text, OKLCH architecture |
| 1.4.8 Visual Presentation | AAA | Text measure 72ch, paragraph spacing, line-height |
| 1.4.10 Reflow | AA | No fixed dimensions on text containers |
| 1.4.11 Non-text Contrast | AA | Border 3:1, icon 3:1, focus ring 3:1 |
| 1.4.12 Text Spacing | AA | Override-resilient spacing, no overflow:hidden on text |
| 1.4.13 Content on Hover/Focus | AA | Layer system dismissible, hoverable, persistent |
| 2.1.1 Keyboard | A | Toolbar pattern, native buttons |
| 2.1.2 No Keyboard Trap | A | Layer manager + focus trap + Escape LIFO |
| 2.2.1 Timing Adjustable | A | Toast pause on hover/focus, error toasts persistent |
| 2.2.2 Pause Stop Hide | A | Cursor blink default off, controllable |
| 2.3.2 Three Flashes | AAA | Zero flash policy |
| 2.3.3 Animation from Interactions | AAA | prefers-reduced-motion respected, functional vs decorative |
| 2.4.3 Focus Order | A | Toolbar arrow nav, portal DOM order |
| 2.4.7 Focus Visible | AA | Superseded by 2.4.13 |
| 2.4.12 Focus Not Obscured Enhanced | AAA | Layer inert, focus never covered |
| 2.4.13 Focus Appearance | AAA | 2px outline, 2px offset, 3:1 contrast per variant |
| 2.5.8 Target Size Enhanced | AAA | 44px minimum, no exceptions |
| 4.1.2 Name Role Value | A | aria-label icon buttons, aria-pressed toggles |
| 4.1.3 Status Messages | AA | aria-live on toasts, aria-busy on loading buttons |
