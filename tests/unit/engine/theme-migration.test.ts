import { describe, it, expect } from 'vitest';
import { extractVariables } from '$lib/engine/theme-engine';
import darkTheme from '$lib/themes/reasonance-dark.json';
import lightTheme from '$lib/themes/reasonance-light.json';
import type { ThemeFile } from '$lib/engine/theme-types';

describe('migration verification', () => {
  it('dark theme has all expected variables', () => {
    const vars = extractVariables(darkTheme as ThemeFile);
    // Core colors
    expect(vars['--bg-primary']).toBe('#121212');
    expect(vars['--accent']).toBe('#8ab8ff');
    expect(vars['--text-body']).toBe('#d4d4d4');
    // Typography
    expect(vars['--font-size-base']).toBe('1rem');
    expect(vars['--font-weight-body']).toBe(500);
    // Spacing
    expect(vars['--space-4']).toBe('1rem');
    // Borders
    expect(vars['--border-width']).toBe('2px');
    // Layout
    expect(vars['--toolbar-height']).toBe('52px');
    // Layers
    expect(vars['--layer-modal']).toBe(2000);
    // Transitions
    expect(vars['--transition-fast']).toBe('0.1s ease');
  });

  it('light theme has all expected overrides', () => {
    const vars = extractVariables(lightTheme as ThemeFile);
    expect(vars['--bg-primary']).toBe('#fafafa');
    expect(vars['--text-primary']).toBe('#0a0a0a');
    expect(vars['--font-weight-body']).toBe(400);
  });

  it('dark and light themes have same variable count', () => {
    const darkVars = Object.keys(extractVariables(darkTheme as ThemeFile));
    const lightVars = Object.keys(extractVariables(lightTheme as ThemeFile));
    expect(darkVars.length).toBe(lightVars.length);
    expect(darkVars.sort()).toEqual(lightVars.sort());
  });
});
