// src/lib/stores/analytics.ts
import { writable, get } from 'svelte/store';
import type { Adapter } from '$lib/adapter/index';
import type {
  LiveSessionMetrics, ProviderAnalytics, ModelAnalytics,
  DailyStats, TimeRange, AnalyticsBudget, BudgetAlert, StoreState,
} from '$lib/types/analytics';
import type { AgentEventPayload } from '$lib/types/agent-event';

// === CONSTANTS ===
const CACHE_TTL_HISTORICAL = 30_000;

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

// === FETCH FUNCTIONS ===

export async function fetchProviderAnalytics(
  adapter: Adapter,
  timeRange?: TimeRange,
  forceRefresh = false,
): Promise<void> {
  const cacheKey = `provider-${timeRange?.from ?? 'all'}`;
  if (!forceRefresh) {
    const cached = getCached<ProviderAnalytics[]>(cacheKey, CACHE_TTL_HISTORICAL);
    if (cached) { providerAnalytics.set({ data: cached, status: 'ready', error: null }); return; }
  }
  providerAnalytics.update(s => ({ ...s, status: 'loading' }));
  try {
    const data = await adapter.analyticsCompare(timeRange?.from ?? undefined, timeRange?.to ?? undefined);
    setCache(cacheKey, data);
    providerAnalytics.set({ data, status: 'ready', error: null });
  } catch (e) {
    providerAnalytics.set({ data: null, status: 'error', error: String(e) });
  }
}

// Spec-named alias for fetchProviderAnalytics
export const fetchCompareProviders = fetchProviderAnalytics;

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
          const elapsed = (now - oldest.time) / 60_000;
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
    const todayCost = daily.length > 0
      ? (daily[0].total_cost_usd ?? (daily[0].input_tokens + daily[0].output_tokens) * 0.00001)
      : 0;

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
