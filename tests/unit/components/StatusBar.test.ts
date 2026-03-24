import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import StatusBar from '$lib/components/StatusBar.svelte';
import { activeFilePath } from '$lib/stores/files';
import { llmConfigs } from '$lib/stores/config';
import { terminalInstances, activeInstanceId } from '$lib/stores/terminals';

beforeEach(() => {
  activeFilePath.set(null);
  llmConfigs.set([]);
  terminalInstances.set([]);
  activeInstanceId.set(null);
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
    activeFilePath.set('/project/src/main.ts');
    render(StatusBar);
    await new Promise((r) => setTimeout(r, 0));
    const fileName = document.querySelector('.file-name');
    expect(fileName?.textContent).toBe('main.ts');
  });

  it('shows TypeScript language label for .ts files', async () => {
    activeFilePath.set('/project/src/main.ts');
    render(StatusBar);
    await new Promise((r) => setTimeout(r, 0));
    const lang = document.querySelector('.file-lang');
    expect(lang?.textContent).toBe('TypeScript');
  });

  it('shows UTF-8 encoding in right panel when a file is active', async () => {
    activeFilePath.set('/project/src/app.svelte');
    render(StatusBar);
    await new Promise((r) => setTimeout(r, 0));
    const encoding = document.querySelector('.file-encoding');
    expect(encoding?.textContent).toBe('UTF-8');
  });

  it('does not render file info when no file is active', () => {
    activeFilePath.set(null);
    render(StatusBar);
    expect(document.querySelector('.file-name')).toBeNull();
  });
});
