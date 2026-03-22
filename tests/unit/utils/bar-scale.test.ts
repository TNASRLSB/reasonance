// tests/unit/utils/bar-scale.test.ts
import { describe, it, expect } from 'vitest';
import { normalizeBarScale } from '$lib/utils/bar-scale';

describe('normalizeBarScale', () => {
  it('normalizes proportionally to max value', () => {
    const result = normalizeBarScale([
      { key: 'a', value: 100 },
      { key: 'b', value: 50 },
      { key: 'c', value: 25 },
    ]);
    expect(result[0].widthPercent).toBe(90);
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
    expect(result[1].widthPercent).toBeGreaterThan(5);
  });
});
