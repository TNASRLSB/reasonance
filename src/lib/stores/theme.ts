import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { ThemeFile, ThemePreferences } from '$lib/engine/theme-types';
import { extractVariables, mergeModifier, buildCssString, injectStyles, applyColorScheme } from '$lib/engine/theme-engine';
import { validateTheme } from '$lib/engine/theme-validator';
import { FALLBACK_THEME } from '$lib/engine/fallback-theme';
import { editorTheme } from '$lib/stores/ui';

// --- Stores ---
export const activeThemeName = writable<string>('reasonance-dark');
export const activeTheme = writable<ThemeFile>(FALLBACK_THEME);
export const activeModifierNames = writable<string[]>([]);
export const activeModifiers = writable<ThemeFile[]>([]);
export const colorScheme = writable<'dark' | 'light'>('dark');

/** Backward compatibility — derived from colorScheme */
export const isDark = derived(colorScheme, ($cs) => $cs === 'dark');

// --- Backward compatibility exports ---
/** @deprecated Use activeThemeName + loadBuiltinTheme instead */
export type ThemeMode = 'light' | 'dark' | 'system';
/** @deprecated Use activeThemeName + loadBuiltinTheme instead */
export const themeMode = writable<ThemeMode>('dark');

// --- Built-in theme registry ---
const builtinThemes: Record<string, () => Promise<ThemeFile>> = {
  'reasonance-dark': () => import('$lib/themes/reasonance-dark.json').then((m) => m.default as ThemeFile),
  'reasonance-light': () => import('$lib/themes/reasonance-light.json').then((m) => m.default as ThemeFile),
  'elegant-dark': () => import('$lib/themes/elegant-dark.json').then((m) => m.default as ThemeFile),
};

const builtinModifiers: Record<string, () => Promise<ThemeFile>> = {
  'enhanced-readability': () => import('$lib/themes/enhanced-readability.json').then((m) => m.default as ThemeFile),
  '_high-contrast': () => import('$lib/themes/_high-contrast.json').then((m) => m.default as ThemeFile),
  '_reduced-motion': () => import('$lib/themes/_reduced-motion.json').then((m) => m.default as ThemeFile),
};

// --- Persistence ---

let initialized = false;

activeThemeName.subscribe(() => { if (initialized) savePreferences(); });
activeModifierNames.subscribe(() => { if (initialized) savePreferences(); });

async function savePreferences(): Promise<void> {
  try {
    await invoke('save_theme_preferences', {
      prefs: {
        activeTheme: get(activeThemeName),
        activeModifiers: get(activeModifierNames),
      },
    });
  } catch (e) {
    console.warn('Failed to save theme preferences:', e);
  }
}

// --- Actions ---

function reapply(): void {
  const theme = get(activeTheme);
  const mods = get(activeModifiers);
  const cs = theme.meta.colorScheme ?? 'dark';

  let vars = extractVariables(theme);
  for (const mod of mods) {
    vars = mergeModifier(vars, mod, cs);
  }

  injectStyles('reasonance-theme', buildCssString(vars));
  applyColorScheme(cs);
  colorScheme.set(cs);

  // Auto-select editor theme from app theme metadata
  const editorThemeName = theme.meta.editorTheme;
  if (editorThemeName) {
    editorTheme.set(editorThemeName);
  }
}

export async function loadBuiltinTheme(name: string): Promise<void> {
  const loader = builtinThemes[name];
  if (!loader) {
    console.error(`Theme not found: ${name}, falling back`);
    activeTheme.set(FALLBACK_THEME);
    activeThemeName.set('fallback');
    reapply();
    return;
  }

  const theme = await loader();
  const validation = validateTheme(theme);
  if (!validation.valid) {
    console.error(`Invalid theme ${name}:`, validation.errors);
    activeTheme.set(FALLBACK_THEME);
    activeThemeName.set('fallback');
    reapply();
    return;
  }

  activeTheme.set(theme);
  activeThemeName.set(name);
  reapply();
}

export async function toggleModifier(name: string): Promise<void> {
  const current = get(activeModifierNames);
  if (current.includes(name)) {
    activeModifierNames.set(current.filter((n) => n !== name));
    activeModifiers.update((mods) => mods.filter((m) => (m as any)._registryKey !== name));
    reapply();
  } else {
    activeModifierNames.set([...current, name]);
    await loadModifierByName(name);
    reapply();
  }
}

async function loadModifierByName(name: string): Promise<void> {
  const loader = builtinModifiers[name];
  if (!loader) {
    console.warn(`Modifier not found: ${name}`);
    return;
  }
  const mod = await loader();
  (mod as any)._registryKey = name;
  activeModifiers.update((mods) => [...mods.filter((m) => (m as any)._registryKey !== name), mod]);
}

export function setPreviewVariable(name: string, value: string): void {
  document.documentElement.style.setProperty(name, value);
}

function setupSystemModifiers(): void {
  const contrastQuery = window.matchMedia('(prefers-contrast: more)');
  const motionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');

  const handleSystemModifier = async (key: string, active: boolean) => {
    if (active) {
      const loader = builtinModifiers[key];
      if (loader) {
        const mod = await loader();
        (mod as any)._registryKey = key;
        activeModifiers.update((mods) => [...mods.filter((m) => (m as any)._registryKey !== key), mod]);
        reapply();
      }
    } else {
      activeModifiers.update((mods) => mods.filter((m) => (m as any)._registryKey !== key));
      reapply();
    }
  };

  const handleContrast = (e: MediaQueryListEvent | MediaQueryList) => {
    handleSystemModifier('_high-contrast', e.matches);
  };
  const handleMotion = (e: MediaQueryListEvent | MediaQueryList) => {
    handleSystemModifier('_reduced-motion', e.matches);
  };

  handleContrast(contrastQuery);
  handleMotion(motionQuery);
  contrastQuery.addEventListener('change', handleContrast);
  motionQuery.addEventListener('change', handleMotion);
}

/** @deprecated Use initThemeEngine instead */
export function initTheme(): void {
  initThemeEngine();
}

export async function initThemeEngine(): Promise<void> {
  try {
    const prefs = await invoke<ThemePreferences>('load_theme_preferences');

    // Try user theme first, then built-in
    try {
      const json = await invoke<string>('load_user_theme', { name: prefs.activeTheme });
      const theme = JSON.parse(json) as ThemeFile;
      const validation = validateTheme(theme);
      if (validation.valid) {
        activeTheme.set(theme);
        activeThemeName.set(prefs.activeTheme);
      } else {
        await loadBuiltinTheme(prefs.activeTheme);
      }
    } catch {
      await loadBuiltinTheme(prefs.activeTheme);
    }

    // Load saved modifiers
    for (const name of prefs.activeModifiers) {
      await loadModifierByName(name);
    }

    reapply();
  } catch {
    // Tauri not available or preferences error — use defaults
    await loadBuiltinTheme('reasonance-dark');
  }

  setupSystemModifiers();

  // Hot reload on file changes
  try {
    await listen('theme://changed', () => {
      const name = get(activeThemeName);
      loadBuiltinTheme(name);
    });
  } catch {
    // listen not available outside Tauri
  }

  initialized = true;
}
