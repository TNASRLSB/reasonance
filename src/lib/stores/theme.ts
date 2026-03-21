import { writable } from 'svelte/store';

export type ThemeMode = 'light' | 'dark' | 'system';
export const themeMode = writable<ThemeMode>('system');
export const isDark = writable(true);

export function initTheme() {
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  isDark.set(mq.matches);
  mq.addEventListener('change', (e) => isDark.set(e.matches));
}
