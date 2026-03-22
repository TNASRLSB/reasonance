// src/lib/utils/tween.ts
import { readable, type Readable } from 'svelte/store';

export function easeOutCubic(t: number): number {
  return 1 - Math.pow(1 - t, 3);
}

export function tweenValue(
  from: number,
  to: number,
  duration = 600,
  easing: (t: number) => number = easeOutCubic,
): Readable<number> {
  const reducedMotion = typeof window !== 'undefined'
    && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  if (reducedMotion || duration <= 0) {
    return readable(to);
  }

  return readable(from, (set) => {
    const start = performance.now();
    let frame: number;

    function tick(now: number) {
      const elapsed = now - start;
      const t = Math.min(elapsed / duration, 1);
      set(from + (to - from) * easing(t));
      if (t < 1) {
        frame = requestAnimationFrame(tick);
      }
    }

    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  });
}
