import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  openFiles,
  activeFilePath,
  projectRoot,
  recentProjects,
  addOpenFile,
  closeFile,
  addRecentProject,
} from '$lib/stores/files';
import type { OpenFile } from '$lib/stores/files';

const makeFile = (path: string): OpenFile => ({
  path,
  name: path.split('/').pop() ?? path,
  content: `// ${path}`,
  isDirty: false,
  isDeleted: false,
});

describe('files store', () => {
  beforeEach(() => {
    openFiles.set([]);
    activeFilePath.set(null);
    projectRoot.set('');
    recentProjects.set([]);
  });

  it('has empty default state', () => {
    expect(get(openFiles)).toEqual([]);
    expect(get(activeFilePath)).toBeNull();
    expect(get(projectRoot)).toBe('');
    expect(get(recentProjects)).toEqual([]);
  });

  it('addOpenFile adds a file and sets it active', () => {
    const file = makeFile('/project/src/main.ts');
    addOpenFile(file);

    expect(get(openFiles)).toHaveLength(1);
    expect(get(openFiles)[0].path).toBe('/project/src/main.ts');
    expect(get(activeFilePath)).toBe('/project/src/main.ts');
  });

  it('addOpenFile does not duplicate already-open files', () => {
    const file = makeFile('/project/src/main.ts');
    addOpenFile(file);
    addOpenFile(file);

    expect(get(openFiles)).toHaveLength(1);
  });

  it('addOpenFile switches active file without duplicating', () => {
    const fileA = makeFile('/project/a.ts');
    const fileB = makeFile('/project/b.ts');
    addOpenFile(fileA);
    addOpenFile(fileB);
    addOpenFile(fileA); // re-open already open file

    expect(get(openFiles)).toHaveLength(2);
    expect(get(activeFilePath)).toBe('/project/a.ts');
  });

  it('closeFile removes the file from openFiles', () => {
    const fileA = makeFile('/project/a.ts');
    const fileB = makeFile('/project/b.ts');
    addOpenFile(fileA);
    addOpenFile(fileB);

    closeFile('/project/a.ts');

    const files = get(openFiles);
    expect(files).toHaveLength(1);
    expect(files[0].path).toBe('/project/b.ts');
  });

  it('closeFile clears activeFilePath when active file is closed', () => {
    const file = makeFile('/project/main.ts');
    addOpenFile(file);
    expect(get(activeFilePath)).toBe('/project/main.ts');

    closeFile('/project/main.ts');
    expect(get(activeFilePath)).toBeNull();
  });

  it('closeFile keeps activeFilePath when a different file is closed', () => {
    const fileA = makeFile('/project/a.ts');
    const fileB = makeFile('/project/b.ts');
    addOpenFile(fileA);
    addOpenFile(fileB);
    // fileB is now active
    closeFile('/project/a.ts');

    expect(get(activeFilePath)).toBe('/project/b.ts');
  });

  it('addRecentProject prepends and deduplicates', () => {
    addRecentProject('/projects/alpha');
    addRecentProject('/projects/beta');
    addRecentProject('/projects/alpha'); // duplicate

    const recent = get(recentProjects);
    expect(recent[0]).toBe('/projects/alpha');
    expect(recent).toHaveLength(2);
  });

  it('addRecentProject keeps at most 10 entries', () => {
    for (let i = 0; i < 12; i++) {
      addRecentProject(`/projects/proj-${i}`);
    }
    expect(get(recentProjects)).toHaveLength(10);
  });

  it('projectRoot is settable', () => {
    projectRoot.set('/home/user/myproject');
    expect(get(projectRoot)).toBe('/home/user/myproject');
  });
});
