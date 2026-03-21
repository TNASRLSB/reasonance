/**
 * Interaction tests: Search overlays (SearchPalette + FindInFiles)
 *
 * SearchPalette: triggered by Ctrl+P — renders .palette-overlay > .palette
 * FindInFiles:   triggered by Ctrl+Shift+F — renders .fif-overlay > .fif-panel
 *
 * NOTE: In web-only mode (no Tauri runtime) the file listing inside SearchPalette
 * will be empty (adapter.listDir is a no-op stub). The overlay itself still renders.
 */

import { test, expect } from '@playwright/test';

test.describe('Interaction: Search Palette (Ctrl+P)', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('Ctrl+P opens search palette', async ({ page }) => {
    await page.keyboard.press('Control+p');
    const overlay = page.locator('.palette-overlay');
    await expect(overlay).toBeVisible({ timeout: 3000 });
  });

  test('search palette contains input field', async ({ page }) => {
    await page.keyboard.press('Control+p');
    const input = page.locator('.palette-input');
    await expect(input).toBeVisible();
    await expect(input).toBeFocused();
  });

  test('search palette placeholder text is correct', async ({ page }) => {
    await page.keyboard.press('Control+p');
    const input = page.locator('.palette-input');
    await expect(input).toHaveAttribute('placeholder', 'Go to file…');
  });

  test('Escape closes search palette', async ({ page }) => {
    await page.keyboard.press('Control+p');
    await expect(page.locator('.palette-overlay')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.locator('.palette-overlay')).not.toBeVisible({ timeout: 3000 });
  });

  test('clicking outside overlay closes search palette', async ({ page }) => {
    await page.keyboard.press('Control+p');
    const overlay = page.locator('.palette-overlay');
    await expect(overlay).toBeVisible();
    // Click the overlay backdrop (not the inner .palette dialog)
    await overlay.click({ position: { x: 10, y: 10 } });
    await expect(overlay).not.toBeVisible({ timeout: 3000 });
  });

  test('typing in search palette filters results', async ({ page }) => {
    await page.keyboard.press('Control+p');
    const input = page.locator('.palette-input');
    await input.type('svelte');
    // Either a list of results or the empty state message should be visible
    const list = page.locator('.palette-list, .palette-empty');
    await expect(list.first()).toBeVisible({ timeout: 3000 });
  });

  test('search palette dialog has correct aria attributes', async ({ page }) => {
    await page.keyboard.press('Control+p');
    const dialog = page.locator('[role="dialog"][aria-label="File search"]');
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAttribute('aria-modal', 'true');
  });
});

test.describe('Interaction: Find in Files (Ctrl+Shift+F)', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('Ctrl+Shift+F opens find-in-files panel', async ({ page }) => {
    await page.keyboard.press('Control+Shift+f');
    const overlay = page.locator('.fif-overlay');
    await expect(overlay).toBeVisible({ timeout: 3000 });
  });

  test('find-in-files panel has title "Find in Files"', async ({ page }) => {
    await page.keyboard.press('Control+Shift+f');
    const title = page.locator('.fif-title');
    await expect(title).toHaveText('Find in Files');
  });

  test('find-in-files panel has search input', async ({ page }) => {
    await page.keyboard.press('Control+Shift+f');
    const input = page.locator('.fif-input');
    await expect(input).toBeVisible();
    await expect(input).toBeFocused();
  });

  test('find-in-files panel has search button', async ({ page }) => {
    await page.keyboard.press('Control+Shift+f');
    const btn = page.locator('.search-btn');
    await expect(btn).toBeVisible();
    // Button disabled when input is empty
    await expect(btn).toBeDisabled();
  });

  test('search button enables when text is typed', async ({ page }) => {
    await page.keyboard.press('Control+Shift+f');
    const input = page.locator('.fif-input');
    await input.type('hello');
    const btn = page.locator('.search-btn');
    await expect(btn).toBeEnabled();
  });

  test('Escape closes find-in-files panel', async ({ page }) => {
    await page.keyboard.press('Control+Shift+f');
    await expect(page.locator('.fif-overlay')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.locator('.fif-overlay')).not.toBeVisible({ timeout: 3000 });
  });

  test('close button (✕) closes find-in-files panel', async ({ page }) => {
    await page.keyboard.press('Control+Shift+f');
    const closeBtn = page.locator('.fif-panel .close-btn');
    await closeBtn.click();
    await expect(page.locator('.fif-overlay')).not.toBeVisible({ timeout: 3000 });
  });

  test('find-in-files dialog has correct aria attributes', async ({ page }) => {
    await page.keyboard.press('Control+Shift+f');
    const dialog = page.locator('[role="dialog"][aria-label="Find in files"]');
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAttribute('aria-modal', 'true');
  });
});
