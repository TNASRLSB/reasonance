import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';

// Mock theme-engine DOM functions
vi.mock('$lib/engine/theme-engine', async () => {
  const actual = await vi.importActual('$lib/engine/theme-engine');
  return {
    ...actual,
    injectStyles: vi.fn(),
    applyColorScheme: vi.fn(),
  };
});

import {
  activeThemeName,
  activeModifierNames,
  colorScheme,
  isDark,
  loadBuiltinTheme,
  toggleModifier,
} from '$lib/stores/theme';

describe('theme store', () => {
  beforeEach(() => {
    activeThemeName.set('reasonance-dark');
    activeModifierNames.set([]);
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

  it('loads a built-in theme by name', async () => {
    await loadBuiltinTheme('reasonance-light');
    expect(get(activeThemeName)).toBe('reasonance-light');
    expect(get(colorScheme)).toBe('light');
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
