import { test, expect } from '@playwright/test';

test.describe('Phase 3C — Color Contrast & Visual Checks', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('computed color contrast for all text elements', async ({ page }) => {
    const results = await page.evaluate(() => {
      function getLuminance(r: number, g: number, b: number): number {
        const [rs, gs, bs] = [r, g, b].map(c => {
          c = c / 255;
          return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
        });
        return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
      }

      function getContrastRatio(l1: number, l2: number): number {
        const lighter = Math.max(l1, l2);
        const darker = Math.min(l1, l2);
        return (lighter + 0.05) / (darker + 0.05);
      }

      function parseColor(color: string): [number, number, number] | null {
        const match = color.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
        if (match) return [parseInt(match[1]), parseInt(match[2]), parseInt(match[3])];
        return null;
      }

      const textElements = document.querySelectorAll('*');
      const failures: Array<{
        tag: string;
        text: string;
        fg: string;
        bg: string;
        ratio: number;
        fontSize: string;
        required: number;
      }> = [];
      const passes: number[] = [];

      for (const el of textElements) {
        const htmlEl = el as HTMLElement;
        if (!htmlEl.innerText?.trim() || htmlEl.children.length > 0) continue;
        if (htmlEl.offsetWidth === 0 || htmlEl.offsetHeight === 0) continue;

        const style = window.getComputedStyle(htmlEl);
        const fgColor = style.color;
        const fontSize = parseFloat(style.fontSize);
        const fontWeight = parseInt(style.fontWeight) || 400;

        // Walk up to find background
        let bgColor = 'rgba(0, 0, 0, 0)';
        let parent: HTMLElement | null = htmlEl;
        while (parent) {
          const parentStyle = window.getComputedStyle(parent);
          const bg = parentStyle.backgroundColor;
          if (bg && bg !== 'rgba(0, 0, 0, 0)' && bg !== 'transparent') {
            bgColor = bg;
            break;
          }
          parent = parent.parentElement;
        }

        const fg = parseColor(fgColor);
        const bg = parseColor(bgColor);
        if (!fg || !bg) continue;

        const fgLum = getLuminance(...fg);
        const bgLum = getLuminance(...bg);
        const ratio = getContrastRatio(fgLum, bgLum);

        // Large text: >= 18pt (24px) or >= 14pt (18.66px) bold
        const isLargeText = fontSize >= 24 || (fontSize >= 18.66 && fontWeight >= 700);
        const required = isLargeText ? 3 : 4.5;

        if (ratio < required) {
          failures.push({
            tag: `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 40)}`,
            text: htmlEl.innerText.substring(0, 40),
            fg: fgColor,
            bg: bgColor,
            ratio: Math.round(ratio * 100) / 100,
            fontSize: `${fontSize}px`,
            required,
          });
        } else {
          passes.push(ratio);
        }
      }

      return { failures, passCount: passes.length, totalChecked: failures.length + passes.length };
    });

    console.log(`\n=== COLOR CONTRAST RESULTS ===`);
    console.log(`Total text elements checked: ${results.totalChecked}`);
    console.log(`Passing: ${results.passCount}`);
    console.log(`Failing: ${results.failures.length}`);

    if (results.failures.length > 0) {
      console.log(`\nFailing elements:`);
      // Group by similar ratios
      const byRatio = new Map<string, typeof results.failures>();
      for (const f of results.failures) {
        const key = `${f.fg}|${f.bg}`;
        if (!byRatio.has(key)) byRatio.set(key, []);
        byRatio.get(key)!.push(f);
      }

      for (const [key, items] of byRatio) {
        const first = items[0];
        console.log(`\n  Color pair: fg=${first.fg} bg=${first.bg} ratio=${first.ratio}:1 (need ${first.required}:1)`);
        for (const item of items.slice(0, 3)) {
          console.log(`    ${item.tag}: "${item.text}" (${item.fontSize})`);
        }
        if (items.length > 3) {
          console.log(`    ... and ${items.length - 3} more elements`);
        }
      }
    }

    expect(results.totalChecked).toBeGreaterThan(0);
  });

  test('touch target sizes for interactive elements', async ({ page }) => {
    const results = await page.evaluate(() => {
      const interactive = document.querySelectorAll('button, a, input, select, textarea, [role="button"], [role="tab"], [role="menuitem"], [tabindex]');
      const tooSmall: Array<{
        tag: string;
        text: string;
        width: number;
        height: number;
        ariaLabel: string;
      }> = [];
      const ok: number[] = [];

      for (const el of interactive) {
        const rect = el.getBoundingClientRect();
        if (rect.width === 0 && rect.height === 0) continue;

        const minSize = 24; // WCAG 2.5.8 AA minimum
        if (rect.width < minSize || rect.height < minSize) {
          tooSmall.push({
            tag: `${el.tagName.toLowerCase()}.${el.className?.toString().substring(0, 40)}`,
            text: (el as HTMLElement).innerText?.substring(0, 30) || '',
            width: Math.round(rect.width),
            height: Math.round(rect.height),
            ariaLabel: el.getAttribute('aria-label') || '',
          });
        } else {
          ok.push(1);
        }
      }

      return { tooSmall, okCount: ok.length, totalChecked: tooSmall.length + ok.length };
    });

    console.log(`\n=== TOUCH TARGET SIZES ===`);
    console.log(`Total interactive elements: ${results.totalChecked}`);
    console.log(`Meeting 24px minimum: ${results.okCount}`);
    console.log(`Too small: ${results.tooSmall.length}`);

    if (results.tooSmall.length > 0) {
      console.log(`\nUndersized elements:`);
      for (const el of results.tooSmall) {
        const label = el.ariaLabel || el.text || el.tag;
        console.log(`  ❌ ${label}: ${el.width}×${el.height}px (need 24×24)`);
      }
    }

    expect(results.totalChecked).toBeGreaterThan(0);
  });

  test('heading hierarchy', async ({ page }) => {
    const headings = await page.evaluate(() => {
      const hs = document.querySelectorAll('h1, h2, h3, h4, h5, h6');
      return Array.from(hs).map(h => ({
        level: parseInt(h.tagName[1]),
        text: h.textContent?.substring(0, 50) || '',
        visible: (h as HTMLElement).offsetWidth > 0,
      }));
    });

    console.log(`\n=== HEADING HIERARCHY ===`);
    if (headings.length === 0) {
      console.log('⚠️ No headings found — screen readers need headings for navigation');
    } else {
      let issues = 0;
      let prevLevel = 0;
      for (const h of headings) {
        const skip = h.level > prevLevel + 1 && prevLevel > 0;
        const marker = skip ? '⚠️ SKIP' : '✅';
        console.log(`  ${'  '.repeat(h.level - 1)}h${h.level}: "${h.text}" ${h.visible ? '' : '(hidden)'} ${marker}`);
        if (skip) issues++;
        prevLevel = h.level;
      }
      if (issues > 0) {
        console.log(`\n  ${issues} heading level skip(s) found — bad for screen reader navigation`);
      }
    }
  });

  test('ARIA landmarks', async ({ page }) => {
    const landmarks = await page.evaluate(() => {
      const roles = ['banner', 'navigation', 'main', 'complementary', 'contentinfo', 'search', 'form', 'region'];
      const found: Array<{ role: string; label: string; tag: string }> = [];

      for (const role of roles) {
        const els = document.querySelectorAll(`[role="${role}"]`);
        els.forEach(el => {
          found.push({
            role,
            label: el.getAttribute('aria-label') || el.getAttribute('aria-labelledby') || '',
            tag: el.tagName.toLowerCase(),
          });
        });
      }

      // Also check semantic elements
      const semanticMap: Record<string, string> = {
        header: 'banner', nav: 'navigation', main: 'main',
        aside: 'complementary', footer: 'contentinfo',
      };
      for (const [tag, role] of Object.entries(semanticMap)) {
        const els = document.querySelectorAll(tag);
        els.forEach(el => {
          if (!el.getAttribute('role')) {
            found.push({
              role: `implicit:${role}`,
              label: el.getAttribute('aria-label') || '',
              tag,
            });
          }
        });
      }

      return found;
    });

    console.log(`\n=== ARIA LANDMARKS ===`);
    if (landmarks.length === 0) {
      console.log('❌ No ARIA landmarks found — critical for screen reader navigation');
    } else {
      for (const lm of landmarks) {
        console.log(`  ${lm.role}: <${lm.tag}> ${lm.label ? `"${lm.label}"` : '(unlabeled)'}`);
      }

      // Check for required landmarks
      const hasMain = landmarks.some(l => l.role === 'main' || l.role === 'implicit:main');
      const hasNav = landmarks.some(l => l.role === 'navigation' || l.role === 'implicit:navigation');
      console.log(`\n  Has main landmark: ${hasMain ? '✅' : '❌'}`);
      console.log(`  Has navigation landmark: ${hasNav ? '✅' : '❌'}`);
    }
  });

  test('reduced motion — animations with prefers-reduced-motion', async ({ page }) => {
    // Emulate reduced motion
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const animations = await page.evaluate(() => {
      const allElements = document.querySelectorAll('*');
      const animated: Array<{ tag: string; property: string; value: string }> = [];

      for (const el of allElements) {
        const style = window.getComputedStyle(el);
        const transition = style.transition;
        const animation = style.animation;

        if (transition && transition !== 'none' && transition !== 'all 0s ease 0s') {
          animated.push({
            tag: `${el.tagName.toLowerCase()}.${el.className?.toString().substring(0, 30)}`,
            property: 'transition',
            value: transition.substring(0, 80),
          });
        }
        if (animation && animation !== 'none') {
          animated.push({
            tag: `${el.tagName.toLowerCase()}.${el.className?.toString().substring(0, 30)}`,
            property: 'animation',
            value: animation.substring(0, 80),
          });
        }
      }

      return animated;
    });

    console.log(`\n=== REDUCED MOTION CHECK ===`);
    if (animations.length === 0) {
      console.log('✅ No animations/transitions active with prefers-reduced-motion: reduce');
    } else {
      console.log(`⚠️ ${animations.length} elements still animated with reduced motion:`);
      for (const a of animations.slice(0, 10)) {
        console.log(`  ${a.tag}: ${a.property}=${a.value}`);
      }
      if (animations.length > 10) {
        console.log(`  ... and ${animations.length - 10} more`);
      }
    }
  });
});
