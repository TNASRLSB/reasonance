/**
 * Visual regression tests: Theme rendering
 *
 * NOTE: Theme is controlled by the `themeMode` store (localStorage key "reasonance-theme").
 * We inject the value via localStorage before navigation so the app picks it up on load.
 * Tauri-dependent features may not render; baseline screenshots should be captured with
 * `npx playwright test --update-snapshots` once a dev server is running.
 */

import { test, expect } from '@playwright/test';

async function setTheme(page: import('@playwright/test').Page, theme: 'dark' | 'light' | 'system') {
  await page.addInitScript((t) => {
    localStorage.setItem('reasonance-theme', t);
  }, theme);
}

test.describe('Visual: Themes', () => {
  test('dark theme', async ({ page }) => {
    await setTheme(page, 'dark');
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('theme-dark.png', { fullPage: true });
  });

  test('light theme', async ({ page }) => {
    await setTheme(page, 'light');
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('theme-light.png', { fullPage: true });
  });

  test('system theme (respects OS preference)', async ({ page }) => {
    // Emulate a dark OS preference for deterministic output
    await page.emulateMedia({ colorScheme: 'dark' });
    await setTheme(page, 'system');
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('theme-system-dark.png', { fullPage: true });
  });

  test('system theme light OS preference', async ({ page }) => {
    await page.emulateMedia({ colorScheme: 'light' });
    await setTheme(page, 'system');
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('theme-system-light.png', { fullPage: true });
  });

  test('dark theme — editor panel', async ({ page }) => {
    await setTheme(page, 'dark');
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    const editorPanel = page.locator('.panel.editor').first();
    if (await editorPanel.isVisible()) {
      await expect(editorPanel).toHaveScreenshot('theme-dark-editor.png');
    } else {
      // Fallback: full page if editor panel is not individually identifiable
      await expect(page).toHaveScreenshot('theme-dark-editor-fallback.png', { fullPage: true });
    }
  });
});
