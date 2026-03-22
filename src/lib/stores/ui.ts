import { writable } from 'svelte/store';

export const fileTreeWidth = writable(200);
export const terminalWidth = writable(300);
export const activeEditorTab = writable<string | null>(null);
export const showSettings = writable(false);
export const yoloMode = writable(false);
export const showDiff = writable(false);
export const fontFamily = writable("'Atkinson Hyperlegible Mono', monospace");
export const fontSize = writable(14);
export const enhancedReadability = writable(false);
export const editorTheme = writable<string>('forge-dark');

// Swarm canvas state
export const showSwarmCanvas = writable<boolean>(false);
export const swarmViewMode = writable<'visual' | 'code' | 'split'>('visual');
export const selectedNodeId = writable<string | null>(null);

import type { AnalyticsDashboardState } from '$lib/types/analytics';
export const analyticsDashboard = writable<AnalyticsDashboardState>({ open: false, focus: null });
