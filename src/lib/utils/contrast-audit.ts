/**
 * contrast-audit.ts
 * DEV-ONLY utility — not shipped to production.
 *
 * Provides:
 *   - OKLCH → sRGB conversion
 *   - Hex → sRGB conversion
 *   - Alpha compositing (effective opaque color)
 *   - WCAG 2.x relative luminance + contrast ratio
 *   - APCA-W3 simplified Lc value
 *   - Colorblind (grayscale) distinctness check
 */

// ── Types ─────────────────────────────────────────────────────────────

export interface RGB {
  r: number; // 0–1
  g: number; // 0–1
  b: number; // 0–1
}

export interface ContrastResult {
  wcag:   number;   // WCAG 2.x ratio (e.g. 7.34)
  apca:   number;   // APCA Lc absolute value
  normal: 'AAA' | 'AA' | 'FAIL';  // ≥7 AAA, ≥4.5 AA, else FAIL
  large:  'AAA' | 'AA' | 'FAIL';  // ≥4.5 AAA, ≥3 AA, else FAIL
}

// ── OKLCH → linear-light sRGB ─────────────────────────────────────────

/** Convert OKLCH (L 0–1, C 0–0.4, H degrees) to linear-light sRGB */
export function oklchToLinearRGB(L: number, C: number, H: number): RGB {
  const hRad = (H * Math.PI) / 180;
  const a = C * Math.cos(hRad);
  const b = C * Math.sin(hRad);

  // OKLab → XYZ (D65)
  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.2914855480 * b;

  const l = l_ * l_ * l_;
  const m = m_ * m_ * m_;
  const s = s_ * s_ * s_;

  // Linear sRGB
  return {
    r:  4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    g: -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    b: -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
  };
}

/** Clamp linear-light sRGB to [0,1] then apply sRGB transfer function */
function linearToSRGB(v: number): number {
  const c = Math.max(0, Math.min(1, v));
  return c <= 0.0031308 ? 12.92 * c : 1.055 * Math.pow(c, 1 / 2.4) - 0.055;
}

export function oklchToRGB(L: number, C: number, H: number): RGB {
  const lin = oklchToLinearRGB(L, C, H);
  return {
    r: linearToSRGB(lin.r),
    g: linearToSRGB(lin.g),
    b: linearToSRGB(lin.b),
  };
}

// ── Hex → RGB ─────────────────────────────────────────────────────────

export function hexToRGB(hex: string): RGB {
  const h = hex.replace('#', '');
  const full = h.length === 3
    ? h.split('').map(c => c + c).join('')
    : h;
  return {
    r: parseInt(full.slice(0, 2), 16) / 255,
    g: parseInt(full.slice(2, 4), 16) / 255,
    b: parseInt(full.slice(4, 6), 16) / 255,
  };
}

// ── Alpha compositing ─────────────────────────────────────────────────

/** Composite fg (with alpha) onto bg, returns opaque RGB */
export function alphaComposite(fg: RGB, alpha: number, bg: RGB): RGB {
  return {
    r: fg.r * alpha + bg.r * (1 - alpha),
    g: fg.g * alpha + bg.g * (1 - alpha),
    b: fg.b * alpha + bg.b * (1 - alpha),
  };
}

// ── WCAG 2.x luminance & contrast ────────────────────────────────────

function toLinear(c: number): number {
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

export function relativeLuminance(rgb: RGB): number {
  return 0.2126 * toLinear(rgb.r) + 0.7152 * toLinear(rgb.g) + 0.0722 * toLinear(rgb.b);
}

export function wcagContrast(text: RGB, bg: RGB): number {
  const L1 = relativeLuminance(text);
  const L2 = relativeLuminance(bg);
  const lighter = Math.max(L1, L2);
  const darker  = Math.min(L1, L2);
  return (lighter + 0.05) / (darker + 0.05);
}

// ── APCA-W3 (simplified Lc) ───────────────────────────────────────────
// Based on APCA-W3 0.0.98G-4g algorithm

const SAPCA = {
  mainTRC: 2.4,
  sRco: 0.2126729, sGco: 0.7151522, sBco: 0.0721750,
  normBG: 0.56, normTXT: 0.57, revTXT: 0.62, revBG: 0.65,
  scaleBoW: 1.14, scaleWoB: 1.14,
  loBoWoffset: 0.027, loWoBoffset: 0.027,
  loClip: 0.1, deltaYmin: 0.0005,
};

function apcaSRGBtoY(rgb: RGB): number {
  const r = Math.pow(Math.max(0, rgb.r), SAPCA.mainTRC);
  const g = Math.pow(Math.max(0, rgb.g), SAPCA.mainTRC);
  const b = Math.pow(Math.max(0, rgb.b), SAPCA.mainTRC);
  return SAPCA.sRco * r + SAPCA.sGco * g + SAPCA.sBco * b;
}

export function apcaContrast(textRGB: RGB, bgRGB: RGB): number {
  const Ytxt = apcaSRGBtoY(textRGB);
  const Ybg  = apcaSRGBtoY(bgRGB);

  if (Math.abs(Ybg - Ytxt) < SAPCA.deltaYmin) return 0;

  let Lc = 0;
  if (Ybg > Ytxt) {
    // Black on white (BoW)
    const Sapc = (Math.pow(Ybg, SAPCA.normBG) - Math.pow(Ytxt, SAPCA.normTXT)) * SAPCA.scaleBoW;
    Lc = Sapc < SAPCA.loClip ? 0 : Sapc - SAPCA.loBoWoffset;
  } else {
    // White on black (WoB)
    const Sapc = (Math.pow(Ybg, SAPCA.revBG) - Math.pow(Ytxt, SAPCA.revTXT)) * SAPCA.scaleWoB;
    Lc = Sapc > -SAPCA.loClip ? 0 : Sapc + SAPCA.loWoBoffset;
  }
  return Math.round(Math.abs(Lc) * 100);
}

// ── WCAG grade ────────────────────────────────────────────────────────

export function gradeWCAG(ratio: number): ContrastResult['normal'] {
  if (ratio >= 7)   return 'AAA';
  if (ratio >= 4.5) return 'AA';
  return 'FAIL';
}

export function gradeWCAGLarge(ratio: number): ContrastResult['large'] {
  if (ratio >= 4.5) return 'AAA';
  if (ratio >= 3)   return 'AA';
  return 'FAIL';
}

export function contrastResult(text: RGB, bg: RGB): ContrastResult {
  const wcag = wcagContrast(text, bg);
  const apca = apcaContrast(text, bg);
  return {
    wcag:   Math.round(wcag * 100) / 100,
    apca,
    normal: gradeWCAG(wcag),
    large:  gradeWCAGLarge(wcag),
  };
}

// ── Colorblind (grayscale) luminosity ────────────────────────────────

/** Return perceptual luminance 0–100 for a color (grayscale simulation) */
export function luminosity(rgb: RGB): number {
  return Math.round(relativeLuminance(rgb) * 100);
}

// ── Parse color string to RGB ─────────────────────────────────────────
// Handles: #rrggbb, oklch(L C H), oklch(L C H / alpha)

export function parseColor(str: string): RGB | null {
  str = str.trim();

  if (str.startsWith('#')) {
    return hexToRGB(str);
  }

  const oklchMatch = str.match(/^oklch\(\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*[\d.]+)?\s*\)$/);
  if (oklchMatch) {
    return oklchToRGB(
      parseFloat(oklchMatch[1]),
      parseFloat(oklchMatch[2]),
      parseFloat(oklchMatch[3]),
    );
  }

  return null;
}
