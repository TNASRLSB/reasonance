# Phase 7C: Frontend — Provider Settings UI, Analytics UI, Responsive Layouts

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the frontend layer for provider configuration and analytics visualization — an AnalyticsBar with live session metrics in the chat, a full AnalyticsDashboard tab, and a progressive-disclosure Provider Settings section.

**Architecture:** Standalone Svelte 5 components + dedicated analytics store with dual data feeding (live event subscription + historical fetch). Follows existing codebase patterns (writable stores + helper functions + Tauri invoke bridge). Full WCAG AA compliance with selective AAA extensions.

**Tech Stack:** Svelte 5 (runes), TypeScript, Tauri 2 invoke/listen, CSS variables (brutalist design system), Intl APIs for locale-aware formatting.

---

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Provider Settings depth | Progressive disclosure: toggle → expand → open TOML in editor | A (minimal) as default, escalates to full TOML editing in IDE |
| Provider Settings layout | Accordion in-place | Everything on one page, no drill-down navigation needed |
| Analytics Livello 1 | Compact bar with context progress under chat input | Inspired by Claude UI status bar; live session metrics |
| Analytics Livello 2 | Tab in editor area | Replaces editor content, consistent with existing view-switching pattern |
| Chart style | Hybrid triple: precise numbers + bars with unique patterns + distinct colors | Maximum accessibility: text + color + pattern redundancy |
| Responsiveness | Base (like VSCode) | Collapsible/togglable panels, adaptive grid, no breakpoints/hamburger |
| Accessibility | WCAG AA + selective AAA | AA baseline, plus Atkinson Hyperlegible, Enhanced Readability mode, reduced motion, pattern-not-just-color |

---

## 1. Analytics Store (`src/lib/stores/analytics.ts`)

### Dual Data Feeding

**Live subscription** — subscribes to `onAgentEvent` via adapter, updates current session metrics in real-time (cost rising, tokens growing, duration ticking).

**Historical fetch** — calls 6 Tauri analytics commands for aggregated data (dashboard, provider comparison, trends). Cached with TTL (30s historical, 5s live).

### Types (`src/lib/types/analytics.ts`)

```typescript
interface LiveSessionMetrics {
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
  // Derived live
  cost_velocity_usd_per_min: number;
  cost_projection_usd: number | null; // null if < 2 turns
  vs_avg_ratio: number | null;        // current cost / historical avg
}

interface SessionMetrics {
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

interface ProviderAnalytics {
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

interface ModelAnalytics {
  model: string;
  provider: string;
  total_sessions: number;
  total_cost_usd: number;
  total_tokens: number;
  avg_duration_ms: number;
}

interface DailyStats {
  date: string;
  sessions: number;
  total_cost_usd: number;
  total_tokens: number;
  providers_used: string[];
}

interface TimeRange {
  start_epoch_ms: number;
  end_epoch_ms: number;
}

interface AnalyticsBudget {
  daily_limit_usd: number | null;
  weekly_limit_usd: number | null;
  notify_at_percent: number; // default 80
}

interface BudgetAlert {
  type: 'approaching' | 'exceeded';
  period: 'daily' | 'weekly';
  current_usd: number;
  limit_usd: number;
}

interface ConnectionTestStep {
  step: 'binary' | 'api_key' | 'connection';
  status: 'checking' | 'ok' | 'failed';
  detail: string | null;
}

interface DayHealth {
  date: string;
  status: 'ok' | 'degraded' | 'down' | 'unused';
  errors: number;
  recovered: number;
}
```

### Graceful Degradation

Every store wraps data in a state envelope:

```typescript
interface StoreState<T> {
  data: T | null;
  status: 'idle' | 'loading' | 'ready' | 'error';
  error: string | null;
}
```

If a fetch fails: section shows skeleton + "Dati non disponibili". Retry after 10s, max 3 attempts. Analytics is informational — never shows blocking red errors.

### Cache with TTL

```typescript
const CACHE_TTL_HISTORICAL = 30_000;  // 30s
const CACHE_TTL_LIVE = 5_000;          // 5s
```

Dashboard opens instantly if data is fresh. Force refresh via 🔄 button.

### Store Exports

```typescript
// === LIVE ===
export const liveMetrics = writable<LiveSessionMetrics | null>(null);
export const budgetAlerts = writable<BudgetAlert[]>([]);
export function startLiveTracking(adapter: Adapter): () => void

// === HISTORICAL ===
export const providerAnalytics = writable<StoreState<ProviderAnalytics[]>>({...});
export const dailyStats = writable<StoreState<DailyStats[]>>({...});
export const modelBreakdown = writable<StoreState<ModelAnalytics[]>>({...});
export async function fetchProviderAnalytics(adapter, timeRange?, forceRefresh?): Promise<void>
export async function fetchDailyStats(adapter, days?, forceRefresh?): Promise<void>
export async function fetchModelBreakdown(adapter, provider, forceRefresh?): Promise<void>
export async function fetchCompareProviders(adapter, forceRefresh?): Promise<ProviderAnalytics[]>

// === BUDGET ===
export const budget = writable<AnalyticsBudget>({...});
export async function checkBudget(adapter: Adapter): Promise<void>

// === CONFIG PROPAGATION ===
export const providerConfigVersion = writable<number>(0);
```

### `startLiveTracking` Behavior

- Subscribes to `adapter.onAgentEvent(callback)`
- On `usage` events: increment tokens, calculate cost with provider rates
- On `metrics` events: update context_percent, duration_ms
- On `tool_use` events: increment num_turns
- On `error`/recovery events: update error counters
- On `done` events: mark `is_streaming = false`, trigger `checkBudget`
- Cost velocity: calculated on 30s sliding window
- Cost projection: appears after turn 2, linear extrapolation: `(cost_so_far / turns_completed) * avg_turns_per_session_for_provider`. Falls back to `cost_so_far * 2` if no historical average available
- vs_avg_ratio: current session cost / historical average for that provider
- Returns cleanup function for `onDestroy`

---

## 2. AnalyticsBar (`src/lib/components/AnalyticsBar.svelte`)

### Position

Under chat input in `ResponsePanel.svelte`. Visible only when a session is active.

### Layout

```
┌──────────────────────────────────────────────────────────────────┐
│ Ctx: 31% ██████░░░░░░░░|░░░  $0.12 ↗$0.03/min  (~$0.45)  📊  │
│ claude-sonnet-4-6  ⚡62% cache  12.4K tok  T:3  4.1s  vs ●avg  │
└──────────────────────────────────────────────────────────────────┘
```

**Row 1:** Context % with progress bar (danger marker at 80%, progressive color) · cost + velocity + projection · dashboard link
**Row 2:** Model · cache hit rate · token count · turns · duration · vs-average indicator

### Dynamic Behaviors

| Data | Behavior |
|------|----------|
| Cost velocity | `↑` accelerating, `→` stable, `↓` decelerating. 30s sliding window |
| Projection | Appears after turn 2. Linear extrapolation: `(cost / turns) * avg_turns_for_provider`. Fallback: `cost * 2` if no history |
| vs average | Dot: 🟢 below avg, 🟡 ±20%, 🔴 >50% above. Tooltip with exact value |
| Context danger | `\|` marker at 80%. Bar: `--accent` to 60%, `--warning` 60-80%, `--danger` 80-100% |
| Cache hit rate | `⚡NN%` — visible only if > 0%. Green if > 50% |
| Error flash | Red pulse 0.5s on error, green pulse 0.5s on recovery. Reduced motion: instant color change only |
| Budget alert | Bar border changes to `--warning` / `--danger`. System notification via `adapter.showNotification()` |

### Deep Linking

Click 📊 opens dashboard focused on current provider/session:

```typescript
analyticsDashboard.set({
  open: true,
  focus: { provider: liveMetrics.provider, sessionId: liveMetrics.session_id }
})
```

### Accessibility

- `role="status"` + `aria-live="polite"` throttled to 5s via `analyticsAnnouncer`
- Semantic label via `barLabel()` from a11y-labels
- Progress bar: `role="progressbar"` + `aria-valuenow` + `aria-valuetext="31%, safe zone"` / `"85%, danger zone"`
- Error flash: `aria-live="assertive"` (bypasses throttle)
- Tab order: progress bar → cost → dashboard link
- All interactive elements: min 44x44px touch target

### States

- `liveMetrics === null` + session active → skeleton loading (2-row pulsing placeholder)
- `liveMetrics` in error → shows `—` for all values, no error message. `aria-label="Metrics unavailable"`
- No active session → bar hidden entirely

### Responsive

- Wide: 2-row layout as above
- Narrow (side panels reduce space): collapses to `$0.12 ↗ │ 12.4K │ Ctx 31% │ 📊`

---

## 3. AnalyticsDashboard (`src/lib/components/AnalyticsDashboard.svelte`)

### Position

Alternative view in editor area. Toggled via `analyticsDashboard` store. Opened from AnalyticsBar 📊 link or Toolbar button or `Ctrl+Shift+A`.

### Structure

Vertically scrollable, 4 sections + header. No internal tabs.

```
┌─────────────────────────────────────────────────┐
│ 📊 Analytics          Oggi | 7g | 14g | 30g |  │
│                       Tutto | ⇄ Confronta       │
│                                      [Esporta ↓]│
├─────────────────────────────────────────────────┤
│ KPI Cards (4) with sparklines + delta           │
├─────────────────────────────────────────────────┤
│ 💡 Insights (max 3, deterministic rules)        │
├─────────────────────────────────────────────────┤
│ Provider Comparison (hybrid bars + drill-down)  │
├─────────────────────────────────────────────────┤
│ Daily Trend (vertical bars + comparison overlay)│
└─────────────────────────────────────────────────┘
```

### 3a. Period Selector

`Oggi | 7g | 14g | 30g | Tutto | ⇄ Confronta`

- `role="radiogroup"` with `aria-label="Select period"`
- Changing period re-triggers all fetches (cache-aware)
- "⇄ Confronta" toggle: `aria-pressed`, shows delta in KPIs + overlay bars in trend

### 3b. KPI Cards

4 cards in responsive grid (4 cols → 2x2 narrow):

| Card | Primary | Secondary | Sparkline |
|------|---------|-----------|-----------|
| Total Cost | `$2.45` | trend vs prev period | 7-point SVG polyline |
| Total Tokens | `145K` | "in: 98K · out: 47K" | 7-point |
| Sessions | `12` | "3 providers · avg 4.2 turns" | 7-point |
| Errors | `2` | "1 recovered · rate 3.2%" | 7-point, red if >10% |

- Sparkline: SVG inline, `aria-hidden="true"` (decorative)
- Delta: `↓12%` green for cost decrease (good), red for increase
- `aria-label` via `kpiLabel()`: "Total cost: 2 dollars 45 cents, down 12% vs previous period"
- Number tween on mount via `tweenValue(0, value, 600)`
- Each card: `role="group"` + descriptive `aria-label`

### 3c. Insights

Deterministic rules computed on render, max 3 visible:

| Rule | Condition | Message template |
|------|-----------|------------------|
| Haiku savings | Short sessions (≤3 turns) on expensive model | "Haiku costs X% less for short sessions" |
| Cache drop | Cache hit rate dropped >20% vs previous day | "Cache hit rate dropped from X% to Y%" |
| Error spike | Error rate >5% on a provider | "Provider X has Y% error rate" |
| Cost anomaly | Session >2x average | "N anomalous sessions detected" |
| Unused provider | Configured but unused 7+ days | "Provider X unused for N days" |
| Efficiency win | One provider <50% cost of another for similar results | "Provider X is cheaper for similar tasks" |

- Container: `role="log"` + `aria-label="Analytics suggestions"`
- Each insight: dismissable with "×", `aria-live="polite"`
- No applicable insights → section hidden
- i18n: all messages via translation keys

### 3d. Provider Comparison

Hybrid triple style (numbers + patterned bars + colors):

```
Claude    $1.72 · 98K tok · 7 sess · err 1.2%
██████████████████████████████████████░░░░░░░░░░  70%  ⚠

Gemini    $0.53 · 34K tok · 3 sess · err 0%
▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░░  22%
```

**Provider patterns (colorblind-safe):**

| Provider | Color | CSS Pattern | Label |
|----------|-------|-------------|-------|
| Claude | `#1d4ed8` blue | Solid | "solid blue" |
| Gemini | `#16a34a` green | Vertical stripes 4px | "green vertical stripes" |
| Qwen | `#ca8a04` amber | Diagonal 45° | "amber diagonal" |
| Kimi | `#9333ea` purple | Dotted | "purple dotted" |
| Codex | `#dc2626` red | Horizontal stripes | "red horizontal stripes" |

All patterns distinguishable in grayscale. Verified for protanopia, deuteranopia, tritanopia.

- Sortable: click header to sort by cost, tokens, sessions, error rate. `aria-sort`
- `role="grid"` with `use:gridNavigation` for arrow-key navigation
- Each bar: `role="meter"` + `aria-valuenow` + `aria-valuetext` via `providerBarLabel()`
- Anomaly marker `⚠` on providers with sessions >2x average. Click → expands session list
- Bar widths calculated via `normalizeBarScale()`

### 3e. Drill-Down

Click provider → accordion with session list:

```
▼ Claude (7 sessions)
  14:32  sonnet-4-6  $0.42  18K  3 turns  ⚠
  11:15  haiku-4-5   $0.08   4K  1 turn
```

Click session → breadcrumb `Dashboard › Claude › 14:32`:

```
Session 2026-03-22 14:32
Model: claude-sonnet-4-6
Duration: 45s (API: 38s)
Cost: $0.42
Tokens: in 14K · out 4K · cache create 2K · cache read 8K
Turns: 3 · Tool calls: 5
Errors: 1 (recovered)
Stop reason: end_turn
```

- `aria-expanded` on accordion triggers
- Focus management via `focusManager.push()` / `pop()`
- Keyboard: Enter to expand, Escape to collapse, breadcrumb navigable with Tab

### 3f. Model Breakdown

Expandable per provider within the provider comparison. Click provider → shows model sub-bars in lighter tint of same color pattern.

### 3g. Daily Trend

Vertical bars, last 7/14/30 days (selector).

- Each bar: `use:tooltip` with day detail
- Keyboard: ← → between days, focus visible with `--focus-ring`
- Anomaly markers `⚠` on days with sessions >2x average
- Comparison mode: overlay semi-transparent bars from previous period (`opacity: 0.3`)
- `aria-label` per bar via `trendBarLabel()`

### 3h. Export

Button top-right: `Esporta ↓` → menu `CSV | JSON`

- Respects selected period and active filters
- Uses `adapter.saveFileDialog()` for destination
- CSV: one row per session, columns for all metrics
- `aria-haspopup="menu"` on button

### Skeleton Loading

Each section has independent skeleton. Appear in fetch order: KPI → Insights → Provider → Trend.

- Rectangles `var(--bg-tertiary)` with `animation: pulse 1.5s ease-in-out infinite`
- `aria-busy="true"` on sections loading
- Reduced motion: no pulse animation, static gray blocks

### Deep Link Focus

On mount, if `analyticsDashboard.focus` is set:
- Scroll to focused provider/session with `scrollIntoView({ behavior: 'smooth' })`
- Highlight with temporary border flash
- Reduced motion: `behavior: 'auto'`, no flash

### Responsive

- Wide: KPI 4 columns, bars full-width, trend 7+ bars
- Narrow: KPI 2x2, bars abbreviate (cost + bar only, details in tooltip), trend compresses

---

## 4. Provider Settings (in `Settings.svelte`)

### Position

New accordion section in existing Settings modal, after current sections (theme, font, language, update).

### Progressive Disclosure: 3 Levels

#### Level 1: Provider List

```
┌─────────────────────────────────────────────────────────────┐
│ PROVIDER                                    [🔍 Scan CLI]  │
├─────────────────────────────────────────────────────────────┤
│ ☑ Claude  claude-sonnet-4-6  ●●●●●●● ⌘1  🔌 ▼ Expand    │
│ ☑ Gemini  gemini-2.5-pro     ●●●○●●● ⌘2  🔌 ▶ Expand    │
│ ☐ Kimi    — Not configured               🔌 ▶ Setup      │
│ ☑ Qwen    qwen3-coder        ●●●●●●● ⌘3  🔌 ▶ Expand    │
│ ☐ Codex   — Not configured               🔌 ▶ Setup      │
├─────────────────────────────────────────────────────────────┤
│ 💰 Budget   Daily: $5.00  Weekly: $25.00  Notify: 80%    │
└─────────────────────────────────────────────────────────────┘
```

- Toggle: `role="switch"` + `aria-checked`
- Health mini-timeline: 7 dots for last 7 days. `aria-label` via `healthTimelineLabel()`
- Dots: hover/focus tooltip via `use:tooltip` with date + detail
- "🔍 Scan CLI": calls `adapter.discoverLlms()`, updates detected/not detected
- Unconfigured providers: "Setup" label instead of "Expand"

#### Level 2: Expanded — Configured Provider

```
▼ Claude
│ Default model  [claude-sonnet-4-6           ▾]
│                 ├ claude-sonnet-4-6
│                 │  $3/1M in · $15/1M out · 200K ctx
│                 ├ claude-haiku-4-5
│                 │  $0.25/1M in · $1.25/1M out · 200K ctx
│                 └ claude-opus-4-6
│                    $15/1M in · $75/1M out · 200K ctx
│
│ Binary           [/usr/bin/claude               ]
│ Max tokens       [8192                          ]
│ API Key env      [ANTHROPIC_API_KEY             ]
│ Shortcut         [⌘1                            ]
│
│ 🔌 Test connection
│   ✅ Binary found
│   ✅ API key configured
│   ⏳ Sending test request...
│
│ Health (7d)  ●●●●○●●  6/7 ok · 2 err (recovered)
│
│ 📄 Edit TOML in editor →
```

**Model comparison in selector:**
- Custom `<select>` showing per option: name, cost per 1M tokens (in/out), context window
- Data from `src/lib/data/model-info.ts` (hardcoded, updatable)
- Highlights cheapest model with "💰 Economico" tag
- `aria-label` on each option with full pricing info

**Connection test (event-driven):**
- Calls `adapter.testProviderConnection(provider)` — starts async test
- Listens to `adapter.onConnectionTest(callback)` for step-by-step results
- 3 progressive steps with visual feedback:
  1. Binary found → path shown
  2. API key env var → variable name shown
  3. Connection test → latency shown or error message
- `role="list"` with `aria-live="polite"` — screen reader announces each step
- Button disabled during test: "⏳ Testing..."
- All pass → toast "Claude ready ✅"

**Validation:**
- Binary path: debounced existence check
- Max tokens: numeric range validation
- `aria-invalid="true"` + `aria-errormessage` on failure

**Shortcut field:**
- Captures next key combination pressed (like keybinding editors)
- Conflict detection with existing shortcuts → inline warning
- Max 9 providers (⌘1-⌘9)

#### Level 2 Alternative: Unconfigured Provider (Onboarding)

```
▼ Kimi — Setup
│  Configure Kimi for use in Reasonance
│
│  1. Install CLI      ❌ Not found
│     [Installation guide →]
│
│  2. Configure API key  ○ Waiting for step 1
│     Env variable: [MOONSHOT_API_KEY          ]
│
│  3. Verify             ○ Waiting for step 2
│     [🔌 Test connection]
│
│  Steps unlock progressively.
│  📄 Or configure TOML manually →
```

- Locked steps: `aria-disabled="true"`, visually dimmed
- Steps unlock progressively as previous completes
- All complete → provider auto-enables with toggle ON
- "Installation guide" → `adapter.openExternal()` to provider docs

#### Level 3: TOML in Editor

Click "📄 Edit TOML in editor →":
1. Closes Settings modal
2. Opens `normalizers/{provider}.toml` in editor as regular file
3. After save, backend reloads via `reload_normalizers` command

### Budget Section

At bottom of Provider section:

- Daily limit: `<input type="number" step="0.50">`
- Weekly limit: same
- Notify threshold: `<input type="range" min="50" max="95" step="5">` with `aria-valuetext`
- Persisted via `adapter.writeConfig()`
- Values feed `AnalyticsBudget` in analytics store

### Auto-Save

- Debounce 1s per field, no explicit "Save" button
- Toast feedback "Settings saved" with `aria-live="polite"`
- Validation failure: field highlighted, save blocked for that field only
- Backend rejection: field reverts to previous value with error toast

### Accessibility

- Section: `role="region"` + `aria-label="Provider settings"`
- Focus trap in expanded accordion
- Tab order: toggle → model → expand → (if expanded) fields → TOML link → next provider
- All interactive elements: min 44x44px
- High contrast: all text ≥ 4.5:1 on background

---

## 5. Adapter Interface + Integration

### New Adapter Methods (`src/lib/adapter/index.ts`)

```typescript
// Analytics
analyticsProvider(provider: string, timeRange?: TimeRange): Promise<ProviderAnalytics>;
analyticsCompare(timeRange?: TimeRange): Promise<ProviderAnalytics[]>;
analyticsModelBreakdown(provider: string, timeRange?: TimeRange): Promise<ModelAnalytics[]>;
analyticsSession(sessionId: string): Promise<SessionMetrics>;
analyticsDaily(days: number): Promise<DailyStats[]>;
analyticsActive(): Promise<SessionMetrics[]>;

// Provider management
testProviderConnection(provider: string): Promise<void>;
onConnectionTest(callback: (step: ConnectionTestStep) => void): Promise<() => void>;
reloadNormalizers(): Promise<void>;
```

### New Backend Commands

**`test_provider_connection`** (`src-tauri/src/commands/provider.rs`):
- Emits `connection_test_step` events via `app.emit()` for each step
- Step 1: verify binary with `which`
- Step 2: verify env var with `std::env::var()`
- Step 3: spawn minimal "hi" prompt with 10s timeout
- Frontend listens via `listen()`, updates UI progressively

**`reload_normalizers`** (`src-tauri/src/commands/provider.rs`):
- Calls `NormalizerRegistry::load_from_dir()` fresh
- Replaces registry in `Arc<Mutex<>>`
- Updates retry policies

### App Integration (`src/lib/components/App.svelte`)

```typescript
onMount(() => {
  startUpdateChecker();
  const stopTracking = startLiveTracking(adapter);
  return () => stopTracking();
});
```

Editor area switches between editor and analytics:

```svelte
{#if $analyticsDashboard.open}
  <AnalyticsDashboard {adapter} />
{:else if editor}
  {@render editor()}
{/if}
```

### Dashboard State Store (`src/lib/stores/ui.ts`)

```typescript
interface AnalyticsDashboardState {
  open: boolean;
  focus: {
    provider?: string;
    sessionId?: string;
    section?: 'kpi' | 'insights' | 'providers' | 'trend';
  } | null;
}
export const analyticsDashboard = writable<AnalyticsDashboardState>({ open: false, focus: null });
```

### Chat Integration (`ResponsePanel.svelte`)

```svelte
<div class="chat-input-area">
  <textarea .../>
</div>
<AnalyticsBar {adapter} onOpenDashboard={...} />
```

### Toolbar Integration

```svelte
<button
  class="toolbar-btn"
  aria-label={$tr('toolbar.analytics')}
  aria-pressed={$analyticsDashboard.open}
  onclick={() => analyticsDashboard.update(v => ({ ...v, open: !v.open }))}
>📊</button>
```

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+A` | Toggle Analytics dashboard |
| `Ctrl+1..9` | Switch provider (configurable) |

### Config Change Propagation

When Settings saves a provider change:
1. Calls `adapter.reloadNormalizers()`
2. Increments `providerConfigVersion`
3. Invalidates analytics cache
4. Active sessions pick up new config on next request

### Data Flow

```
Backend EventBus ──event──→ Tauri event ──listen──→ onAgentEvent
                                                       │
                                                  startLiveTracking()
                                                       │
                                                  liveMetrics ──→ AnalyticsBar
                                                       │
                                                  checkBudget() ──→ budgetAlerts ──→ Toast

User clicks 📊 ──→ analyticsDashboard.open = true
                        │
                   AnalyticsDashboard mounts
                        │
                   fetch* (cache-aware) ──invoke──→ Tauri commands ──→ stores
```

---

## 6. Utilities

### 6a. Format Analytics (`src/lib/utils/format-analytics.ts`)

Centralized locale-aware formatter using `Intl.NumberFormat` / `Intl.DateTimeFormat`:

| Function | Input → Output |
|----------|----------------|
| `formatCurrency(usd)` | 0.055 → "$0.06", 0.001 → "<$0.01" |
| `formatTokenCount(count)` | 145000 → "145K", 1200000 → "1.2M" |
| `formatDuration(ms)` | 4100 → "4.1s", 83000 → "1m 23s" |
| `formatPercent(ratio)` | 0.623 → "62%" |
| `formatCostVelocity(usdPerMin)` | 0.03 → "$0.03/min" |
| `formatTokenRate(tps)` | 45.2 → "45 tok/s" |
| `formatDate(epochMs)` | → "Mar 20" (locale-dependent) |
| `formatDateFull(epochMs)` | → "Thursday March 20, 2026, 14:32" |

Edge cases: null/undefined → "—", NaN/Infinity → "—", negative → displayed with minus, 0 → "0" (not hidden).

### 6b. Screen Reader Announcer (`src/lib/utils/a11y-announcer.ts`)

Singleton with configurable throttle (default 5s):

- `announce(message)`: polite, throttled, queued. Fuses queued messages on flush
- `announceUrgent(message)`: assertive, immediate, bypasses throttle
- Mounts hidden `<div aria-live>` in DOM
- Auto-cleanup on destroy

### 6c. Semantic Labels Builder (`src/lib/utils/a11y-labels.ts`)

Centralized `aria-label` generation — single source of truth:

- `barLabel(metrics)` → full AnalyticsBar description
- `kpiLabel(title, value, delta, unit)` → KPI card description
- `providerBarLabel(provider, analytics, totalCost)` → provider comparison bar
- `contextProgressLabel(percent)` → "31%, safe zone" / "85%, danger zone"
- `healthTimelineLabel(days)` → "6 days ok, 1 day with recovered errors"
- `trendBarLabel(day)` → daily trend bar detail
- `budgetAlertLabel(alert)` → budget warning
- `connectionStepLabel(step)` → test connection step

All use i18n translation keys internally.

### 6d. Reduced Motion (`src/lib/utils/a11y-motion.ts`)

Readable store reflecting `prefers-reduced-motion` media query + helper for conditional transitions:

```typescript
export const prefersReducedMotion = readable<boolean>(...)
export function motionTransition(duration: string): string
```

### 6e. Focus Manager (`src/lib/utils/a11y-focus.ts`)

Stack-based focus management for nested navigation:

- `push(target)`: save current focused element, move focus to target
- `pop()`: restore focus to previously pushed element
- `reset()`: clear stack (dashboard close)
- Handles elements removed from DOM before pop (falls back to nearest parent)

### 6f. Accessible Tooltip (`src/lib/utils/tooltip.ts`)

Svelte action `use:tooltip`:

- Shows on mouseenter (300ms delay) + focus
- Hides on mouseleave + blur + Escape
- `role="tooltip"` + `aria-describedby` linkage
- Singleton (one visible at a time)
- Viewport-aware positioning with flip
- Reduced motion: instant show/hide

### 6g. Number Tween (`src/lib/utils/tween.ts`)

Returns a readable store that animates from → to:

- `tweenValue(from, to, duration, easing)` → `Readable<number>`
- Uses `requestAnimationFrame`
- Reduced motion: returns static store with `to` value immediately
- Auto-cleanup on completion

### 6h. Grid Navigation (`src/lib/utils/grid-navigation.ts`)

Svelte action `use:gridNavigation` for arrow-key navigation in data tables:

- ↑↓←→ between cells, Home/End, Ctrl+Home/End
- Enter to activate/expand, Escape to collapse
- Roving tabindex pattern
- Compatible with expandable rows

### 6i. Bar Scale Normalizer (`src/lib/utils/bar-scale.ts`)

Normalizes values to percentages for bar rendering:

- `normalizeBarScale(items, options)` → `BarValue[]`
- Options: maxWidthPercent (90), minWidthPercent (2), scaleMode (proportional | logarithmic)
- Handles: all zeros, single item, negative values, null/undefined

### 6j. Provider Patterns (`src/lib/utils/provider-patterns.ts`)

Centralized provider → color + CSS pattern map:

- `PROVIDER_VISUALS` record with color, pattern CSS, patternLabel, contrastColor
- `getProviderVisual(provider)` with fallback for unmapped providers
- `barStyle(provider, widthPercent)` generates inline style
- All colors verified ≥ 4.5:1 contrast on both dark/light backgrounds

---

## File Inventory

### New Files (16)

| File | Purpose |
|------|---------|
| `src/lib/types/analytics.ts` | All TypeScript types |
| `src/lib/stores/analytics.ts` | Live + historical + budget + cache |
| `src/lib/data/model-info.ts` | Model pricing/specs for selector |
| `src/lib/components/AnalyticsBar.svelte` | Compact bar in chat |
| `src/lib/components/AnalyticsDashboard.svelte` | Full dashboard tab |
| `src/lib/utils/format-analytics.ts` | Locale-aware formatters |
| `src/lib/utils/a11y-announcer.ts` | Screen reader announcer |
| `src/lib/utils/a11y-labels.ts` | Semantic label builder |
| `src/lib/utils/a11y-focus.ts` | Focus manager stack |
| `src/lib/utils/tooltip.ts` | Accessible tooltip action |
| `src/lib/utils/tween.ts` | Number animation |
| `src/lib/utils/grid-navigation.ts` | Grid keyboard navigation |
| `src/lib/utils/bar-scale.ts` | Bar width normalizer |
| `src/lib/utils/a11y-motion.ts` | Reduced motion store |
| `src/lib/utils/provider-patterns.ts` | Colorblind-safe patterns |
| `src-tauri/src/commands/provider.rs` | Connection test + reload commands |

### Modified Files (8)

| File | Changes |
|------|---------|
| `src/lib/adapter/index.ts` | +9 methods |
| `src/lib/adapter/tauri.ts` | +9 implementations |
| `src/lib/stores/ui.ts` | +analyticsDashboard state |
| `src/lib/components/App.svelte` | +startLiveTracking, +analytics view |
| `src/lib/components/ResponsePanel.svelte` | +AnalyticsBar |
| `src/lib/components/Settings.svelte` | +Provider section + budget |
| `src/lib/components/Toolbar.svelte` | +analytics toggle button |
| `src-tauri/src/lib.rs` | +new command registrations |
| `src/lib/i18n/locales/*.json` (×9) | +analytics/provider keys |

---

## i18n Keys

New key namespaces for all 9 languages:

- `analytics.bar.*` — bar labels
- `analytics.dashboard.*` — section titles, periods, export
- `analytics.insights.*` — insight message templates
- `analytics.budget.*` — budget labels and alerts
- `analytics.a11y.*` — screen reader labels (safe zone, danger zone, etc.)
- `settings.provider.*` — provider section labels
- `settings.provider.test.*` — connection test messages
- `settings.provider.setup.*` — onboarding messages
- `toolbar.analytics` — toolbar button tooltip

---

## Accessibility Summary

| Feature | Implementation |
|---------|---------------|
| WCAG AA contrast | All text ≥ 4.5:1, verified on dark + light themes |
| Keyboard navigation | Full: Tab, arrows, Enter, Escape, Home/End |
| Screen reader | aria-live throttled, semantic labels, roles on all interactive elements |
| Color blindness | Pattern + color + text triple redundancy on all data bars |
| Reduced motion | All animations respect prefers-reduced-motion |
| Focus management | Stack-based push/pop for nested drill-down |
| Touch targets | All interactive elements ≥ 44x44px |
| Enhanced readability | Existing mode applies to all new components via CSS variables |
| Error resilience | Graceful degradation — analytics never shows blocking errors |
