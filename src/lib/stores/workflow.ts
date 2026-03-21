import { writable, derived } from 'svelte/store';
import type { Workflow } from '$lib/adapter/index';

export const currentWorkflow = writable<Workflow | null>(null);
export const currentWorkflowPath = writable<string | null>(null);
export const workflowDirty = writable<boolean>(false);
export const workflowList = writable<string[]>([]);
export const nodeCount = derived(currentWorkflow, ($wf) => $wf?.nodes.length ?? 0);
export const edgeCount = derived(currentWorkflow, ($wf) => $wf?.edges.length ?? 0);
