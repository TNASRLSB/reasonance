// SHIM: This file re-exports from the projects namespace layer.
// Components importing from here continue to work unchanged.

import { writable } from 'svelte/store';

/** @deprecated Use ProjectFileState from '$lib/stores/projects' */
export interface OpenFile {
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
  isDeleted: boolean;
}

// Re-export from namespace layer
export {
  projectRoot,
  openFiles,
  activeFilePath,
  openFile as addOpenFile,
  closeFile,
} from './projects';

export {
  recentProjectsList as recentProjects,
} from './projects';

// These cursor stores remain global (not per-project — they track the current editor cursor)
export const pendingLine = writable<number | null>(null);
export const cursorLine = writable<number>(1);
export const cursorCol = writable<number>(1);
