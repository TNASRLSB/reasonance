import { writable } from 'svelte/store';

export interface OpenFile {
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
  isDeleted: boolean;
}

export const openFiles = writable<OpenFile[]>([]);
export const activeFilePath = writable<string | null>(null);
export const projectRoot = writable<string>('');
export const recentProjects = writable<string[]>([]);

export function addRecentProject(path: string) {
  recentProjects.update((list) => {
    const filtered = list.filter((p) => p !== path);
    return [path, ...filtered].slice(0, 10);
  });
}

export function addOpenFile(file: OpenFile) {
  openFiles.update((files) => {
    if (files.some((f) => f.path === file.path)) {
      // Already open — just switch to it
      return files;
    }
    return [...files, file];
  });
  activeFilePath.set(file.path);
}

/** Pending line number to scroll to after opening a file */
export const pendingLine = writable<number | null>(null);

/** Current cursor position in the active editor */
export const cursorLine = writable<number>(1);
export const cursorCol = writable<number>(1);

export function closeFile(path: string) {
  openFiles.update((files) => files.filter((f) => f.path !== path));
  activeFilePath.update((active) => (active === path ? null : active));
}
