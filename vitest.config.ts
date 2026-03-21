import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    conditions: ['browser'],
    alias: {
      '$lib': path.resolve('./src/lib'),
      '$app': path.resolve('./src/app'),
    },
  },
  test: {
    environment: 'jsdom',
    pool: 'forks',
    include: ['tests/**/*.test.ts'],
    exclude: ['tests/visual/**', 'tests/interaction/**'],
    globals: true,
    alias: {
      '@tauri-apps/api/event': path.resolve('./tests/mocks/tauri-api.ts'),
      '@tauri-apps/api/core': path.resolve('./tests/mocks/tauri-api.ts'),
      '@tauri-apps/api/window': path.resolve('./tests/mocks/tauri-api.ts'),
      '@tauri-apps/api': path.resolve('./tests/mocks/tauri-api.ts'),
      '@tauri-apps/plugin-fs': path.resolve('./tests/mocks/tauri-plugins.ts'),
      '@tauri-apps/plugin-store': path.resolve('./tests/mocks/tauri-plugins.ts'),
      '@tauri-apps/plugin-dialog': path.resolve('./tests/mocks/tauri-plugins.ts'),
      '@tauri-apps/plugin-clipboard-manager': path.resolve('./tests/mocks/tauri-plugins.ts'),
      '@tauri-apps/plugin-notification': path.resolve('./tests/mocks/tauri-plugins.ts'),
      '@tauri-apps/plugin-global-shortcut': path.resolve('./tests/mocks/tauri-plugins.ts'),
      '@tauri-apps/plugin-process': path.resolve('./tests/mocks/tauri-plugins.ts'),
      '@tauri-apps/plugin-deep-link': path.resolve('./tests/mocks/tauri-plugins.ts'),
      '@tauri-apps/plugin-window-state': path.resolve('./tests/mocks/tauri-plugins.ts'),
      '@tauri-apps/plugin-updater': path.resolve('./tests/mocks/tauri-plugins.ts'),
      '@tauri-apps/plugin-opener': path.resolve('./tests/mocks/tauri-plugins.ts'),
    },
    coverage: {
      provider: 'v8',
      include: ['src/lib/**'],
      reporter: ['text', 'html'],
    },
  },
});
