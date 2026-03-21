import { writable, derived } from 'svelte/store';
import type { WorkflowRun, RunStatus, NodeRunState } from '$lib/adapter/index';

export const currentRun = writable<WorkflowRun | null>(null);
export const currentRunId = writable<string | null>(null);

export const runStatus = derived(currentRun, ($run): RunStatus => $run?.status ?? 'idle');

export const nodeStates = derived(currentRun, ($run): NodeRunState[] =>
  $run ? Object.values($run.node_states) : []
);

export const completedNodeCount = derived(nodeStates, ($states) =>
  $states.filter(s => s.state === 'success').length
);

export const totalNodeCount = derived(nodeStates, ($states) => $states.length);

export const activeNodeCount = derived(nodeStates, ($states) =>
  $states.filter(s => s.state === 'running').length
);

export const errorNodeCount = derived(nodeStates, ($states) =>
  $states.filter(s => s.state === 'error').length
);

export const statusSummary = derived(
  [completedNodeCount, totalNodeCount, activeNodeCount, errorNodeCount],
  ([$completed, $total, $active, $errors]) => {
    const parts: string[] = [];
    parts.push(`${$completed}/${$total} complete`);
    if ($active > 0) parts.push(`${$active} active`);
    if ($errors > 0) parts.push(`${$errors} error${$errors > 1 ? 's' : ''}`);
    return parts.join(', ');
  }
);
