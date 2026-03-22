// src/lib/utils/bar-scale.ts
import type { BarValue } from '$lib/types/analytics';

interface BarScaleOptions {
  maxWidthPercent?: number;
  minWidthPercent?: number;
  scaleMode?: 'proportional' | 'logarithmic';
}

export function normalizeBarScale(
  items: Array<{ key: string; value: number }>,
  options: BarScaleOptions = {},
): BarValue[] {
  const {
    maxWidthPercent = 90,
    minWidthPercent = 2,
    scaleMode = 'proportional',
  } = options;

  const valid = items.filter(i => i.value != null && isFinite(i.value) && i.value >= 0);
  if (valid.length === 0) return items.map(i => ({ key: i.key, value: i.value, widthPercent: 0 }));

  const maxVal = Math.max(...valid.map(i => i.value));
  if (maxVal === 0) return valid.map(i => ({ ...i, widthPercent: 0 }));

  return valid.map(item => {
    let ratio: number;
    if (scaleMode === 'logarithmic' && maxVal > 0) {
      ratio = Math.log1p(item.value) / Math.log1p(maxVal);
    } else {
      ratio = item.value / maxVal;
    }
    const width = Math.max(item.value > 0 ? minWidthPercent : 0, ratio * maxWidthPercent);
    return { key: item.key, value: item.value, widthPercent: width };
  });
}
