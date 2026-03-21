import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export type ThemeMode = 'light' | 'dark' | 'system';
export const themeMode = writable<ThemeMode>('system');

// Default to dark — WebKitGTK on KDE does not support prefers-color-scheme.
// The async detectSystemDark() will correct if the system is actually light.
export const isDark = writable(true);

export function initTheme() {
  const mq = window.matchMedia('(prefers-color-scheme: dark)');

  async function detectSystemDark(): Promise<boolean> {
    // First try Rust-side KDE detection (reads kdeglobals, computes luminance)
    try {
      const colors = await invoke<Record<string, string>>('get_system_colors');
      if (colors.is_dark === 'true') return true;
      if (colors.is_dark === 'false') return false;
    } catch { /* ignore */ }
    // Fallback to media query (works on GTK themes that set it)
    if (mq.matches !== undefined) return mq.matches;
    // Ultimate fallback: dark
    return true;
  }

  function applyTheme(dark: boolean) {
    document.documentElement.classList.toggle('light', !dark);
    isDark.set(dark);
  }

  function resolveTheme(mode: ThemeMode, systemIsDark: boolean): boolean {
    if (mode === 'dark') return true;
    if (mode === 'light') return false;
    return systemIsDark; // 'system'
  }

  let currentMode: ThemeMode = 'system';
  let systemDark = true; // assume dark until detection completes

  themeMode.subscribe((mode) => {
    currentMode = mode;
    applyTheme(resolveTheme(currentMode, systemDark));
  });

  mq.addEventListener('change', (e) => {
    systemDark = e.matches;
    if (currentMode === 'system') {
      applyTheme(resolveTheme(currentMode, systemDark));
    }
  });

  // Async detection — corrects the default if system is actually light
  detectSystemDark().then((dark) => {
    systemDark = dark;
    applyTheme(resolveTheme(currentMode, systemDark));
  });
}

export async function loadSystemColors(): Promise<Record<string, string>> {
  try {
    return await invoke<Record<string, string>>('get_system_colors');
  } catch {
    return {};
  }
}
