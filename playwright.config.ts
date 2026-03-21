import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  testMatch: ['**/*.visual.ts', '**/interaction/**/*.test.ts'],
  timeout: 30000,
  use: {
    baseURL: 'http://localhost:1420',
    screenshot: 'only-on-failure',
  },
  webServer: {
    command: 'npm run dev',
    port: 1420,
    reuseExistingServer: true,
    timeout: 60000,
  },
  expect: {
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.02,
    },
  },
  projects: [
    {
      name: 'chromium',
      use: { browserName: 'chromium' },
    },
  ],
});
