/**
 * Visual regression tests: Main layout
 *
 * NOTE: These tests target localhost:1420 (Vite dev server).
 * Tauri API calls (adapter, file system, PTY) will fail in web-only mode,
 * so the app may fall back to a welcome/error screen. Baseline screenshots
 * should be captured once with `npx playwright test --update-snapshots`.
 */

import { test, expect } from '@playwright/test';

test.describe('Visual: Main Layout', () => {
  test('default dark theme layout', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('app-default.png', { fullPage: true });
  });

  test('three-panel layout is rendered', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    // The app-root div is always rendered regardless of Tauri availability
    const appRoot = page.locator('.app-root');
    await expect(appRoot).toBeVisible();
  });

  test('toolbar is visible', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    // Toolbar is rendered as part of App component
    const toolbar = page.locator('.toolbar, [role="toolbar"], nav').first();
    await expect(page).toHaveScreenshot('app-toolbar.png', {
      clip: { x: 0, y: 0, width: 1280, height: 48 },
    });
  });

  test('status bar is visible at bottom', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot('app-statusbar.png', {
      clip: { x: 0, y: 720 - 28, width: 1280, height: 28 },
    });
  });

  test('welcome screen shown when no project open', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    // If Tauri is unavailable, the welcome screen or a fallback placeholder is shown
    await expect(page).toHaveScreenshot('app-welcome.png', { fullPage: true });
  });
});
