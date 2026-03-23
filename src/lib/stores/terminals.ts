import { writable } from 'svelte/store';

export interface TerminalInstance {
  id: string;        // PTY id (or synthetic id for API-only instances)
  llmName: string;
  label: string;     // "inst. 1", "inst. 2", etc.
  apiOnly?: boolean; // true for API-type LLMs (no PTY, chat view only)
  contextPercent?: number;
  tokenCount?: string;
  activeMode?: string;
  modelName?: string;
  messagesLeft?: number;
  resetTimer?: string;
  progressState?: number;  // 0=remove, 1=normal, 2=error, 3=indeterminate, 4=paused
  progressValue?: number;  // 0-100
}

export interface TerminalTab {
  llmName: string;
  instances: TerminalInstance[];
}

export const terminalTabs = writable<TerminalTab[]>([]);
export const activeTerminalTab = writable<string | null>(null);
export const activeInstanceId = writable<string | null>(null);
