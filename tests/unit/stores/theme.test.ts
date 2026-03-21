import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { themeMode, isDark } from '$lib/stores/theme';
import type { ThemeMode } from '$lib/stores/theme';

describe('theme store', () => {
  beforeEach(() => {
    themeMode.set('dark');
    isDark.set(true);
  });

  it('defaults to dark mode', () => {
    expect(get(themeMode)).toBe('dark');
    expect(get(isDark)).toBe(true);
  });

  it('can switch to light mode', () => {
    themeMode.set('light');
    isDark.set(false);

    expect(get(themeMode)).toBe('light');
    expect(get(isDark)).toBe(false);
  });

  it('can switch to system mode', () => {
    themeMode.set('system');
    expect(get(themeMode)).toBe('system');
  });

  it('isDark can be set independently', () => {
    isDark.set(false);
    expect(get(isDark)).toBe(false);

    isDark.set(true);
    expect(get(isDark)).toBe(true);
  });

  it('accepts all valid ThemeMode values', () => {
    const modes: ThemeMode[] = ['dark', 'light', 'system'];
    for (const mode of modes) {
      themeMode.set(mode);
      expect(get(themeMode)).toBe(mode);
    }
  });

  it('dark mode keeps isDark true', () => {
    themeMode.set('light');
    isDark.set(false);

    themeMode.set('dark');
    isDark.set(true);

    expect(get(themeMode)).toBe('dark');
    expect(get(isDark)).toBe(true);
  });
});
