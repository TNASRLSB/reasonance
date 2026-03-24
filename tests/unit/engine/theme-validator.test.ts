import { describe, it, expect } from 'vitest';
import { validateTheme, type ValidationResult } from '$lib/engine/theme-validator';
import darkTheme from '$lib/themes/reasonance-dark.json';
import lightTheme from '$lib/themes/reasonance-light.json';
import enhancedReadability from '$lib/themes/enhanced-readability.json';

describe('theme-validator', () => {
  it('accepts valid dark theme', () => {
    const result = validateTheme(darkTheme);
    expect(result.valid).toBe(true);
    expect(result.errors).toEqual([]);
  });

  it('accepts valid light theme', () => {
    const result = validateTheme(lightTheme);
    expect(result.valid).toBe(true);
  });

  it('accepts valid modifier', () => {
    const result = validateTheme(enhancedReadability);
    expect(result.valid).toBe(true);
  });

  it('rejects theme missing meta', () => {
    const result = validateTheme({ colors: {} });
    expect(result.valid).toBe(false);
    expect(result.errors[0]).toContain('meta');
  });

  it('rejects theme with missing required section', () => {
    const incomplete = { ...darkTheme };
    delete (incomplete as any).colors;
    const result = validateTheme(incomplete);
    expect(result.valid).toBe(false);
  });

  it('rejects modifier with zero sections', () => {
    const empty = { meta: { name: 'Empty', type: 'modifier', schemaVersion: 1 } };
    const result = validateTheme(empty);
    expect(result.valid).toBe(false);
  });

  it('rejects unknown schemaVersion', () => {
    const future = { ...darkTheme, meta: { ...darkTheme.meta, schemaVersion: 999 } };
    const result = validateTheme(future);
    expect(result.valid).toBe(false);
  });
});
