import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import elegantDarkRaw from '../../../elegant-dark.json';

// Mock theme-engine DOM functions
vi.mock('$lib/engine/theme-engine', async () => {
  const actual = await vi.importActual('$lib/engine/theme-engine');
  return {
    ...actual,
    injectStyles: vi.fn(),
    applyColorScheme: vi.fn(),
  };
});

// The Tauri APIs are aliased to tests/mocks/tauri-api.ts in vitest.config.
// To control invoke behavior per-test, we spy on the shared mock module.
import * as tauriApi from '@tauri-apps/api/core';

import {
  activeThemeName,
  activeTheme,
  activeModifierNames,
  colorScheme,
  isDark,
  loadBuiltinTheme,
  toggleModifier,
} from '$lib/stores/theme';

// elegant-dark.json loaded via Vite JSON import
const elegantDarkJson = JSON.stringify(elegantDarkRaw);

function resetInvoke() {
  vi.spyOn(tauriApi, 'invoke').mockRejectedValue(new Error('Tauri not available'));
}

describe('theme store — basics', () => {
  beforeEach(() => {
    activeThemeName.set('reasonance-dark');
    activeModifierNames.set([]);
    resetInvoke();
  });

  it('defaults to reasonance-dark', () => {
    expect(get(activeThemeName)).toBe('reasonance-dark');
  });

  it('defaults to dark colorScheme', () => {
    expect(get(colorScheme)).toBe('dark');
  });

  it('isDark is derived from colorScheme', () => {
    expect(get(isDark)).toBe(true);
    colorScheme.set('light');
    expect(get(isDark)).toBe(false);
  });
});

describe('theme store — built-in themes', () => {
  beforeEach(() => {
    activeThemeName.set('reasonance-dark');
    activeModifierNames.set([]);
    resetInvoke();
  });

  it('loads a built-in theme by name', async () => {
    await loadBuiltinTheme('reasonance-light');
    expect(get(activeThemeName)).toBe('reasonance-light');
    expect(get(colorScheme)).toBe('light');
  });

  it('loads dark theme and sets colorScheme to dark', async () => {
    await loadBuiltinTheme('reasonance-dark');
    expect(get(activeThemeName)).toBe('reasonance-dark');
    expect(get(colorScheme)).toBe('dark');
  });

  it('switching themes updates activeTheme object', async () => {
    await loadBuiltinTheme('reasonance-light');
    const theme = get(activeTheme);
    expect(theme.meta.name).toBe('Reasonance Light');
    expect(theme.meta.colorScheme).toBe('light');
  });
});

describe('theme store — user themes from disk', () => {
  beforeEach(async () => {
    resetInvoke();
    await loadBuiltinTheme('reasonance-dark');
    activeModifierNames.set([]);
  });

  it('loads a user theme from disk when not built-in', async () => {
    vi.spyOn(tauriApi, 'invoke').mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === 'load_user_theme' && args?.name === 'elegant-dark') {
        return elegantDarkJson as any;
      }
      throw new Error('Not found');
    });

    await loadBuiltinTheme('elegant-dark');
    expect(get(activeThemeName)).toBe('elegant-dark');
    expect(get(activeTheme).meta.name).toBe('Elegant Dark');
    expect(get(colorScheme)).toBe('dark');
  });

  it('user theme sets correct colorScheme', async () => {
    vi.spyOn(tauriApi, 'invoke').mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === 'load_user_theme' && args?.name === 'elegant-dark') {
        return elegantDarkJson as any;
      }
      throw new Error('Not found');
    });

    await loadBuiltinTheme('elegant-dark');
    expect(get(isDark)).toBe(true);
  });

  it('user theme variables are accessible', async () => {
    vi.spyOn(tauriApi, 'invoke').mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === 'load_user_theme' && args?.name === 'elegant-dark') {
        return elegantDarkJson as any;
      }
      throw new Error('Not found');
    });

    await loadBuiltinTheme('elegant-dark');
    const theme = get(activeTheme);
    expect(theme.colors?.['--bg-primary']).toBe('#1b1d23');
    expect(theme.colors?.['--accent']).toBe('#7c8af5');
    expect(theme.borders?.['--radius']).toBe('8px');
  });

  it('falls back when user theme is invalid JSON', async () => {
    vi.spyOn(tauriApi, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'load_user_theme') return '{ broken json' as any;
      throw new Error('Not found');
    });

    await loadBuiltinTheme('broken-theme');
    expect(get(activeThemeName)).toBe('reasonance-dark');
  });

  it('falls back when user theme fails validation', async () => {
    vi.spyOn(tauriApi, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'load_user_theme') {
        return JSON.stringify({ meta: { name: 'Bad', type: 'theme', schemaVersion: 999 } }) as any;
      }
      throw new Error('Not found');
    });

    await loadBuiltinTheme('bad-theme');
    expect(get(activeThemeName)).toBe('reasonance-dark');
  });

  it('falls back when theme not found anywhere', async () => {
    await loadBuiltinTheme('nonexistent-theme');
    expect(get(activeThemeName)).toBe('reasonance-dark');
  });
});

describe('theme store — modifiers', () => {
  beforeEach(() => {
    activeThemeName.set('reasonance-dark');
    activeModifierNames.set([]);
    resetInvoke();
  });

  it('toggles a modifier on', async () => {
    await toggleModifier('enhanced-readability');
    expect(get(activeModifierNames)).toContain('enhanced-readability');
  });

  it('toggles a modifier off', async () => {
    activeModifierNames.set(['enhanced-readability']);
    await toggleModifier('enhanced-readability');
    expect(get(activeModifierNames)).not.toContain('enhanced-readability');
  });
});

describe('theme store — validator integration', () => {
  it('rejects a theme with missing sections via disk load', async () => {
    vi.spyOn(tauriApi, 'invoke').mockImplementation(async (cmd: string) => {
      if (cmd === 'load_user_theme') {
        return JSON.stringify({
          meta: { name: 'Incomplete', type: 'theme', colorScheme: 'dark', schemaVersion: 1 },
          colors: { '--bg-primary': '#000' },
          // missing 10 other required sections
        }) as any;
      }
      throw new Error('Not found');
    });

    await loadBuiltinTheme('incomplete-theme');
    expect(get(activeThemeName)).toBe('reasonance-dark');
  });
});
