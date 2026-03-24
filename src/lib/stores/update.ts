import { writable } from 'svelte/store';

export interface UpdateState {
  /** null = no update available, or not checked yet */
  newVersion: string | null;
  /** Download progress 0-100, null = not downloading */
  downloadProgress: number | null;
  /** The update object from Tauri for installing */
  updateHandle: unknown | null;
}

export const updateState = writable<UpdateState>({
  newVersion: null,
  downloadProgress: null,
  updateHandle: null,
});
