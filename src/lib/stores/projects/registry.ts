import { writable, get } from 'svelte/store';
import type { ProjectContext, RecentProject } from './types';

// --- Core state ---
export const projects = writable<Map<string, ProjectContext>>(new Map());
export const activeProjectId = writable<string | null>(null);
export const recentProjectsList = writable<RecentProject[]>([]);

// --- Color palette for auto-assignment ---
const PROJECT_COLORS = [
  '#4A9EFF', '#FF6B6B', '#4ADE80', '#FBBF24', '#A78BFA',
  '#F472B6', '#34D399', '#FB923C', '#60A5FA', '#C084FC',
];

let colorIndex = 0;
function nextColor(): string {
  const color = PROJECT_COLORS[colorIndex % PROJECT_COLORS.length];
  colorIndex++;
  return color;
}

// --- Project lifecycle ---

export function addProject(rootPath: string): ProjectContext {
  const normalized = rootPath.replace(/\/+$/, '');

  // Check for duplicates
  const existing = [...get(projects).values()].find(p => p.rootPath === normalized);
  if (existing) {
    activeProjectId.set(existing.id);
    return existing;
  }

  const id = crypto.randomUUID();
  const label = normalized.split('/').pop() ?? normalized;
  const currentProjects = get(projects);
  const maxSort = Math.max(0, ...[...currentProjects.values()].map(p => p.sortOrder));

  const ctx: ProjectContext = {
    id,
    rootPath: normalized,
    label,
    color: nextColor(),
    sortOrder: maxSort + 1000,
    addedAt: Date.now(),
    pinned: false,
    trustLevel: 'trusted',
    openFiles: [],
    activeFilePath: null,
    terminalInstances: [],
    activeTerminalId: null,
    agentSessionIds: [],
    fileTreeState: { collapsed: false, expandedDirs: [] },
    gitState: null,
  };

  projects.update(map => {
    const next = new Map(map);
    next.set(id, ctx);
    return next;
  });

  activeProjectId.set(id);
  return ctx;
}

export function removeProject(id: string): void {
  const current = get(activeProjectId);

  if (current === id) {
    const all = [...get(projects).values()]
      .filter(p => p.id !== id)
      .sort((a, b) => a.sortOrder - b.sortOrder);
    activeProjectId.set(all.length > 0 ? all[0].id : null);
  }

  const ctx = get(projects).get(id);
  if (ctx) {
    recentProjectsList.update(list => {
      const filtered = list.filter(r => r.path !== ctx.rootPath);
      return [
        { path: ctx.rootPath, label: ctx.label, color: ctx.color, lastOpened: Date.now() },
        ...filtered,
      ].slice(0, 20);
    });
  }

  projects.update(map => {
    const next = new Map(map);
    next.delete(id);
    return next;
  });
}

export function switchProject(targetId: string): void {
  const map = get(projects);
  if (!map.has(targetId)) return;
  activeProjectId.set(targetId);
}

export function reorderProject(id: string, newSortOrder: number): void {
  projects.update(map => {
    const ctx = map.get(id);
    if (!ctx) return map;
    const next = new Map(map);
    next.set(id, { ...ctx, sortOrder: newSortOrder });
    return next;
  });
}

export function updateProjectContext(id: string, partial: Partial<ProjectContext>): void {
  projects.update(map => {
    const ctx = map.get(id);
    if (!ctx) return map;
    const next = new Map(map);
    next.set(id, { ...ctx, ...partial });
    return next;
  });
}
