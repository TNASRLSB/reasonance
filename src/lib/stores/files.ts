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

export function closeFile(path: string) {
  openFiles.update((files) => files.filter((f) => f.path !== path));
  activeFilePath.update((active) => (active === path ? null : active));
}
