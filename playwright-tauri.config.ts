import { defineConfig } from '@playwright/test';

/**
 * Playwright config for testing against the real Tauri app via tauri-driver (WebDriver).
 *
 * Usage:
 *   1. Build the app: npm run tauri build -- --debug
 *   2. Run tests:     npx playwright test --config playwright-tauri.config.ts
 *
 * tauri-driver acts as a WebDriver server on port 4444.
 * It launches the Tauri binary and connects Playwright to the real webview.
 */
export default defineConfig({
  testDir: './tests/tauri',
  testMatch: ['**/*.test.ts'],
  timeout: 60000,
  expect: {
    timeout: 10000,
  },
  use: {
    // Connect to tauri-driver's WebDriver endpoint
    connectOptions: {
      wsEndpoint: 'ws://127.0.0.1:4444',
    },
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'tauri',
      use: {
        browserName: 'chromium',
      },
    },
  ],
});
