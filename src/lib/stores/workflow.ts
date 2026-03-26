import { writable } from 'svelte/store';
import type { Workflow } from '$lib/adapter/index';

export const currentWorkflow = writable<Workflow | null>(null);
export const currentWorkflowPath = writable<string | null>(null);
export const workflowDirty = writable<boolean>(false);
