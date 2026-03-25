import { derived, type Readable } from 'svelte/store';
import { projects, activeProjectId } from './registry';
import { agentSessions } from '$lib/stores/agent-session';
import type { ProjectSummary } from './types';

export interface ProjectStatus {
  hasRunningAgent: boolean;
  unsavedCount: number;
  terminalCount: number;
}

export const projectSummaries: Readable<ProjectSummary[]> = derived(
  [projects, activeProjectId, agentSessions],
  ([$projects, $activeId, $sessions]) => {
    return [...$projects.values()]
      .sort((a, b) => {
        if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
        return a.sortOrder - b.sortOrder;
      })
      .map(ctx => ({
        id: ctx.id,
        label: ctx.label,
        color: ctx.color,
        rootPath: ctx.rootPath,
        sortOrder: ctx.sortOrder,
        pinned: ctx.pinned,
        isActive: ctx.id === $activeId,
        hasUnsavedChanges: ctx.openFiles.some(f => f.isDirty),
        activeTerminals: ctx.terminalInstances.length,
        hasRunningAgent: ctx.agentSessionIds.some(sid => {
          const s = $sessions.get(sid);
          return s?.status === 'streaming' || s?.status === 'active';
        }),
      }));
  }
);

export const projectStatuses: Readable<Map<string, ProjectStatus>> = derived(
  [projects, agentSessions],
  ([$projects, $sessions]) => {
    const statuses = new Map<string, ProjectStatus>();
    for (const [id, ctx] of $projects) {
      statuses.set(id, {
        hasRunningAgent: ctx.agentSessionIds.some(sid => {
          const s = $sessions.get(sid);
          return s?.status === 'streaming' || s?.status === 'active';
        }),
        unsavedCount: ctx.openFiles.filter(f => f.isDirty).length,
        terminalCount: ctx.terminalInstances.length,
      });
    }
    return statuses;
  }
);
