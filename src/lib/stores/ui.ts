import { writable } from 'svelte/store';

export const fileTreeWidth = writable(200);
export const terminalWidth = writable(300);
export const activeEditorTab = writable<string | null>(null);
export const showSettings = writable(false);
export const yoloMode = writable(false);
export const showDiff = writable(false);
export const fontFamily = writable("'Atkinson Hyperlegible Next', system-ui, sans-serif");
export const fontSize = writable(13);
export const enhancedReadability = writable(false);
