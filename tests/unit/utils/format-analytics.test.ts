import { describe, it, expect } from 'vitest';
import {
  formatCurrency,
  formatTokenCount,
  formatDuration,
  formatPercent,
  formatCostVelocity,
  formatTokenRate,
} from '$lib/utils/format-analytics';

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
