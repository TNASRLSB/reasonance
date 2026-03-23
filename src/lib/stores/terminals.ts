import { writable, derived } from 'svelte/store';

export interface TerminalInstance {
  id: string;
  provider: string;       // was llmName
  label: string;          // display label (computed externally)
  modelName?: string;     // resolved from model_path in first response
  apiOnly?: boolean;
  contextPercent?: number;
  tokenCount?: string;
  activeMode?: string;
  messagesLeft?: number;
  resetTimer?: string;
  progressState?: number;
  progressValue?: number;
}

// Flat list of all instances (replaces terminalTabs)
export const terminalInstances = writable<TerminalInstance[]>([]);

// Active instance ID (unchanged)
export const activeInstanceId = writable<string | null>(null);

// Derived: active instance data
export const activeInstance = derived(
  [terminalInstances, activeInstanceId],
  ([$instances, $id]) => $id ? $instances.find(i => i.id === $id) ?? null : null
);

// Derived: computed tab labels with dedup numbering
// Uses modelName when resolved, falls back to "Provider ..." while awaiting first response
export const computedLabels = derived(terminalInstances, ($instances) => {
  const nameCount = new Map<string, number>();
  for (const inst of $instances) {
    const name = inst.modelName ?? inst.provider;
    nameCount.set(name, (nameCount.get(name) ?? 0) + 1);
  }

  const nameIndex = new Map<string, number>();
  const labels = new Map<string, string>();
  for (const inst of $instances) {
    const name = inst.modelName ?? inst.provider;
    // Show "Provider ..." suffix when model name hasn't resolved yet
    const displayName = inst.modelName ? name : `${name} ...`;
    const count = nameCount.get(name) ?? 1;
    if (count === 1) {
      labels.set(inst.id, displayName);
    } else {
      const idx = (nameIndex.get(name) ?? 0) + 1;
      nameIndex.set(name, idx);
      labels.set(inst.id, `${name} ${idx}`);
    }
  }
  return labels;
});

// Helper: add an instance
export function addInstance(instance: TerminalInstance): void {
  terminalInstances.update(list => [...list, instance]);
}

// Helper: remove an instance
export function removeInstance(id: string): void {
  terminalInstances.update(list => list.filter(i => i.id !== id));
}

// Helper: update an instance by id
export function updateInstance(id: string, patch: Partial<TerminalInstance>): void {
  terminalInstances.update(list =>
    list.map(i => i.id === id ? { ...i, ...patch } : i)
  );
}
