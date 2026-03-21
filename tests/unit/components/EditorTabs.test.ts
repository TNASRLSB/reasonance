import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import { get } from 'svelte/store';
import EditorTabs from '$lib/components/EditorTabs.svelte';
import { openFiles, activeFilePath, addOpenFile, closeFile } from '$lib/stores/files';
import type { OpenFile } from '$lib/stores/files';

function makeFile(path: string, isDirty = false, isDeleted = false): OpenFile {
  return {
    path,
    name: path.split('/').pop() ?? path,
    content: '',
    isDirty,
    isDeleted,
  };
}

beforeEach(() => {
  openFiles.set([]);
  activeFilePath.set(null);
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
    addOpenFile(makeFile('/proj/a.ts'));
    addOpenFile(makeFile('/proj/b.ts'));
    render(EditorTabs);
    const tabs = document.querySelectorAll('[role="tab"]');
    expect(tabs.length).toBe(2);
  });

  it('renders the file name inside a tab', () => {
    addOpenFile(makeFile('/proj/hello.svelte'));
    render(EditorTabs);
    const names = document.querySelectorAll('.tab-name');
    expect(names[0]?.textContent?.trim()).toContain('hello.svelte');
  });

  it('marks the active tab with aria-selected=true', () => {
    addOpenFile(makeFile('/proj/a.ts'));
    addOpenFile(makeFile('/proj/b.ts'));
    activeFilePath.set('/proj/a.ts');
    render(EditorTabs);
    const tabs = document.querySelectorAll('[role="tab"]');
    const activeTab = Array.from(tabs).find((t) => t.getAttribute('aria-selected') === 'true');
    expect(activeTab).not.toBeUndefined();
  });

  it('adds a dirty indicator (bullet) when file is dirty', () => {
    addOpenFile(makeFile('/proj/dirty.ts', true));
    render(EditorTabs);
    const name = document.querySelector('.tab-name');
    expect(name?.textContent).toContain('●');
  });

  it('renders a close button per tab with accessible label', () => {
    addOpenFile(makeFile('/proj/close-me.ts'));
    render(EditorTabs);
    const closeBtn = document.querySelector('button.tab-close');
    expect(closeBtn).not.toBeNull();
    expect(closeBtn?.getAttribute('aria-label')).toContain('close-me.ts');
  });

  it('removes a file from the store when closeFile is called directly', () => {
    addOpenFile(makeFile('/proj/x.ts'));
    expect(get(openFiles)).toHaveLength(1);
    closeFile('/proj/x.ts');
    expect(get(openFiles)).toHaveLength(0);
  });
});
