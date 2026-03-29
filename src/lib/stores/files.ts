// SHIM: This file re-exports from the projects namespace layer.
// Components importing from here continue to work unchanged.

import { writable } from 'svelte/store';
import type { EditorView } from '@codemirror/view';

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

/**
 * W3.5 — Anchor-based navigation.
 *
 * activeEditorView: the live EditorView instance (null when no editor is mounted).
 * FindInFiles dispatches setAnchors effects through this view and sets
 * pendingAnchorIndex to navigate to a specific anchor position.
 *
 * pendingAnchorIndex: index into the searchAnchorsField positions array.
 * The Editor consumes this and clears it (same pattern as pendingLine).
 */
export const activeEditorView = writable<EditorView | null>(null);
export const pendingAnchorIndex = writable<number | null>(null);
