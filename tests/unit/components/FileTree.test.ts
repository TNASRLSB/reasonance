import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import FileTree from '$lib/components/FileTree.svelte';
import { projectRoot } from '$lib/stores/files';
import { createMockAdapter } from '../../mocks/adapter';
import type { FileEntry } from '$lib/adapter';

beforeEach(() => {
  projectRoot.set('');
});

describe('FileTree component', () => {
  it('renders the file-tree container', () => {
    const adapter = createMockAdapter();
    render(FileTree, { props: { adapter } });
    const tree = document.querySelector('.file-tree');
    expect(tree).not.toBeNull();
  });

  it('renders the FILES header when no project root is set', () => {
    const adapter = createMockAdapter();
    render(FileTree, { props: { adapter } });
    const header = document.querySelector('.tree-header');
    expect(header?.textContent).toBe('FILES');
  });

  it('renders project name in header when projectRoot is set', async () => {
    projectRoot.set('/home/user/myproject');
    const adapter = createMockAdapter();
    render(FileTree, { props: { adapter } });
    await new Promise((r) => setTimeout(r, 10));
    const header = document.querySelector('.tree-header');
    expect(header?.textContent).toBe('myproject');
  });

  it('renders file entries returned by listDir', async () => {
    const files: FileEntry[] = [
      { path: '/proj/src/main.ts', name: 'main.ts', isDir: false, isGitignored: false, size: 0, modified: 0 },
      { path: '/proj/src/app.ts', name: 'app.ts', isDir: false, isGitignored: false, size: 0, modified: 0 },
    ];
    const adapter = createMockAdapter({
      listDir: () => Promise.resolve(files),
    });
    projectRoot.set('/proj/src');
    render(FileTree, { props: { adapter } });
    await new Promise((r) => setTimeout(r, 20));
    const items = document.querySelectorAll('.tree-item');
    expect(items.length).toBe(2);
  });

  it('renders file names for entries', async () => {
    const files: FileEntry[] = [
      { path: '/proj/README.md', name: 'README.md', isDir: false, isGitignored: false, size: 0, modified: 0 },
    ];
    const adapter = createMockAdapter({
      listDir: () => Promise.resolve(files),
    });
    projectRoot.set('/proj');
    render(FileTree, { props: { adapter } });
    await new Promise((r) => setTimeout(r, 20));
    const nameEl = document.querySelector('.name');
    expect(nameEl?.textContent).toBe('README.md');
  });

  it('renders no tree-items when listDir returns empty array', async () => {
    const adapter = createMockAdapter({
      listDir: () => Promise.resolve([]),
    });
    projectRoot.set('/empty-proj');
    render(FileTree, { props: { adapter } });
    await new Promise((r) => setTimeout(r, 20));
    const items = document.querySelectorAll('.tree-item');
    expect(items.length).toBe(0);
  });

  it('applies gitignored class to ignored entries', async () => {
    const files: FileEntry[] = [
      { path: '/proj/node_modules', name: 'node_modules', isDir: true, isGitignored: true, size: 0, modified: 0 },
    ];
    const adapter = createMockAdapter({
      listDir: () => Promise.resolve(files),
    });
    projectRoot.set('/proj');
    render(FileTree, { props: { adapter } });
    await new Promise((r) => setTimeout(r, 20));
    const item = document.querySelector('.tree-item');
    expect(item?.classList.contains('gitignored')).toBe(true);
  });
});
