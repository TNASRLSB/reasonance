import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import { get } from 'svelte/store';
import EditorTabs from '$lib/components/EditorTabs.svelte';
import { openFiles, activeFilePath, closeFile } from '$lib/stores/files';
import { openFile } from '$lib/stores/projects';
import { setupTestProject, resetProjectState } from '../../helpers/project-setup';

beforeEach(() => {
  resetProjectState();
});

describe('EditorTabs component', () => {
  it('renders the tab list container', () => {
    render(EditorTabs);
    const tablist = document.querySelector('[role="tablist"]');
    expect(tablist).not.toBeNull();
  });

  it('renders no tabs when openFiles is empty', () => {
    render(EditorTabs);
    const tabs = document.querySelectorAll('[role="tab"]');
    expect(tabs.length).toBe(0);
  });

  it('renders one tab per open file', () => {
    setupTestProject({
      rootPath: '/proj',
      openFiles: [
        { path: '/proj/a.ts' },
        { path: '/proj/b.ts' },
      ],
      activeFilePath: '/proj/b.ts',
    });
    render(EditorTabs);
    const tabs = document.querySelectorAll('[role="tab"]');
    expect(tabs.length).toBe(2);
  });

  it('renders the file name inside a tab', () => {
    setupTestProject({
      rootPath: '/proj',
      openFiles: [{ path: '/proj/hello.svelte' }],
      activeFilePath: '/proj/hello.svelte',
    });
    render(EditorTabs);
    const names = document.querySelectorAll('.tab-name');
    expect(names[0]?.textContent?.trim()).toContain('hello.svelte');
  });

  it('marks the active tab with aria-selected=true', () => {
    setupTestProject({
      rootPath: '/proj',
      openFiles: [
        { path: '/proj/a.ts' },
        { path: '/proj/b.ts' },
      ],
      activeFilePath: '/proj/a.ts',
    });
    render(EditorTabs);
    const tabs = document.querySelectorAll('[role="tab"]');
    const activeTab = Array.from(tabs).find((t) => t.getAttribute('aria-selected') === 'true');
    expect(activeTab).not.toBeUndefined();
  });

  it('adds a dirty indicator (bullet) when file is dirty', () => {
    setupTestProject({
      rootPath: '/proj',
      openFiles: [{ path: '/proj/dirty.ts', isDirty: true }],
      activeFilePath: '/proj/dirty.ts',
    });
    render(EditorTabs);
    const name = document.querySelector('.tab-name');
    expect(name?.textContent).toContain('\u25CF');
  });

  it('renders a close button per tab with accessible label', () => {
    setupTestProject({
      rootPath: '/proj',
      openFiles: [{ path: '/proj/close-me.ts' }],
      activeFilePath: '/proj/close-me.ts',
    });
    render(EditorTabs);
    const closeBtn = document.querySelector('button.tab-close');
    expect(closeBtn).not.toBeNull();
    expect(closeBtn?.getAttribute('aria-label')).toContain('close-me.ts');
  });

  it('removes a file from the store when closeFile is called directly', () => {
    setupTestProject({
      rootPath: '/proj',
      openFiles: [{ path: '/proj/x.ts' }],
      activeFilePath: '/proj/x.ts',
    });
    expect(get(openFiles)).toHaveLength(1);
    closeFile('/proj/x.ts');
    expect(get(openFiles)).toHaveLength(0);
  });
});
