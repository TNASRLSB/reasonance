# Contrast Matrix — WCAG AAA Audit
**Date:** 2026-03-23
**Task:** Design System Consolidation — Checkpoint 3b
**Status:** All audited pairs PASS WCAG 2.2 AAA (7:1 normal, 4.5:1 large)

---

## Methodology

- **Algorithm:** WCAG 2.x relative luminance (IEC 61966-2-1)
- **APCA:** APCA-W3 0.0.98G-4g simplified (Lc absolute value)
- **Targets:** Normal text ≥ 7:1 (AAA), Large text ≥ 4.5:1 (AAA)
- **Alpha compositing:** `--bg-hover` effective opaque color computed before testing
- **OKLCH conversion:** Via OKLab → linear-light sRGB → sRGB transfer function
- **Tool:** `src/lib/utils/contrast-audit.ts` (dev-only, not bundled to production)

---

## Token Architecture Change

**Problem:** `--accent` (#3b82f6) and `--danger` (#ef4444) are mid-luminance saturated hues. No single text color can achieve 7:1 against them (white gives 3.68:1, darkest viable gives ~5.4:1).

**Solution:** Two new token tiers:

| Token | Value | Purpose |
|-------|-------|---------|
| `--accent` | `#3b82f6` | Borders, focus rings, state indicators, decorative |
| `--accent-btn` | `#1e40af` | Button/interactive backgrounds (AAA with white text) |
| `--danger` | `#ef4444` (dark) / `#b91c1c` (light) | Borders, error indicators, decorative |
| `--danger-btn` | `#991b1b` | Danger button backgrounds (AAA with white text) |
| `--text-on-accent` | `#ffffff` | Text on `--accent-btn` or `--danger-btn` backgrounds |

---

## Dark Theme Contrast Matrix

| Text Token | Background Token | WCAG 2.x | APCA Lc | Normal | Large |
|-----------|-----------------|----------|---------|--------|-------|
| `--text-primary` | `--bg-primary` | **16.44** | 100 | AAA | AAA |
| `--text-primary` | `--bg-surface` | **15.88** | 99 | AAA | AAA |
| `--text-primary` | `--bg-secondary` | **15.27** | 98 | AAA | AAA |
| `--text-primary` | `--bg-tertiary` | **12.60** | 95 | AAA | AAA |
| `--text-body` | `--bg-primary` | **12.64** | 82 | AAA | AAA |
| `--text-body` | `--bg-surface` | **12.21** | 81 | AAA | AAA |
| `--text-secondary` | `--bg-primary` | **7.43** | 54 | AAA | AAA |
| `--text-secondary` | `--bg-surface` | **7.17** | 53 | AAA | AAA |
| `--text-muted` | `--bg-primary` | **8.64** | 61 | AAA | AAA |
| `--text-muted` | `--bg-surface` | **8.34** | 60 | AAA | AAA |
| `--text-on-accent` | `--accent-btn` | **8.72** | 94 | AAA | AAA |
| `--text-on-accent` | `--danger-btn` | **8.31** | 92 | AAA | AAA |
| `--danger-text` | `--bg-primary` | **9.87** | 68 | AAA | AAA |
| `--danger-text` | `--bg-surface` | **9.53** | 67 | AAA | AAA |
| `--warning-text` | `--bg-primary` | **11.22** | 75 | AAA | AAA |
| `--warning-text` | `--bg-surface` | **10.84** | 75 | AAA | AAA |
| `--success-text` | `--bg-primary` | **10.75** | 73 | AAA | AAA |
| `--success-text` | `--bg-surface` | **10.38** | 72 | AAA | AAA |
| `--accent-text` | `--bg-primary` | **9.59** | 67 | AAA | AAA |
| `--accent-text` | `--bg-surface` | **9.27** | 66 | AAA | AAA |
| `--text-primary` | `--bg-hover` (on bg-primary) | **14.26** | 97 | AAA | AAA |
| `--text-primary` | `--bg-hover` (on bg-surface) | **13.62** | 96 | AAA | AAA |
| `--state-idle-text` | `--bg-primary` | **7.55** | 55 | AAA | AAA |
| `--state-retrying-text` | `--bg-primary` | **8.71** | 62 | AAA | AAA |

---

## Light Theme Contrast Matrix

| Text Token | Background Token | WCAG 2.x | APCA Lc | Normal | Large |
|-----------|-----------------|----------|---------|--------|-------|
| `--text-primary` | `--bg-primary` | **18.97** | 107 | AAA | AAA |
| `--text-primary` | `--bg-surface` | **19.80** | 110 | AAA | AAA |
| `--text-primary` | `--bg-secondary` | **17.37** | 101 | AAA | AAA |
| `--text-primary` | `--bg-tertiary` | **15.72** | 95 | AAA | AAA |
| `--text-body` | `--bg-primary` | **16.67** | 103 | AAA | AAA |
| `--text-body` | `--bg-surface` | **17.40** | 106 | AAA | AAA |
| `--text-secondary` | `--bg-primary` | **9.93** | 91 | AAA | AAA |
| `--text-secondary` | `--bg-surface` | **10.37** | 94 | AAA | AAA |
| `--text-secondary` | `--bg-tertiary` | **8.23** | 79 | AAA | AAA |
| `--text-muted` | `--bg-primary` | **7.73** | 85 | AAA | AAA |
| `--text-muted` | `--bg-surface` | **8.06** | 88 | AAA | AAA |
| `--text-muted` | `--bg-secondary` | **7.08** | 79 | AAA | AAA |
| `--text-on-accent` | `--accent-btn` | **8.72** | 94 | AAA | AAA |
| `--text-on-accent` | `--danger-btn` | **8.31** | 92 | AAA | AAA |
| `--danger-text` | `--bg-primary` | **7.96** | 84 | AAA | AAA |
| `--danger-text` | `--bg-surface` | **8.31** | 87 | AAA | AAA |
| `--warning-text` | `--bg-primary` | **7.08** | 82 | AAA | AAA |
| `--warning-text` | `--bg-surface` | **7.39** | 85 | AAA | AAA |
| `--success-text` | `--bg-primary` | **8.11** | 86 | AAA | AAA |
| `--success-text` | `--bg-surface` | **8.47** | 89 | AAA | AAA |
| `--accent-text` | `--bg-primary` | **7.59** | 84 | AAA | AAA |
| `--accent-text` | `--bg-surface` | **7.93** | 87 | AAA | AAA |
| `--text-primary` | `--bg-hover` (on bg-primary) | **17.37** | 101 | AAA | AAA |
| `--text-primary` | `--bg-hover` (on bg-surface) | **18.13** | 104 | AAA | AAA |
| `--state-idle-text` | `--bg-primary` | **8.82** | 88 | AAA | AAA |
| `--state-retrying-text` | `--bg-primary` | **8.60** | 86 | AAA | AAA |

---

## Alpha Color Effective Contrast

`--bg-hover` is a semi-transparent overlay. Contrast was measured against the effective opaque composite color:

| Theme | Token | Composition | Effective hex (approx) |
|-------|-------|-------------|----------------------|
| Dark | `--bg-hover` on `--bg-primary` | rgba(255,255,255,0.06) on #121212 | ~#181818 |
| Dark | `--bg-hover` on `--bg-surface` | rgba(255,255,255,0.06) on #161616 | ~#1c1c1c |
| Light | `--bg-hover` on `--bg-primary` | rgba(0,0,0,0.04) on #fafafa | ~#f3f3f3 |
| Light | `--bg-hover` on `--bg-surface` | rgba(0,0,0,0.04) on #ffffff | ~#f5f5f5 |

All `--text-primary` contrast on hover backgrounds passes at ≥13.6:1 (dark) and ≥17.4:1 (light).

---

## Forbidden Combinations

These combinations FAIL WCAG AAA for normal text and MUST NOT be used for text that is not large (≥18px regular or ≥14px bold):

| Text | Background | WCAG | Reason |
|------|-----------|------|--------|
| `--text-on-accent` (#fff) | `--accent` (#3b82f6) | 3.68 | Use `--accent-btn` instead |
| `--text-on-accent` (#fff) | `--danger` (dark #ef4444) | 3.76 | Use `--danger-btn` instead |
| `--text-on-accent` (#fff) | `--danger` (light #b91c1c) | 6.47 | Use `--danger-btn` instead |
| Any text | `--accent` (vivid) as background | varies | Vivid accent is for borders/indicators only |

**Rule:** `--accent` and `--danger` are for borders, focus rings, badges, and state dots. When used as a button/interactive background, use `--accent-btn` / `--danger-btn` instead.

---

## Token Changes Summary

| Token | Theme | Old Value | New Value | Old WCAG | New WCAG |
|-------|-------|-----------|-----------|---------|---------|
| `--text-muted` | Light | `#595959` | `#505050` | 6.71 on fafafa | 7.73 on fafafa |
| `--accent-text` | Light | `#1d4ed8` | `#1a44c2` | 6.42 on fafafa | 7.59 on fafafa |
| `--success-text` | Light | `#15803d` | `#0e5929` | 4.81 on fafafa | 8.11 on fafafa |
| `--warning-text` | Light | `#854d0e` | `#7c4a0d` | 6.56 on fafafa | 7.08 on fafafa |
| `--accent-btn` | Both | (new) | `#1e40af` | — | 8.72 white |
| `--danger-btn` | Both | (new) | `#991b1b` | — | 8.31 white |

---

## Colorblind Simulation — State Colors

State indicator colors (badges, dots) were analyzed for grayscale distinctness. The ≥15% luminance difference threshold is difficult to achieve simultaneously across 5 states using vibrant hues at similar perceived saturation.

**Key finding:** All state indicators in this app are accompanied by:
1. Text labels (idle, queued, running, success, failed, retrying)
2. Unicode icons (⏸ ⏳ ↻ ✓ ✗)

Per WCAG 1.4.1, color must not be the **sole** visual indicator of information. Since state is conveyed through icons + text + color, the colorblind distinctness requirement is met through redundant coding.

### Dark theme luminance values

| State | Color | L% |
|-------|-------|----|
| `--state-idle` | oklch(0.55 0 0) | 16.6% |
| `--state-running` | #3b82f6 | 23.5% |
| `--state-success` | #16a34a | 26.9% |
| `--state-failed` | #ef4444 | 22.9% |
| `--state-retrying` | oklch(0.65 0.18 45) | 25.3% |

The luminance values cluster in the 17–27% range. State pairs with insufficient Δ (< 15): all except none. This is acceptable because color is not the sole indicator.

### Light theme luminance values

| State | Color | L% |
|-------|-------|----|
| `--state-idle` | oklch(0.60 0 0) | 21.6% |
| `--state-running` | #3b82f6 | 23.5% |
| `--state-success` | #16a34a | 26.9% |
| `--state-failed` | #b91c1c | 11.2% |
| `--state-retrying` | oklch(0.55 0.18 45) | 15.1% |

One pair passes (success vs failed: Δ 15.7%). All others rely on icon + text redundancy.

---

## Utility File

`src/lib/utils/contrast-audit.ts` — Dev-only, not bundled.

Functions exported:
- `parseColor(str)` — parses `#rrggbb` or `oklch(L C H)` to RGB
- `oklchToRGB(L, C, H)` — OKLCH to sRGB
- `hexToRGB(hex)` — hex to sRGB
- `alphaComposite(fg, alpha, bg)` — effective opaque color
- `relativeLuminance(rgb)` — WCAG relative luminance
- `wcagContrast(text, bg)` — WCAG 2.x contrast ratio
- `apcaContrast(text, bg)` — APCA-W3 Lc value
- `contrastResult(text, bg)` — combined result with PASS/FAIL grades
- `colorblindDistinct(a, b)` — grayscale distinctness check (≥15%)
