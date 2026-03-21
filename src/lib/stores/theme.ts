import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export type ThemeMode = 'light' | 'dark' | 'system';
export const themeMode = writable<ThemeMode>('system');
export const isDark = writable(true);

export function initTheme() {
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  isDark.set(mq.matches);
  mq.addEventListener('change', (e) => isDark.set(e.matches));

  // Apply light/dark class on <html> based on themeMode + system preference
  function applyThemeClass(mode: ThemeMode, systemDark: boolean) {
    const isLight = mode === 'light' || (mode === 'system' && !systemDark);
    document.documentElement.classList.toggle('light', isLight);
  }

  let currentMode: ThemeMode = 'system';
  let currentDark = mq.matches;

  themeMode.subscribe((mode) => {
    currentMode = mode;
    applyThemeClass(currentMode, currentDark);
  });
  isDark.subscribe((dark) => {
    currentDark = dark;
    applyThemeClass(currentMode, currentDark);
  });
}

export async function loadSystemColors(): Promise<Record<string, string>> {
  try {
    return await invoke<Record<string, string>>('get_system_colors');
  } catch {
    return {};
  }
}
