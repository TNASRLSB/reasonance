import { writable } from 'svelte/store';

function persistedWidth(key: string, fallback: number) {
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(key) : null;
  const initial = stored ? Number(stored) : fallback;
  const store = writable(isNaN(initial) ? fallback : initial);
  store.subscribe((v) => {
    if (typeof localStorage !== 'undefined') localStorage.setItem(key, String(v));
  });
  return store;
}

export const fileTreeWidth = persistedWidth('reasonance:fileTreeWidth', 200);
export const terminalWidth = persistedWidth('reasonance:terminalWidth', 300);
export const activeEditorTab = writable<string | null>(null);
export const showSettings = writable(false);
export const showDiff = writable(false);
export const fontFamily = writable("'Atkinson Hyperlegible Mono', monospace");
export const fontSize = writable(14);
export const enhancedReadability = writable(false);
export const editorTheme = writable<string>('forge-dark');

export const cursorBlink = writable(false);

// Swarm canvas state
export const showSwarmCanvas = writable<boolean>(false);
export const swarmViewMode = writable<'visual' | 'code' | 'split'>('visual');
export const selectedNodeId = writable<string | null>(null);

import type { AnalyticsDashboardState } from '$lib/types/analytics';
export const analyticsDashboard = writable<AnalyticsDashboardState>({ open: false, focus: null });
