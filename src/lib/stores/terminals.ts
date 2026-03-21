import { writable } from 'svelte/store';

export interface TerminalInstance {
  id: string;        // PTY id
  llmName: string;
  label: string;     // "inst. 1", "inst. 2", etc.
  contextPercent?: number;
  tokenCount?: string;
  activeMode?: string;
}

export interface TerminalTab {
  llmName: string;
  instances: TerminalInstance[];
}

export const terminalTabs = writable<TerminalTab[]>([]);
export const activeTerminalTab = writable<string | null>(null);
export const activeInstanceId = writable<string | null>(null);
