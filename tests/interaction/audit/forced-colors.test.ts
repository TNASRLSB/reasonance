import { test, expect } from '@playwright/test';

test.describe('Phase 3C — Forced Colors & High Contrast', () => {
  test('forced-colors: active — element visibility', async ({ browser }) => {
    const context = await browser.newContext({
      forcedColors: 'active',
    });
    const page = await context.newPage();
    await page.goto('http://127.0.0.1:1420/');
    await page.waitForLoadState('networkidle');

    const analysis = await page.evaluate(() => {
      const results = {
        invisibleElements: [] as string[],
        bordersLost: [] as string[],
        iconsLost: [] as string[],
        focusIssues: [] as string[],
        totalChecked: 0,
      };

      const elements = document.querySelectorAll('button, a, input, [role="button"], [role="tab"], [role="menuitem"], .icon, svg');
      for (const el of elements) {
        const htmlEl = el as HTMLElement;
        if (htmlEl.offsetWidth === 0 && htmlEl.offsetHeight === 0) continue;
        results.totalChecked++;

        const style = window.getComputedStyle(htmlEl);

        // Check if element relies on background-color for visibility
        // In forced-colors, backgrounds are overridden
        if (style.backgroundColor !== 'rgba(0, 0, 0, 0)' && !style.border && !style.outline) {
          const text = htmlEl.innerText?.trim();
          if (!text && htmlEl.tagName !== 'INPUT') {
            results.invisibleElements.push(
              `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 40)}`
            );
          }
        }

        // Check for icon-only buttons (no text, rely on color/background)
        if (htmlEl.tagName === 'BUTTON' || htmlEl.getAttribute('role') === 'button') {
          const text = htmlEl.innerText?.trim();
          const ariaLabel = htmlEl.getAttribute('aria-label');
          if (!text && !ariaLabel) {
            results.iconsLost.push(
              `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 40)}`
            );
          }
        }

        // Check focus styles — box-shadow doesn't work in forced-colors
        if (style.outlineStyle === 'none' && style.boxShadow !== 'none') {
          results.focusIssues.push(
            `${htmlEl.tagName.toLowerCase()}.${htmlEl.className?.toString().substring(0, 40)}: uses box-shadow for focus (invisible in forced-colors)`
          );
        }
      }

      return results;
    });

    console.log(`\n=== FORCED COLORS ANALYSIS ===`);
    console.log(`Elements checked: ${analysis.totalChecked}`);

    if (analysis.invisibleElements.length > 0) {
      console.log(`\n❌ Potentially invisible elements (${analysis.invisibleElements.length}):`);
      analysis.invisibleElements.forEach(e => console.log(`  ${e}`));
    } else {
      console.log('✅ No invisible elements detected');
    }

    if (analysis.iconsLost.length > 0) {
      console.log(`\n⚠️ Icon-only buttons without aria-label (${analysis.iconsLost.length}):`);
      analysis.iconsLost.forEach(e => console.log(`  ${e}`));
    }

    if (analysis.focusIssues.length > 0) {
      console.log(`\n⚠️ Focus indicators using box-shadow (${analysis.focusIssues.length}):`);
      analysis.focusIssues.forEach(e => console.log(`  ${e}`));
    }

    await page.screenshot({ path: 'docs/audit/screenshot-forced-colors.png', fullPage: true });
    console.log('\nScreenshot saved: docs/audit/screenshot-forced-colors.png');

    await context.close();
  });

  test('prefers-contrast: more — check readability', async ({ page }) => {
    await page.emulateMedia({ contrast: 'more' } as any);
    await page.goto('http://127.0.0.1:1420/');
    await page.waitForLoadState('networkidle');

    const cssResponds = await page.evaluate(() => {
      return window.matchMedia('(prefers-contrast: more)').matches;
    });

    // Check if any CSS responds to prefers-contrast
    const contrastStyles = await page.evaluate(() => {
      const sheets = Array.from(document.styleSheets);
      let hasContrastMedia = false;
      for (const sheet of sheets) {
        try {
          const rules = Array.from(sheet.cssRules);
          for (const rule of rules) {
            if (rule instanceof CSSMediaRule && rule.conditionText?.includes('prefers-contrast')) {
              hasContrastMedia = true;
              break;
            }
          }
        } catch { /* cross-origin sheet */ }
        if (hasContrastMedia) break;
      }
      return hasContrastMedia;
    });

    console.log(`\n=== PREFERS-CONTRAST: MORE ===`);
    console.log(`Media query matches: ${cssResponds ? '✅' : '❌'}`);
    console.log(`CSS responds to prefers-contrast: ${contrastStyles ? '✅' : '❌ No prefers-contrast rules found'}`);

    await page.screenshot({ path: 'docs/audit/screenshot-high-contrast.png', fullPage: true });
  });
});
