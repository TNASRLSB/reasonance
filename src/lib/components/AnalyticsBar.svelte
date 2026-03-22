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
  const hasBudgetWarning = $derived(alerts.length > 0 && alerts.some((a: { type: string }) => a.type === 'approaching'));
  const hasBudgetDanger = $derived(alerts.length > 0 && alerts.some((a: { type: string }) => a.type === 'exceeded'));

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
    if (metrics && metrics.errors > prevErrors) {
      prevErrors = metrics.errors;
      errorFlash = true;
      setTimeout(() => { errorFlash = false; }, 500);
    }
  });

  $effect(() => {
    if (metrics && metrics.errors_recovered > prevRecovered) {
      prevRecovered = metrics.errors_recovered;
      recoveryFlash = true;
      setTimeout(() => { recoveryFlash = false; }, 500);
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

      {#if velocityArrow && metrics.cost_velocity_usd_per_min > 0}
        <span class="metric velocity" use:tooltip={formatCostVelocity(metrics.cost_velocity_usd_per_min)}>
          {velocityArrow} {formatCostVelocity(metrics.cost_velocity_usd_per_min)}
        </span>
      {/if}

      {#if metrics.cost_projection_usd != null && metrics.num_turns >= 2}
        <span class="metric projection" use:tooltip={"Projected session total"}>
          → {formatCurrency(metrics.cost_projection_usd)}
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
    gap: 2px;
    padding: 4px 8px;
    background: var(--bg-primary);
    border-top: var(--border-width, 2px) solid var(--border);
    border-radius: 0;
    font-family: var(--font-mono);
    font-size: var(--font-size-small, 11px);
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
    gap: 8px;
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
    border-radius: 0;
    overflow: visible;
    cursor: default;
  }

  .progress-fill {
    height: 100%;
    border-radius: 0;
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
    left: 80%;
    width: 1px;
    height: 8px;
    background: var(--danger);
    opacity: 0.7;
  }

  .ctx-label {
    font-size: var(--font-size-small, 11px);
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
    color: var(--warning);
    font-size: calc(var(--font-size-small, 11px) - 1px);
  }

  .metric.projection {
    color: var(--text-secondary);
    opacity: 0.8;
    font-size: calc(var(--font-size-small, 11px) - 1px);
  }

  .metric.provider {
    font-weight: 600;
    max-width: 100px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .metric.cache {
    color: var(--text-secondary);
  }

  .metric.cache.cache-good {
    color: #22c55e;
  }

  .metric.vs-avg {
    display: flex;
    align-items: center;
    gap: 3px;
  }

  .metric.vs-avg.above {
    color: var(--warning);
  }

  .metric.vs-avg.below {
    color: #22c55e;
  }

  /* vs-avg dot */
  .vs-avg-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 0;
    flex-shrink: 0;
  }

  .vs-avg-dot.dot-safe {
    background: #22c55e;
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
    font-size: 14px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 44px;
    min-height: 44px;
    border-radius: 0;
    color: var(--text-secondary);
    margin-left: auto;
    flex-shrink: 0;
    transition: opacity 0.15s;
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
    border-radius: 0;
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

  /* Narrow-width collapse: show only essential metrics in 1 row */
  @container analytics-bar (max-width: 280px) {
    .row2 {
      display: none;
    }

    .row1 {
      flex-wrap: nowrap;
    }

    .metric.velocity,
    .metric.projection {
      display: none;
    }
  }
</style>
