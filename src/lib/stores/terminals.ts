// SHIM: Re-exports from projects namespace layer.

// Legacy type — kept for backward compat
export { type TerminalInstanceMeta as TerminalInstance } from './projects';

// Re-export from namespace
export {
  terminalInstances,
  activeTerminalId as activeInstanceId,
  computedLabels,
  addTerminalInstance as addInstance,
  removeTerminalInstance as removeInstance,
  updateTerminalInstance as updateInstance,
  setActiveTerminal,
} from './projects';

// Derived store for active instance (convenience)
import { derived } from 'svelte/store';
import { terminalInstances, activeTerminalId } from './projects';

export const activeInstance = derived(
  [terminalInstances, activeTerminalId],
  ([$instances, $activeId]) => $instances.find(i => i.id === $activeId) ?? null
);
