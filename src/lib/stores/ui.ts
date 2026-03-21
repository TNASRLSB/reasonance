import { writable } from 'svelte/store';

export const fileTreeWidth = writable(250);
export const terminalWidth = writable(500);
export const activeEditorTab = writable<string | null>(null);
export const showSettings = writable(false);
export const yoloMode = writable(false);
export const showDiff = writable(false);
export const fontFamily = writable("'JetBrains Mono', 'Fira Code', monospace");
export const fontSize = writable(14);
