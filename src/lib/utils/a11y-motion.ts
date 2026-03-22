import { readable } from 'svelte/store';

export const prefersReducedMotion = readable<boolean>(false, (set) => {
  if (typeof window === 'undefined') return;
  const query = window.matchMedia('(prefers-reduced-motion: reduce)');
  set(query.matches);
  const handler = (e: MediaQueryListEvent) => set(e.matches);
  query.addEventListener('change', handler);
  return () => query.removeEventListener('change', handler);
});

export function motionTransition(duration: string): string {
  return duration;
}
