import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export type ThemeMode = 'light' | 'dark' | 'system';
export const themeMode = writable<ThemeMode>('system');

// Initialize synchronously from media query to avoid flash of wrong theme
const mqInit = typeof window !== 'undefined'
  ? window.matchMedia('(prefers-color-scheme: dark)').matches
  : true;
export const isDark = writable(mqInit);

export function initTheme() {
  const mq = window.matchMedia('(prefers-color-scheme: dark)');

  async function detectSystemDark(): Promise<boolean> {
    if (mq.matches) return true;
    try {
      const colors = await invoke<Record<string, string>>('get_system_colors');
      if (colors.is_dark === 'true') return true;
      if (colors.is_dark === 'false') return false;
    } catch { /* ignore */ }
    return true; // Final fallback: dark
  }

  function applyThemeClass(mode: ThemeMode, dark: boolean) {
    const isLight = mode === 'light' || (mode === 'system' && !dark);
    document.documentElement.classList.toggle('light', isLight);
    isDark.set(!isLight);
  }

  let currentMode: ThemeMode = 'system';

  themeMode.subscribe((mode) => {
    currentMode = mode;
    applyThemeClass(currentMode, get(isDark));
  });

  mq.addEventListener('change', (e) => {
    if (currentMode === 'system') {
      isDark.set(e.matches);
      applyThemeClass(currentMode, e.matches);
    }
  });

  detectSystemDark().then((dark) => {
    isDark.set(dark);
    applyThemeClass(currentMode, dark);
  });
}

export async function loadSystemColors(): Promise<Record<string, string>> {
  try {
    return await invoke<Record<string, string>>('get_system_colors');
  } catch {
    return {};
  }
}
