/**
 * Interaction tests: Keyboard shortcuts
 *
 * Shortcuts registered in +page.svelte via registerKeybinding():
 *   Ctrl+P         → open SearchPalette   (.palette-overlay)
 *   Ctrl+Shift+F   → open FindInFiles     (.fif-overlay)
 *   Ctrl+,         → open Settings        (.settings-overlay → .settings-modal)
 *   F1             → toggle HelpPanel     (.help-panel)
 *   Escape         → close active overlay
 *
 * NOTE: In web-only mode Tauri adapter is stubbed; overlay rendering itself is unaffected.
 */

import { test, expect } from '@playwright/test';

test.describe('Keyboard Shortcuts', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    // Click body to ensure the page has keyboard focus
    await page.locator('body').click();
  });

  // ─── Settings (Ctrl+,) ─────────────────────────────────────────────────────

  test('Ctrl+, opens settings modal', async ({ page }) => {
    await page.keyboard.press('Control+,');
    const modal = page.locator('.settings-modal');
    await expect(modal).toBeVisible({ timeout: 3000 });
  });

  test('settings modal has correct aria attributes', async ({ page }) => {
    await page.keyboard.press('Control+,');
    const dialog = page.locator('[role="dialog"][aria-label="Settings"]');
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAttribute('aria-modal', 'true');
  });

  test('Escape closes settings modal', async ({ page }) => {
    await page.keyboard.press('Control+,');
    await expect(page.locator('.settings-modal')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.locator('.settings-modal')).not.toBeVisible({ timeout: 3000 });
  });

  test('clicking settings overlay backdrop closes modal', async ({ page }) => {
    await page.keyboard.press('Control+,');
    const overlay = page.locator('.settings-overlay');
    await expect(overlay).toBeVisible();
    // Click the very edge of the backdrop (outside .settings-modal)
    await overlay.click({ position: { x: 5, y: 5 } });
    await expect(page.locator('.settings-modal')).not.toBeVisible({ timeout: 3000 });
  });

  // ─── Help / Docs (F1) ──────────────────────────────────────────────────────

  test('F1 opens help panel', async ({ page }) => {
    await page.keyboard.press('F1');
    const helpPanel = page.locator('.help-panel');
    await expect(helpPanel).toBeVisible({ timeout: 3000 });
  });

  test('F1 toggles help panel off when already open', async ({ page }) => {
    // Open
    await page.keyboard.press('F1');
    await expect(page.locator('.help-panel')).toBeVisible({ timeout: 3000 });
    // Toggle off
    await page.keyboard.press('F1');
    await expect(page.locator('.help-panel')).not.toBeVisible({ timeout: 3000 });
  });

  // ─── Search Palette (Ctrl+P) ────────────────────────────────────────────────

  test('Ctrl+P opens search palette', async ({ page }) => {
    await page.keyboard.press('Control+p');
    await expect(page.locator('.palette-overlay')).toBeVisible({ timeout: 3000 });
  });

  test('Escape closes search palette opened with Ctrl+P', async ({ page }) => {
    await page.keyboard.press('Control+p');
    await expect(page.locator('.palette-overlay')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.locator('.palette-overlay')).not.toBeVisible({ timeout: 3000 });
  });

  // ─── Find in Files (Ctrl+Shift+F) ──────────────────────────────────────────

  test('Ctrl+Shift+F opens find-in-files panel', async ({ page }) => {
    await page.keyboard.press('Control+Shift+f');
    await expect(page.locator('.fif-overlay')).toBeVisible({ timeout: 3000 });
  });

  test('Escape closes find-in-files panel', async ({ page }) => {
    await page.keyboard.press('Control+Shift+f');
    await expect(page.locator('.fif-overlay')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.locator('.fif-overlay')).not.toBeVisible({ timeout: 3000 });
  });

  // ─── Overlay isolation ─────────────────────────────────────────────────────

  test('only one overlay is visible at a time', async ({ page }) => {
    // Open settings then search palette — settings should close first
    await page.keyboard.press('Control+,');
    await expect(page.locator('.settings-modal')).toBeVisible();
    // Close settings, then open search palette
    await page.keyboard.press('Escape');
    await page.keyboard.press('Control+p');
    // Only search palette should be visible
    await expect(page.locator('.palette-overlay')).toBeVisible({ timeout: 3000 });
    await expect(page.locator('.settings-modal')).not.toBeVisible();
  });

  // ─── Sequential open/close ─────────────────────────────────────────────────

  test('settings can be opened and closed multiple times', async ({ page }) => {
    for (let i = 0; i < 3; i++) {
      await page.keyboard.press('Control+,');
      await expect(page.locator('.settings-modal')).toBeVisible({ timeout: 3000 });
      await page.keyboard.press('Escape');
      await expect(page.locator('.settings-modal')).not.toBeVisible({ timeout: 3000 });
    }
  });
});
