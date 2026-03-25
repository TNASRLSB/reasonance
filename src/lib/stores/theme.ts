import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { ThemeFile, ThemePreferences } from '$lib/engine/theme-types';
import { extractVariables, mergeModifier, buildCssString, injectStyles, applyColorScheme } from '$lib/engine/theme-engine';
import { validateTheme } from '$lib/engine/theme-validator';
import { FALLBACK_THEME } from '$lib/engine/fallback-theme';
import { editorTheme } from '$lib/stores/ui';
import { appAnnouncer } from '$lib/utils/a11y-announcer';

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

let lastAppliedCss = '';
let suppressThemeChanged = false;
let themeChangedTimer: ReturnType<typeof setTimeout> | null = null;
let loadGeneration = 0;

function reapply(): void {
  const theme = get(activeTheme);
  const mods = get(activeModifiers);
  const cs = theme.meta.colorScheme ?? 'dark';

  let vars = extractVariables(theme);
  for (const mod of mods) {
    vars = mergeModifier(vars, mod, cs);
  }

  const css = buildCssString(vars);

  // Skip if CSS hasn't changed (prevents infinite loop from HMR/subscriptions)
  if (css === lastAppliedCss) return;
  lastAppliedCss = css;

  console.info(`[Theme] reapply: "${theme.meta.name}", scheme=${cs}, vars=${Object.keys(vars).length}, css=${css.length} chars`);
  injectStyles('reasonance-theme', css);
  applyColorScheme(cs);
  colorScheme.set(cs);

  if (initialized) {
    appAnnouncer.announce(`Theme changed to ${theme.meta.name}`);
  }

  // Auto-select editor theme from app theme metadata
  const editorThemeName = theme.meta.editorTheme;
  if (editorThemeName) {
    editorTheme.set(editorThemeName);
  }
}

export async function loadBuiltinTheme(name: string): Promise<void> {
  // Cancel any pending debounced reload from theme://changed
  if (themeChangedTimer) {
    clearTimeout(themeChangedTimer);
    themeChangedTimer = null;
  }
  const gen = ++loadGeneration;

  // Try built-in first
  const loader = builtinThemes[name];
  if (loader) {
    const theme = await loader();
    if (gen !== loadGeneration) return;
    const validation = validateTheme(theme);
    if (validation.valid) {
      activeTheme.set(theme);
      activeThemeName.set(name);
      reapply();
      return;
    }
    console.error(`Invalid built-in theme ${name}:`, validation.errors);
  }

  // Try user theme from disk
  try {
    const json = await invoke<string>('load_user_theme', { name });
    if (gen !== loadGeneration) return;
    const theme = JSON.parse(json) as ThemeFile;
    const validation = validateTheme(theme);
    if (validation.valid) {
      activeTheme.set(theme);
      activeThemeName.set(name);
      reapply();
      return;
    }
    console.error(`Invalid user theme ${name}:`, validation.errors);
  } catch (e) {
    console.warn(`User theme "${name}" not found on disk:`, e);
  }

  // Fallback — use reasonance-dark as default, not 'fallback'
  console.error(`Theme not found: ${name}, falling back to reasonance-dark`);
  const fallbackLoader = builtinThemes['reasonance-dark'];
  if (fallbackLoader) {
    try {
      const fallbackTheme = await fallbackLoader();
      if (gen !== loadGeneration) return;
      activeTheme.set(fallbackTheme);
      activeThemeName.set('reasonance-dark');
      reapply();
      return;
    } catch {
      // Even built-in failed — use hardcoded emergency theme
    }
  }
  activeTheme.set(FALLBACK_THEME);
  activeThemeName.set('reasonance-dark');
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

/** Suppress theme://changed events during import to prevent ping-pong. */
export function suppressFileWatcher(suppress: boolean): void {
  suppressThemeChanged = suppress;
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
    const themeName = prefs.activeTheme === 'fallback' ? 'reasonance-dark' : prefs.activeTheme;

    // Try user theme first, then built-in
    try {
      const json = await invoke<string>('load_user_theme', { name: themeName });
      const theme = JSON.parse(json) as ThemeFile;
      const validation = validateTheme(theme);
      if (validation.valid) {
        activeTheme.set(theme);
        activeThemeName.set(themeName);
      } else {
        await loadBuiltinTheme(themeName);
      }
    } catch {
      await loadBuiltinTheme(themeName);
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

  // Hot reload on file changes (debounced to avoid spurious reloads)
  try {
    await listen('theme://changed', () => {
      console.info(`[Theme] theme://changed event fired, suppress=${suppressThemeChanged}, active="${get(activeThemeName)}"`);
      if (suppressThemeChanged) return;
      if (themeChangedTimer) clearTimeout(themeChangedTimer);
      // Only reload user themes (built-in themes don't live on disk)
      const name = get(activeThemeName);
      if (builtinThemes[name]) return;
      themeChangedTimer = setTimeout(() => {
        themeChangedTimer = null;
        loadBuiltinTheme(name);
      }, 300);
    });
  } catch {
    // listen not available outside Tauri
  }

  initialized = true;
}
