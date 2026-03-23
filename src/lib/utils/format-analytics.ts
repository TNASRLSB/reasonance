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
  const loc = getLocale();
  if (count >= 1_000_000) {
    return new Intl.NumberFormat(loc, { maximumFractionDigits: 1 }).format(count / 1_000_000) + 'M';
  }
  if (count >= 1_000) {
    return new Intl.NumberFormat(loc, { maximumFractionDigits: count >= 10_000 ? 0 : 1 }).format(count / 1_000) + 'K';
  }
  return new Intl.NumberFormat(loc).format(count);
}

export function formatDuration(ms: number | null | undefined): string {
  if (ms == null || !isFinite(ms)) return '—';
  const loc = getLocale();
  if (ms < 1000) return `${new Intl.NumberFormat(loc).format(Math.round(ms))}ms`;
  if (ms < 60_000) return `${new Intl.NumberFormat(loc, { maximumFractionDigits: 1 }).format(ms / 1000)}s`;
  const minutes = Math.floor(ms / 60_000);
  const seconds = Math.round((ms % 60_000) / 1000);
  return `${new Intl.NumberFormat(loc).format(minutes)}m ${new Intl.NumberFormat(loc).format(seconds)}s`;
}

export function formatPercent(ratio: number | null | undefined): string {
  if (ratio == null || !isFinite(ratio)) return '—';
  return new Intl.NumberFormat(getLocale(), {
    style: 'percent',
    minimumFractionDigits: ratio > 0 && ratio < 0.1 ? 1 : 0,
    maximumFractionDigits: ratio > 0 && ratio < 0.1 ? 1 : 0,
  }).format(ratio);
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
