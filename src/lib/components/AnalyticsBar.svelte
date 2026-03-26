<script lang="ts">
  import { untrack } from 'svelte';
  import { liveMetrics, budgetAlerts } from '$lib/stores/analytics';
  import { analyticsDashboard } from '$lib/stores/ui';
  import { prefersReducedMotion } from '$lib/utils/a11y-motion';
  import { analyticsAnnouncer } from '$lib/utils/a11y-announcer';
  import { barLabel, contextProgressLabel } from '$lib/utils/a11y-labels';
  import { formatCurrency, formatTokenCount, formatDuration, formatPercent, formatCostVelocity } from '$lib/utils/format-analytics';
  import { getProviderVisual } from '$lib/utils/provider-patterns';
  import { tooltip } from '$lib/utils/tooltip';
  import { tr } from '$lib/i18n/index';
  import type { Adapter } from '$lib/adapter/index';

  let { adapter, onOpenDashboard }: { adapter: Adapter | undefined; onOpenDashboard: () => void } = $props();

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

  // Cache hit rate
  const cacheHitRate = $derived(
    metrics && metrics.cache_read_tokens > 0 && metrics.input_tokens > 0
      ? metrics.cache_read_tokens / Math.max(1, metrics.input_tokens)
      : null
  );
  const cacheIsGood = $derived(cacheHitRate != null && cacheHitRate > 0.5);

  // vs-average dot color
  const vsAvgColor = $derived(
    metrics?.vs_avg_ratio == null ? null
    : metrics.vs_avg_ratio > 1.5 ? 'danger'
    : metrics.vs_avg_ratio > 1.2 ? 'warning'
    : 'safe'
  );

  // Pace delta: how fast quota is consumed relative to the reset window
  const paceMetrics = $derived((() => {
    if (!metrics?.duration_ms || !metrics?.context_percent) return null;

    // Assume 5-hour window (configurable later via settings)
    const windowSecs = 5 * 3600;
    const elapsedSecs = metrics.duration_ms / 1000;
    const remainingSecs = Math.max(0, windowSecs - elapsedSecs);
    const usagePercent = metrics.context_percent;

    const timeUsedPercent = ((windowSecs - remainingSecs) * 100) / windowSecs;
    const paceDelta = timeUsedPercent - usagePercent;

    // Projected exhaustion time
    const burnRate = usagePercent / Math.max(elapsedSecs, 1); // % per second
    const projectedExhaustionSecs = burnRate > 0 ? (100 - usagePercent) / burnRate : null;

    const resetMinutes = Math.ceil(remainingSecs / 60);
    const resetHours = Math.floor(resetMinutes / 60);
    const resetMins = resetMinutes % 60;

    return {
      paceDelta: Math.round(paceDelta),
      resetCountdown: resetHours > 0 ? `${resetHours}h ${resetMins}m` : `${resetMins}m`,
      projectedExhaustionSecs,
    };
  })());

  // Cost velocity direction arrow
  const velocityArrow = $derived(
    !metrics?.cost_velocity_usd_per_min ? null
    : metrics.cost_velocity_usd_per_min > 0.01 ? '↑'
    : metrics.cost_velocity_usd_per_min < -0.01 ? '↓'
    : '→'
  );

  // Error flash state
  let errorFlash = $state(false);
  let recoveryFlash = $state(false);
  let prevErrors = $state(0);
  let prevRecovered = $state(0);

  $effect(() => {
    const currentErrors = metrics?.errors ?? 0;
    const prev = untrack(() => prevErrors);
    if (currentErrors > prev) {
      prevErrors = currentErrors;
      errorFlash = true;
      const t = setTimeout(() => { errorFlash = false; }, 500);
      return () => clearTimeout(t);
    }
  });

  $effect(() => {
    const currentRecovered = metrics?.errors_recovered ?? 0;
    const prev = untrack(() => prevRecovered);
    if (currentRecovered > prev) {
      prevRecovered = currentRecovered;
      recoveryFlash = true;
      const t = setTimeout(() => { recoveryFlash = false; }, 500);
      return () => clearTimeout(t);
    }
  });

  // Mount announcer live regions into the DOM
  let barEl: HTMLElement | undefined = $state(undefined);
  $effect(() => {
    if (barEl) {
      analyticsAnnouncer.mount(barEl);
      return () => analyticsAnnouncer.destroy();
    }
  });

  // Announce to screen reader on significant changes (throttled by announcer)
  $effect(() => {
    if (metrics) {
      analyticsAnnouncer.announce(barLabel(metrics));
    }
  });
</script>

{#if metrics}
  <div
    bind:this={barEl}
    class="analytics-bar"
    class:budget-warning={hasBudgetWarning}
    class:budget-danger={hasBudgetDanger}
    class:error-flash={errorFlash && !$prefersReducedMotion}
    class:error-instant={errorFlash && $prefersReducedMotion}
    class:recovery-flash={recoveryFlash && !$prefersReducedMotion}
    class:recovery-instant={recoveryFlash && $prefersReducedMotion}
    role="status"
    aria-label={metrics ? barLabel(metrics) : $tr('analytics.bar.metricsUnavailable')}
  >
    <!-- Row 1: Context progress bar + cost + velocity + projection + dashboard link -->
    <div class="analytics-bar-row row1">
      {#if metrics.context_percent != null}
        <div
          class="progress-track"
          role="progressbar"
          aria-valuenow={Math.round(metrics.context_percent)}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-label={contextProgressLabel(metrics.context_percent)}
          use:tooltip={contextProgressLabel(metrics.context_percent)}
        >
          <div
            class="progress-fill zone-{contextZone}"
            style:width="{metrics.context_percent}%"
            style:transition={$prefersReducedMotion ? 'none' : 'width 0.3s ease'}
          ></div>
          <div class="danger-marker" aria-hidden="true"></div>
        </div>
        <span class="ctx-label">{Math.round(metrics.context_percent)}%</span>
      {/if}

      <span class="metric cost" use:tooltip={$tr('analytics.bar.cost')}>
        {formatCurrency(metrics.cost_usd)}
      </span>

      <span class="metric compact-tokens">
        {formatTokenCount(metrics.input_tokens + metrics.output_tokens)}
      </span>

      {#if velocityArrow}
        <span class="metric velocity" use:tooltip={formatCostVelocity(metrics.cost_velocity_usd_per_min)}>
          {velocityArrow} {formatCostVelocity(metrics.cost_velocity_usd_per_min)}
        </span>
      {/if}

      {#if metrics.cost_projection_usd != null && metrics.num_turns >= 2}
        <span class="metric projection" use:tooltip={"Projected session total"}>
          → {formatCurrency(metrics.cost_projection_usd)}
        </span>
      {/if}

      {#if paceMetrics}
        <span
          class="metric pace"
          class:pace-warning={paceMetrics.paceDelta > 25}
          class:pace-danger={paceMetrics.paceDelta > 50}
          use:tooltip={`Pace relative to quota window. ${paceMetrics.paceDelta > 0 ? 'Consuming faster than sustainable' : 'On pace or below'}`}
        >
          ⟳ {paceMetrics.paceDelta > 0 ? '+' : ''}{paceMetrics.paceDelta}%
        </span>
      {/if}

      <button
        class="dashboard-link"
        onclick={onOpenDashboard}
        aria-label={$tr('analytics.bar.details')}
        use:tooltip={$tr('analytics.bar.details')}
      >
        📊
      </button>
    </div>

    <!-- Row 2: Model + cache + tokens + turns + duration + vs-avg -->
    <div class="analytics-bar-row row2">
      <span
        class="metric provider"
        style:color={getProviderVisual(metrics.provider).color}
        use:tooltip={metrics.provider}
      >
        {metrics.model || metrics.provider}
      </span>

      <span class="metric" use:tooltip={$tr('analytics.bar.tokens')}>
        {formatTokenCount(metrics.input_tokens + metrics.output_tokens)} {$tr('analytics.bar.tokens')}
      </span>

      {#if cacheHitRate != null}
        <span
          class="metric cache"
          class:cache-good={cacheIsGood}
          use:tooltip={"Cache hit rate"}
        >
          ⚡{formatPercent(cacheHitRate)}
        </span>
      {/if}

      <span class="metric" use:tooltip={$tr('analytics.bar.turns')}>
        {metrics.num_turns} {$tr('analytics.bar.turns')}
      </span>

      <span class="metric" use:tooltip={"Session duration"}>
        {formatDuration(metrics.duration_ms)}
      </span>

      {#if metrics.vs_avg_ratio != null}
        <span
          class="metric vs-avg"
          class:above={metrics.vs_avg_ratio > 1.2}
          class:below={metrics.vs_avg_ratio < 0.8}
          use:tooltip={"vs. session average"}
        >
          <span class="vs-avg-dot dot-{vsAvgColor}" aria-hidden="true"></span>
          {metrics.vs_avg_ratio > 1 ? '↑' : '↓'}{formatPercent(Math.abs(metrics.vs_avg_ratio - 1))} vs avg
        </span>
      {/if}
    </div>
  </div>
{:else}
  <!-- Skeleton state -->
  <div class="analytics-bar skeleton-bar" aria-label={$tr('analytics.bar.metricsUnavailable')}>
    <div class="analytics-bar-row row1">
      <div class="skeleton skeleton-progress"></div>
      <div class="skeleton skeleton-text"></div>
    </div>
    <div class="analytics-bar-row row2">
      <div class="skeleton skeleton-text-sm"></div>
      <div class="skeleton skeleton-text-sm"></div>
      <div class="skeleton skeleton-text-sm"></div>
    </div>
  </div>
{/if}

<style>
  .analytics-bar {
    display: flex;
    flex-direction: column;
    gap: var(--stack-tight);
    padding: var(--space-1) var(--space-2);
    background: var(--bg-primary);
    border-top: var(--border-width, 2px) solid var(--border);
    border-radius: var(--radius);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    container-type: inline-size;
    container-name: analytics-bar;
  }

  .analytics-bar.budget-warning {
    border-top-color: var(--warning);
  }

  .analytics-bar.budget-danger {
    border-top-color: var(--danger);
  }

  /* Error flash animations */
  @keyframes error-pulse {
    0% { background: var(--bg-primary); }
    25% { background: color-mix(in srgb, var(--danger) 20%, var(--bg-primary)); }
    100% { background: var(--bg-primary); }
  }

  @keyframes recovery-pulse {
    0% { background: var(--bg-primary); }
    25% { background: color-mix(in srgb, var(--accent) 20%, var(--bg-primary)); }
    100% { background: var(--bg-primary); }
  }

  .analytics-bar.error-flash {
    animation: error-pulse 0.5s ease-out;
  }

  .analytics-bar.error-instant {
    background: color-mix(in srgb, var(--danger) 15%, var(--bg-primary));
  }

  .analytics-bar.recovery-flash {
    animation: recovery-pulse 0.5s ease-out;
  }

  .analytics-bar.recovery-instant {
    background: color-mix(in srgb, var(--accent) 15%, var(--bg-primary));
  }

  /* Rows */
  .analytics-bar-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: nowrap;
    overflow: hidden;
    min-height: 20px;
  }

  /* Progress bar */
  .progress-track {
    position: relative;
    flex: 1;
    min-width: 40px;
    max-width: 120px;
    height: 4px;
    background: var(--bg-tertiary);
    border-radius: var(--radius);
    overflow: visible;
    cursor: default;
  }

  .progress-fill {
    height: 100%;
    border-radius: var(--radius);
    min-width: 0;
  }

  .progress-fill.zone-safe {
    background: var(--accent);
  }

  .progress-fill.zone-warning {
    background: var(--warning);
  }

  .progress-fill.zone-danger {
    background: var(--danger);
  }

  /* Danger marker at 80% */
  .danger-marker {
    position: absolute;
    top: -2px;
    inset-inline-start: 80%;
    width: 1px;
    height: 8px;
    background: var(--danger);
    opacity: 0.7;
  }

  .ctx-label {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  /* Metrics */
  .metric {
    white-space: nowrap;
    flex-shrink: 0;
    color: var(--text-secondary);
    line-height: 1;
  }

  .metric.cost {
    color: var(--text-primary);
    font-weight: 500;
  }

  .metric.velocity {
    color: var(--warning-text);
    font-size: var(--font-size-sm);
  }

  .metric.projection {
    color: var(--text-secondary);
    opacity: 0.8;
    font-size: var(--font-size-sm);
  }

  .metric.pace {
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }

  .metric.pace.pace-warning {
    color: var(--warning-text, #f0ad4e);
  }

  .metric.pace.pace-danger {
    color: var(--danger);
    font-weight: 600;
  }

  .metric.provider {
    font-weight: 600;
    max-width: 100px;
    overflow: auto;
    text-overflow: ellipsis;
  }

  .metric.cache {
    color: var(--text-secondary);
  }

  .metric.cache.cache-good {
    color: var(--success-text);
  }

  .metric.vs-avg {
    display: flex;
    align-items: center;
    gap: var(--stack-tight);
  }

  .metric.vs-avg.above {
    color: var(--warning-text);
  }

  .metric.vs-avg.below {
    color: var(--success-text);
  }

  /* vs-avg dot */
  .vs-avg-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: var(--radius);
    flex-shrink: 0;
  }

  .vs-avg-dot.dot-safe {
    background: var(--success);
  }

  .vs-avg-dot.dot-warning {
    background: var(--warning);
  }

  .vs-avg-dot.dot-danger {
    background: var(--danger);
  }

  /* Dashboard button */
  .dashboard-link {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    font-size: var(--font-size-base);
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 44px;
    min-height: 44px;
    border-radius: var(--radius);
    color: var(--text-secondary);
    margin-inline-start: auto;
    flex-shrink: 0;
    transition: opacity var(--transition-fast);
  }

  .dashboard-link:hover {
    opacity: 0.75;
  }

  .dashboard-link:focus-visible {
    outline: var(--border-width, 2px) solid var(--accent);
    outline-offset: 1px;
  }

  /* Skeleton */
  .skeleton-bar {
    opacity: 0.5;
  }

  .skeleton {
    background: var(--bg-tertiary);
    border-radius: var(--radius);
    animation: shimmer 1.5s ease-in-out infinite;
  }

  @keyframes shimmer {
    0%, 100% { opacity: 0.5; }
    50% { opacity: 1; }
  }

  .skeleton-progress {
    flex: 1;
    min-width: 40px;
    max-width: 120px;
    height: 4px;
  }

  .skeleton-text {
    width: 50px;
    height: 12px;
  }

  .skeleton-text-sm {
    width: 36px;
    height: 10px;
  }

  /* Narrow-width collapse: show cost | tokens | ctx% | 📊 */
  @container analytics-bar (max-width: 280px) {
    .row2 {
      display: none;
    }

    .row1 {
      flex-wrap: nowrap;
    }

    .metric.velocity,
    .metric.projection,
    .metric.pace {
      display: none;
    }

    .progress-track {
      display: none;
    }
  }

  /* Add compact token count to row1 for narrow views */
  .compact-tokens {
    display: none;
  }

  @container analytics-bar (max-width: 280px) {
    .compact-tokens {
      display: inline;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .skeleton {
      animation: none;
    }
  }
</style>
