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
