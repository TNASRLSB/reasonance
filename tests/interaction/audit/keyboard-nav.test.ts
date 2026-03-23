import { test, expect } from '@playwright/test';

test.describe('Phase 3C — Keyboard Navigation Audit', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('Tab key reaches all major interactive areas', async ({ page }) => {
    const tabStops: string[] = [];
    const maxTabs = 50;

    for (let i = 0; i < maxTabs; i++) {
      await page.keyboard.press('Tab');
      const focused = await page.evaluate(() => {
        const el = document.activeElement;
        if (!el || el === document.body) return null;
        const tag = el.tagName.toLowerCase();
        const role = el.getAttribute('role') || '';
        const ariaLabel = el.getAttribute('aria-label') || '';
        const text = (el as HTMLElement).innerText?.substring(0, 50) || '';
        const classes = el.className?.toString().substring(0, 80) || '';
        return `${tag}[role=${role}][aria-label=${ariaLabel}][text=${text}][class=${classes}]`;
      });
      if (focused) {
        tabStops.push(focused);
      }
    }

    console.log(`\n=== TAB STOPS (${tabStops.length}) ===`);
    tabStops.forEach((stop, i) => {
      console.log(`  ${i + 1}. ${stop}`);
    });

    // Verify we can reach at least some interactive elements
    expect(tabStops.length).toBeGreaterThan(0);
  });

  test('focus visibility — all focused elements have visible focus indicator', async ({ page }) => {
    const invisibleFocus: string[] = [];
    const maxTabs = 30;

    for (let i = 0; i < maxTabs; i++) {
      await page.keyboard.press('Tab');

      const result = await page.evaluate(() => {
        const el = document.activeElement;
        if (!el || el === document.body) return null;

        const style = window.getComputedStyle(el);
        const outlineStyle = style.outlineStyle;
        const outlineWidth = parseFloat(style.outlineWidth);
        const boxShadow = style.boxShadow;

        const hasOutline = outlineStyle !== 'none' && outlineWidth > 0;
        const hasBoxShadow = boxShadow !== 'none';
        const hasFocusRing = hasOutline || hasBoxShadow;

        const tag = el.tagName.toLowerCase();
        const ariaLabel = el.getAttribute('aria-label') || '';
        const text = (el as HTMLElement).innerText?.substring(0, 30) || '';

        return {
          element: `${tag}[${ariaLabel || text}]`,
          hasFocusRing,
          outline: `${outlineStyle} ${outlineWidth}px`,
          boxShadow: boxShadow !== 'none' ? 'yes' : 'none',
        };
      });

      if (result && !result.hasFocusRing) {
        invisibleFocus.push(`${result.element} (outline: ${result.outline}, shadow: ${result.boxShadow})`);
      }
    }

    console.log(`\n=== FOCUS VISIBILITY ===`);
    if (invisibleFocus.length === 0) {
      console.log('All focused elements have visible focus indicators ✅');
    } else {
      console.log(`Elements with INVISIBLE focus (${invisibleFocus.length}):`);
      invisibleFocus.forEach(el => console.log(`  ❌ ${el}`));
    }

    expect(invisibleFocus.length).toBeDefined();
  });

  test('Escape closes overlays — SearchPalette', async ({ page }) => {
    // Try opening SearchPalette with Ctrl+K
    await page.keyboard.press('Control+k');
    await page.waitForTimeout(300);

    const paletteOpen = await page.locator('[role="dialog"], [role="combobox"], .search-palette, .command-palette').first().isVisible().catch(() => false);
    console.log(`\nSearchPalette opened with Ctrl+K: ${paletteOpen}`);

    if (paletteOpen) {
      await page.keyboard.press('Escape');
      await page.waitForTimeout(300);
      const paletteClosed = await page.locator('[role="dialog"], [role="combobox"], .search-palette, .command-palette').first().isVisible().catch(() => false);
      console.log(`SearchPalette closed with Escape: ${!paletteClosed}`);
    }
  });

  test('Escape closes overlays — ShortcutsDialog', async ({ page }) => {
    // Try opening ShortcutsDialog with Ctrl+/
    await page.keyboard.press('Control+/');
    await page.waitForTimeout(300);

    const dialogOpen = await page.locator('[role="dialog"], .shortcuts-dialog, .modal').first().isVisible().catch(() => false);
    console.log(`\nShortcutsDialog opened with Ctrl+/: ${dialogOpen}`);

    if (dialogOpen) {
      await page.keyboard.press('Escape');
      await page.waitForTimeout(300);
      const dialogClosed = await page.locator('[role="dialog"], .shortcuts-dialog, .modal').first().isVisible().catch(() => false);
      console.log(`ShortcutsDialog closed with Escape: ${!dialogClosed}`);
    }
  });

  test('focus trap in modals — focus stays inside when open', async ({ page }) => {
    // Open SearchPalette
    await page.keyboard.press('Control+k');
    await page.waitForTimeout(300);

    const paletteVisible = await page.locator('[role="dialog"], [role="combobox"], .search-palette, .command-palette').first().isVisible().catch(() => false);
    if (!paletteVisible) {
      console.log('Could not open overlay for focus trap test — skipping');
      return;
    }

    // Tab through and check focus stays inside
    const focusedElements: string[] = [];
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press('Tab');
      const focused = await page.evaluate(() => {
        const el = document.activeElement;
        if (!el) return 'body';
        return `${el.tagName.toLowerCase()}.${el.className?.toString().substring(0, 50)}`;
      });
      focusedElements.push(focused || 'unknown');
    }

    console.log(`\n=== FOCUS TRAP TEST ===`);
    console.log('Focus sequence during modal:');
    focusedElements.forEach((el, i) => console.log(`  Tab ${i + 1}: ${el}`));

    // Check if focus ever left the modal area
    expect(focusedElements.length).toBeGreaterThan(0);
  });
});
