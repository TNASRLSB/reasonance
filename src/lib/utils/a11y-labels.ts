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
  const cost = a.total_cost_usd ?? 0;
  const pct = totalCost > 0 ? Math.round((cost / totalCost) * 100) : 0;
  return `${provider}: ${formatCurrency(cost)}, ${pct}% of total, ${formatTokenCount(a.total_input_tokens + a.total_output_tokens)} tokens, ${a.total_sessions} sessions, error rate ${formatPercent(a.error_rate)}`;
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
  return `${day.date}: ${formatTokenCount(day.input_tokens + day.output_tokens)} tokens, ${day.sessions} sessions`;
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
