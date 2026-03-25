import { derived, get, type Readable } from 'svelte/store';
import { projects, activeProjectId, updateProjectContext } from './registry';
import type { ProjectFileState, TerminalInstanceMeta } from './types';

// --- Derived stores (read-only, project-scoped) ---

export const projectRoot: Readable<string> = derived(
  [projects, activeProjectId],
  ([$projects, $id]) => $id ? ($projects.get($id)?.rootPath ?? '') : ''
);

export const openFiles: Readable<ProjectFileState[]> = derived(
  [projects, activeProjectId],
  ([$projects, $id]) => $id ? ($projects.get($id)?.openFiles ?? []) : []
);

export const activeFilePath: Readable<string | null> = derived(
  [projects, activeProjectId],
  ([$projects, $id]) => $id ? ($projects.get($id)?.activeFilePath ?? null) : null
);

export const terminalInstances: Readable<TerminalInstanceMeta[]> = derived(
  [projects, activeProjectId],
  ([$projects, $id]) => $id ? ($projects.get($id)?.terminalInstances ?? []) : []
);

export const activeTerminalId: Readable<string | null> = derived(
  [projects, activeProjectId],
  ([$projects, $id]) => $id ? ($projects.get($id)?.activeTerminalId ?? null) : null
);

export const projectTrustLevel: Readable<string> = derived(
  [projects, activeProjectId],
  ([$projects, $id]) => $id ? ($projects.get($id)?.trustLevel ?? 'blocked') : 'blocked'
);

// Per-project computedLabels (scoped, not global)
export const computedLabels: Readable<Map<string, string>> = derived(
  terminalInstances,
  ($instances) => {
    const nameCount = new Map<string, number>();
    const nameIndex = new Map<string, number>();
    const labels = new Map<string, string>();

    for (const inst of $instances) {
      const name = inst.modelName ?? inst.provider;
      nameCount.set(name, (nameCount.get(name) ?? 0) + 1);
    }

    for (const inst of $instances) {
      const name = inst.modelName ?? inst.provider;
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
  }
);

// --- Action wrappers (operate on active project) ---

export function openFile(path: string, content: string = ''): void {
  const id = get(activeProjectId);
  if (!id) return;

  const map = get(projects);
  const ctx = map.get(id);
  if (!ctx) return;

  const alreadyOpen = ctx.openFiles.some(f => f.path === path);
  updateProjectContext(id, {
    openFiles: alreadyOpen
      ? ctx.openFiles
      : [...ctx.openFiles, {
          path,
          name: path.split('/').pop() ?? path,
          content,
          isDirty: false,
          isDeleted: false,
          scrollPosition: { line: 0, col: 0 },
          cursorPosition: { line: 0, col: 0 },
        }],
    activeFilePath: path,
  });
}

export function closeFile(path: string): void {
  const id = get(activeProjectId);
  if (!id) return;

  const map = get(projects);
  const ctx = map.get(id);
  if (!ctx) return;

  const remaining = ctx.openFiles.filter(f => f.path !== path);
  const newActive = ctx.activeFilePath === path
    ? (remaining.length > 0 ? remaining[remaining.length - 1].path : null)
    : ctx.activeFilePath;

  updateProjectContext(id, {
    openFiles: remaining,
    activeFilePath: newActive,
  });
}

export function setActiveFile(path: string): void {
  const id = get(activeProjectId);
  if (!id) return;
  updateProjectContext(id, { activeFilePath: path });
}

export function updateFileContent(path: string, content: string, isDirty: boolean): void {
  const id = get(activeProjectId);
  if (!id) return;

  const map = get(projects);
  const ctx = map.get(id);
  if (!ctx) return;

  updateProjectContext(id, {
    openFiles: ctx.openFiles.map(f =>
      f.path === path ? { ...f, content, isDirty } : f
    ),
  });
}

export function updateFileState(path: string, partial: Partial<ProjectFileState>): void {
  const id = get(activeProjectId);
  if (!id) return;

  const map = get(projects);
  const ctx = map.get(id);
  if (!ctx) return;

  updateProjectContext(id, {
    openFiles: ctx.openFiles.map(f =>
      f.path === path ? { ...f, ...partial } : f
    ),
  });
}

export function addTerminalInstance(instance: TerminalInstanceMeta): void {
  const id = get(activeProjectId);
  if (!id) return;

  const map = get(projects);
  const ctx = map.get(id);
  if (!ctx) return;

  updateProjectContext(id, {
    terminalInstances: [...ctx.terminalInstances, { ...instance, projectId: id }],
    activeTerminalId: instance.id,
  });
}

export function removeTerminalInstance(instanceId: string): void {
  const id = get(activeProjectId);
  if (!id) return;

  const map = get(projects);
  const ctx = map.get(id);
  if (!ctx) return;

  const remaining = ctx.terminalInstances.filter(t => t.id !== instanceId);
  const newActive = ctx.activeTerminalId === instanceId
    ? (remaining.length > 0 ? remaining[remaining.length - 1].id : null)
    : ctx.activeTerminalId;

  updateProjectContext(id, {
    terminalInstances: remaining,
    activeTerminalId: newActive,
  });
}

export function updateTerminalInstance(instanceId: string, patch: Partial<TerminalInstanceMeta>): void {
  const id = get(activeProjectId);
  if (!id) return;

  const map = get(projects);
  const ctx = map.get(id);
  if (!ctx) return;

  updateProjectContext(id, {
    terminalInstances: ctx.terminalInstances.map(t =>
      t.id === instanceId ? { ...t, ...patch } : t
    ),
  });
}

export function setActiveTerminal(instanceId: string): void {
  const id = get(activeProjectId);
  if (!id) return;
  updateProjectContext(id, { activeTerminalId: instanceId });
}

export function addAgentSession(sessionId: string): void {
  const id = get(activeProjectId);
  if (!id) return;

  const map = get(projects);
  const ctx = map.get(id);
  if (!ctx) return;

  if (!ctx.agentSessionIds.includes(sessionId)) {
    updateProjectContext(id, {
      agentSessionIds: [...ctx.agentSessionIds, sessionId],
    });
  }
}
