import { describe, it, expect } from 'vitest';
import { darken, lighten, suggestFamily } from '$lib/engine/color-harmony';

describe('color-harmony', () => {
  it('darkens a hex color by percentage', () => {
    const result = darken('#3b82f6', 0.15);
    expect(result).toMatch(/^#[0-9a-f]{6}$/);
  });

  it('lightens a hex color by percentage', () => {
    const result = lighten('#3b82f6', 0.15);
    expect(result).toMatch(/^#[0-9a-f]{6}$/);
  });

  it('suggests accent family from base color', () => {
    const suggestions = suggestFamily('#3b82f6', '#121212');
    expect(suggestions).toHaveProperty('--accent-text');
    expect(suggestions).toHaveProperty('--accent-hover');
    expect(suggestions).toHaveProperty('--accent-btn');
    Object.values(suggestions).forEach((v) => {
      expect(v).toMatch(/^#[0-9a-f]{6}$/);
    });
  });
});
