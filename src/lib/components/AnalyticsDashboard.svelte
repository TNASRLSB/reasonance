<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import type { Adapter } from '$lib/adapter/index';
  import {
    providerAnalytics, dailyStats, modelBreakdown,
    fetchProviderAnalytics, fetchDailyStats, fetchModelBreakdown
  } from '$lib/stores/analytics';
  import { analyticsDashboard } from '$lib/stores/ui';
  import { analyticsAnnouncer } from '$lib/utils/a11y-announcer';
  import { kpiLabel, providerBarLabel, trendBarLabel } from '$lib/utils/a11y-labels';
  import {
    formatCurrency, formatTokenCount, formatDuration, formatPercent, formatDate
  } from '$lib/utils/format-analytics';
  import { getProviderVisual, barStyle } from '$lib/utils/provider-patterns';
  import { normalizeBarScale } from '$lib/utils/bar-scale';
  import { gridNavigation } from '$lib/utils/grid-navigation';
  import { tooltip } from '$lib/utils/tooltip';
  import { tr } from '$lib/i18n/index';

  let { adapter }: { adapter: Adapter } = $props();

  // === Period ===
  type Period = '1d' | '7d' | '14d' | '30d' | 'all';
  let selectedPeriod = $state<Period>('7d');
  let compareMode = $state(false);

  // === Drill-down ===
  let expandedProvider = $state<string | null>(null);
  let selectedSession = $state<string | null>(null);

  // === Insights ===
  let dismissedInsights = $state<Set<string>>(new Set());

  // === Export ===
  let showExportMenu = $state(false);

  // Provider row refs for deep link scroll
  let providerRowRefs = $state<Record<string, HTMLElement>>({});

  // Announcer container ref
  let announceContainer = $state<HTMLElement | null>(null);

  function periodToDays(p: Period): number | undefined {
    if (p === '1d') return 1;
    if (p === '7d') return 7;
    if (p === '14d') return 14;
    if (p === '30d') return 30;
    return undefined;
  }

  function periodToTimeRange(p: Period) {
    const days = periodToDays(p);
    if (days == null) return undefined;
    const from = Date.now() - days * 24 * 60 * 60 * 1000;
    return { from, to: null };
  }

  $effect(() => {
    const timeRange = periodToTimeRange(selectedPeriod);
    const days = periodToDays(selectedPeriod) ?? 30;
    fetchProviderAnalytics(adapter, timeRange);
    fetchDailyStats(adapter, days);
  });

  // === KPI derived values ===
  const totalCost = $derived.by(() => {
    const data = $providerAnalytics.data;
    if (!data) return null;
    return data.reduce((s, p) => s + (p.total_cost_usd ?? 0), 0);
  });

  const totalInputTokens = $derived.by(() => {
    const data = $providerAnalytics.data;
    if (!data) return 0;
    return data.reduce((s, p) => s + p.total_input_tokens, 0);
  });

  const totalOutputTokens = $derived.by(() => {
    const data = $providerAnalytics.data;
    if (!data) return 0;
    return data.reduce((s, p) => s + p.total_output_tokens, 0);
  });

  const totalSessions = $derived.by(() => {
    const data = $providerAnalytics.data;
    if (!data) return 0;
    return data.reduce((s, p) => s + p.total_sessions, 0);
  });

  const totalErrors = $derived.by(() => {
    const data = $providerAnalytics.data;
    if (!data) return 0;
    return data.reduce((s, p) => s + p.total_errors, 0);
  });

  const totalRecovered = $derived.by(() => {
    const data = $providerAnalytics.data;
    if (!data) return 0;
    return data.reduce((s, p) => s + p.recovered_errors, 0);
  });

  const overallErrorRate = $derived.by(() => {
    const data = $providerAnalytics.data;
    if (!data || !data.length) return 0;
    const totalSess = data.reduce((s, p) => s + p.total_sessions, 0);
    const totalErr = data.reduce((s, p) => s + p.total_errors, 0);
    return totalSess > 0 ? totalErr / totalSess : 0;
  });

  // === Insights ===
  interface Insight {
    id: string;
    key: string;
    severity: 'warning' | 'danger' | 'info';
    params: Record<string, string>;
  }

  const insights = $derived.by((): Insight[] => {
    const data = $providerAnalytics.data;
    if (!data) return [];
    const result: Insight[] = [];

    // Error spike: provider with error_rate > 0.05
    for (const p of data) {
      if (p.error_rate > 0.05) {
        const id = `errorSpike-${p.provider}`;
        if (!dismissedInsights.has(id)) {
          result.push({
            id,
            key: 'analytics.insights.errorSpike',
            severity: 'danger',
            params: { provider: p.provider, rate: formatPercent(p.error_rate) },
          });
        }
      }
    }

    // Cost anomaly: provider with cost > 2x average
    const costs = data.map(p => p.total_cost_usd ?? 0);
    const avgCost = costs.length ? costs.reduce((a, b) => a + b, 0) / costs.length : 0;
    for (const p of data) {
      const cost = p.total_cost_usd ?? 0;
      if (avgCost > 0 && cost > 2 * avgCost) {
        const id = `costAnomaly-${p.provider}`;
        if (!dismissedInsights.has(id)) {
          result.push({
            id,
            key: 'analytics.insights.costAnomaly',
            severity: 'warning',
            params: { count: '1', provider: p.provider },
          });
        }
      }
    }

    // Cache drop: provider with cache_hit_rate < 0.3
    for (const p of data) {
      if (p.cache_hit_rate < 0.3 && p.total_sessions > 0) {
        const id = `cacheDrop-${p.provider}`;
        if (!dismissedInsights.has(id)) {
          result.push({
            id,
            key: 'analytics.insights.cacheDrop',
            severity: 'info',
            params: { from: formatPercent(0.5), to: formatPercent(p.cache_hit_rate) },
          });
        }
      }
    }

    return result.slice(0, 3);
  });

  // === Provider comparison bars ===
  const providerBars = $derived.by(() => {
    const data = $providerAnalytics.data;
    if (!data) return [];
    const items = data.map(p => ({ key: p.provider, value: p.total_cost_usd ?? 0 }));
    return normalizeBarScale(items);
  });

  const totalProviderCost = $derived.by(() => {
    const data = $providerAnalytics.data;
    if (!data) return 0;
    return data.reduce((s, p) => s + (p.total_cost_usd ?? 0), 0);
  });

  // === Daily trend ===
  const trendBars = $derived.by(() => {
    const data = $dailyStats.data;
    if (!data) return [];
    const items = data.map(d => ({ key: d.date, value: d.input_tokens + d.output_tokens }));
    return normalizeBarScale(items);
  });

  // Last 7 daily stats for sparklines
  const sparklineData = $derived.by(() => {
    const data = $dailyStats.data;
    if (!data) return [];
    return data.slice(-7);
  });

  function sparklinePath(values: number[]): string {
    if (values.length === 0) return '';
    const max = Math.max(...values, 1);
    const w = 60;
    const h = 24;
    const pts = values.map((v, i) => {
      const x = (i / Math.max(values.length - 1, 1)) * w;
      const y = h - (v / max) * h;
      return `${x},${y}`;
    });
    return pts.join(' ');
  }

  // === Export ===
  async function handleExport(format: 'csv' | 'json') {
    const providers = get(providerAnalytics);
    if (!providers.data) return;
    const ext = format === 'csv' ? 'csv' : 'json';
    const path = await adapter.saveFileDialog(undefined, [{ name: ext.toUpperCase(), extensions: [ext] }]);
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
    showExportMenu = false;
  }

  function dismissInsight(id: string) {
    dismissedInsights = new Set([...dismissedInsights, id]);
  }

  function toggleProvider(provider: string) {
    if (expandedProvider === provider) {
      expandedProvider = null;
    } else {
      expandedProvider = provider;
      fetchModelBreakdown(adapter, provider);
    }
  }

  function handlePeriodKeydown(e: KeyboardEvent, period: Period) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      selectedPeriod = period;
    }
  }

  onMount(() => {
    if (announceContainer) {
      analyticsAnnouncer.mount(announceContainer);
    }

    // Deep link focus
    const focus = get(analyticsDashboard).focus;
    if (focus?.provider) {
      const target = focus.provider;
      // After a tick, scroll to provider
      setTimeout(() => {
        const el = providerRowRefs[target];
        if (el) el.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }, 100);
    }
  });
</script>

<div class="analytics-dashboard" role="region" aria-label="Analytics Dashboard">
  <!-- Announcer (visually hidden) -->
  <div bind:this={announceContainer} class="sr-only-container"></div>

  <!-- Header -->
  <div class="dashboard-header">
    <h2 class="dashboard-title">{$tr('analytics.dashboard.title')}</h2>
    <div class="header-actions">
      <!-- Period selector -->
      <div class="period-selector" role="radiogroup" aria-label="Time period">
        {#each ([['1d', 'analytics.dashboard.period.today'], ['7d', 'analytics.dashboard.period.7d'], ['14d', 'analytics.dashboard.period.14d'], ['30d', 'analytics.dashboard.period.30d'], ['all', 'analytics.dashboard.period.all']] as [Period, string][]) as [period, labelKey]}
          <button
            class="period-btn"
            class:active={selectedPeriod === period}
            role="radio"
            aria-checked={selectedPeriod === period}
            onclick={() => { selectedPeriod = period; }}
            onkeydown={(e) => handlePeriodKeydown(e, period)}
          >{$tr(labelKey)}</button>
        {/each}
        <button
          class="period-btn compare-btn"
          class:active={compareMode}
          aria-pressed={compareMode}
          onclick={() => { compareMode = !compareMode; }}
        >⇄ {$tr('analytics.dashboard.compare')}</button>
      </div>

      <!-- Export -->
      <div class="export-wrapper">
        <button
          class="header-btn"
          aria-haspopup="true"
          aria-expanded={showExportMenu}
          onclick={() => { showExportMenu = !showExportMenu; }}
        >{$tr('analytics.dashboard.export')} ▾</button>
        {#if showExportMenu}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="export-menu" role="menu">
            <button role="menuitem" onclick={() => handleExport('csv')}>Export CSV</button>
            <button role="menuitem" onclick={() => handleExport('json')}>Export JSON</button>
          </div>
        {/if}
      </div>

      <button
        class="header-btn"
        onclick={() => {
          const tr = periodToTimeRange(selectedPeriod);
          const days = periodToDays(selectedPeriod) ?? 30;
          fetchProviderAnalytics(adapter, tr, true);
          fetchDailyStats(adapter, days, true);
        }}
        title={$tr('analytics.dashboard.refresh')}
        aria-label={$tr('analytics.dashboard.refresh')}
      >↻</button>
    </div>
  </div>

  <div class="dashboard-body">

    <!-- ── Section 1: KPI Cards ── -->
    <section class="section kpi-section" aria-label="Key metrics" aria-busy={$providerAnalytics.status === 'loading'}>
      {#if $providerAnalytics.status === 'loading'}
        <div class="kpi-grid">
          {#each [0,1,2,3] as _}
            <div class="kpi-card skeleton" aria-hidden="true">
              <div class="skel-line skel-title"></div>
              <div class="skel-line skel-value"></div>
              <div class="skel-line skel-sub"></div>
            </div>
          {/each}
        </div>
      {:else if $providerAnalytics.status === 'error'}
        <p class="error-msg">{$providerAnalytics.error}</p>
      {:else}
        <div class="kpi-grid">
          <!-- Cost -->
          <div
            class="kpi-card"
            role="article"
            aria-label={kpiLabel($tr('analytics.dashboard.kpi.totalCost'), formatCurrency(totalCost))}
          >
            <div class="kpi-top">
              <span class="kpi-label">{$tr('analytics.dashboard.kpi.totalCost')}</span>
              <svg class="sparkline" viewBox="0 0 60 24" aria-hidden="true" width="60" height="24">
                <polyline
                  points={sparklinePath(sparklineData.map(d => d.input_tokens + d.output_tokens))}
                  fill="none"
                  stroke="var(--accent)"
                  stroke-width="1.5"
                />
              </svg>
            </div>
            <div class="kpi-value">{formatCurrency(totalCost)}</div>
            <div class="kpi-secondary">{$tr('analytics.dashboard.kpi.sessions')}: {totalSessions}</div>
          </div>

          <!-- Tokens -->
          <div
            class="kpi-card"
            role="article"
            aria-label={kpiLabel($tr('analytics.dashboard.kpi.totalTokens'), formatTokenCount(totalInputTokens + totalOutputTokens))}
          >
            <div class="kpi-top">
              <span class="kpi-label">{$tr('analytics.dashboard.kpi.totalTokens')}</span>
              <svg class="sparkline" viewBox="0 0 60 24" aria-hidden="true" width="60" height="24">
                <polyline
                  points={sparklinePath(sparklineData.map(d => d.input_tokens + d.output_tokens))}
                  fill="none"
                  stroke="var(--success)"
                  stroke-width="1.5"
                />
              </svg>
            </div>
            <div class="kpi-value">{formatTokenCount(totalInputTokens + totalOutputTokens)}</div>
            <div class="kpi-secondary">in: {formatTokenCount(totalInputTokens)} · out: {formatTokenCount(totalOutputTokens)}</div>
          </div>

          <!-- Sessions -->
          <div
            class="kpi-card"
            role="article"
            aria-label={kpiLabel($tr('analytics.dashboard.kpi.sessions'), String(totalSessions))}
          >
            <div class="kpi-top">
              <span class="kpi-label">{$tr('analytics.dashboard.kpi.sessions')}</span>
              <svg class="sparkline" viewBox="0 0 60 24" aria-hidden="true" width="60" height="24">
                <polyline
                  points={sparklinePath(sparklineData.map(d => d.sessions))}
                  fill="none"
                  stroke="var(--text-secondary)"
                  stroke-width="1.5"
                />
              </svg>
            </div>
            <div class="kpi-value">{totalSessions}</div>
            <div class="kpi-secondary">{($providerAnalytics.data?.length ?? 0)} providers</div>
          </div>

          <!-- Errors -->
          <div
            class="kpi-card"
            class:kpi-danger={totalErrors > 0}
            role="article"
            aria-label={kpiLabel($tr('analytics.dashboard.kpi.errors'), String(totalErrors))}
          >
            <div class="kpi-top">
              <span class="kpi-label">{$tr('analytics.dashboard.kpi.errors')}</span>
              <svg class="sparkline" viewBox="0 0 60 24" aria-hidden="true" width="60" height="24">
                <polyline
                  points={sparklinePath(sparklineData.map(d => d.errors))}
                  fill="none"
                  stroke="var(--danger)"
                  stroke-width="1.5"
                />
              </svg>
            </div>
            <div class="kpi-value">{totalErrors}</div>
            <div class="kpi-secondary">
              {totalRecovered} {$tr('analytics.dashboard.kpi.recovered')} · {$tr('analytics.dashboard.kpi.errorRate')} {formatPercent(overallErrorRate)}
            </div>
          </div>
        </div>
      {/if}
    </section>

    <!-- ── Section 2: Insights ── -->
    {#if insights.length > 0}
      <section class="section insights-section" aria-label={$tr('analytics.insights.title')}>
        <h3 class="section-title">{$tr('analytics.insights.title')}</h3>
        <div class="insights-list">
          {#each insights as insight (insight.id)}
            <div class="insight-card insight-{insight.severity}" role="alert">
              <span class="insight-text">{$tr(insight.key, insight.params)}</span>
              <button
                class="insight-dismiss"
                aria-label="Dismiss insight"
                onclick={() => dismissInsight(insight.id)}
              >✕</button>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    <!-- ── Section 3: Provider Comparison ── -->
    <section
      class="section providers-section"
      aria-label={$tr('analytics.dashboard.providers')}
      aria-busy={$providerAnalytics.status === 'loading'}
    >
      <h3 class="section-title">{$tr('analytics.dashboard.providers')}</h3>

      {#if $providerAnalytics.status === 'loading'}
        <div class="providers-skeleton">
          {#each [0,1,2] as _}
            <div class="skel-provider" aria-hidden="true">
              <div class="skel-line skel-label"></div>
              <div class="skel-bar"></div>
            </div>
          {/each}
        </div>
      {:else if $providerAnalytics.status === 'error'}
        <p class="error-msg">{$providerAnalytics.error}</p>
      {:else if !$providerAnalytics.data?.length}
        <p class="no-data">{$tr('analytics.dashboard.noData')}</p>
      {:else}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          class="providers-grid"
          role="grid"
          aria-label={$tr('analytics.dashboard.providers')}
          use:gridNavigation
        >
          <!-- Header row -->
          <div class="provider-header-row" role="row">
            <div class="provider-col-name" role="columnheader">Provider</div>
            <div class="provider-col-bar" role="columnheader">Cost</div>
            <div class="provider-col-sessions" role="columnheader">Sessions</div>
            <div class="provider-col-tokens" role="columnheader">Tokens</div>
            <div class="provider-col-errors" role="columnheader">Errors</div>
          </div>

          {#each $providerAnalytics.data as provider (provider.provider)}
            {@const bar = providerBars.find(b => b.key === provider.provider)}
            {@const visual = getProviderVisual(provider.provider)}
            <div
              class="provider-row"
              role="row"
              bind:this={providerRowRefs[provider.provider]}
            >
              <div
                class="provider-col-name"
                role="rowheader"
                tabindex="0"
                aria-label={providerBarLabel(provider.provider, provider, totalProviderCost)}
              >
                <button
                  class="provider-expand-btn"
                  aria-expanded={expandedProvider === provider.provider}
                  aria-controls="provider-detail-{provider.provider}"
                  onclick={() => toggleProvider(provider.provider)}
                >
                  <span class="provider-dot" style="background: {visual.color}"></span>
                  <span class="provider-name">{provider.provider}</span>
                  <span class="expand-chevron">{expandedProvider === provider.provider ? '▲' : '▼'}</span>
                </button>
              </div>

              <div class="provider-col-bar" role="gridcell" tabindex="-1">
                <div class="bar-track" aria-hidden="true">
                  <div class="bar-fill" style={barStyle(provider.provider, bar?.widthPercent ?? 0)}></div>
                </div>
                <span class="bar-value">{formatCurrency(provider.total_cost_usd)}</span>
              </div>

              <div class="provider-col-sessions" role="gridcell" tabindex="-1">
                {provider.total_sessions}
              </div>

              <div class="provider-col-tokens" role="gridcell" tabindex="-1">
                {formatTokenCount(provider.total_input_tokens + provider.total_output_tokens)}
              </div>

              <div
                class="provider-col-errors"
                role="gridcell"
                tabindex="-1"
                class:cell-danger={provider.error_rate > 0.05}
              >
                {provider.total_errors} ({formatPercent(provider.error_rate)})
              </div>
            </div>

            <!-- Accordion drill-down -->
            {#if expandedProvider === provider.provider}
              <div
                id="provider-detail-{provider.provider}"
                class="provider-detail"
                role="region"
                aria-label="Details for {provider.provider}"
              >
                <div class="provider-detail-stats">
                  <div class="detail-stat">
                    <span class="detail-label">Model</span>
                    <span class="detail-value">{provider.most_used_model}</span>
                  </div>
                  <div class="detail-stat">
                    <span class="detail-label">Avg Duration</span>
                    <span class="detail-value">{formatDuration(provider.avg_duration_ms)}</span>
                  </div>
                  <div class="detail-stat">
                    <span class="detail-label">Cache Rate</span>
                    <span class="detail-value">{formatPercent(provider.cache_hit_rate)}</span>
                  </div>
                  <div class="detail-stat">
                    <span class="detail-label">Tokens/s</span>
                    <span class="detail-value">{provider.avg_tokens_per_second.toFixed(1)}</span>
                  </div>
                  <div class="detail-stat">
                    <span class="detail-label">Tools</span>
                    <span class="detail-value">{provider.total_tool_invocations}</span>
                  </div>
                  <div class="detail-stat">
                    <span class="detail-label">Recovered</span>
                    <span class="detail-value">{provider.recovered_errors}</span>
                  </div>
                </div>

                {#if $modelBreakdown.status === 'loading'}
                  <p class="detail-loading">Loading model breakdown…</p>
                {:else if $modelBreakdown.data}
                  <table class="model-table" aria-label="Model breakdown for {provider.provider}">
                    <thead>
                      <!-- svelte-ignore component_name_lowercase -->
                      <tr>
                        <th>Model</th>
                        <th>Sessions</th>
                        <th>Avg In</th>
                        <th>Avg Out</th>
                        <th>Error Rate</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each $modelBreakdown.data.filter(m => m.provider === provider.provider) as model (model.model)}
                        <!-- svelte-ignore component_name_lowercase -->
                        <tr>
                          <td>{model.model}</td>
                          <td>{model.session_count}</td>
                          <td>{formatTokenCount(model.avg_input_tokens)}</td>
                          <td>{formatTokenCount(model.avg_output_tokens)}</td>
                          <td class:text-danger={model.error_rate > 0.05}>{formatPercent(model.error_rate)}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                {/if}
              </div>
            {/if}
          {/each}
        </div>
      {/if}
    </section>

    <!-- ── Section 4: Daily Trend ── -->
    <section
      class="section trend-section"
      aria-label={$tr('analytics.dashboard.trend')}
      aria-busy={$dailyStats.status === 'loading'}
    >
      <h3 class="section-title">{$tr('analytics.dashboard.trend')}</h3>

      {#if $dailyStats.status === 'loading'}
        <div class="trend-skeleton" aria-hidden="true">
          {#each Array(7) as _}
            <div class="skel-trend-bar"></div>
          {/each}
        </div>
      {:else if $dailyStats.status === 'error'}
        <p class="error-msg">{$dailyStats.error}</p>
      {:else if !$dailyStats.data?.length}
        <p class="no-data">{$tr('analytics.dashboard.noData')}</p>
      {:else}
        <div class="trend-chart" role="img" aria-label="Daily token trend">
          {#each trendBars as bar, i}
            {@const day = $dailyStats.data![i]}
            <div
              class="trend-bar-col"
              use:tooltip={day ? trendBarLabel(day) : bar.key}
            >
              <div class="trend-bar-track">
                <div
                  class="trend-bar-fill"
                  style="height: {bar.widthPercent}%"
                  aria-hidden="true"
                ></div>
                {#if compareMode}
                  <div
                    class="trend-bar-compare"
                    style="height: {Math.max(0, bar.widthPercent * 0.7)}%; opacity: 0.3"
                    aria-hidden="true"
                  ></div>
                {/if}
              </div>
              <span class="trend-bar-label">{bar.key.slice(5)}</span>
            </div>
          {/each}
        </div>
      {/if}
    </section>

  </div>
</div>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<svelte:window onclick={() => { showExportMenu = false; }} />

<style>
  .analytics-dashboard {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-family: var(--font-ui);
  }

  .sr-only-container {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    pointer-events: none;
  }

  /* Header */
  .dashboard-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    background: var(--bg-surface);
    flex-shrink: 0;
  }

  .dashboard-title {
    font-size: var(--font-size-tiny);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin: 0;
    color: var(--text-primary);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
    flex-wrap: wrap;
  }

  /* Period selector */
  .period-selector {
    display: flex;
    gap: var(--stack-tight);
    align-items: center;
  }

  .period-btn {
    background: var(--bg-tertiary);
    border: var(--border-width, 2px) solid var(--border);
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: var(--space-1) var(--space-2);
    cursor: pointer;
    border-radius: 0;
    min-height: 24px;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
  }

  .period-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .period-btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }

  .compare-btn {
    margin-inline-start: var(--stack-tight);
  }

  .compare-btn.active {
    background: var(--warning);
    border-color: var(--warning);
    color: var(--text-primary);
  }

  /* Header buttons */
  .header-btn {
    background: var(--bg-tertiary);
    border: var(--border-width, 2px) solid var(--border);
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: var(--space-1) var(--space-2);
    cursor: pointer;
    border-radius: 0;
    min-height: 24px;
    text-transform: uppercase;
    transition: background 0.1s, color 0.1s;
  }

  .header-btn:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
    border-color: var(--text-primary);
  }

  /* Export */
  .export-wrapper {
    position: relative;
  }

  .export-menu {
    position: absolute;
    top: 100%;
    right: 0;
    z-index: 200;
    background: var(--bg-secondary);
    border: var(--border-width, 2px) solid var(--border);
    min-width: 140px;
  }

  .export-menu button {
    display: block;
    width: 100%;
    text-align: start;
    background: transparent;
    border: none;
    border-radius: 0;
    padding: var(--space-1) var(--space-3);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 600;
    color: var(--text-body);
    cursor: pointer;
    min-height: unset;
    text-transform: none;
    letter-spacing: normal;
  }

  .export-menu button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    border: none;
  }

  /* Body */
  .dashboard-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--inset-section);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  /* Sections */
  .section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .section-title {
    font-size: var(--font-size-small);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
    margin: 0;
    padding-bottom: var(--space-2);
    border-bottom: var(--border-width, 2px) solid var(--border);
  }

  /* KPI Grid */
  .kpi-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--stack-normal);
  }

  @media (max-width: 900px) {
    .kpi-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  @media (max-width: 500px) {
    .kpi-grid {
      grid-template-columns: 1fr;
    }
  }

  .kpi-card {
    background: var(--bg-surface);
    border: var(--border-width, 2px) solid var(--border);
    padding: var(--inset-component);
    display: flex;
    flex-direction: column;
    gap: var(--stack-tight);
    min-width: 0;
  }

  .kpi-card.kpi-danger {
    border-color: var(--danger);
  }

  .kpi-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .kpi-label {
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }

  .sparkline {
    flex-shrink: 0;
  }

  .kpi-value {
    font-family: var(--font-mono);
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-hero);
    color: var(--text-primary);
    line-height: var(--line-height-hero);
  }

  .kpi-secondary {
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  /* Skeleton */
  .skeleton {
    pointer-events: none;
  }

  .skel-line {
    background: var(--bg-tertiary);
    border-radius: 0;
    animation: pulse 1.5s ease-in-out infinite;
  }

  .skel-title {
    height: 10px;
    width: 60%;
    margin-bottom: var(--stack-normal);
  }

  .skel-value {
    height: 20px;
    width: 80%;
    margin-bottom: var(--interactive-gap);
  }

  .skel-sub {
    height: 8px;
    width: 50%;
  }

  .skel-label {
    height: 10px;
    width: 40%;
  }

  .skel-bar {
    height: 16px;
    width: 100%;
    background: var(--bg-tertiary);
    animation: pulse 1.5s ease-in-out infinite;
  }

  .skel-provider {
    display: flex;
    flex-direction: column;
    gap: var(--interactive-gap);
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--border);
  }

  .trend-skeleton {
    display: flex;
    align-items: flex-end;
    gap: var(--stack-tight);
    height: 80px;
  }

  .skel-trend-bar {
    flex: 1;
    background: var(--bg-tertiary);
    animation: pulse 1.5s ease-in-out infinite;
  }

  .skel-trend-bar:nth-child(1) { height: 30%; }
  .skel-trend-bar:nth-child(2) { height: 50%; }
  .skel-trend-bar:nth-child(3) { height: 70%; }
  .skel-trend-bar:nth-child(4) { height: 45%; }
  .skel-trend-bar:nth-child(5) { height: 80%; }
  .skel-trend-bar:nth-child(6) { height: 60%; }
  .skel-trend-bar:nth-child(7) { height: 40%; }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  /* Insights */
  .insights-list {
    display: flex;
    flex-direction: column;
    gap: var(--interactive-gap);
  }

  .insight-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: var(--border-width, 2px) solid var(--border);
    background: var(--bg-surface);
    font-size: var(--font-size-small);
  }

  .insight-warning {
    border-color: var(--warning);
  }

  .insight-danger {
    border-color: var(--danger);
  }

  .insight-info {
    border-color: var(--accent);
  }

  .insight-text {
    flex: 1;
    color: var(--text-primary);
  }

  .insight-dismiss {
    background: transparent;
    border: none;
    border-radius: 0;
    color: var(--text-muted);
    cursor: pointer;
    font-size: var(--font-size-sm);
    padding: var(--stack-tight) var(--space-1);
    min-height: unset;
    flex-shrink: 0;
    font-family: var(--font-ui);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: normal;
  }

  .insight-dismiss:hover {
    color: var(--text-primary);
    background: var(--bg-tertiary);
    border: none;
  }

  /* Providers grid */
  .providers-grid {
    border: var(--border-width, 2px) solid var(--border);
    overflow-x: auto;
  }

  .provider-header-row,
  .provider-row {
    display: grid;
    grid-template-columns: 200px 1fr 80px 100px 120px;
    align-items: center;
    min-width: 600px;
  }

  .provider-header-row {
    background: var(--bg-tertiary);
    border-bottom: var(--border-width, 2px) solid var(--border);
    font-size: var(--font-size-tiny);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }

  .provider-header-row [role="columnheader"],
  .provider-row [role="gridcell"],
  .provider-row [role="rowheader"] {
    padding: var(--space-2) var(--space-2);
    font-size: var(--font-size-small);
    font-family: var(--font-mono);
    color: var(--text-primary);
    border-inline-end: 1px solid var(--border);
  }

  .provider-header-row [role="columnheader"]:last-child,
  .provider-row [role="gridcell"]:last-child,
  .provider-row [role="rowheader"]:last-child {
    border-inline-end: none;
  }

  .provider-row {
    border-bottom: 1px solid var(--border);
    transition: background 0.1s;
  }

  .provider-row:hover {
    background: var(--bg-secondary);
  }

  .provider-row:last-of-type {
    border-bottom: none;
  }

  .provider-expand-btn {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
    background: transparent;
    border: none;
    border-radius: 0;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--font-size-small);
    font-weight: 600;
    cursor: pointer;
    padding: 0;
    min-height: unset;
    text-transform: none;
    letter-spacing: normal;
    width: 100%;
    text-align: start;
  }

  .provider-expand-btn:hover {
    background: transparent;
    border: none;
    color: var(--accent);
  }

  .provider-dot {
    width: 10px;
    height: 10px;
    border-radius: 0;
    flex-shrink: 0;
  }

  .provider-name {
    flex: 1;
    text-transform: capitalize;
  }

  .expand-chevron {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .bar-track {
    width: 100%;
    height: 14px;
    background: var(--bg-tertiary);
    position: relative;
    overflow: hidden;
  }

  .bar-fill {
    position: absolute;
    top: 0;
    left: 0;
    bottom: 0;
  }

  .bar-value {
    display: block;
    font-size: var(--font-size-tiny);
    color: var(--text-secondary);
    margin-top: var(--stack-tight);
  }

  .provider-col-bar {
    display: flex;
    flex-direction: column;
    gap: var(--stack-tight);
    justify-content: center;
  }

  .cell-danger {
    color: var(--danger) !important;
  }

  /* Provider detail accordion */
  .provider-detail {
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    padding: var(--inset-component) var(--inset-section);
  }

  .provider-detail-stats {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
    margin-bottom: var(--space-3);
  }

  .detail-stat {
    display: flex;
    flex-direction: column;
    gap: var(--stack-tight);
    min-width: 80px;
  }

  .detail-label {
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .detail-value {
    font-family: var(--font-mono);
    font-size: var(--font-size-small);
    color: var(--text-primary);
    font-weight: 600;
  }

  .detail-loading {
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin: 0;
  }

  .model-table {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
  }

  .model-table th {
    text-align: start;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    padding: var(--space-1) var(--space-2);
    border-bottom: var(--border-width, 2px) solid var(--border);
  }

  .model-table td {
    padding: var(--space-1) var(--space-2);
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
  }

  .text-danger {
    color: var(--danger);
  }

  /* Daily trend */
  .trend-chart {
    display: flex;
    align-items: flex-end;
    gap: var(--stack-tight);
    height: 100px;
    overflow-x: auto;
    padding-bottom: var(--stack-tight);
  }

  .trend-bar-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--stack-tight);
    flex: 1;
    min-width: 28px;
    height: 100%;
  }

  .trend-bar-track {
    flex: 1;
    width: 100%;
    position: relative;
    display: flex;
    align-items: flex-end;
    background: var(--bg-tertiary);
  }

  .trend-bar-fill {
    width: 100%;
    background: var(--accent);
    transition: height 0.2s ease;
    min-height: 2px;
  }

  .trend-bar-compare {
    position: absolute;
    bottom: 0;
    left: 0;
    width: 100%;
    background: var(--text-secondary);
    pointer-events: none;
  }

  .trend-bar-label {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    font-family: var(--font-mono);
    white-space: nowrap;
    text-align: center;
  }

  /* Misc */
  .error-msg {
    color: var(--danger);
    font-size: var(--font-size-small);
    font-family: var(--font-mono);
    margin: 0;
  }

  .no-data {
    color: var(--text-muted);
    font-size: var(--font-size-small);
    margin: 0;
  }

  .providers-skeleton {
    border: var(--border-width, 2px) solid var(--border);
    padding: var(--space-2) var(--space-3);
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .skeleton, .skel-line { animation: none; }
    .trend-bar-fill, .trend-bar-compare { transition: none; }
  }
</style>
