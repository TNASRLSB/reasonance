import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import StatusBar from '$lib/components/StatusBar.svelte';
import { llmConfigs } from '$lib/stores/config';
import { setupTestProject, resetProjectState } from '../../helpers/project-setup';

beforeEach(() => {
  resetProjectState();
  llmConfigs.set([]);
});

describe('StatusBar component', () => {
  it('renders the status bar element', () => {
    render(StatusBar);
    const bar = document.querySelector('.status-bar');
    expect(bar).not.toBeNull();
  });

  it('shows REASONANCE app name in normal mode', () => {
    render(StatusBar);
    const appName = document.querySelector('.app-name');
    expect(appName?.textContent).toBe('REASONANCE');
  });

  it('shows file name in right panel when a file is active', async () => {
    setupTestProject({
      rootPath: '/project',
      activeFilePath: '/project/src/main.ts',
      openFiles: [{ path: '/project/src/main.ts' }],
    });
    render(StatusBar);
    await new Promise((r) => setTimeout(r, 0));
    const fileName = document.querySelector('.file-name');
    expect(fileName?.textContent).toBe('main.ts');
  });

  it('shows TypeScript language label for .ts files', async () => {
    setupTestProject({
      rootPath: '/project',
      activeFilePath: '/project/src/main.ts',
      openFiles: [{ path: '/project/src/main.ts' }],
    });
    render(StatusBar);
    await new Promise((r) => setTimeout(r, 0));
    const lang = document.querySelector('.file-lang');
    expect(lang?.textContent).toBe('TypeScript');
  });

  it('shows UTF-8 encoding in right panel when a file is active', async () => {
    setupTestProject({
      rootPath: '/project',
      activeFilePath: '/project/src/app.svelte',
      openFiles: [{ path: '/project/src/app.svelte' }],
    });
    render(StatusBar);
    await new Promise((r) => setTimeout(r, 0));
    const encoding = document.querySelector('.file-encoding');
    expect(encoding?.textContent).toBe('UTF-8');
  });

  it('does not render file info when no file is active', () => {
    render(StatusBar);
    expect(document.querySelector('.file-name')).toBeNull();
  });
});
