import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  openFiles,
  activeFilePath,
  projectRoot,
  recentProjects,
  addOpenFile,
  closeFile,
} from '$lib/stores/files';
import { setupTestProject, resetProjectState } from '../../helpers/project-setup';

describe('files store', () => {
  beforeEach(() => {
    resetProjectState();
  });

  it('has empty default state', () => {
    expect(get(openFiles)).toEqual([]);
    expect(get(activeFilePath)).toBeNull();
    expect(get(projectRoot)).toBe('');
  });

  it('addOpenFile adds a file and sets it active', () => {
    setupTestProject({ rootPath: '/project' });
    addOpenFile('/project/src/main.ts', '// /project/src/main.ts');

    expect(get(openFiles)).toHaveLength(1);
    expect(get(openFiles)[0].path).toBe('/project/src/main.ts');
    expect(get(activeFilePath)).toBe('/project/src/main.ts');
  });

  it('addOpenFile does not duplicate already-open files', () => {
    setupTestProject({ rootPath: '/project' });
    addOpenFile('/project/src/main.ts', '// /project/src/main.ts');
    addOpenFile('/project/src/main.ts', '// /project/src/main.ts');

    expect(get(openFiles)).toHaveLength(1);
  });

  it('addOpenFile switches active file without duplicating', () => {
    setupTestProject({ rootPath: '/project' });
    addOpenFile('/project/a.ts', '// /project/a.ts');
    addOpenFile('/project/b.ts', '// /project/b.ts');
    addOpenFile('/project/a.ts', '// /project/a.ts'); // re-open already open file

    expect(get(openFiles)).toHaveLength(2);
    expect(get(activeFilePath)).toBe('/project/a.ts');
  });

  it('closeFile removes the file from openFiles', () => {
    setupTestProject({ rootPath: '/project' });
    addOpenFile('/project/a.ts', '// /project/a.ts');
    addOpenFile('/project/b.ts', '// /project/b.ts');

    closeFile('/project/a.ts');

    const files = get(openFiles);
    expect(files).toHaveLength(1);
    expect(files[0].path).toBe('/project/b.ts');
  });

  it('closeFile clears activeFilePath when active file is closed', () => {
    setupTestProject({ rootPath: '/project' });
    addOpenFile('/project/main.ts');
    expect(get(activeFilePath)).toBe('/project/main.ts');

    closeFile('/project/main.ts');
    expect(get(activeFilePath)).toBeNull();
  });

  it('closeFile keeps activeFilePath when a different file is closed', () => {
    setupTestProject({ rootPath: '/project' });
    addOpenFile('/project/a.ts');
    addOpenFile('/project/b.ts');
    // fileB is now active
    closeFile('/project/a.ts');

    expect(get(activeFilePath)).toBe('/project/b.ts');
  });

  it('projectRoot reflects the active project rootPath', () => {
    setupTestProject({ rootPath: '/home/user/myproject' });
    expect(get(projectRoot)).toBe('/home/user/myproject');
  });
});
