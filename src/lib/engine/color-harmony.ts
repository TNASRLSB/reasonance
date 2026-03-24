import { hexToRgb, contrastRatio } from './contrast-checker';

function rgbToHex(r: number, g: number, b: number): string {
  const clamp = (v: number) => Math.max(0, Math.min(255, Math.round(v)));
  return '#' + [clamp(r), clamp(g), clamp(b)].map((v) => v.toString(16).padStart(2, '0')).join('');
}

export function darken(hex: string, amount: number): string {
  const { r, g, b } = hexToRgb(hex);
  const factor = 1 - amount;
  return rgbToHex(r * factor, g * factor, b * factor);
}

export function lighten(hex: string, amount: number): string {
  const { r, g, b } = hexToRgb(hex);
  return rgbToHex(r + (255 - r) * amount, g + (255 - g) * amount, b + (255 - b) * amount);
}

function findAaaVariant(base: string, background: string, direction: 'lighten' | 'darken'): string {
  let best = base;
  for (let i = 1; i <= 20; i++) {
    const candidate = direction === 'lighten' ? lighten(base, i * 0.05) : darken(base, i * 0.05);
    if (contrastRatio(candidate, background) >= 7) {
      return candidate;
    }
    best = candidate;
  }
  return best;
}

export function suggestFamily(
  baseAccent: string,
  background: string
): Record<string, string> {
  return {
    '--accent-text': findAaaVariant(baseAccent, background, 'lighten'),
    '--accent-hover': darken(baseAccent, 0.15),
    '--accent-btn': darken(baseAccent, 0.40),
    '--accent-statusbar': darken(baseAccent, 0.40),
  };
}
