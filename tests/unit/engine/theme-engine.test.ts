import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  extractVariables,
  mergeModifier,
  buildCssString,
} from '$lib/engine/theme-engine';
import darkTheme from '$lib/themes/reasonance-dark.json';
import enhancedReadability from '$lib/themes/enhanced-readability.json';
import type { ThemeFile } from '$lib/engine/theme-types';

describe('extractVariables', () => {
  it('extracts all CSS variables from a theme, skipping meta', () => {
    const vars = extractVariables(darkTheme as ThemeFile);
    expect(vars['--bg-primary']).toBe('#121212');
    expect(vars['--accent']).toBe('#8ab8ff');
    expect(vars['--layer-toast']).toBe(5000);
    expect(vars['meta']).toBeUndefined();
  });

  it('handles number values', () => {
    const vars = extractVariables(darkTheme as ThemeFile);
    expect(vars['--hue-accent']).toBe(220);
    expect(vars['--line-height-base']).toBe(1.5);
  });
});

describe('mergeModifier', () => {
  it('overrides base variables with modifier values', () => {
    const base = extractVariables(darkTheme as ThemeFile);
    const merged = mergeModifier(base, enhancedReadability as ThemeFile, 'dark');
    expect(merged['--font-size-base']).toBe('1.125rem');
    expect(merged['--border-width']).toBe('3px');
  });

  it('preserves base variables not in modifier', () => {
    const base = extractVariables(darkTheme as ThemeFile);
    const merged = mergeModifier(base, enhancedReadability as ThemeFile, 'dark');
    expect(merged['--bg-primary']).toBe('#121212');
    expect(merged['--accent']).toBe('#8ab8ff');
  });

  it('applies when-dark conditionals for dark colorScheme', () => {
    const base = extractVariables(darkTheme as ThemeFile);
    const merged = mergeModifier(base, enhancedReadability as ThemeFile, 'dark');
    expect(merged['--font-weight-body']).toBe(500);
  });

  it('applies when-light conditionals for light colorScheme', () => {
    const base = extractVariables(darkTheme as ThemeFile);
    const merged = mergeModifier(base, enhancedReadability as ThemeFile, 'light');
    expect(merged['--font-weight-body']).toBe(500);
  });
});

describe('buildCssString', () => {
  it('builds valid CSS :root block', () => {
    const vars = { '--bg-primary': '#121212', '--accent': '#3b82f6' };
    const css = buildCssString(vars);
    expect(css).toContain(':root {');
    expect(css).toContain('--bg-primary: #121212;');
    expect(css).toContain('--accent: #3b82f6;');
    expect(css).toContain('}');
  });

  it('converts numbers to strings', () => {
    const vars = { '--hue-accent': 220, '--line-height': 1.5 };
    const css = buildCssString(vars);
    expect(css).toContain('--hue-accent: 220;');
    expect(css).toContain('--line-height: 1.5;');
  });
});
