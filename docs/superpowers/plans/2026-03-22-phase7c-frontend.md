# Phase 7C: Frontend — Provider Settings UI, Analytics UI, Responsive Layouts

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the frontend layer for provider configuration and analytics visualization — AnalyticsBar (live session metrics), AnalyticsDashboard (full tab), and Provider Settings with progressive disclosure.

**Architecture:** Standalone Svelte 5 components + dedicated analytics store with dual data feeding (live event subscription + historical Tauri invoke). Utilities-first approach: build formatters and a11y tools first, then stores, then components.

**Tech Stack:** Svelte 5 (runes: `$state`, `$derived`, `$effect`, `$props`), TypeScript, Tauri 2 `invoke`/`listen`, CSS variables (brutalist design system with `--radius: 0`), `Intl` APIs.

**Spec:** `.claude/docs/specs/2026-03-22-phase7c-frontend-design.md`

---

## File Structure

### New Files (16)

| File | Responsibility |
|------|---------------|
| `src/lib/types/analytics.ts` | All TypeScript interfaces for analytics |
| `src/lib/data/model-info.ts` | Hardcoded model pricing/specs for selector |
| `src/lib/utils/format-analytics.ts` | Locale-aware number/date formatters |
| `src/lib/utils/a11y-motion.ts` | `prefers-reduced-motion` store |
| `src/lib/utils/a11y-announcer.ts` | Throttled screen reader announcer |
| `src/lib/utils/a11y-labels.ts` | Centralized `aria-label` builders |
| `src/lib/utils/a11y-focus.ts` | Stack-based focus manager |
| `src/lib/utils/tooltip.ts` | Svelte action `use:tooltip` |
| `src/lib/utils/tween.ts` | Number animation store |
| `src/lib/utils/bar-scale.ts` | Bar width normalizer |
| `src/lib/utils/grid-navigation.ts` | Svelte action `use:gridNavigation` |
| `src/lib/utils/provider-patterns.ts` | Colorblind-safe provider visual map |
| `src/lib/stores/analytics.ts` | Live + historical + budget stores |
| `src/lib/components/AnalyticsBar.svelte` | Compact bar under chat input |
| `src/lib/components/AnalyticsDashboard.svelte` | Full dashboard tab in editor area |
| `src-tauri/src/commands/provider.rs` | `test_provider_connection` + `reload_normalizers` |

### Modified Files (9+)

| File | Changes |
|------|---------|
| `src/lib/adapter/index.ts` | +9 methods to `Adapter` interface |
| `src/lib/adapter/tauri.ts` | +9 `invoke` implementations |
| `src/lib/stores/ui.ts` | +`analyticsDashboard` store |
| `src/lib/components/App.svelte` | +`startLiveTracking`, +analytics view switch |
| `src/lib/components/ResponsePanel.svelte` | +AnalyticsBar integration |
| `src/lib/components/Settings.svelte` | +Provider section + budget |
| `src/lib/components/Toolbar.svelte` | +analytics toggle button |
| `src-tauri/src/commands/mod.rs` | +`pub mod provider;` |
| `src-tauri/src/lib.rs` | +register new commands |
| `src/lib/i18n/en.json` (+ 8 other locales) | +analytics/provider i18n keys |

---

### Task 1: Types + Model Info

**Files:**
- Create: `src/lib/types/analytics.ts`
- Create: `src/lib/data/model-info.ts`

This task defines all TypeScript interfaces used across the analytics system, plus static model pricing data for the provider settings selector.

- [ ] **Step 1: Create analytics types file**

Create `src/lib/types/analytics.ts` with all interfaces from the spec. These mirror the Rust types in `src-tauri/src/analytics/mod.rs` plus frontend-only types.

```typescript
// src/lib/types/analytics.ts

// === Backend mirrors (match Rust serde output) ===

export interface SessionMetrics {
  session_id: string;
  provider: string;
  model: string;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cost_usd: number;
  duration_ms: number;
  duration_api_ms: number;
  num_turns: number;
  tool_calls: number;
  errors_total: number;
  errors_recovered: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  stop_reason: string | null;
  started_at: number;
  completed_at: number | null;
}

export interface ProviderAnalytics {
  provider: string;
  total_sessions: number;
  total_cost_usd: number;
  total_input_tokens: number;
  total_output_tokens: number;
  avg_duration_ms: number;
  avg_tokens_per_session: number;
  error_rate: number;
  cache_hit_rate: number;
}

export interface ModelAnalytics {
  model: string;
  provider: string;
  total_sessions: number;
  total_cost_usd: number;
  total_tokens: number;
  avg_duration_ms: number;
}

export interface DailyStats {
  date: string;
  sessions: number;
  total_cost_usd: number;
  total_tokens: number;
  providers_used: string[];
}

export interface TimeRange {
  start_epoch_ms: number;
  end_epoch_ms: number;
}

// === Frontend-only types ===

export interface LiveSessionMetrics {
  session_id: string;
  provider: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cost_usd: number;
  duration_ms: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  context_percent: number | null;
  num_turns: number;
  errors: number;
  errors_recovered: number;
  is_streaming: boolean;
  cost_velocity_usd_per_min: number;
  cost_projection_usd: number | null;
  vs_avg_ratio: number | null;
}

export interface AnalyticsBudget {
  daily_limit_usd: number | null;
  weekly_limit_usd: number | null;
  notify_at_percent: number;
}

export interface BudgetAlert {
  type: 'approaching' | 'exceeded';
  period: 'daily' | 'weekly';
  current_usd: number;
  limit_usd: number;
}

export interface ConnectionTestStep {
  step: 'binary' | 'api_key' | 'connection';
  status: 'checking' | 'ok' | 'failed';
  detail: string | null;
}

export interface DayHealth {
  date: string;
  status: 'ok' | 'degraded' | 'down' | 'unused';
  errors: number;
  recovered: number;
}

export interface StoreState<T> {
  data: T | null;
  status: 'idle' | 'loading' | 'ready' | 'error';
  error: string | null;
}

export interface AnalyticsDashboardState {
  open: boolean;
  focus: {
    provider?: string;
    sessionId?: string;
    section?: 'kpi' | 'insights' | 'providers' | 'trend';
  } | null;
}

export interface BarValue {
  key: string;
  value: number;
  widthPercent: number;
}

export interface ProviderVisual {
  color: string;
  pattern: string;
  patternLabel: string;
  contrastColor: string;
  backgroundSize?: string;
}

export interface ModelInfo {
  id: string;
  provider: string;
  name: string;
  cost_per_1m_input: number;
  cost_per_1m_output: number;
  context_window: number;
}
```

- [ ] **Step 2: Create model info data file**

Create `src/lib/data/model-info.ts` with hardcoded model pricing. This file is referenced by the provider settings model selector.

```typescript
// src/lib/data/model-info.ts
import type { ModelInfo } from '$lib/types/analytics';

export const MODEL_INFO: ModelInfo[] = [
  // Claude
  { id: 'claude-opus-4-6', provider: 'claude', name: 'Claude Opus 4.6', cost_per_1m_input: 15, cost_per_1m_output: 75, context_window: 200_000 },
  { id: 'claude-sonnet-4-6', provider: 'claude', name: 'Claude Sonnet 4.6', cost_per_1m_input: 3, cost_per_1m_output: 15, context_window: 200_000 },
  { id: 'claude-haiku-4-5', provider: 'claude', name: 'Claude Haiku 4.5', cost_per_1m_input: 0.25, cost_per_1m_output: 1.25, context_window: 200_000 },
  // Gemini
  { id: 'gemini-2.5-pro', provider: 'gemini', name: 'Gemini 2.5 Pro', cost_per_1m_input: 1.25, cost_per_1m_output: 10, context_window: 1_000_000 },
  { id: 'gemini-2.5-flash', provider: 'gemini', name: 'Gemini 2.5 Flash', cost_per_1m_input: 0.15, cost_per_1m_output: 0.60, context_window: 1_000_000 },
  // Qwen
  { id: 'qwen3-coder', provider: 'qwen', name: 'Qwen3 Coder', cost_per_1m_input: 0.16, cost_per_1m_output: 0.16, context_window: 128_000 },
  // Kimi
  { id: 'kimi-k2', provider: 'kimi', name: 'Kimi K2', cost_per_1m_input: 0.60, cost_per_1m_output: 2.40, context_window: 128_000 },
  // Codex
  { id: 'codex-mini', provider: 'codex', name: 'Codex Mini', cost_per_1m_input: 1.50, cost_per_1m_output: 6, context_window: 200_000 },
];

export function getModelsForProvider(provider: string): ModelInfo[] {
  return MODEL_INFO.filter(m => m.provider === provider);
}

export function getModelInfo(modelId: string): ModelInfo | undefined {
  return MODEL_INFO.find(m => m.id === modelId);
}

export function getCheapestModel(provider: string): ModelInfo | undefined {
  const models = getModelsForProvider(provider);
  return models.reduce((cheapest, m) =>
    !cheapest || m.cost_per_1m_input < cheapest.cost_per_1m_input ? m : cheapest,
    undefined as ModelInfo | undefined
  );
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd src-tauri && cd .. && npx tsc --noEmit --pretty 2>&1 | head -20`

Expected: No errors related to the new files (other pre-existing errors are OK).

- [ ] **Step 4: Commit**

```bash
git add src/lib/types/analytics.ts src/lib/data/model-info.ts
git commit -m "feat(analytics): add TypeScript types and model info data"
```

---

### Task 2: Utility — Formatters

**Files:**
- Create: `src/lib/utils/format-analytics.ts`
- Test: Vitest unit tests inline

All analytics components depend on these formatters. Build and test them first.

- [ ] **Step 1: Create the formatter file**

```typescript
// src/lib/utils/format-analytics.ts
import { get } from 'svelte/store';
import { locale } from '$lib/i18n/index';

function getLocale(): string {
  return get(locale) || 'en';
}

export function formatCurrency(usd: number | null | undefined): string {
  if (usd == null || !isFinite(usd)) return '—';
  if (Math.abs(usd) > 0 && Math.abs(usd) < 0.01) return '<$0.01';
  return new Intl.NumberFormat(getLocale(), {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(usd);
}

export function formatTokenCount(count: number | null | undefined): string {
  if (count == null || !isFinite(count)) return '—';
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(count >= 10_000 ? 0 : 1)}K`;
  return String(count);
}

export function formatDuration(ms: number | null | undefined): string {
  if (ms == null || !isFinite(ms)) return '—';
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  const minutes = Math.floor(ms / 60_000);
  const seconds = Math.round((ms % 60_000) / 1000);
  return `${minutes}m ${seconds}s`;
}

export function formatPercent(ratio: number | null | undefined): string {
  if (ratio == null || !isFinite(ratio)) return '—';
  const pct = ratio * 100;
  if (pct < 10 && pct > 0) return `${pct.toFixed(1)}%`;
  return `${Math.round(pct)}%`;
}

export function formatCostVelocity(usdPerMin: number | null | undefined): string {
  if (usdPerMin == null || !isFinite(usdPerMin)) return '—';
  return `${formatCurrency(usdPerMin)}/min`;
}

export function formatTokenRate(tokensPerSec: number | null | undefined): string {
  if (tokensPerSec == null || !isFinite(tokensPerSec)) return '—';
  return `${Math.round(tokensPerSec)} tok/s`;
}

export function formatDate(epochMs: number | null | undefined): string {
  if (epochMs == null || !isFinite(epochMs)) return '—';
  return new Intl.DateTimeFormat(getLocale(), { month: 'short', day: 'numeric' }).format(new Date(epochMs));
}

export function formatDateFull(epochMs: number | null | undefined): string {
  if (epochMs == null || !isFinite(epochMs)) return '—';
  return new Intl.DateTimeFormat(getLocale(), {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(epochMs));
}
```

- [ ] **Step 2: Create formatter tests**

Create `src/lib/utils/format-analytics.test.ts`:

```typescript
// src/lib/utils/format-analytics.test.ts
import { describe, it, expect } from 'vitest';
import { formatCurrency, formatTokenCount, formatDuration, formatPercent, formatCostVelocity, formatTokenRate } from './format-analytics';

describe('formatCurrency', () => {
  it('formats USD with 2 decimals', () => {
    expect(formatCurrency(1.5)).toBe('$1.50');
  });
  it('returns em-dash for null/undefined', () => {
    expect(formatCurrency(null)).toBe('—');
    expect(formatCurrency(undefined)).toBe('—');
  });
  it('returns <$0.01 for tiny values', () => {
    expect(formatCurrency(0.005)).toBe('<$0.01');
  });
  it('handles zero', () => {
    expect(formatCurrency(0)).toBe('$0.00');
  });
  it('returns em-dash for NaN/Infinity', () => {
    expect(formatCurrency(NaN)).toBe('—');
    expect(formatCurrency(Infinity)).toBe('—');
  });
});

describe('formatTokenCount', () => {
  it('formats millions', () => {
    expect(formatTokenCount(1_500_000)).toBe('1.5M');
  });
  it('formats thousands', () => {
    expect(formatTokenCount(98_000)).toBe('98K');
    expect(formatTokenCount(1_500)).toBe('1.5K');
  });
  it('formats small numbers as-is', () => {
    expect(formatTokenCount(42)).toBe('42');
  });
  it('returns em-dash for null', () => {
    expect(formatTokenCount(null)).toBe('—');
  });
});

describe('formatDuration', () => {
  it('formats milliseconds', () => {
    expect(formatDuration(500)).toBe('500ms');
  });
  it('formats seconds', () => {
    expect(formatDuration(3500)).toBe('3.5s');
  });
  it('formats minutes and seconds', () => {
    expect(formatDuration(125_000)).toBe('2m 5s');
  });
  it('returns em-dash for null', () => {
    expect(formatDuration(null)).toBe('—');
  });
});

describe('formatPercent', () => {
  it('formats ratio to percent', () => {
    expect(formatPercent(0.85)).toBe('85%');
  });
  it('shows decimal for small percentages', () => {
    expect(formatPercent(0.035)).toBe('3.5%');
  });
  it('returns em-dash for null', () => {
    expect(formatPercent(null)).toBe('—');
  });
});

describe('formatCostVelocity', () => {
  it('formats as currency per minute', () => {
    expect(formatCostVelocity(0.05)).toMatch(/\$0\.05\/min/);
  });
});

describe('formatTokenRate', () => {
  it('formats tokens per second', () => {
    expect(formatTokenRate(45.7)).toBe('46 tok/s');
  });
});
```

- [ ] **Step 3: Run formatter tests**

Run: `npx vitest run src/lib/utils/format-analytics.test.ts 2>&1 | tail -15`

Expected: All tests pass.

- [ ] **Step 4: Verify no compile errors**

Run: `npx tsc --noEmit --pretty 2>&1 | grep format-analytics || echo "OK"`

- [ ] **Step 5: Commit**

```bash
git add src/lib/utils/format-analytics.ts src/lib/utils/format-analytics.test.ts
git commit -m "feat(analytics): add locale-aware formatters with tests"
```

---

### Task 3: Utility — Accessibility Foundation

**Files:**
- Create: `src/lib/utils/a11y-motion.ts`
- Create: `src/lib/utils/a11y-announcer.ts`
- Create: `src/lib/utils/a11y-labels.ts`
- Create: `src/lib/utils/a11y-focus.ts`

Four small, focused utilities. Each is independent.

- [ ] **Step 1: Create reduced motion store**

```typescript
// src/lib/utils/a11y-motion.ts
import { readable } from 'svelte/store';

export const prefersReducedMotion = readable<boolean>(false, (set) => {
  if (typeof window === 'undefined') return;
  const query = window.matchMedia('(prefers-reduced-motion: reduce)');
  set(query.matches);
  const handler = (e: MediaQueryListEvent) => set(e.matches);
  query.addEventListener('change', handler);
  return () => query.removeEventListener('change', handler);
});

export function motionTransition(duration: string): string {
  // Note: this reads the store synchronously — use within Svelte components
  // where the store value is available via $prefersReducedMotion
  return duration;
}
```

- [ ] **Step 2: Create screen reader announcer**

```typescript
// src/lib/utils/a11y-announcer.ts

class ScreenReaderAnnouncer {
  private queue: string[] = [];
  private throttleMs: number;
  private lastAnnounce = 0;
  private politeEl: HTMLElement | null = null;
  private assertiveEl: HTMLElement | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;

  constructor(throttleMs = 5000) {
    this.throttleMs = throttleMs;
  }

  mount(container: HTMLElement): void {
    this.politeEl = this.createLiveRegion(container, 'polite');
    this.assertiveEl = this.createLiveRegion(container, 'assertive');
  }

  private createLiveRegion(container: HTMLElement, politeness: string): HTMLElement {
    const el = document.createElement('div');
    el.setAttribute('aria-live', politeness);
    el.setAttribute('aria-atomic', 'true');
    el.setAttribute('role', 'status');
    Object.assign(el.style, {
      position: 'absolute', width: '1px', height: '1px',
      padding: '0', margin: '-1px', overflow: 'hidden',
      clip: 'rect(0,0,0,0)', whiteSpace: 'nowrap', border: '0',
    });
    container.appendChild(el);
    return el;
  }

  announce(message: string): void {
    const now = Date.now();
    if (now - this.lastAnnounce < this.throttleMs) {
      this.queue.push(message);
      if (!this.timer) {
        this.timer = setTimeout(() => this.flush(), this.throttleMs - (now - this.lastAnnounce));
      }
      return;
    }
    this.doAnnounce(message, this.politeEl);
  }

  announceUrgent(message: string): void {
    this.doAnnounce(message, this.assertiveEl);
  }

  private doAnnounce(message: string, el: HTMLElement | null): void {
    if (!el) return;
    el.textContent = '';
    requestAnimationFrame(() => { el.textContent = message; });
    this.lastAnnounce = Date.now();
  }

  private flush(): void {
    this.timer = null;
    if (this.queue.length === 0) return;
    const fused = this.queue.join('. ');
    this.queue = [];
    this.doAnnounce(fused, this.politeEl);
  }

  destroy(): void {
    if (this.timer) clearTimeout(this.timer);
    this.politeEl?.remove();
    this.assertiveEl?.remove();
  }
}

export const analyticsAnnouncer = new ScreenReaderAnnouncer(5000);
```

- [ ] **Step 3: Create semantic labels builder**

```typescript
// src/lib/utils/a11y-labels.ts
import { formatCurrency, formatTokenCount, formatDuration, formatPercent } from './format-analytics';
import type { LiveSessionMetrics, ProviderAnalytics, DailyStats, BudgetAlert, ConnectionTestStep, DayHealth } from '$lib/types/analytics';

export function barLabel(m: LiveSessionMetrics): string {
  const parts: string[] = [];
  if (m.context_percent != null) parts.push(`context at ${Math.round(m.context_percent)}%`);
  parts.push(`cost ${formatCurrency(m.cost_usd)}`);
  if (m.cost_velocity_usd_per_min > 0) parts.push(`at ${formatCurrency(m.cost_velocity_usd_per_min)} per minute`);
  if (m.cost_projection_usd != null) parts.push(`projected ${formatCurrency(m.cost_projection_usd)}`);
  parts.push(`${formatTokenCount(m.input_tokens + m.output_tokens)} tokens`);
  if (m.cache_read_tokens > 0) {
    const rate = m.cache_read_tokens / Math.max(1, m.input_tokens);
    parts.push(`cache efficiency ${formatPercent(rate)}`);
  }
  return `Current session: ${parts.join(', ')}`;
}

export function kpiLabel(title: string, value: string, delta?: string): string {
  return delta ? `${title}: ${value}, ${delta}` : `${title}: ${value}`;
}

export function providerBarLabel(provider: string, a: ProviderAnalytics, totalCost: number): string {
  const pct = totalCost > 0 ? Math.round((a.total_cost_usd / totalCost) * 100) : 0;
  return `${provider}: ${formatCurrency(a.total_cost_usd)}, ${pct}% of total, ${formatTokenCount(a.total_input_tokens + a.total_output_tokens)} tokens, ${a.total_sessions} sessions, error rate ${formatPercent(a.error_rate)}`;
}

export function contextProgressLabel(percent: number): string {
  const zone = percent > 80 ? 'danger zone' : percent > 60 ? 'warning zone' : 'safe zone';
  return `Context usage: ${Math.round(percent)}%, ${zone}`;
}

export function healthTimelineLabel(days: DayHealth[]): string {
  const ok = days.filter(d => d.status === 'ok').length;
  const degraded = days.filter(d => d.status === 'degraded').length;
  const down = days.filter(d => d.status === 'down').length;
  const parts = [`${ok} days ok`];
  if (degraded) parts.push(`${degraded} with errors`);
  if (down) parts.push(`${down} down`);
  return `Health last ${days.length} days: ${parts.join(', ')}`;
}

export function trendBarLabel(day: DailyStats): string {
  return `${day.date}: ${formatCurrency(day.total_cost_usd)}, ${formatTokenCount(day.total_tokens)} tokens, ${day.sessions} sessions`;
}

export function budgetAlertLabel(alert: BudgetAlert): string {
  const pct = Math.round((alert.current_usd / alert.limit_usd) * 100);
  return `${alert.type === 'exceeded' ? 'Exceeded' : 'Approaching'} ${alert.period} budget: ${formatCurrency(alert.current_usd)} of ${formatCurrency(alert.limit_usd)} (${pct}%)`;
}

export function connectionStepLabel(step: ConnectionTestStep): string {
  const names = { binary: 'Binary check', api_key: 'API key check', connection: 'Connection test' };
  const status = step.status === 'ok' ? 'passed' : step.status === 'failed' ? 'failed' : 'in progress';
  return `${names[step.step]}: ${status}${step.detail ? ` — ${step.detail}` : ''}`;
}
```

- [ ] **Step 4: Create focus manager**

```typescript
// src/lib/utils/a11y-focus.ts

class FocusManager {
  private stack: HTMLElement[] = [];

  push(target: HTMLElement): void {
    const current = document.activeElement as HTMLElement;
    if (current) this.stack.push(current);
    target.focus();
  }

  pop(): void {
    const prev = this.stack.pop();
    if (!prev) return;
    // Check if element is still in DOM
    if (prev.isConnected) {
      prev.focus();
    } else {
      // Fallback: find closest connected ancestor
      let parent = prev.parentElement;
      while (parent && !parent.isConnected) parent = parent.parentElement;
      parent?.focus();
    }
  }

  reset(): void {
    this.stack = [];
  }

  get depth(): number {
    return this.stack.length;
  }
}

export const focusManager = new FocusManager();
```

- [ ] **Step 5: Verify no compile errors**

Run: `npx tsc --noEmit --pretty 2>&1 | grep -E "a11y-(motion|announcer|labels|focus)" || echo "OK"`

- [ ] **Step 6: Commit**

```bash
git add src/lib/utils/a11y-motion.ts src/lib/utils/a11y-announcer.ts src/lib/utils/a11y-labels.ts src/lib/utils/a11y-focus.ts
git commit -m "feat(a11y): add motion, announcer, labels, and focus utilities"
```

---

### Task 4: Utility — Tooltip, Tween, Bar Scale, Grid Nav, Provider Patterns

**Files:**
- Create: `src/lib/utils/tooltip.ts`
- Create: `src/lib/utils/tween.ts`
- Create: `src/lib/utils/bar-scale.ts`
- Create: `src/lib/utils/grid-navigation.ts`
- Create: `src/lib/utils/provider-patterns.ts`

Five small, independent utilities. Each is a single-responsibility module.

- [ ] **Step 1: Create tooltip Svelte action**

```typescript
// src/lib/utils/tooltip.ts
import type { ActionReturn } from 'svelte/action';

interface TooltipOptions {
  text: string;
  position?: 'top' | 'bottom' | 'left' | 'right';
  delay?: number;
}

let activeTooltip: HTMLElement | null = null;
let uid = 0;

export function tooltip(node: HTMLElement, param: string | TooltipOptions): ActionReturn<string | TooltipOptions> {
  let opts: TooltipOptions = typeof param === 'string' ? { text: param } : param;
  const id = `tooltip-${++uid}`;
  let el: HTMLElement | null = null;
  let showTimer: ReturnType<typeof setTimeout> | null = null;

  function show() {
    if (activeTooltip) activeTooltip.remove();
    el = document.createElement('div');
    el.id = id;
    el.role = 'tooltip';
    el.textContent = opts.text;
    Object.assign(el.style, {
      position: 'fixed', zIndex: '9999',
      padding: '4px 8px', maxWidth: '250px',
      background: 'var(--bg-tertiary, #2a2a2a)',
      color: 'var(--text-primary, #f0f0f0)',
      border: '2px solid var(--border, #5a5a5a)',
      fontSize: 'var(--font-size-small, 12px)',
      fontFamily: 'var(--font-ui)',
      pointerEvents: 'none',
    });
    document.body.appendChild(el);
    activeTooltip = el;
    node.setAttribute('aria-describedby', id);
    position(el);
  }

  function position(tip: HTMLElement) {
    const rect = node.getBoundingClientRect();
    const tipRect = tip.getBoundingClientRect();
    const pos = opts.position ?? 'top';
    let top = 0, left = 0;
    if (pos === 'top') {
      top = rect.top - tipRect.height - 6;
      left = rect.left + (rect.width - tipRect.width) / 2;
      if (top < 4) { top = rect.bottom + 6; } // flip
    } else if (pos === 'bottom') {
      top = rect.bottom + 6;
      left = rect.left + (rect.width - tipRect.width) / 2;
    }
    left = Math.max(4, Math.min(left, window.innerWidth - tipRect.width - 4));
    tip.style.top = `${top}px`;
    tip.style.left = `${left}px`;
  }

  function hide() {
    if (showTimer) { clearTimeout(showTimer); showTimer = null; }
    if (el) { el.remove(); el = null; activeTooltip = null; }
    node.removeAttribute('aria-describedby');
  }

  function onEnter() { showTimer = setTimeout(show, opts.delay ?? 300); }
  function onLeave() { hide(); }
  function onFocus() { show(); }
  function onBlur() { hide(); }
  function onKeydown(e: KeyboardEvent) { if (e.key === 'Escape') hide(); }

  node.addEventListener('mouseenter', onEnter);
  node.addEventListener('mouseleave', onLeave);
  node.addEventListener('focus', onFocus);
  node.addEventListener('blur', onBlur);
  node.addEventListener('keydown', onKeydown);

  return {
    update(newParam: string | TooltipOptions) {
      opts = typeof newParam === 'string' ? { text: newParam } : newParam;
      if (el) el.textContent = opts.text;
    },
    destroy() {
      hide();
      node.removeEventListener('mouseenter', onEnter);
      node.removeEventListener('mouseleave', onLeave);
      node.removeEventListener('focus', onFocus);
      node.removeEventListener('blur', onBlur);
      node.removeEventListener('keydown', onKeydown);
    },
  };
}
```

- [ ] **Step 2: Create number tween utility**

```typescript
// src/lib/utils/tween.ts
import { readable, type Readable } from 'svelte/store';

export function easeOutCubic(t: number): number {
  return 1 - Math.pow(1 - t, 3);
}

export function tweenValue(
  from: number,
  to: number,
  duration = 600,
  easing: (t: number) => number = easeOutCubic,
): Readable<number> {
  // Check reduced motion preference synchronously
  const reducedMotion = typeof window !== 'undefined'
    && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  if (reducedMotion || duration <= 0) {
    return readable(to);
  }

  return readable(from, (set) => {
    const start = performance.now();
    let frame: number;

    function tick(now: number) {
      const elapsed = now - start;
      const t = Math.min(elapsed / duration, 1);
      set(from + (to - from) * easing(t));
      if (t < 1) {
        frame = requestAnimationFrame(tick);
      }
    }

    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  });
}
```

- [ ] **Step 3: Create bar scale normalizer**

```typescript
// src/lib/utils/bar-scale.ts
import type { BarValue } from '$lib/types/analytics';

interface BarScaleOptions {
  maxWidthPercent?: number;
  minWidthPercent?: number;
  scaleMode?: 'proportional' | 'logarithmic';
}

export function normalizeBarScale(
  items: Array<{ key: string; value: number }>,
  options: BarScaleOptions = {},
): BarValue[] {
  const {
    maxWidthPercent = 90,
    minWidthPercent = 2,
    scaleMode = 'proportional',
  } = options;

  // Filter invalid
  const valid = items.filter(i => i.value != null && isFinite(i.value) && i.value >= 0);
  if (valid.length === 0) return items.map(i => ({ key: i.key, value: i.value, widthPercent: 0 }));

  const maxVal = Math.max(...valid.map(i => i.value));
  if (maxVal === 0) return valid.map(i => ({ ...i, widthPercent: 0 }));

  return valid.map(item => {
    let ratio: number;
    if (scaleMode === 'logarithmic' && maxVal > 0) {
      ratio = Math.log1p(item.value) / Math.log1p(maxVal);
    } else {
      ratio = item.value / maxVal;
    }
    const width = Math.max(item.value > 0 ? minWidthPercent : 0, ratio * maxWidthPercent);
    return { key: item.key, value: item.value, widthPercent: width };
  });
}
```

- [ ] **Step 4: Create grid navigation action**

```typescript
// src/lib/utils/grid-navigation.ts
import type { ActionReturn } from 'svelte/action';

export function gridNavigation(node: HTMLElement): ActionReturn {
  function getCells(): HTMLElement[] {
    return Array.from(node.querySelectorAll<HTMLElement>('[role="gridcell"], [role="rowheader"]'))
      .filter(el => el.offsetParent !== null);
  }

  function getRows(): HTMLElement[][] {
    const rows: HTMLElement[][] = [];
    node.querySelectorAll<HTMLElement>('[role="row"]').forEach(row => {
      if (row.offsetParent === null) return;
      const cells = Array.from(row.querySelectorAll<HTMLElement>('[role="gridcell"], [role="rowheader"]'))
        .filter(el => el.offsetParent !== null);
      if (cells.length) rows.push(cells);
    });
    return rows;
  }

  function findPosition(cell: HTMLElement): [number, number] {
    const rows = getRows();
    for (let r = 0; r < rows.length; r++) {
      const c = rows[r].indexOf(cell);
      if (c >= 0) return [r, c];
    }
    return [-1, -1];
  }

  function focusCell(row: number, col: number) {
    const rows = getRows();
    if (row < 0 || row >= rows.length) return;
    const clampedCol = Math.min(col, rows[row].length - 1);
    const target = rows[row][clampedCol];
    // Roving tabindex
    getCells().forEach(c => c.setAttribute('tabindex', '-1'));
    target.setAttribute('tabindex', '0');
    target.focus();
  }

  function handleKeydown(e: KeyboardEvent) {
    const active = document.activeElement as HTMLElement;
    const [row, col] = findPosition(active);
    if (row < 0) return;
    const rows = getRows();

    switch (e.key) {
      case 'ArrowDown': e.preventDefault(); focusCell(row + 1, col); break;
      case 'ArrowUp': e.preventDefault(); focusCell(row - 1, col); break;
      case 'ArrowRight': e.preventDefault(); focusCell(row, col + 1); break;
      case 'ArrowLeft': e.preventDefault(); focusCell(row, col - 1); break;
      case 'Home':
        e.preventDefault();
        focusCell(e.ctrlKey ? 0 : row, 0);
        break;
      case 'End':
        e.preventDefault();
        focusCell(e.ctrlKey ? rows.length - 1 : row, Infinity);
        break;
      case 'Enter':
        e.preventDefault();
        active.click();
        break;
    }
  }

  // Initialize: first cell gets tabindex 0
  const cells = getCells();
  cells.forEach((c, i) => c.setAttribute('tabindex', i === 0 ? '0' : '-1'));

  node.addEventListener('keydown', handleKeydown);
  return { destroy: () => node.removeEventListener('keydown', handleKeydown) };
}
```

- [ ] **Step 5: Create provider patterns map**

```typescript
// src/lib/utils/provider-patterns.ts
import type { ProviderVisual } from '$lib/types/analytics';

export const PROVIDER_VISUALS: Record<string, ProviderVisual> = {
  claude: {
    color: '#1d4ed8',
    pattern: '#1d4ed8',
    patternLabel: 'solid blue',
    contrastColor: '#ffffff',
  },
  gemini: {
    color: '#16a34a',
    pattern: 'repeating-linear-gradient(90deg, #16a34a 0px, #16a34a 4px, transparent 4px, transparent 6px)',
    patternLabel: 'green vertical stripes',
    contrastColor: '#ffffff',
  },
  qwen: {
    color: '#ca8a04',
    pattern: 'repeating-linear-gradient(45deg, #ca8a04 0px, #ca8a04 3px, transparent 3px, transparent 6px)',
    patternLabel: 'amber diagonal',
    contrastColor: '#000000',
  },
  kimi: {
    color: '#9333ea',
    pattern: 'radial-gradient(circle, #9333ea 1px, transparent 1px)',
    patternLabel: 'purple dotted',
    contrastColor: '#ffffff',
    backgroundSize: '4px 4px',
  },
  codex: {
    color: '#dc2626',
    pattern: 'repeating-linear-gradient(0deg, #dc2626 0px, #dc2626 3px, transparent 3px, transparent 6px)',
    patternLabel: 'red horizontal stripes',
    contrastColor: '#ffffff',
  },
};

const FALLBACK_VISUAL: ProviderVisual = {
  color: '#6b7280',
  pattern: '#6b7280',
  patternLabel: 'gray solid',
  contrastColor: '#ffffff',
};

export function getProviderVisual(provider: string): ProviderVisual {
  return PROVIDER_VISUALS[provider.toLowerCase()] ?? FALLBACK_VISUAL;
}

export function barStyle(provider: string, widthPercent: number): string {
  const v = getProviderVisual(provider);
  const bg = v.pattern.includes('gradient') || v.pattern.includes('radial')
    ? `background: ${v.pattern}`
    : `background-color: ${v.pattern}`;
  const size = v.backgroundSize ? `; background-size: ${v.backgroundSize}` : '';
  return `${bg}${size}; width: ${widthPercent}%; height: 100%`;
}
```

- [ ] **Step 6: Create tests for bar-scale and tween**

Create `src/lib/utils/bar-scale.test.ts`:

```typescript
// src/lib/utils/bar-scale.test.ts
import { describe, it, expect } from 'vitest';
import { normalizeBarScale } from './bar-scale';

describe('normalizeBarScale', () => {
  it('normalizes proportionally to max value', () => {
    const result = normalizeBarScale([
      { key: 'a', value: 100 },
      { key: 'b', value: 50 },
      { key: 'c', value: 25 },
    ]);
    expect(result[0].widthPercent).toBe(90); // max
    expect(result[1].widthPercent).toBe(45);
    expect(result[2].widthPercent).toBe(22.5);
  });

  it('returns 0 width for all-zero values', () => {
    const result = normalizeBarScale([
      { key: 'a', value: 0 },
      { key: 'b', value: 0 },
    ]);
    expect(result.every(r => r.widthPercent === 0)).toBe(true);
  });

  it('handles empty input', () => {
    expect(normalizeBarScale([])).toEqual([]);
  });

  it('applies minimum width for non-zero values', () => {
    const result = normalizeBarScale([
      { key: 'a', value: 1000 },
      { key: 'b', value: 1 },
    ]);
    expect(result[1].widthPercent).toBeGreaterThanOrEqual(2);
  });

  it('supports logarithmic scale', () => {
    const result = normalizeBarScale(
      [{ key: 'a', value: 1000 }, { key: 'b', value: 1 }],
      { scaleMode: 'logarithmic' },
    );
    // Log scale compresses large differences
    expect(result[1].widthPercent).toBeGreaterThan(5);
  });
});
```

Create `src/lib/utils/tween.test.ts`:

```typescript
// src/lib/utils/tween.test.ts
import { describe, it, expect } from 'vitest';
import { easeOutCubic } from './tween';

describe('easeOutCubic', () => {
  it('returns 0 at t=0', () => {
    expect(easeOutCubic(0)).toBe(0);
  });
  it('returns 1 at t=1', () => {
    expect(easeOutCubic(1)).toBe(1);
  });
  it('returns ~0.875 at t=0.5 (decelerating)', () => {
    expect(easeOutCubic(0.5)).toBe(0.875);
  });
  it('is monotonically increasing', () => {
    let prev = 0;
    for (let t = 0.1; t <= 1; t += 0.1) {
      const val = easeOutCubic(t);
      expect(val).toBeGreaterThan(prev);
      prev = val;
    }
  });
});
```

- [ ] **Step 7: Run utility tests**

Run: `npx vitest run src/lib/utils/bar-scale.test.ts src/lib/utils/tween.test.ts 2>&1 | tail -15`

Expected: All tests pass.

- [ ] **Step 8: Verify no compile errors**

Run: `npx tsc --noEmit --pretty 2>&1 | grep -E "(tooltip|tween|bar-scale|grid-nav|provider-patterns)" || echo "OK"`

- [ ] **Step 9: Commit**

```bash
git add src/lib/utils/tooltip.ts src/lib/utils/tween.ts src/lib/utils/tween.test.ts src/lib/utils/bar-scale.ts src/lib/utils/bar-scale.test.ts src/lib/utils/grid-navigation.ts src/lib/utils/provider-patterns.ts
git commit -m "feat(analytics): add tooltip, tween, bar-scale, grid-nav, and provider patterns utilities with tests"
```

---

### Task 5: Adapter + Backend — New Methods

**Files:**
- Modify: `src/lib/adapter/index.ts`
- Modify: `src/lib/adapter/tauri.ts`
- Create: `src-tauri/src/commands/provider.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

Add 9 new adapter methods and 2 new Rust commands.

- [ ] **Step 1: Add analytics methods to Adapter interface**

In `src/lib/adapter/index.ts`, add these imports at the top:

```typescript
import type { ProviderAnalytics, ModelAnalytics, DailyStats, SessionMetrics, TimeRange, ConnectionTestStep } from '$lib/types/analytics';
```

Then add these methods at the end of the `Adapter` interface (before the closing `}`). Read the file to find the exact last method, then add after it:

```typescript
  // Analytics
  analyticsProvider(provider: string, from?: number, to?: number): Promise<ProviderAnalytics>;
  analyticsCompare(from?: number, to?: number): Promise<ProviderAnalytics[]>;
  analyticsModelBreakdown(provider: string, from?: number, to?: number): Promise<ModelAnalytics[]>;
  analyticsSession(sessionId: string): Promise<SessionMetrics | null>;
  analyticsDaily(provider?: string, days?: number): Promise<DailyStats[]>;
  analyticsActive(): Promise<SessionMetrics[]>;

  // Provider management
  testProviderConnection(provider: string): Promise<void>;
  onConnectionTest(callback: (step: ConnectionTestStep) => void): Promise<() => void>;
  reloadNormalizers(): Promise<void>;
```

- [ ] **Step 2: Add implementations to TauriAdapter**

In `src/lib/adapter/tauri.ts`, add the import at the top:

```typescript
import type { ProviderAnalytics, ModelAnalytics, DailyStats, SessionMetrics, ConnectionTestStep } from '$lib/types/analytics';
```

Then add these methods at the end of the `TauriAdapter` class:

```typescript
  async analyticsProvider(provider: string, from?: number, to?: number): Promise<ProviderAnalytics> {
    return invoke<ProviderAnalytics>('analytics_provider', { provider, from, to });
  }
  async analyticsCompare(from?: number, to?: number): Promise<ProviderAnalytics[]> {
    return invoke<ProviderAnalytics[]>('analytics_compare', { from, to });
  }
  async analyticsModelBreakdown(provider: string, from?: number, to?: number): Promise<ModelAnalytics[]> {
    return invoke<ModelAnalytics[]>('analytics_model_breakdown', { provider, from, to });
  }
  async analyticsSession(sessionId: string): Promise<SessionMetrics | null> {
    return invoke<SessionMetrics | null>('analytics_session', { sessionId });
  }
  async analyticsDaily(provider?: string, days?: number): Promise<DailyStats[]> {
    return invoke<DailyStats[]>('analytics_daily', { provider, days });
  }
  async analyticsActive(): Promise<SessionMetrics[]> {
    return invoke<SessionMetrics[]>('analytics_active');
  }
  async testProviderConnection(provider: string): Promise<void> {
    return invoke<void>('test_provider_connection', { provider });
  }
  async onConnectionTest(callback: (step: ConnectionTestStep) => void): Promise<() => void> {
    const { listen } = await import('@tauri-apps/api/event');
    return listen<ConnectionTestStep>('connection_test_step', (event) => {
      callback(event.payload);
    });
  }
  async reloadNormalizers(): Promise<void> {
    return invoke<void>('reload_normalizers');
  }
```

- [ ] **Step 3: Add `api_key_env` field to `CliConfig`**

The `CliConfig` struct in `src-tauri/src/normalizer/mod.rs` currently has no `api_key_env` field. Add it:

In `src-tauri/src/normalizer/mod.rs`, find the `CliConfig` struct (around line 36) and add the new field:

```rust
pub struct CliConfig {
    pub name: String,
    pub binary: String,
    #[serde(default)]
    pub programmatic_args: Vec<String>,
    #[serde(default)]
    pub resume_args: Vec<String>,
    #[serde(default)]
    pub version_command: Vec<String>,
    #[serde(default)]
    pub update_command: Vec<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,  // NEW: env var name for API key (e.g. "ANTHROPIC_API_KEY")
}
```

Then add `api_key_env` to the normalizer TOML files that need it:

In `src-tauri/normalizers/claude.toml`, under `[cli]` add:
```toml
api_key_env = "ANTHROPIC_API_KEY"
```

In `src-tauri/normalizers/gemini.toml`, under `[cli]` add:
```toml
api_key_env = "GEMINI_API_KEY"
```

In `src-tauri/normalizers/qwen.toml`, under `[cli]` add:
```toml
api_key_env = "DASHSCOPE_API_KEY"
```

In `src-tauri/normalizers/kimi.toml`, under `[cli]` add:
```toml
api_key_env = "MOONSHOT_API_KEY"
```

In `src-tauri/normalizers/codex.toml`, under `[cli]` add:
```toml
api_key_env = "OPENAI_API_KEY"
```

- [ ] **Step 4: Create Rust provider commands**

Create `src-tauri/src/commands/provider.rs`.

**IMPORTANT**: The `NormalizerRegistry` is NOT registered as standalone managed state. It lives inside `StructuredAgentTransport`, which IS managed. Access it via `transport.registry()`.

```rust
use crate::transport::StructuredAgentTransport;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, serde::Serialize)]
struct ConnectionTestStep {
    step: String,
    status: String,
    detail: Option<String>,
}

#[tauri::command]
pub async fn test_provider_connection(
    provider: String,
    transport: State<'_, StructuredAgentTransport>,
    app: AppHandle,
) -> Result<(), String> {
    let registry = transport.registry();
    let reg = registry.lock().unwrap();
    let config = reg.get_config(&provider)
        .ok_or_else(|| format!("Unknown provider: {}", provider))?;
    let binary = config.cli.binary.clone();
    let api_key_env = config.cli.api_key_env.clone();
    let version_cmd = config.cli.version_command.clone();
    drop(reg);

    // Step 1: Binary check
    let _ = app.emit("connection_test_step", ConnectionTestStep {
        step: "binary".into(),
        status: "checking".into(),
        detail: None,
    });

    let binary_path = which::which(&binary).ok();
    let _ = app.emit("connection_test_step", ConnectionTestStep {
        step: "binary".into(),
        status: if binary_path.is_some() { "ok" } else { "failed" }.into(),
        detail: binary_path.as_ref().map(|p| p.display().to_string()),
    });

    if binary_path.is_none() {
        return Ok(());
    }

    // Step 2: API key check
    let _ = app.emit("connection_test_step", ConnectionTestStep {
        step: "api_key".into(),
        status: "checking".into(),
        detail: None,
    });

    let api_key_set = api_key_env.as_ref()
        .map(|env| std::env::var(env).is_ok())
        .unwrap_or(true); // No env var required = OK

    let _ = app.emit("connection_test_step", ConnectionTestStep {
        step: "api_key".into(),
        status: if api_key_set { "ok" } else { "failed" }.into(),
        detail: api_key_env.clone(),
    });

    if !api_key_set {
        return Ok(());
    }

    // Step 3: Connection test (use version_command from TOML if available, else --version)
    let _ = app.emit("connection_test_step", ConnectionTestStep {
        step: "connection".into(),
        status: "checking".into(),
        detail: None,
    });

    let start = std::time::Instant::now();
    let output = if !version_cmd.is_empty() {
        tokio::process::Command::new(&version_cmd[0])
            .args(&version_cmd[1..])
            .output()
            .await
    } else {
        tokio::process::Command::new(&binary)
            .args(["--version"])
            .output()
            .await
    };

    match output {
        Ok(o) if o.status.success() => {
            let latency = start.elapsed().as_millis();
            let _ = app.emit("connection_test_step", ConnectionTestStep {
                step: "connection".into(),
                status: "ok".into(),
                detail: Some(format!("{}ms", latency)),
            });
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let _ = app.emit("connection_test_step", ConnectionTestStep {
                step: "connection".into(),
                status: "failed".into(),
                detail: Some(stderr.to_string()),
            });
        }
        Err(e) => {
            let _ = app.emit("connection_test_step", ConnectionTestStep {
                step: "connection".into(),
                status: "failed".into(),
                detail: Some(e.to_string()),
            });
        }
    }

    Ok(())
}

#[tauri::command]
pub fn reload_normalizers(
    transport: State<'_, StructuredAgentTransport>,
) -> Result<(), String> {
    let normalizers_dir = Path::new("normalizers");
    let new_registry = crate::normalizer::NormalizerRegistry::load_from_dir(normalizers_dir)?;

    let registry = transport.registry();
    *registry.lock().unwrap() = new_registry;

    // Note: RetryPolicy updates should also be handled. The transport holds
    // retry_policies internally — if a public method is needed, add
    // `transport.reload_retry_policies()` to StructuredAgentTransport.
    // For now, the registry reload is sufficient for config changes.
    Ok(())
}
```

- [ ] **Step 5: Register the new module and commands**

In `src-tauri/src/commands/mod.rs`, add:

```rust
pub mod provider;
```

In `src-tauri/src/lib.rs`, add the two new commands to the `invoke_handler` list (before the closing `]`):

```rust
            commands::provider::test_provider_connection,
            commands::provider::reload_normalizers,
```

- [ ] **Step 6: Verify Rust compiles**

Run: `cd src-tauri && cargo check 2>&1 | tail -10`

Expected: Should compile. If `which` crate is missing, add it: `cargo add which`.

- [ ] **Step 7: Verify frontend compiles**

Run: `npx tsc --noEmit --pretty 2>&1 | tail -5`

- [ ] **Step 8: Commit**

```bash
git add src/lib/adapter/index.ts src/lib/adapter/tauri.ts src-tauri/src/commands/provider.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/src/normalizer/mod.rs src-tauri/normalizers/*.toml
git commit -m "feat(analytics): add adapter methods, provider backend commands, and api_key_env config"
```

---

### Task 6: Analytics Store

**Files:**
- Create: `src/lib/stores/analytics.ts`
- Modify: `src/lib/stores/ui.ts`

The core store — live tracking + historical fetch + budget + cache.

- [ ] **Step 1: Add analytics dashboard state to ui.ts**

In `src/lib/stores/ui.ts`, add at the end:

```typescript
import type { AnalyticsDashboardState } from '$lib/types/analytics';
export const analyticsDashboard = writable<AnalyticsDashboardState>({ open: false, focus: null });
```

- [ ] **Step 2: Create analytics store**

Create `src/lib/stores/analytics.ts`. This is the largest single file. Key behaviors:

- `liveMetrics` writable — updated by `startLiveTracking` via event subscription
- Historical stores wrapped in `StoreState<T>` for graceful degradation
- Cache with TTL — fetches skip if data is fresh
- `checkBudget` compares daily spend against configured limits

```typescript
// src/lib/stores/analytics.ts
import { writable, get } from 'svelte/store';
import type { Adapter } from '$lib/adapter/index';
import type {
  LiveSessionMetrics, SessionMetrics, ProviderAnalytics, ModelAnalytics,
  DailyStats, TimeRange, AnalyticsBudget, BudgetAlert, StoreState,
} from '$lib/types/analytics';
import type { AgentEventPayload } from '$lib/types/agent-event';

// === CONSTANTS ===
const CACHE_TTL_HISTORICAL = 30_000;
const CACHE_TTL_LIVE = 5_000;
const MAX_RETRIES = 3;
const RETRY_DELAY = 10_000;

// === LIVE ===
export const liveMetrics = writable<LiveSessionMetrics | null>(null);
export const budgetAlerts = writable<BudgetAlert[]>([]);

// === HISTORICAL ===
function emptyState<T>(): StoreState<T> {
  return { data: null, status: 'idle', error: null };
}

export const providerAnalytics = writable<StoreState<ProviderAnalytics[]>>(emptyState());
export const dailyStats = writable<StoreState<DailyStats[]>>(emptyState());
export const modelBreakdown = writable<StoreState<ModelAnalytics[]>>(emptyState());

// === BUDGET ===
export const budget = writable<AnalyticsBudget>({
  daily_limit_usd: null,
  weekly_limit_usd: null,
  notify_at_percent: 80,
});

// === CONFIG PROPAGATION ===
export const providerConfigVersion = writable<number>(0);

// === CACHE ===
interface CacheEntry<T> {
  data: T;
  fetchedAt: number;
}

const cache: Record<string, CacheEntry<unknown>> = {};

function getCached<T>(key: string, ttl: number): T | null {
  const entry = cache[key];
  if (entry && Date.now() - entry.fetchedAt < ttl) return entry.data as T;
  return null;
}

function setCache<T>(key: string, data: T): void {
  cache[key] = { data, fetchedAt: Date.now() };
}

export function invalidateCache(): void {
  Object.keys(cache).forEach(k => delete cache[k]);
}

// Spec-named alias for fetchProviderAnalytics
export { fetchProviderAnalytics as fetchCompareProviders };

// === FETCH FUNCTIONS ===

export async function fetchProviderAnalytics(
  adapter: Adapter,
  timeRange?: TimeRange,
  forceRefresh = false,
): Promise<void> {
  const cacheKey = `provider-${timeRange?.start_epoch_ms ?? 'all'}`;
  if (!forceRefresh) {
    const cached = getCached<ProviderAnalytics[]>(cacheKey, CACHE_TTL_HISTORICAL);
    if (cached) { providerAnalytics.set({ data: cached, status: 'ready', error: null }); return; }
  }
  providerAnalytics.update(s => ({ ...s, status: 'loading' }));
  try {
    const data = await adapter.analyticsCompare(timeRange?.start_epoch_ms, timeRange?.end_epoch_ms);
    setCache(cacheKey, data);
    providerAnalytics.set({ data, status: 'ready', error: null });
  } catch (e) {
    providerAnalytics.set({ data: null, status: 'error', error: String(e) });
  }
}

export async function fetchDailyStats(
  adapter: Adapter,
  days = 30,
  forceRefresh = false,
): Promise<void> {
  const cacheKey = `daily-${days}`;
  if (!forceRefresh) {
    const cached = getCached<DailyStats[]>(cacheKey, CACHE_TTL_HISTORICAL);
    if (cached) { dailyStats.set({ data: cached, status: 'ready', error: null }); return; }
  }
  dailyStats.update(s => ({ ...s, status: 'loading' }));
  try {
    const data = await adapter.analyticsDaily(undefined, days);
    setCache(cacheKey, data);
    dailyStats.set({ data, status: 'ready', error: null });
  } catch (e) {
    dailyStats.set({ data: null, status: 'error', error: String(e) });
  }
}

export async function fetchModelBreakdown(
  adapter: Adapter,
  provider: string,
  forceRefresh = false,
): Promise<void> {
  const cacheKey = `model-${provider}`;
  if (!forceRefresh) {
    const cached = getCached<ModelAnalytics[]>(cacheKey, CACHE_TTL_HISTORICAL);
    if (cached) { modelBreakdown.set({ data: cached, status: 'ready', error: null }); return; }
  }
  modelBreakdown.update(s => ({ ...s, status: 'loading' }));
  try {
    const data = await adapter.analyticsModelBreakdown(provider);
    setCache(cacheKey, data);
    modelBreakdown.set({ data, status: 'ready', error: null });
  } catch (e) {
    modelBreakdown.set({ data: null, status: 'error', error: String(e) });
  }
}

// === LIVE TRACKING ===

export function startLiveTracking(adapter: Adapter): () => void {
  let unsubscribe: (() => void) | null = null;
  let costHistory: Array<{ time: number; cost: number }> = [];

  const setup = async () => {
    unsubscribe = await adapter.onAgentEvent((payload: AgentEventPayload) => {
      const { event } = payload;
      const meta = event.metadata;

      liveMetrics.update(current => {
        if (!current) {
          // Initialize on first event
          current = {
            session_id: payload.session_id,
            provider: meta.provider,
            model: meta.model ?? '',
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: 0,
            duration_ms: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            context_percent: null,
            num_turns: 0,
            errors: 0,
            errors_recovered: 0,
            is_streaming: true,
            cost_velocity_usd_per_min: 0,
            cost_projection_usd: null,
            vs_avg_ratio: null,
          };
        }

        switch (event.event_type) {
          case 'usage':
            if (meta.input_tokens) current.input_tokens += meta.input_tokens;
            if (meta.output_tokens) current.output_tokens += meta.output_tokens;
            // Rough cost estimate based on model info
            // Will be refined when we have actual cost data
            break;
          case 'tool_use':
            current.num_turns++;
            break;
          case 'error':
            current.errors++;
            break;
          case 'done':
            current.is_streaming = false;
            break;
        }

        // Update cost velocity (30s window)
        const now = Date.now();
        costHistory.push({ time: now, cost: current.cost_usd });
        costHistory = costHistory.filter(h => now - h.time < 30_000);
        if (costHistory.length >= 2) {
          const oldest = costHistory[0];
          const elapsed = (now - oldest.time) / 60_000; // minutes
          current.cost_velocity_usd_per_min = elapsed > 0
            ? (current.cost_usd - oldest.cost) / elapsed
            : 0;
        }

        return { ...current };
      });

      // Check budget on done
      if (event.event_type === 'done') {
        checkBudget(adapter);
      }
    });
  };

  setup().catch(console.error);
  return () => {
    unsubscribe?.();
    liveMetrics.set(null);
    costHistory = [];
  };
}

// === BUDGET ===

export async function checkBudget(adapter: Adapter): Promise<void> {
  const b = get(budget);
  if (!b.daily_limit_usd && !b.weekly_limit_usd) return;

  try {
    const daily = await adapter.analyticsDaily(undefined, 1);
    const todayCost = daily.length > 0 ? daily[0].total_cost_usd : 0;

    const alerts: BudgetAlert[] = [];
    if (b.daily_limit_usd) {
      const ratio = todayCost / b.daily_limit_usd;
      if (ratio >= 1) {
        alerts.push({ type: 'exceeded', period: 'daily', current_usd: todayCost, limit_usd: b.daily_limit_usd });
      } else if (ratio >= b.notify_at_percent / 100) {
        alerts.push({ type: 'approaching', period: 'daily', current_usd: todayCost, limit_usd: b.daily_limit_usd });
      }
    }

    budgetAlerts.set(alerts);
  } catch {
    // Budget check is best-effort
  }
}
```

- [ ] **Step 3: Verify compile**

Run: `npx tsc --noEmit --pretty 2>&1 | tail -10`

- [ ] **Step 4: Commit**

```bash
git add src/lib/stores/analytics.ts src/lib/stores/ui.ts
git commit -m "feat(analytics): add analytics store with live tracking, cache, and budget"
```

---

### Task 7: i18n Keys

**Files:**
- Modify: `src/lib/i18n/en.json` (+ 8 other locale files)

Add all analytics/provider i18n keys.

- [ ] **Step 1: Read the English locale file to find the end**

Read `src/lib/i18n/en.json` to find the last key before `}`.

- [ ] **Step 2: Add English keys**

Add these keys at the end of `src/lib/i18n/en.json` (before the closing `}`):

```json
  "toolbar.analytics": "Analytics Dashboard",
  "analytics.bar.session": "Session",
  "analytics.bar.cost": "Cost",
  "analytics.bar.tokens": "Tokens",
  "analytics.bar.turns": "Turns",
  "analytics.bar.cache": "Cache",
  "analytics.bar.details": "Details",
  "analytics.bar.metricsUnavailable": "Metrics unavailable",
  "analytics.dashboard.title": "Analytics",
  "analytics.dashboard.period.today": "Today",
  "analytics.dashboard.period.7d": "7d",
  "analytics.dashboard.period.14d": "14d",
  "analytics.dashboard.period.30d": "30d",
  "analytics.dashboard.period.all": "All",
  "analytics.dashboard.compare": "Compare",
  "analytics.dashboard.export": "Export",
  "analytics.dashboard.refresh": "Refresh data",
  "analytics.dashboard.kpi.totalCost": "Total Cost",
  "analytics.dashboard.kpi.totalTokens": "Total Tokens",
  "analytics.dashboard.kpi.sessions": "Sessions",
  "analytics.dashboard.kpi.errors": "Errors",
  "analytics.dashboard.kpi.recovered": "recovered",
  "analytics.dashboard.kpi.errorRate": "error rate",
  "analytics.dashboard.providers": "Provider Comparison",
  "analytics.dashboard.trend": "Daily Trend",
  "analytics.dashboard.noData": "Data unavailable",
  "analytics.insights.title": "Insights",
  "analytics.insights.haikuSavings": "{model} costs {percent}% less for short sessions. Estimated savings: ~{amount}/week.",
  "analytics.insights.cacheDrop": "Cache hit rate dropped from {from}% to {to}%.",
  "analytics.insights.errorSpike": "{provider} has {rate}% error rate.",
  "analytics.insights.costAnomaly": "{count} anomalous sessions detected.",
  "analytics.insights.unusedProvider": "{provider} unused for {days} days.",
  "analytics.insights.efficiencyWin": "{cheap} is cheaper than {expensive} for similar tasks.",
  "analytics.budget.daily": "Daily limit",
  "analytics.budget.weekly": "Weekly limit",
  "analytics.budget.notifyAt": "Notify at",
  "analytics.budget.approaching": "Approaching {period} budget: {current} of {limit}",
  "analytics.budget.exceeded": "Exceeded {period} budget: {current} of {limit}",
  "analytics.a11y.safeZone": "safe zone",
  "analytics.a11y.warningZone": "warning zone",
  "analytics.a11y.dangerZone": "danger zone",
  "analytics.a11y.contextProgress": "Context usage: {percent}%, {zone}",
  "settings.provider.title": "Providers",
  "settings.provider.scanCli": "Scan CLI",
  "settings.provider.expand": "Expand",
  "settings.provider.setup": "Setup",
  "settings.provider.notConfigured": "Not configured",
  "settings.provider.defaultModel": "Default model",
  "settings.provider.binary": "Binary",
  "settings.provider.maxTokens": "Max tokens",
  "settings.provider.apiKeyEnv": "API Key env",
  "settings.provider.shortcut": "Shortcut",
  "settings.provider.editToml": "Edit TOML in editor",
  "settings.provider.cheapest": "Cheapest",
  "settings.provider.test.button": "Test connection",
  "settings.provider.test.testing": "Testing...",
  "settings.provider.test.binaryFound": "Binary found",
  "settings.provider.test.binaryNotFound": "Binary not found",
  "settings.provider.test.apiKeyOk": "API key configured",
  "settings.provider.test.apiKeyMissing": "API key not set",
  "settings.provider.test.connectionOk": "Connection OK",
  "settings.provider.test.connectionFailed": "Connection failed",
  "settings.provider.test.ready": "{provider} ready",
  "settings.provider.setup.installCli": "Install CLI",
  "settings.provider.setup.installGuide": "Installation guide",
  "settings.provider.setup.configureApiKey": "Configure API key",
  "settings.provider.setup.waitingPrevious": "Waiting for previous step",
  "settings.provider.setup.verify": "Verify",
  "settings.provider.setup.orManual": "Or configure TOML manually",
  "settings.provider.health.title": "Health",
  "settings.provider.health.ok": "ok",
  "settings.provider.health.degraded": "degraded",
  "settings.provider.health.down": "down"
```

- [ ] **Step 3: Add Italian keys**

Add the same keys translated to Italian in `src/lib/i18n/it.json`. Key translations:

```json
  "toolbar.analytics": "Dashboard Analytics",
  "analytics.bar.session": "Sessione",
  "analytics.bar.cost": "Costo",
  "analytics.bar.tokens": "Token",
  "analytics.bar.turns": "Turni",
  "analytics.bar.cache": "Cache",
  "analytics.bar.details": "Dettagli",
  "analytics.bar.metricsUnavailable": "Metriche non disponibili",
  "analytics.dashboard.title": "Analytics",
  "analytics.dashboard.period.today": "Oggi",
  "analytics.dashboard.period.all": "Tutto",
  "analytics.dashboard.compare": "Confronta",
  "analytics.dashboard.export": "Esporta",
  "analytics.dashboard.refresh": "Aggiorna dati",
  "analytics.dashboard.kpi.totalCost": "Costo Totale",
  "analytics.dashboard.kpi.totalTokens": "Token Totali",
  "analytics.dashboard.kpi.sessions": "Sessioni",
  "analytics.dashboard.kpi.errors": "Errori",
  "analytics.dashboard.kpi.recovered": "recuperati",
  "analytics.dashboard.kpi.errorRate": "tasso errore",
  "analytics.dashboard.providers": "Confronto Provider",
  "analytics.dashboard.trend": "Trend Giornaliero",
  "analytics.dashboard.noData": "Dati non disponibili",
  "analytics.insights.title": "Suggerimenti",
  "settings.provider.title": "Provider",
  "settings.provider.scanCli": "Scansiona CLI",
  "settings.provider.expand": "Espandi",
  "settings.provider.setup": "Configura",
  "settings.provider.notConfigured": "Non configurato",
  "settings.provider.test.button": "Testa connessione",
  "settings.provider.test.testing": "Test in corso...",
  "settings.provider.editToml": "Modifica TOML nell'editor"
```

- [ ] **Step 4: Add placeholder keys to remaining 7 locales**

For `de.json`, `es.json`, `fr.json`, `pt.json`, `zh.json`, `hi.json`, `ar.json`: copy the English keys as-is (placeholders for future translation). Add the same block from Step 2 to each file.

- [ ] **Step 5: Commit**

```bash
git add src/lib/i18n/
git commit -m "feat(i18n): add analytics and provider settings keys for all 9 locales"
```

---

### Task 8: AnalyticsBar Component

**Files:**
- Create: `src/lib/components/AnalyticsBar.svelte`
- Modify: `src/lib/components/ResponsePanel.svelte`

The compact bar under the chat input showing live session metrics.

- [ ] **Step 1: Read ResponsePanel.svelte to understand integration point**

Read the full file to find where the AnalyticsBar should be inserted (after the chat input area).

- [ ] **Step 2: Create AnalyticsBar.svelte**

Create `src/lib/components/AnalyticsBar.svelte`. This is a visual component — follow the brutalist design system (CSS variables, `--radius: 0`, `--border-width: 2px`).

The component should:
- Subscribe to `liveMetrics` from analytics store
- Show 2-row layout: context progress + cost/velocity/projection + 📊 link, then model + cache + tokens + turns + duration + vs-avg
- Collapse to 1 row on narrow width
- Handle skeleton state when `liveMetrics === null`
- Use `analyticsAnnouncer` for throttled screen reader updates
- Deep-link to dashboard on 📊 click via `analyticsDashboard` store
- Show budget alert border when `budgetAlerts` is non-empty

Component skeleton (implementer fills in the template details from spec Section 2):

```svelte
<script lang="ts">
  import { liveMetrics, budgetAlerts } from '$lib/stores/analytics';
  import { analyticsDashboard } from '$lib/stores/ui';
  import { prefersReducedMotion } from '$lib/utils/a11y-motion';
  import { analyticsAnnouncer } from '$lib/utils/a11y-announcer';
  import { barLabel, contextProgressLabel } from '$lib/utils/a11y-labels';
  import { formatCurrency, formatTokenCount, formatDuration, formatPercent, formatCostVelocity } from '$lib/utils/format-analytics';
  import { getProviderVisual } from '$lib/utils/provider-patterns';
  import { tooltip } from '$lib/utils/tooltip';
  import { tr } from '$lib/i18n/index';

  let { adapter, onOpenDashboard }: { adapter: any; onOpenDashboard: () => void } = $props();

  const metrics = $derived($liveMetrics);
  const alerts = $derived($budgetAlerts);
  const hasBudgetWarning = $derived(alerts.length > 0 && alerts.some(a => a.type === 'approaching'));
  const hasBudgetDanger = $derived(alerts.length > 0 && alerts.some(a => a.type === 'exceeded'));

  // Context progress zone
  const contextZone = $derived(
    !metrics?.context_percent ? 'safe'
    : metrics.context_percent > 80 ? 'danger'
    : metrics.context_percent > 60 ? 'warning'
    : 'safe'
  );

  // Announce to screen reader on significant changes (throttled by announcer)
  $effect(() => {
    if (metrics) {
      analyticsAnnouncer.announce(barLabel(metrics));
    }
  });
</script>

{#if metrics}
  <div
    class="analytics-bar"
    class:budget-warning={hasBudgetWarning}
    class:budget-danger={hasBudgetDanger}
    role="status"
    aria-label={metrics ? barLabel(metrics) : $tr('analytics.bar.metricsUnavailable')}
  >
    <!-- Row 1: Context progress bar + cost + velocity + projection + dashboard link -->
    <div class="analytics-bar-row1">
      {#if metrics.context_percent != null}
        <div class="progress-track" role="progressbar"
          aria-valuenow={Math.round(metrics.context_percent)}
          aria-valuemin={0} aria-valuemax={100}
          aria-label={contextProgressLabel(metrics.context_percent)}
        >
          <div class="progress-fill {contextZone}"
            style:width="{metrics.context_percent}%"
            style:transition={$prefersReducedMotion ? 'none' : 'width 0.3s'}
          ></div>
          <div class="danger-marker" aria-hidden="true"></div>
        </div>
      {/if}
      <span class="metric" use:tooltip={$tr('analytics.bar.cost')}>
        {formatCurrency(metrics.cost_usd)}
      </span>
      {#if metrics.cost_velocity_usd_per_min > 0}
        <span class="metric velocity">
          {formatCostVelocity(metrics.cost_velocity_usd_per_min)}
        </span>
      {/if}
      {#if metrics.cost_projection_usd != null}
        <span class="metric projection">
          → {formatCurrency(metrics.cost_projection_usd)}
        </span>
      {/if}
      <button class="dashboard-link" onclick={onOpenDashboard}
        aria-label={$tr('analytics.bar.details')}>
        📊
      </button>
    </div>

    <!-- Row 2: Model + cache + tokens + turns + duration + vs-avg -->
    <div class="analytics-bar-row2">
      <span class="metric provider" style:color={getProviderVisual(metrics.provider).color}>
        {metrics.model || metrics.provider}
      </span>
      <span class="metric">{formatTokenCount(metrics.input_tokens + metrics.output_tokens)} {$tr('analytics.bar.tokens')}</span>
      {#if metrics.cache_read_tokens > 0}
        <span class="metric">{$tr('analytics.bar.cache')} {formatPercent(metrics.cache_read_tokens / Math.max(1, metrics.input_tokens))}</span>
      {/if}
      <span class="metric">{metrics.num_turns} {$tr('analytics.bar.turns')}</span>
      <span class="metric">{formatDuration(metrics.duration_ms)}</span>
      {#if metrics.vs_avg_ratio != null}
        <span class="metric vs-avg" class:above={metrics.vs_avg_ratio > 1.2} class:below={metrics.vs_avg_ratio < 0.8}>
          {metrics.vs_avg_ratio > 1 ? '↑' : '↓'}{formatPercent(Math.abs(metrics.vs_avg_ratio - 1))} vs avg
        </span>
      {/if}
    </div>
  </div>
{:else}
  <!-- Skeleton state -->
  <div class="analytics-bar" aria-label={$tr('analytics.bar.metricsUnavailable')}>
    <div class="analytics-bar-row1">
      <div class="skeleton" style="width: 60%; height: 4px;"></div>
      <span class="skeleton" style="width: 50px; height: 14px;"></span>
    </div>
  </div>
{/if}

<style>
  /* Implementer: expand with full styles from spec Section 2 */
  .analytics-bar { border-top: var(--border-width) solid var(--border); padding: 4px 8px; font-size: var(--font-size-small); font-family: var(--font-mono); }
  .analytics-bar.budget-warning { border-color: var(--warning); }
  .analytics-bar.budget-danger { border-color: var(--danger); }
  .analytics-bar-row1, .analytics-bar-row2 { display: flex; align-items: center; gap: 8px; }
  .analytics-bar-row2 { margin-top: 2px; }
  .metric { color: var(--text-secondary); white-space: nowrap; }
  .progress-track { flex: 1; height: 4px; background: var(--bg-tertiary); position: relative; }
  .progress-fill { height: 100%; }
  .progress-fill.safe { background: var(--accent); }
  .progress-fill.warning { background: var(--warning); }
  .progress-fill.danger { background: var(--danger); }
  .danger-marker { position: absolute; left: 80%; width: 2px; height: 8px; background: var(--danger); top: -2px; }
  .dashboard-link { background: none; border: none; cursor: pointer; padding: 0 4px; font-size: 14px; }
  .skeleton { background: var(--bg-tertiary); animation: pulse 1.5s ease-in-out infinite; }
  @media (prefers-reduced-motion: reduce) { .skeleton { animation: none; } }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
</style>
```

The implementer should refine and expand this skeleton using the spec Section 2 for:
- Narrow-width 1-row collapse behavior
- Full `vs_avg_ratio` comparison logic
- Cost projection display logic
- Any additional responsive CSS from the spec

- [ ] **Step 3: Integrate into ResponsePanel.svelte**

Add the import and component after the chat input area. Read the file first to find the exact location.

```svelte
<script>
  import AnalyticsBar from './AnalyticsBar.svelte';
  import { analyticsDashboard } from '$lib/stores/ui';
</script>

<!-- After the input/textarea area -->
<AnalyticsBar
  {adapter}
  onOpenDashboard={() => analyticsDashboard.set({ open: true, focus: null })}
/>
```

- [ ] **Step 4: Verify the app builds**

Run: `npm run build 2>&1 | tail -10`

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/AnalyticsBar.svelte src/lib/components/ResponsePanel.svelte
git commit -m "feat(analytics): add AnalyticsBar component with live metrics display"
```

---

### Task 9: AnalyticsDashboard Component

**Files:**
- Create: `src/lib/components/AnalyticsDashboard.svelte`
- Modify: `src/lib/components/App.svelte`
- Modify: `src/lib/components/Toolbar.svelte`

The full dashboard tab — this is the largest component.

- [ ] **Step 1: Read App.svelte and Toolbar.svelte for integration points**

Read both files fully to understand where to add the analytics view and toolbar button.

- [ ] **Step 2: Create AnalyticsDashboard.svelte**

Create `src/lib/components/AnalyticsDashboard.svelte`. This is the most complex component.

Structure:
```svelte
<script lang="ts">
  // Imports: adapter prop, all stores, all utilities
  // Period selector state
  // Fetch on mount with $effect
  // Drill-down state (expanded provider, selected session)
  // Insight computation
  // Export handler
</script>

<!-- Period selector (radiogroup) -->
<!-- KPI Cards grid (4 cards with sparklines + tween) -->
<!-- Insights section (computed rules, max 3, dismissable) -->
<!-- Provider Comparison (grid with patterned bars + drill-down) -->
<!-- Daily Trend (vertical bars) -->
```

Key behaviors (implementer should reference spec Section 3):
- Period selector: `role="radiogroup"`, changing triggers `fetchProviderAnalytics` + `fetchDailyStats`
- KPI cards: 4-column grid (responsive 2x2), sparkline as `<svg>` (7-point polyline from last 7 days of `$dailyStats`, `aria-hidden="true"`), number tween via `tweenValue`
- Insights: deterministic rules computed from `$providerAnalytics` data, max 3
- Provider comparison: `use:gridNavigation`, bars via `normalizeBarScale` + `barStyle`, sortable columns
- Drill-down: accordion per provider → session list → session detail, with `focusManager.push/pop`
- Daily trend: vertical bars with `use:tooltip`, `←→` keyboard navigation
- Deep-link focus: if `$analyticsDashboard.focus` is set on mount, scroll to provider
- All sections: independent skeleton loading, `aria-busy="true"` during load, graceful `StoreState` handling

**Comparison mode** (spec Section 3g): The Daily Trend section must support an overlay of the previous period's data:
- When "Compare" toggle is active, fetch `analyticsDaily` for the previous period (e.g., if showing 7d, also fetch previous 7d)
- Render previous-period bars as semi-transparent overlays behind current bars: `opacity: 0.3`
- Each bar gets `aria-label` including both current and previous values
- Toggle state: `let compareMode = $state(false);`

**Export handler** (spec Section 3h): The "Esporta" button must open a dropdown menu with CSV/JSON options:

```typescript
async function handleExport(format: 'csv' | 'json') {
  const providers = get(providerAnalytics);
  if (!providers.data) return;

  const ext = format === 'csv' ? 'csv' : 'json';
  const path = await adapter.saveFileDialog({
    title: `Export Analytics (${ext.toUpperCase()})`,
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
  });
  if (!path) return;

  let content: string;
  if (format === 'csv') {
    const headers = 'Provider,Cost (USD),Input Tokens,Output Tokens,Sessions,Error Rate';
    const rows = providers.data.map(p =>
      `${p.provider},${p.total_cost_usd},${p.total_input_tokens},${p.total_output_tokens},${p.total_sessions},${p.error_rate}`
    );
    content = [headers, ...rows].join('\n');
  } else {
    content = JSON.stringify(providers.data, null, 2);
  }

  await adapter.writeFile(path, content);
}
```

The menu button uses `aria-haspopup="menu"` and renders a simple dropdown with two items.

CSS: Same brutalist system. Use `var(--bg-surface)` for cards, `var(--border)` for borders, `var(--font-size-small)` for data, etc.

- [ ] **Step 3: Integrate into App.svelte**

In `App.svelte`:
1. Import `AnalyticsDashboard` and `analyticsDashboard` store
2. Import `startLiveTracking` from analytics store
3. Add `startLiveTracking` to `onMount`
4. In the editor area, add conditional rendering:

```svelte
{#if $analyticsDashboard.open}
  <AnalyticsDashboard {adapter} />
{:else if editor}
  {@render editor()}
{/if}
```

- [ ] **Step 4: Add toolbar button**

In `Toolbar.svelte`, add an analytics toggle button (before the settings button area):

```svelte
<button
  class="toolbar-btn"
  aria-label={$tr('toolbar.analytics')}
  aria-pressed={$analyticsDashboard.open}
  onclick={() => analyticsDashboard.update(v => ({ ...v, open: !v.open, focus: null }))}
  title={$tr('toolbar.analytics')}
>📊</button>
```

- [ ] **Step 5: Verify the app builds**

Run: `npm run build 2>&1 | tail -10`

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/AnalyticsDashboard.svelte src/lib/components/App.svelte src/lib/components/Toolbar.svelte
git commit -m "feat(analytics): add AnalyticsDashboard with KPI, insights, provider comparison, and trend"
```

---

### Task 10: Provider Settings

**Files:**
- Modify: `src/lib/components/Settings.svelte`

Add the provider section with progressive disclosure (toggle → expand → TOML), budget settings, connection test, and onboarding flow.

- [ ] **Step 1: Read Settings.svelte fully**

Read the entire file to understand structure, state management pattern, and where to add the new section.

- [ ] **Step 2: Add provider section**

Add the provider section to `Settings.svelte` after the existing sections (theme, font, language, update). The section includes:

**State additions:**
```typescript
let expandedProvider = $state<string | null>(null);
let connectionSteps = $state<Map<string, ConnectionTestStep[]>>(new Map());
let testingProvider = $state<string | null>(null);
let localBudget = $state<AnalyticsBudget>({ daily_limit_usd: null, weekly_limit_usd: null, notify_at_percent: 80 });
```

**Template structure:**
- Section header "PROVIDER" with "🔍 Scan CLI" button
- For each provider from `llmConfigs`:
  - Toggle switch (`role="switch"`)
  - Model name (or "Not configured")
  - Health mini-timeline (7 dots) — derive from analytics data
  - Expand/Setup button
  - If expanded: Level 2 form fields (model selector with pricing, binary, max tokens, API key env, shortcut)
  - Connection test button with progressive steps
  - "Edit TOML in editor" link
  - If unconfigured: onboarding wizard (3-step progressive unlock)
- Budget section at bottom (daily limit, weekly limit, notify threshold)

**Key behaviors:**
- Auto-save with 1s debounce per field (see debounce implementation below)
- Model selector shows pricing from `getModelsForProvider()`
- Connection test: calls `adapter.testProviderConnection()` and listens via `adapter.onConnectionTest()`
- "Edit TOML" closes settings modal and opens file in editor
- Budget values persist via `adapter.writeConfig()`

**Debounce implementation** for auto-save:

```typescript
function debounce(fn: (...args: any[]) => void, ms: number) {
  let timer: ReturnType<typeof setTimeout>;
  return (...args: any[]) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

const debouncedSave = debounce(async (field: string, value: unknown) => {
  try {
    await adapter.writeConfig(`providers.${expandedProvider}.${field}`, value);
    providerConfigVersion.update(v => v + 1);
    // Toast: saved (use existing toast if available, else inline feedback)
  } catch (e) {
    // Toast: save failed
  }
}, 1000);
```

**Shortcut capture field** (keybinding editor):

```typescript
let capturingShortcut = $state<string | null>(null); // provider name or null

function handleShortcutCapture(e: KeyboardEvent) {
  if (!capturingShortcut) return;
  e.preventDefault();
  e.stopPropagation();

  // Build key combo string
  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.shiftKey) parts.push('Shift');
  if (e.altKey) parts.push('Alt');
  if (e.metaKey) parts.push('Meta');

  // Ignore modifier-only presses
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return;

  parts.push(e.key.length === 1 ? e.key.toUpperCase() : e.key);
  const combo = parts.join('+');

  // Conflict detection: check against existing provider shortcuts
  const existingConflict = llmConfigs.find(c =>
    c.name !== capturingShortcut && c.shortcut === combo
  );
  if (existingConflict) {
    // Show conflict warning, don't save
    return;
  }

  debouncedSave('shortcut', combo);
  capturingShortcut = null;
}
```

The shortcut field renders as:
```svelte
<button
  class="shortcut-capture"
  class:capturing={capturingShortcut === provider.name}
  onkeydown={handleShortcutCapture}
  onclick={() => capturingShortcut = provider.name}
  aria-label={$tr('settings.provider.shortcut')}
>
  {capturingShortcut === provider.name ? '...' : provider.shortcut || '—'}
</button>
```

- [ ] **Step 3: Verify the app builds**

Run: `npm run build 2>&1 | tail -10`

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/Settings.svelte
git commit -m "feat(settings): add provider settings with progressive disclosure, connection test, and budget"
```

---

### Task 11: Keyboard Shortcuts + Final Integration

**Files:**
- Modify: `src/lib/components/App.svelte`

Add global keyboard shortcuts for analytics toggle and provider switching.

- [ ] **Step 1: Add keyboard handler to App.svelte**

In `App.svelte`, add a `<svelte:window>` handler for global shortcuts (or extend the existing one):

```typescript
function handleGlobalKeydown(e: KeyboardEvent) {
  // Ctrl+Shift+A → toggle analytics
  if (e.ctrlKey && e.shiftKey && e.key === 'A') {
    e.preventDefault();
    analyticsDashboard.update(v => ({ ...v, open: !v.open, focus: null }));
    return;
  }

  // Ctrl+1..9 → switch provider (if providers are configured with shortcuts)
  if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key >= '1' && e.key <= '9') {
    const index = parseInt(e.key) - 1;
    // Get configured providers from llmConfigs store
    const configs = get(llmConfigs);
    if (configs && index < configs.length) {
      e.preventDefault();
      const provider = configs[index];
      if (provider.enabled) {
        // Switch active provider — use the existing provider switch mechanism
        switchProvider(provider.name);
      }
    }
  }
}
```

Note: The `switchProvider` function should already exist or be added alongside the existing provider switching logic. The `llmConfigs` store provides the ordered list of providers. Import `get` from `svelte/store` and `llmConfigs` from the appropriate store.

```svelte
<svelte:window onkeydown={handleGlobalKeydown} />
```

- [ ] **Step 2: Verify the app builds and runs**

Run: `npm run build 2>&1 | tail -10`

Then run `npm run dev` manually and verify:
1. AnalyticsBar appears under chat input when a session is active
2. 📊 button in toolbar toggles dashboard view
3. Dashboard shows KPI cards, provider comparison, trend
4. Settings has provider section with toggle/expand
5. Ctrl+Shift+A toggles dashboard
6. Keyboard navigation works in dashboard grid

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/App.svelte
git commit -m "feat(analytics): add global keyboard shortcuts for analytics"
```

---

### Task 12: Full Build + Smoke Test

**Files:** None new — verification only.

- [ ] **Step 1: Run TypeScript check**

Run: `npx tsc --noEmit --pretty 2>&1 | tail -20`

Expected: No new errors (pre-existing errors are acceptable).

- [ ] **Step 2: Run Rust check**

Run: `cd src-tauri && cargo check 2>&1 | tail -10`

Expected: Should compile without errors.

- [ ] **Step 3: Run full build**

Run: `npm run build 2>&1 | tail -20`

Expected: Build succeeds.

- [ ] **Step 4: Run Rust tests**

Run: `cd src-tauri && cargo test 2>&1 | tail -20`

Expected: All existing tests pass, no regressions.

- [ ] **Step 5: Final commit if any fixes were needed**

If any fixes were made during verification:

```bash
git add -A
git commit -m "fix(analytics): address build/test issues from Phase 7C integration"
```
