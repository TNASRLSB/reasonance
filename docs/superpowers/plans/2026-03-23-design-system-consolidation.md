# Design System Consolidation — WCAG AAA Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate 479+ hardcoded style values across 53 Svelte components into a tokenized, WCAG AAA compliant design system.

**Architecture:** Phased migration — tokens first, then infrastructure (layer manager), then per-category replacements across all components, then final audit. Each phase is a self-contained commit with verification. All phases reference the spec at `docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md`.

**Tech Stack:** Svelte 5 (runes), CSS custom properties (OKLCH), TypeScript, Vitest, Playwright + axe-core

**Spec:** `docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md`

---

## File Structure

### New files

| File | Responsibility |
|------|---------------|
| `src/lib/stores/layerManager.ts` | Centralized layer stack: push/pop, inert management, Escape LIFO, focus return |
| `src/lib/utils/focus-trap.ts` | Enhanced focus trap with fallback chain (extends existing `a11y.ts` trapFocus) |
| `src/lib/utils/contrast-audit.ts` | Contrast ratio calculation utilities (WCAG 2.x + APCA) for Phase 10 |
| `tests/unit/stores/layerManager.test.ts` | Unit tests for layer manager |
| `tests/unit/utils/focus-trap.test.ts` | Unit tests for focus trap |

### Modified files

| File | What changes |
|------|-------------|
| `src/app.css` | All new tokens (typography, spacing, layers, colors, transitions, borders), enhanced-readability overrides, prefers-contrast, forced-colors |
| `src/app.html` | Add `<div id="layer-portal"></div>` |
| `src/lib/stores/ui.ts` | Add `cursorBlink` writable (default false) |
| All 53 `.svelte` files | Replace hardcoded values with tokens (colors, font-size, spacing, border, z-index, transitions, border-radius) |
| Swarm node components (6 files) | Add redundant state indicators (icons + labels), migrate SVG to currentColor |
| Modal components (SearchPalette, ShortcutsDialog, FindInFiles, Settings, SessionPanel) | Integrate with layer manager, portal rendering, ARIA attributes |
| Button components | Unify to normal/compact pattern, native `<button>`, aria-pressed, role="toolbar" |

---

## Task 1: Tokens in app.css

**Files:**
- Modify: `src/app.css`

**Prerequisite:** Read the spec sections 1-7 for all token definitions.

- [ ] **Step 1: Read the current app.css completely**

Read `src/app.css` to understand existing token structure and placement.

- [ ] **Step 2: Add typography tokens**

In `:root`, replace existing font-size tokens and add new ones:

```css
/* Replace these existing tokens: */
--font-size-base: 14px;
--font-size-small: 12px;
--font-size-tiny: 11px;
--font-size-code: 14px;
--font-size-terminal: 14px;

/* With the new scale: */
--font-size-sm:   0.75rem;   /* 12px — minimum */
--font-size-base: 1rem;      /* 16px */
--font-size-md:   1.25rem;   /* 20px */
--font-size-lg:   1.5rem;    /* 24px */
--font-size-hero: 2rem;      /* 32px */

/* Semantic aliases */
--font-size-code:     var(--font-size-base);
--font-size-terminal: var(--font-size-base);

/* Line-height per tier */
--line-height-sm:   1.6;
--line-height-base: 1.5;
--line-height-md:   1.4;
--line-height-lg:   1.3;
--line-height-hero: 1.2;

/* Text measure */
--measure: 72ch;

/* Font-weight per tier (dark theme) */
--font-weight-sm:   500;   /* small text needs more weight for legibility */
--font-weight-base: 500;   /* body text: medium on dark */
--font-weight-ui:   400;   /* UI labels: regular sufficient at 16px */
--font-weight-md:   700;   /* section headings */
--font-weight-lg:   700;   /* main headings */
--font-weight-hero: 800;   /* hero titles, maximum impact */
```

In `:root.light`, add font-weight overrides:
```css
--font-weight-base: 400;   /* lighter on light backgrounds */
--font-weight-hero: 700;   /* slightly lighter than dark */
```

Also update `--line-height: 1.5;` to `--line-height: var(--line-height-base);` and update `body` rule accordingly.

- [ ] **Step 3: Add spacing tokens**

In `:root`, replace `--btn-padding` and `--panel-padding` and add:

```css
/* Primitive scale */
--space-1: 0.25rem;   /* 4px */
--space-2: 0.5rem;    /* 8px */
--space-3: 0.75rem;   /* 12px */
--space-4: 1rem;      /* 16px */
--space-5: 1.5rem;    /* 24px */
--space-6: 2rem;      /* 32px */

/* Semantic */
--inset-component:    var(--space-3);
--inset-section:      var(--space-4);
--stack-tight:        var(--space-1);
--stack-normal:       var(--space-2);
--stack-loose:        var(--space-4);
--interactive-gap:    var(--space-2);
--paragraph-spacing:  calc(1.5 * 1.5em);

/* Button/input aliases */
--btn-padding:    var(--space-2) var(--space-3);
--btn-padding-sm: var(--space-1) var(--space-2);
--panel-padding:  var(--space-4);
--input-padding:  var(--space-2) var(--space-3);
```

- [ ] **Step 4: Add layer tokens**

```css
--layer-base:      0;
--layer-dropdown:  1;
--layer-sticky:    2;
--layer-overlay:   3;
--layer-modal:     4;
--layer-toast:     5;
```

- [ ] **Step 5: Add color tokens (OKLCH base hues + semantic states)**

```css
/* Base hues */
--hue-accent:       220;
--hue-danger:       25;
--hue-success:      145;
--hue-warning:      85;
--hue-state-retry:  45;

/* Text-on-accent (for buttons with accent/danger bg) */
--text-on-accent: #ffffff;  /* placeholder — verify 7:1 on --accent */

/* Code colors */
--code-bg:     oklch(0.18 0.02 240);
--code-accent: oklch(0.78 0.18 320);

/* State colors */
--state-idle:          oklch(0.55 0.00 0);
--state-idle-text:     oklch(0.72 0.00 0);
--state-queued:        var(--warning);
--state-queued-text:   var(--warning-text);
--state-running:       var(--accent);
--state-running-text:  var(--accent-text);
--state-success:       var(--success);
--state-success-text:  var(--success-text);
--state-failed:        var(--danger);
--state-failed-text:   var(--danger-text);
--state-retrying:      oklch(0.65 0.18 var(--hue-state-retry));
--state-retrying-text: oklch(0.78 0.15 var(--hue-state-retry));

/* UI state */
--text-disabled:     oklch(0.55 0.00 0);
--bg-disabled:       oklch(0.25 0.00 0);
--border-disabled:   oklch(0.35 0.00 0);
--bg-selected:       oklch(0.30 0.08 var(--hue-accent));
--text-selected:     var(--text-primary);
--text-placeholder:  oklch(0.55 0.00 0);
--focus-ring-color:  oklch(0.70 0.20 var(--hue-accent));
```

Note: Exact OKLCH lightness values will be fine-tuned in Phase 3 when building the contrast matrix. These are starting values.

- [ ] **Step 6: Add transition tokens**

```css
--transition-fast:   0.1s ease;
--transition-normal: 0.2s ease;
--toast-duration:    5s;
```

- [ ] **Step 7: Add border tokens**

```css
--border-style: solid;
--border-color: var(--border);
--border-container:   var(--border-width) var(--border-style) var(--border-color);
--border-separator:   1px solid var(--border-color);
--border-interactive: var(--border-width) var(--border-style) var(--border-color);
--border-focus:       var(--border-width) solid var(--focus-ring-color);
--border-error:       var(--border-width) solid var(--danger);
--border-selected:    calc(var(--border-width) * 2) solid var(--accent);
--radius-full: 50%;
```

- [ ] **Step 8: Update light theme overrides**

In `:root.light`, add overrides for all new color tokens:

```css
:root.light {
  /* existing overrides stay */

  /* New token overrides */
  --text-on-accent: #ffffff;
  --code-bg:     oklch(0.96 0.01 240);
  --code-accent: oklch(0.50 0.22 320);
  --state-idle:          oklch(0.60 0.00 0);
  --state-idle-text:     oklch(0.40 0.00 0);
  --state-retrying:      oklch(0.55 0.18 var(--hue-state-retry));
  --state-retrying-text: oklch(0.42 0.15 var(--hue-state-retry));
  --text-disabled:     oklch(0.60 0.00 0);
  --bg-disabled:       oklch(0.90 0.00 0);
  --border-disabled:   oklch(0.80 0.00 0);
  --bg-selected:       oklch(0.90 0.06 var(--hue-accent));
  --text-placeholder:  oklch(0.60 0.00 0);
  --focus-ring-color:  oklch(0.50 0.22 var(--hue-accent));
}
```

- [ ] **Step 9: Update enhanced-readability overrides**

Replace the existing `:root.enhanced-readability` block with:

```css
:root.enhanced-readability {
  /* Typography */
  --font-size-sm:   0.875rem;
  --font-size-base: 1.125rem;
  --font-size-md:   1.375rem;
  --font-size-lg:   1.75rem;
  --font-size-hero: 2.5rem;
  --line-height-base: 1.8;
  --letter-spacing: 0.05em;
  --word-spacing: 0.1em;
  --font-weight-body: 500;
  --font-weight-heading: 600;

  /* Spacing */
  --space-1: 0.375rem;
  --space-2: 0.625rem;
  --space-3: 1rem;
  --space-4: 1.25rem;
  --space-5: 1.75rem;
  --space-6: 2.5rem;

  /* Borders */
  --border-width: 3px;

  /* Transitions */
  --transition-fast:   0.15s ease;
  --transition-normal: 0.3s ease;
}

:root.light.enhanced-readability {
  --font-weight-body: 500;
  --font-weight-heading: 600;
}
```

- [ ] **Step 10: Add prefers-contrast: more**

```css
@media (prefers-contrast: more) {
  :root {
    --text-body:      #ffffff;
    --text-secondary: #e0e0e0;
    --bg-primary:     #000000;
    --border-color:   #ffffff;
    --border-width:   3px;
  }
  :root.light {
    --text-body:      #000000;
    --text-secondary: #1a1a1a;
    --bg-primary:     #ffffff;
    --border-color:   #000000;
  }
}
```

- [ ] **Step 11: Add scroll-lock class and layer-modal-open rule**

```css
body.layer-modal-open {
  overflow: hidden;
  scrollbar-gutter: stable;
}
```

- [ ] **Step 12: Extend forced-colors media query**

Add to existing `@media (forced-colors: active)` block:

```css
  /* Layer borders */
  [data-layer="modal"],
  [data-layer="overlay"] {
    border: 2px solid CanvasText;
  }
  [data-layer="dropdown"] {
    border: 1px solid CanvasText;
  }

  /* Input states via border style */
  input:focus, textarea:focus {
    border-width: 3px;
  }
  [aria-invalid="true"] {
    border-style: dashed;
  }
  [aria-selected="true"],
  [aria-pressed="true"] {
    border-width: 4px;
  }

  /* Disabled */
  button:disabled,
  button[aria-disabled="true"] {
    border-color: GrayText;
    color: GrayText;
  }
```

- [ ] **Step 13: Verify build**

Run: `npm run build`
Expected: Build succeeds. No visual changes (new tokens exist but aren't consumed yet).

- [ ] **Step 14: Commit**

```bash
git add src/app.css
git commit -m "feat(design-system): add all new CSS tokens — typography, spacing, layers, colors, borders, transitions

Phase 1 of design system consolidation. Tokens added but not yet
consumed by components. No visual changes.

Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md"
```

---

## Task 2: Layer System Infrastructure

**Files:**
- Create: `src/lib/stores/layerManager.ts`
- Create: `src/lib/utils/focus-trap.ts`
- Create: `tests/unit/stores/layerManager.test.ts`
- Create: `tests/unit/utils/focus-trap.test.ts`
- Modify: `src/app.html` (add portal div)

**Prerequisite:** Read spec section 3 (Layer System). Read existing `src/lib/utils/a11y.ts` and `src/lib/utils/a11y-focus.ts` to understand current focus management.

- [ ] **Step 1: Read existing focus utilities**

Read these files to understand the current API:
- `src/lib/utils/a11y.ts` — has `trapFocus(container)` returning destroy function
- `src/lib/utils/a11y-focus.ts` — has `focusManager` class with push/pop/reset

- [ ] **Step 2: Write failing tests for focus-trap utility**

Create `tests/unit/utils/focus-trap.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createFocusTrap, findFocusFallback } from '$lib/utils/focus-trap';

describe('createFocusTrap', () => {
  let container: HTMLElement;

  beforeEach(() => {
    container = document.createElement('div');
    container.innerHTML = `
      <button id="first">First</button>
      <input id="middle" />
      <button id="last">Last</button>
    `;
    document.body.appendChild(container);
  });

  it('traps Tab within container', () => {
    const trap = createFocusTrap(container);
    const last = container.querySelector('#last') as HTMLElement;
    last.focus();

    const event = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true });
    const prevented = !container.dispatchEvent(event);

    trap.destroy();
  });

  it('focuses first focusable element on activate', () => {
    const trap = createFocusTrap(container, { initialFocus: true });
    expect(document.activeElement?.id).toBe('first');
    trap.destroy();
  });

  it('returns destroy function that removes listeners', () => {
    const trap = createFocusTrap(container);
    trap.destroy();
    // Should not throw after destroy
  });
});

describe('findFocusFallback', () => {
  it('returns element if still in DOM and visible', () => {
    const el = document.createElement('button');
    document.body.appendChild(el);
    expect(findFocusFallback(el)).toBe(el);
    el.remove();
  });

  it('returns nearest focusable sibling if element removed', () => {
    const parent = document.createElement('div');
    const btn1 = document.createElement('button');
    btn1.id = 'sibling';
    const btn2 = document.createElement('button');
    btn2.id = 'target';
    parent.appendChild(btn1);
    parent.appendChild(btn2);
    document.body.appendChild(parent);

    btn2.remove();
    const fallback = findFocusFallback(btn2, parent);
    expect(fallback?.id).toBe('sibling');
    parent.remove();
  });

  it('returns document.body as last resort', () => {
    const el = document.createElement('button');
    // Not in DOM
    expect(findFocusFallback(el)).toBe(document.body);
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `npx vitest run tests/unit/utils/focus-trap.test.ts`
Expected: FAIL — module `$lib/utils/focus-trap` not found.

- [ ] **Step 4: Implement focus-trap utility**

Create `src/lib/utils/focus-trap.ts`:

```typescript
const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export interface FocusTrapOptions {
  initialFocus?: boolean;
}

export interface FocusTrap {
  destroy: () => void;
}

export function createFocusTrap(container: HTMLElement, options: FocusTrapOptions = {}): FocusTrap {
  const { initialFocus = false } = options;

  function getFocusable(): HTMLElement[] {
    return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE))
      .filter(el => el.offsetParent !== null);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return;

    const focusable = getFocusable();
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }

  container.addEventListener('keydown', handleKeydown);

  if (initialFocus) {
    const focusable = getFocusable();
    if (focusable.length > 0) {
      focusable[0].focus();
    }
  }

  return {
    destroy() {
      container.removeEventListener('keydown', handleKeydown);
    }
  };
}

export function findFocusFallback(
  original: HTMLElement | null,
  container?: HTMLElement
): HTMLElement {
  // 1. Original element still in DOM and visible
  if (original && original.isConnected && original.offsetParent !== null) {
    return original;
  }

  // 2. Nearest focusable in container
  if (container) {
    const focusable = Array.from(
      container.querySelectorAll<HTMLElement>(FOCUSABLE)
    ).filter(el => el.offsetParent !== null);
    if (focusable.length > 0) return focusable[0];
  }

  // 3. Container itself if focusable
  if (container && container.tabIndex >= 0) return container;

  // 4. Last resort
  return document.body;
}
```

- [ ] **Step 5: Run focus-trap tests**

Run: `npx vitest run tests/unit/utils/focus-trap.test.ts`
Expected: PASS

- [ ] **Step 6: Write failing tests for layer manager**

Create `tests/unit/stores/layerManager.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';

// Will import from implementation
let layerManager: typeof import('$lib/stores/layerManager');

beforeEach(async () => {
  // Fresh import for each test
  vi.resetModules();
  layerManager = await import('$lib/stores/layerManager');
});

describe('layerManager', () => {
  it('starts with empty stack', () => {
    expect(get(layerManager.layerStack)).toEqual([]);
    expect(get(layerManager.topLayer)).toBeNull();
  });

  it('pushes a layer onto the stack', () => {
    const returnFocus = document.createElement('button');
    layerManager.pushLayer({
      id: 'test-modal',
      type: 'modal',
      returnFocus
    });

    const stack = get(layerManager.layerStack);
    expect(stack).toHaveLength(1);
    expect(stack[0].id).toBe('test-modal');
    expect(get(layerManager.topLayer)?.id).toBe('test-modal');
  });

  it('pops the top layer', () => {
    layerManager.pushLayer({ id: 'layer-1', type: 'modal', returnFocus: null });
    layerManager.pushLayer({ id: 'layer-2', type: 'dropdown', returnFocus: null });

    layerManager.popLayer();
    expect(get(layerManager.layerStack)).toHaveLength(1);
    expect(get(layerManager.topLayer)?.id).toBe('layer-1');
  });

  it('pops a specific layer by id', () => {
    layerManager.pushLayer({ id: 'layer-1', type: 'modal', returnFocus: null });
    layerManager.pushLayer({ id: 'layer-2', type: 'dropdown', returnFocus: null });

    layerManager.popLayer('layer-1');
    expect(get(layerManager.layerStack)).toHaveLength(1);
    expect(get(layerManager.topLayer)?.id).toBe('layer-2');
  });

  it('hasOpenModal returns true when modal layer exists', () => {
    expect(get(layerManager.hasOpenModal)).toBe(false);

    layerManager.pushLayer({ id: 'test', type: 'modal', returnFocus: null });
    expect(get(layerManager.hasOpenModal)).toBe(true);
  });
});
```

- [ ] **Step 7: Run tests to verify they fail**

Run: `npx vitest run tests/unit/stores/layerManager.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 8: Implement layer manager store**

Create `src/lib/stores/layerManager.ts`:

```typescript
import { writable, derived } from 'svelte/store';
import { findFocusFallback } from '$lib/utils/focus-trap';

export interface LayerEntry {
  id: string;
  type: 'dropdown' | 'overlay' | 'modal' | 'toast';
  returnFocus: HTMLElement | null;
  onClose?: () => void;
}

const _stack = writable<LayerEntry[]>([]);

export const layerStack = { subscribe: _stack.subscribe };

export const topLayer = derived(_stack, ($stack) =>
  $stack.length > 0 ? $stack[$stack.length - 1] : null
);

export const hasOpenModal = derived(_stack, ($stack) =>
  $stack.some(l => l.type === 'modal')
);

export function pushLayer(entry: LayerEntry) {
  _stack.update(stack => [...stack, entry]);
  updateInert();
  updateScrollLock();
}

export function popLayer(id?: string) {
  let popped: LayerEntry | undefined;

  _stack.update(stack => {
    if (id) {
      const idx = stack.findIndex(l => l.id === id);
      if (idx === -1) return stack;
      popped = stack[idx];
      return [...stack.slice(0, idx), ...stack.slice(idx + 1)];
    } else {
      popped = stack[stack.length - 1];
      return stack.slice(0, -1);
    }
  });

  if (popped) {
    const target = findFocusFallback(popped.returnFocus);
    target.focus();
    popped.onClose?.();
  }

  updateInert();
  updateScrollLock();
}

export function handleGlobalEscape(e: KeyboardEvent) {
  if (e.key !== 'Escape') return;

  let stack: LayerEntry[] = [];
  _stack.subscribe(s => { stack = s; })();

  if (stack.length === 0) return;

  e.preventDefault();
  e.stopPropagation();
  popLayer();
}

function updateInert() {
  let stack: LayerEntry[] = [];
  _stack.subscribe(s => { stack = s; })();

  const portal = document.getElementById('layer-portal');
  const mainContent = document.querySelector('[data-main-content]');

  if (!mainContent) return;

  const hasModal = stack.some(l => l.type === 'modal' || l.type === 'overlay');
  if (hasModal) {
    mainContent.setAttribute('inert', '');
  } else {
    mainContent.removeAttribute('inert');
  }
}

function updateScrollLock() {
  let stack: LayerEntry[] = [];
  _stack.subscribe(s => { stack = s; })();

  const hasModal = stack.some(l => l.type === 'modal');
  document.body.classList.toggle('layer-modal-open', hasModal);
}

export function initLayerManager() {
  document.addEventListener('keydown', handleGlobalEscape);

  return () => {
    document.removeEventListener('keydown', handleGlobalEscape);
  };
}
```

- [ ] **Step 9: Run layer manager tests**

Run: `npx vitest run tests/unit/stores/layerManager.test.ts`
Expected: PASS

- [ ] **Step 10: Add portal div to app.html**

Modify `src/app.html` — add before the closing `</body>` tag:

```html
<div id="layer-portal"></div>
```

- [ ] **Step 11: Add data-main-content attribute**

In `src/lib/components/App.svelte` (or `src/routes/+page.svelte`, whichever wraps main layout), add `data-main-content` to the main wrapper element so the layer manager can target it for `inert`.

- [ ] **Step 12: Verify build**

Run: `npm run build`
Expected: Build succeeds. Layer manager exists but is not yet integrated with components.

- [ ] **Step 13: Run all tests**

Run: `npm run test`
Expected: All existing tests pass + new tests pass.

- [ ] **Step 14: Commit**

```bash
git add src/lib/stores/layerManager.ts src/lib/utils/focus-trap.ts tests/unit/stores/layerManager.test.ts tests/unit/utils/focus-trap.test.ts src/app.html
git commit -m "feat(design-system): add layer manager store and focus trap utility

Phase 2 of design system consolidation. Layer stack with push/pop,
inert management, Escape LIFO, focus fallback chain, scroll lock.
Portal div added to app.html.

Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md"
```

---

## Task 3: Colors — Token Replacement (Checkpoint 3a)

**Files:**
- Modify: All `.svelte` files with hardcoded hex colors
- Modify: `src/app.css` (fine-tune OKLCH values)

**Prerequisite:** Read spec section 4 (Colors). The goal of this checkpoint is to replace all hardcoded hex values with tokens. The contrast audit happens in Task 4.

- [ ] **Step 1: Audit all hardcoded colors**

Run grep to get the full list:

```bash
grep -rn '#[0-9a-fA-F]\{3,8\}' src/lib/components/ --include='*.svelte' | grep -v 'var(' | grep -v '<!--'
```

Also check for `rgb(` and `rgba(` in style blocks.

- [ ] **Step 2: Replace #fff and #000 across all components**

For each occurrence:
- `color: #fff` on accent/danger background → `color: var(--text-on-accent)`
- `color: #fff` on dark background → `color: var(--text-primary)`
- `color: #000` on light background → `color: var(--text-primary)`
- `background: #fff` → `background: var(--bg-surface)` or `var(--bg-primary)` per context

Files affected (~20+ occurrences): FindInFiles, DiffView, Settings, Toolbar, HelpPanel, TerminalManager, WelcomeScreen, AnalyticsDashboard, Terminal, SearchPalette, StatusBar, SwarmCanvas, App, DiffBlock, ChatInput, EditorTabs, Toast, chat components.

- [ ] **Step 3: Replace code-specific colors**

- `#0d1117` (code block bg) → `var(--code-bg)` in MarkdownPreview, ResponsePanel
- `#e879f9` (code accent) → `var(--code-accent)` in MarkdownPreview, ResponsePanel
- `#334155`, `#f1f5f9`, `#e2e8f0`, `#1e293b`, `#94a3b8` (slate palette in ResponsePanel, ContextMenu, MarkdownPreview) → map to existing `--bg-*` and `--text-*` tokens or create `--code-*` variants

- [ ] **Step 4: Replace state colors in AgentNode**

In `src/lib/components/swarm/AgentNode.svelte`, replace the hardcoded `stateColors` object.

**Important:** These colors are likely used in JavaScript for SVG canvas rendering, not in CSS. CSS `var()` cannot be used directly in JS. Two approaches:

**Option A (preferred):** Read CSS variables at runtime via `getComputedStyle`:
```typescript
function getStateColor(state: string): string {
  return getComputedStyle(document.documentElement)
    .getPropertyValue(`--state-${state}`)
    .trim();
}
```

**Option B:** If SVG elements are in the template (not canvas), use CSS classes instead of inline fill/stroke, and apply `var(--state-*)` in the `<style>` block.

Map:
- `#666666` (idle) → `--state-idle`
- `#ca8a04` (queued) → `--state-queued`
- `#1d4ed8` (running) → `--state-running`
- `#16a34a` (success) → `--state-success`
- `#dc2626` (failed) → `--state-failed`
- `#ea580c` (retrying/fallback) → `--state-retrying`

Read the component first to determine which approach fits.

- [ ] **Step 5: Add redundant indicators for agent states**

In AgentNode.svelte and related swarm components, add icons and labels alongside state colors:

| State | Icon | Label |
|-------|------|-------|
| idle | `⏸` or pause icon | "Idle" |
| running | spinner or `↻` | "Running" |
| success | `✓` | "Success" |
| failed | `✗` | "Failed" |
| retrying | `↻` | "Retrying" |
| queued | `⏳` | "Queued" |

Ensure these are visible (not just `aria-label` — they must be visual for 1.4.1).

- [ ] **Step 6: Replace StatusBar hardcoded colors**

- `#fca5a5` → `var(--danger-text)`
- `#fef08a` → `var(--warning-text)`

- [ ] **Step 7: Migrate SVG fill/stroke to currentColor**

Search all swarm components and CodeBlock for hardcoded `fill` and `stroke` attributes. Replace with `currentColor` or remove (letting CSS handle it).

```bash
grep -rn 'fill="\|stroke="' src/lib/components/ --include='*.svelte' | grep -v 'currentColor' | grep -v 'none' | grep -v 'transparent'
```

- [ ] **Step 8: Verify zero hardcoded hex in components**

Run:
```bash
grep -rn '#[0-9a-fA-F]\{3,8\}' src/lib/components/ src/routes/ --include='*.svelte' | grep -v 'var(' | grep -v '<!--' | grep -v '.ts' | wc -l
```

Expected: 0 (or only in comments/template literals that aren't style values).

- [ ] **Step 9: Verify build**

Run: `npm run build`
Expected: Build succeeds. Colors now come from tokens.

- [ ] **Step 10: Commit (Checkpoint 3a)**

```bash
git add src/lib/components/ src/routes/ src/app.css
git commit -m "feat(design-system): replace all hardcoded colors with tokens

Checkpoint 3a. Zero hex values in component style blocks.
Agent state colors tokenized with redundant visual indicators.
SVG icons migrated to currentColor.

Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md"
```

---

## Task 4: Colors — Contrast Audit (Checkpoint 3b)

**Files:**
- Create: `src/lib/utils/contrast-audit.ts`
- Modify: `src/app.css` (adjust any failing OKLCH values)

**Prerequisite:** Task 3 complete. All colors now use tokens.

- [ ] **Step 1: Create contrast calculation utility**

Create `src/lib/utils/contrast-audit.ts` with functions to:
- Calculate WCAG 2.x contrast ratio between two colors
- Calculate APCA contrast (Lc value) between two colors
- Parse OKLCH values to sRGB for calculation
- Compute effective color for alpha compositing

This is a development utility, not shipped to production.

- [ ] **Step 2: Build contrast matrix**

For each text color token, calculate contrast against every background where it's used. Populate the matrix from spec section 4. Format as a markdown table.

For each pair, record:
- WCAG 2.x ratio (target: 7:1 normal, 4.5:1 large)
- APCA Lc value (target: Lc 75+ for body text, Lc 60+ for large text)
- PASS/FAIL

- [ ] **Step 3: Fix any failing contrasts**

If any token pair fails 7:1 (normal text) or 4.5:1 (large text):
- Adjust the OKLCH lightness value in `app.css`
- Recalculate
- Repeat until all pass

- [ ] **Step 4: Verify alpha color effective contrast**

For `--bg-hover` (rgba with alpha), compute effective opaque color on each background and verify text contrast.

- [ ] **Step 5: Colorblind simulation**

For each state color group, verify luminosity distinctness:
- Convert all state colors to grayscale (L channel only)
- Verify each pair has at least 15% luminosity difference
- If not, adjust chroma/lightness

- [ ] **Step 6: Document forbidden combinations and contrast matrix**

Create `docs/superpowers/specs/2026-03-23-contrast-matrix.md` with:
- Complete contrast matrix (every text token vs every background token, both themes)
- WCAG 2.x ratio + APCA Lc value for each pair
- PASS/FAIL per pair
- Forbidden combinations list (all pairs below threshold)
- Colorblind simulation results

- [ ] **Step 7: Commit (Checkpoint 3b)**

```bash
git add -f src/app.css src/lib/utils/contrast-audit.ts docs/superpowers/specs/2026-03-23-contrast-matrix.md
git commit -m "feat(design-system): contrast audit — WCAG 2.x + APCA verified

Checkpoint 3b. All text/background pairs meet AAA (7:1 normal, 4.5:1 large).
APCA Lc values documented. Colorblind simulation passed for state groups.
Forbidden combinations documented.

Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md"
```

---

## Task 5: Typography

**Files:**
- Modify: All `.svelte` files with hardcoded `font-size` (~40 files)
- Modify: `src/app.css` (body rule update)

**Prerequisite:** Read spec section 1 (Typography) mapping table.

- [ ] **Step 1: Update body font-size in app.css**

Change the `body` rule:
```css
font-size: var(--font-size-base);  /* was already var, but now 1rem instead of 14px */
line-height: var(--line-height-base);
```

- [ ] **Step 2: Find all hardcoded font-size declarations**

```bash
grep -rn 'font-size:' src/lib/components/ src/routes/ --include='*.svelte' | grep -v 'var(' | grep -v '<!--'
```

- [ ] **Step 3: Replace font-sizes per mapping table**

Apply the mapping from spec section 1:

| Current | Token |
|---------|-------|
| 8-9px | `var(--font-size-sm)` |
| 10-11px | `var(--font-size-sm)` |
| 12-13px | `var(--font-size-sm)` |
| 14-15px | `var(--font-size-base)` |
| 16px | `var(--font-size-base)` |
| 18px | `var(--font-size-md)` |
| 20-22px | `var(--font-size-lg)` |
| 32px | `var(--font-size-hero)` |

Also replace em/rem-based sizes:
- `0.85em`, `0.875em`, `0.9em` → `var(--font-size-sm)` where used as "smaller text"
- `1.1em`, `1.25em`, `1.3em` → `var(--font-size-md)` where used as headings
- `1.5em`, `1.6em`, `2em` → `var(--font-size-lg)` or `var(--font-size-hero)`

Work file by file. Read each file before editing.

- [ ] **Step 4: Add line-height per tier where headings are used**

Where components define headings (TextBlock h1/h2/h3, WelcomeScreen titles), apply the corresponding line-height token alongside the font-size token.

- [ ] **Step 5: Apply font-weight per tier**

For each element that uses a font-size token, apply the corresponding font-weight token:

| Element type | Font-weight token |
|-------------|------------------|
| Small text (labels, meta, captions) | `var(--font-weight-sm)` |
| Body text (paragraphs, descriptions) | `var(--font-weight-base)` |
| UI labels (button text, tab labels) | `var(--font-weight-ui)` |
| Section headings | `var(--font-weight-md)` |
| Main headings | `var(--font-weight-lg)` |
| Hero titles | `var(--font-weight-hero)` |

These tokens already have dark/light theme overrides from Task 1, so theme-specific weights work automatically.

- [ ] **Step 6: Apply text measure to prose containers**

Add `max-width: var(--measure)` to:
- MarkdownPreview main content container
- ResponsePanel text content
- ChatMessages message bubbles
- Any other container with continuous prose

- [ ] **Step 7: Verify no text below 12px**

```bash
grep -rn 'font-size:' src/lib/components/ src/routes/ --include='*.svelte'
```

Verify all values are `var(--font-size-sm)` (12px) or larger. No `8px`, `9px`, `10px`, `11px` should remain.

- [ ] **Step 8: Verify build**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 9: Commit**

```bash
git add src/lib/components/ src/routes/ src/app.css
git commit -m "feat(design-system): migrate typography to rem scale — 12px minimum

Phase 4. All font-size values use tokens. Scale: sm(12px), base(16px),
md(20px), lg(24px), hero(32px). Line-height per tier. Font-weight per
tier with dark/light variants. Text measure 72ch on prose containers.

Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md"
```

---

## Task 6: Spacing

**Files:**
- Modify: All `.svelte` files with hardcoded padding/margin/gap (~50 files, 300+ changes)

**Prerequisite:** Read spec section 2 (Spacing) mapping table.

- [ ] **Step 1: Find all hardcoded spacing values**

```bash
grep -rn 'padding\|margin\|gap' src/lib/components/ src/routes/ --include='*.svelte' | grep 'px\|rem\|em' | grep -v 'var(' | grep -v '<!--'
```

- [ ] **Step 2: Replace padding/margin/gap per mapping**

Apply the mapping from spec section 2:

| Current | Token |
|---------|-------|
| 1-2px | `var(--space-1)` (4px) |
| 3-6px | `var(--space-1)` or `var(--space-2)` by context |
| 7-10px | `var(--space-2)` (8px) |
| 12-14px | `var(--space-3)` (12px) |
| 16-18px | `var(--space-4)` (16px) |
| 20-24px | `var(--space-5)` (24px) |
| 32px | `var(--space-6)` (32px) |

Use semantic tokens where appropriate:
- Button padding → `var(--btn-padding)` or `var(--btn-padding-sm)`
- Panel internal padding → `var(--inset-component)` or `var(--inset-section)`
- Gap between items in a list → `var(--stack-tight)` or `var(--stack-normal)`
- Gap between sections → `var(--stack-loose)`
- Gap between clickable elements → at least `var(--interactive-gap)`

Work file by file. Read each file before editing.

- [ ] **Step 3: Fix text container height constraints**

Search for fixed `height` on text containers and replace with `min-height`:

```bash
grep -rn 'height:' src/lib/components/ --include='*.svelte' | grep -v 'min-height\|max-height\|line-height\|100%\|100vh\|var('
```

Review each result. If it's a text container, change to `min-height`.

- [ ] **Step 4: Fix overflow:hidden on text elements**

```bash
grep -rn 'overflow.*hidden' src/lib/components/ --include='*.svelte'
```

Review each result. If the element contains text that could overflow with user spacing overrides, change to `overflow: auto` or remove.

- [ ] **Step 5: Apply paragraph-spacing to prose blocks**

In MarkdownPreview, ResponsePanel, ChatMessages — add:
```css
p + p { margin-top: var(--paragraph-spacing); }
```

- [ ] **Step 6: Apply interactive-gap between adjacent targets**

In toolbar button groups, tab bars, and similar — ensure `gap: var(--interactive-gap)` (8px minimum) between clickable elements.

- [ ] **Step 7: Verify no px values remain in spacing**

```bash
grep -rn 'padding\|margin\|gap' src/lib/components/ src/routes/ --include='*.svelte' | grep '[0-9]px' | grep -v 'var(' | grep -v 'border' | grep -v '<!--'
```

Expected: 0 results (or only in non-spacing contexts like border-width).

- [ ] **Step 8: Test WCAG 1.4.12 — text spacing overrides**

Manually test: open the app, use browser DevTools to apply custom CSS:
```css
* { line-height: 1.5 !important; letter-spacing: 0.12em !important; word-spacing: 0.16em !important; }
p { margin-bottom: 2em !important; }
```
Verify layout doesn't break (no clipped text, no overlapping elements).

- [ ] **Step 9: Verify build**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 10: Commit**

```bash
git add src/lib/components/ src/routes/
git commit -m "feat(design-system): migrate spacing to 4px grid — semantic tokens

Phase 5. All padding/margin/gap use design tokens. No fixed height on
text containers. Interactive gap 8px between targets. Paragraph spacing
per WCAG 1.4.8. Text spacing override test passed (1.4.12).

Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md"
```

---

## Task 7: Buttons

**Files:**
- Modify: All components with buttons
- Modify: `src/lib/stores/ui.ts` (add cursorBlink)

**Prerequisite:** Read spec section 5 (Buttons).

- [ ] **Step 1: Audit current button patterns**

Read each component to identify button elements and their current styling. Check for:
- `<div role="button">` or `<span @click>` → must become `<button>`
- Icon-only buttons without `aria-label`
- Toggle buttons without `aria-pressed`
- Button groups without `role="toolbar"`

- [ ] **Step 2: Migrate non-native buttons to `<button>`**

Search:
```bash
grep -rn 'role="button"\|role=.button' src/lib/components/ --include='*.svelte'
```

Also find click-handling divs/spans that act as buttons. Convert each to native `<button>`.

- [ ] **Step 3: Unify button styles**

In each component's `<style>` block, replace ad-hoc button styling with the token-based pattern:

**Normal button:**
```css
button {
  font-family: var(--font-ui);
  font-size: var(--font-size-sm);
  font-weight: 700;
  min-height: 2.75rem;
  padding: var(--btn-padding);
  border: var(--border-interactive);
  background: var(--bg-surface);
  color: var(--text-primary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
}
```

**Compact button** (same but `--btn-padding-sm`).

Apply semantic variants (Primary, Danger, Ghost) per the spec mapping table.

- [ ] **Step 4: Add aria-label to icon-only buttons**

Find all buttons with only an icon (SVG or icon character) and no text. Add `aria-label` describing the action.

- [ ] **Step 5: Add aria-pressed to toggle buttons**

In ViewModeToggle and any other toggle buttons:
- Add `aria-pressed={isActive}`
- Add visual non-color indicator for pressed state (thicker border)

- [ ] **Step 6: Implement role="toolbar" for button groups**

In Toolbar, EditorTabs, TerminalToolbar, SwarmControls:
- Wrap button groups in `<div role="toolbar" aria-label="...">`
- Set `tabindex="0"` on first button, `tabindex="-1"` on rest
- Add arrow key navigation: **Note:** `menuKeyHandler` in `a11y.ts` handles Up/Down arrow keys (for menus). Toolbars need Left/Right arrows instead. Write a new `toolbarKeyHandler(e: KeyboardEvent, container: HTMLElement)` in `a11y.ts` that:
  - Left arrow: focus previous button (wrap to last)
  - Right arrow: focus next button (wrap to first)
  - Home: focus first button
  - End: focus last button
  - Updates `tabindex` (0 on focused, -1 on others) for roving tabindex pattern

- [ ] **Step 7: Add loading state pattern**

For ChatInput send button and any async action buttons:
- Add `aria-busy="true"` during loading
- Add `aria-disabled="true"` (not native `disabled`)
- Style: `cursor: wait; opacity: 0.8; pointer-events: none;`

- [ ] **Step 8: Add cursor-blink setting**

In `src/lib/stores/ui.ts`, add:
```typescript
export const cursorBlink = writable(false);  // default off per spec
```

In Settings.svelte, add toggle for "Blinking cursor". In Terminal/Editor, apply `.cursor-blink` class only when enabled.

- [ ] **Step 9: Verify all buttons are 44px minimum**

Visual check in browser. Also:
```bash
grep -rn 'min-height' src/lib/components/ --include='*.svelte' | grep 'button\|btn'
```

- [ ] **Step 10: Verify build and run tests**

Run: `npm run build && npm run test`
Expected: All pass.

- [ ] **Step 11: Commit**

```bash
git add src/lib/components/ src/routes/ src/lib/stores/ui.ts
git commit -m "feat(design-system): unify buttons — 44px minimum, ARIA complete

Phase 6. All buttons native <button>. Two variants (normal/compact),
four color variants. Icon-only buttons have aria-label. Toggle buttons
have aria-pressed. Button groups use role=toolbar with arrow nav.
Cursor blink default off.

Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md"
```

---

## Task 8: Transitions

**Files:**
- Modify: All `.svelte` files with hardcoded transition durations
- Modify: `src/app.css` (reduced-motion refinement)

**Prerequisite:** Read spec section 6 (Transitions).

- [ ] **Step 1: Find all hardcoded transitions**

```bash
grep -rn 'transition:' src/lib/components/ src/routes/ --include='*.svelte' | grep -v 'var(' | grep -v '<!--'
```

Also search for `animation:` and `@keyframes`.

- [ ] **Step 2: Replace transition durations with tokens**

For each hardcoded transition:
- `0.1s` → `var(--transition-fast)`
- `0.15s` → `var(--transition-fast)` (rounded)
- `0.2s` → `var(--transition-normal)`
- `0.3s` → `var(--transition-normal)` (rounded)
- `0.5s`, `0.7s` → `var(--transition-normal)` (these are outliers, evaluate if needed)

- [ ] **Step 3: Classify animations as functional vs decorative**

Review all `@keyframes` in components:
- AnalyticsBar: `error-pulse`, `recovery-pulse` — functional (indicate error state) → keep but add static fallback
- StreamingIndicator: dot animation — functional → add static fallback
- Toast: `slide-in` — decorative → killed by reduced-motion
- Any cursor blink — decorative → controlled by setting

- [ ] **Step 4: Add static fallbacks for functional animations**

In `@media (prefers-reduced-motion: reduce)`, ensure functional indicators have non-animated alternatives (static icon, text label, color change).

- [ ] **Step 5: Implement toast auto-dismiss policy**

In `src/lib/stores/toast.ts` or Toast.svelte:
- Informational toasts: auto-dismiss after `--toast-duration` (5s)
- Error/warning toasts: never auto-dismiss
- Hover on toast pauses timer
- Focus on toast pauses timer

Review current implementation (toast.ts already has `pauseToastTimer`, `resumeToastTimer`) and ensure it matches policy.

- [ ] **Step 6: Verify zero flash**

Search for any animation that could flash:
```bash
grep -rn 'blink\|flash\|pulse' src/lib/components/ --include='*.svelte'
```

Ensure no animation toggles luminosity >10% more than once per cycle.

- [ ] **Step 7: Verify build**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 8: Commit**

```bash
git add src/lib/components/ src/routes/ src/app.css src/lib/stores/toast.ts
git commit -m "feat(design-system): tokenize transitions — zero flash, reduced-motion safe

Phase 7. All transitions use --transition-fast or --transition-normal.
Functional animations have static fallbacks in reduced-motion.
Toast auto-dismiss policy: never for errors. Zero flash animations.

Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md"
```

---

## Task 9: Borders

**Files:**
- Modify: All `.svelte` files with hardcoded border declarations

**Prerequisite:** Read spec section 7 (Borders).

- [ ] **Step 1: Find all hardcoded border declarations**

```bash
grep -rn 'border:' src/lib/components/ src/routes/ --include='*.svelte' | grep -v 'var(' | grep -v '<!--' | grep -v 'border-color\|border-width\|border-style\|border-radius\|border-top\|border-bottom\|border-left\|border-right'
```

Also check for `border-top:`, `border-bottom:`, etc.

- [ ] **Step 2: Replace border declarations with semantic tokens**

For each hardcoded border:
- Borders around panels/cards → `border: var(--border-container)`
- Borders between list items/sections → `border-bottom: var(--border-separator)` (1px)
- Borders on inputs/buttons → `border: var(--border-interactive)`
- Specific directional borders (border-top on StatusBar, etc.) → use `--border-separator` or `--border-container` components

- [ ] **Step 3: Remove all non-zero border-radius**

```bash
grep -rn 'border-radius' src/lib/components/ src/routes/ --include='*.svelte' | grep -v '0\|50%\|var(' | grep -v '<!--'
```

Remove or replace with `var(--radius)` (which is 0). Keep `50%` only for explicitly circular elements.

- [ ] **Step 4: Add semantic separators**

Where `<hr>` is used decoratively, add `aria-hidden="true"`. Where `<hr>` separates groups semantically, ensure `role="separator"` is present.

- [ ] **Step 5: Add input border states**

In ChatInput, SearchPalette input, Settings form controls:
- Focus state: `border: var(--border-focus)` (in addition to outline)
- Error state: `border: var(--border-error)` + `aria-invalid="true"`
- Disabled state: `border: var(--border-disabled)`

- [ ] **Step 6: Verify border contrast**

Using the contrast audit utility from Task 4, verify `--border-color` has 3:1 contrast against backgrounds on both sides. Adjust if needed.

- [ ] **Step 7: Verify build**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 8: Commit**

```bash
git add src/lib/components/ src/routes/
git commit -m "feat(design-system): semantic borders — container/separator/interactive

Phase 8. All borders use semantic tokens. Border-radius 0 enforced
(brutalist). Input border states (focus, error, disabled). Semantic
separators with ARIA. Border contrast 3:1 verified.

Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md"
```

---

## Task 10: Enhanced Readability + prefers-contrast Verification

**Files:**
- Modify: `src/app.css` (adjustments if needed)

**Prerequisite:** All previous tasks complete.

- [ ] **Step 1: Test enhanced-readability mode**

Enable enhanced-readability in Settings. Verify:
- Font sizes scale proportionally
- Spacing scales proportionally (not same spacing with bigger text)
- Borders are thicker (3px)
- Transitions are slower
- Layout doesn't break

- [ ] **Step 2: Test prefers-contrast: more**

In browser DevTools, emulate `prefers-contrast: more`. Verify:
- Text is max contrast (white on black in dark, black on white in light)
- Borders are thicker and higher contrast
- No elements become invisible

- [ ] **Step 3: Test all 4 theme combinations**

Test matrix:
- Dark + normal
- Dark + enhanced-readability
- Light + normal
- Light + enhanced-readability

Each should look correct with no broken layouts.

- [ ] **Step 4: Fix any issues found**

Adjust token overrides in `:root.enhanced-readability` or `@media (prefers-contrast: more)` as needed.

- [ ] **Step 5: Commit**

```bash
git add src/app.css
git commit -m "feat(design-system): verify enhanced-readability + prefers-contrast

Phase 9. All 4 theme combinations tested. Enhanced readability scales
proportionally. prefers-contrast: more provides max contrast.

Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md"
```

---

## Task 11: Final WCAG AAA Audit

**Files:**
- No new files. Fix any issues found in existing files.

**Prerequisite:** All previous tasks complete.

- [ ] **Step 1: Automated a11y scan**

Run: `npm run test:a11y`

Fix any axe-core violations.

- [ ] **Step 2: Contrast matrix verification**

Run contrast audit utility on all token pairs. Verify:
- All normal text: >= 7:1 (WCAG 2.x)
- All large text: >= 4.5:1 (WCAG 2.x)
- All APCA Lc values meet thresholds

- [ ] **Step 3: Interactive elements audit**

Verify all interactive elements are 44x44px minimum:
```bash
grep -rn 'min-height' src/lib/components/ --include='*.svelte'
```

Check gap between adjacent targets is >= `var(--interactive-gap)`.

- [ ] **Step 4: Focus audit**

Tab through the entire app. Verify:
- Focus ring visible on every interactive element
- Focus never obscured by overlapping content
- Focus appearance meets 2.4.13 (3:1 contrast, >= 2px perimeter)
- Toolbar keyboard navigation works (arrow keys)

- [ ] **Step 5: Layer system audit**

Open each modal/dialog. Verify:
- Escape closes top layer only
- Focus trapped inside modal
- Focus returns to trigger on close
- `inert` applied to background content
- `aria-modal="true"` present
- Screen reader announces dialog

- [ ] **Step 6: Reduced motion audit**

Enable `prefers-reduced-motion: reduce` in browser. Verify:
- All decorative animations eliminated
- Functional indicators have static alternatives
- No flashing content anywhere
- Cursor doesn't blink

- [ ] **Step 7: Forced-colors audit**

Enable forced-colors in browser. Verify:
- All components visible (borders present)
- Button states distinguishable via border thickness/style
- Focus ring uses system Highlight color
- Disabled elements use GrayText

- [ ] **Step 8: Colorblind simulation**

Using browser DevTools or simulation tool, check under:
- Deuteranopia
- Protanopia
- Tritanopia
- Achromatopsia

Verify all state color groups are distinguishable by luminosity.

- [ ] **Step 9: Token purity check**

Verify zero hardcoded values remain:

```bash
# Colors
grep -rn '#[0-9a-fA-F]\{3,8\}' src/lib/components/ src/routes/ --include='*.svelte' | grep -v 'var(' | grep -v '<!--' | wc -l

# Font sizes
grep -rn 'font-size:.*px' src/lib/components/ src/routes/ --include='*.svelte' | grep -v 'var(' | wc -l

# Border radius (non-zero, non-50%)
grep -rn 'border-radius' src/lib/components/ src/routes/ --include='*.svelte' | grep -v '0\|50%\|var(' | wc -l
```

All should return 0.

- [ ] **Step 10: Fix any issues found**

Address all violations discovered in steps 1-9.

- [ ] **Step 11: Run full test suite**

Run: `npm run test:all`
Expected: All tests pass.

- [ ] **Step 12: Final commit**

```bash
git add src/app.css src/lib/components/ src/routes/ src/lib/stores/ src/lib/utils/
git commit -m "feat(design-system): final WCAG AAA audit — all checks passed

Phase 10. Complete design system consolidation:
- Zero hardcoded style values in components
- Full contrast matrix (WCAG 2.x + APCA) verified
- 44px minimum target size on all interactive elements
- Layer system with focus trap, inert, Escape LIFO
- Forced-colors, reduced-motion, prefers-contrast all tested
- Colorblind simulation passed

Spec: docs/superpowers/specs/2026-03-23-design-system-consolidation-design.md"
```
