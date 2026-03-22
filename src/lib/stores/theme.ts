import { writable } from 'svelte/store';

export type ThemeMode = 'light' | 'dark' | 'system';

// Default is 'dark' — Reasonance is a dark-first IDE.
// 'system' detection is unreliable on WebKitGTK/KDE.
export const themeMode = writable<ThemeMode>('dark');
export const isDark = writable(true);

export function initTheme() {
  function applyTheme(mode: ThemeMode) {
    const dark = mode !== 'light';
    document.documentElement.classList.toggle('light', !dark);
    isDark.set(dark);
  }

  themeMode.subscribe((mode) => {
    applyTheme(mode);
  });
}
