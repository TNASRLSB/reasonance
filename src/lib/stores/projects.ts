/**
 * projects.ts — Multi-project state store for Reasonance.
 *
 * Owns:
 *   - Per-project file state (open files, active file, root path)
 *   - Project registry (list of known projects)
 *   - Recent projects list
 *
 * The legacy files.ts re-exports the single-project symbols from here
 * so existing components continue to work unchanged.
 */

import { writable, derived, get } from 'svelte/store';

// ─────────────────────────────────────────────────────────────────────────────
// File-level types (previously in files.ts)
// ─────────────────────────────────────────────────────────────────────────────

export interface ProjectFileState {
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
  isDeleted: boolean;
}

/** @deprecated Use ProjectFileState */
export type OpenFile = ProjectFileState;

// ─────────────────────────────────────────────────────────────────────────────
// Project-level types
// ─────────────────────────────────────────────────────────────────────────────

export interface RecentProject {
  path: string;
  label: string;
  color: string;
  lastOpened: number;
}

export interface Project {
  id: string;
  label: string;
  rootPath: string;
  color: string;
  lastOpened: number;
  pinned?: boolean;
}

export interface ProjectSummary {
  id: string;
  label: string;
  rootPath: string;
  color: string;
  isActive: boolean;
  pinned: boolean;
  hasUnsavedChanges: boolean;
  hasRunningAgent: boolean;
  activeTerminals: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Persistence helpers
// ─────────────────────────────────────────────────────────────────────────────

const PROJECTS_KEY = 'reasonance:projects';
const RECENT_KEY = 'reasonance:recentProjects';
const ACTIVE_PROJECT_KEY = 'reasonance:activeProjectId';

function loadProjects(): Project[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    const raw = localStorage.getItem(PROJECTS_KEY);
    return raw ? (JSON.parse(raw) as Project[]) : [];
  } catch {
    return [];
  }
}

function saveProjects(projects: Project[]) {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(PROJECTS_KEY, JSON.stringify(projects));
  }
}

function loadRecent(): RecentProject[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    return raw ? (JSON.parse(raw) as RecentProject[]) : [];
  } catch {
    return [];
  }
}

function saveRecent(recent: RecentProject[]) {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(RECENT_KEY, JSON.stringify(recent));
  }
}

function loadActiveProjectId(): string | null {
  if (typeof localStorage === 'undefined') return null;
  return localStorage.getItem(ACTIVE_PROJECT_KEY);
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal state
// ─────────────────────────────────────────────────────────────────────────────

const _projects = writable<Project[]>(loadProjects());
const _recent = writable<RecentProject[]>(loadRecent());
export const activeProjectId = writable<string | null>(loadActiveProjectId());

// Single-project compat: open files, active file path, project root
export const openFiles = writable<ProjectFileState[]>([]);
export const activeFilePath = writable<string | null>(null);
export const projectRoot = writable<string | null>(null);

// ─────────────────────────────────────────────────────────────────────────────
// Derived stores
// ─────────────────────────────────────────────────────────────────────────────

export const recentProjectsList = derived(_recent, ($r) =>
  [...$r].sort((a, b) => b.lastOpened - a.lastOpened).slice(0, 10)
);

export const projectSummaries = derived(
  [_projects, activeProjectId, openFiles],
  ([$projects, $activeId, $openFiles]): ProjectSummary[] => {
    const hasUnsaved = $openFiles.some((f) => f.isDirty);
    return $projects.map((p) => ({
      id: p.id,
      label: p.label,
      rootPath: p.rootPath,
      color: p.color,
      isActive: p.id === $activeId,
      pinned: p.pinned ?? false,
      hasUnsavedChanges: p.id === $activeId ? hasUnsaved : false,
      hasRunningAgent: false,
      activeTerminals: 0,
    }));
  }
);

// ─────────────────────────────────────────────────────────────────────────────
// Project actions
// ─────────────────────────────────────────────────────────────────────────────

export function addProject(path: string, label?: string, color = '#6e6e6e') {
  const projectLabel = label ?? path.split('/').pop() ?? path;

  let existingId: string | null = null;

  _projects.update((projects) => {
    const existing = projects.find((p) => p.rootPath === path);
    if (existing) {
      existingId = existing.id;
      const updated = projects.map((p) =>
        p.rootPath === path ? { ...p, lastOpened: Date.now() } : p
      );
      saveProjects(updated);
      return updated;
    }
    const id = `proj-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    existingId = id;
    const next = [...projects, { id, label: projectLabel, rootPath: path, color, lastOpened: Date.now() }];
    saveProjects(next);
    return next;
  });

  _recent.update((recent) => {
    const filtered = recent.filter((r) => r.path !== path);
    const next = [{ path, label: projectLabel, color, lastOpened: Date.now() }, ...filtered].slice(0, 10);
    saveRecent(next);
    return next;
  });

  // Set as active and update root
  if (existingId) {
    activeProjectId.set(existingId);
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(ACTIVE_PROJECT_KEY, existingId);
    }
  }
  projectRoot.set(path);
}

export function removeProject(id: string) {
  _projects.update((projects) => {
    const next = projects.filter((p) => p.id !== id);
    saveProjects(next);
    return next;
  });

  // If we removed the active project, clear active state
  const currentActive = get(activeProjectId);
  if (currentActive === id) {
    const remaining = get(_projects);
    const next = remaining.length > 0 ? remaining[remaining.length - 1] : null;
    activeProjectId.set(next?.id ?? null);
    projectRoot.set(next?.rootPath ?? null);
    if (typeof localStorage !== 'undefined') {
      if (next?.id) localStorage.setItem(ACTIVE_PROJECT_KEY, next.id);
      else localStorage.removeItem(ACTIVE_PROJECT_KEY);
    }
  }
}

export function switchProject(id: string) {
  const projects = get(_projects);
  const project = projects.find((p) => p.id === id);
  if (!project) return;

  activeProjectId.set(id);
  projectRoot.set(project.rootPath);

  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(ACTIVE_PROJECT_KEY, id);
  }

  _projects.update((ps) => {
    const updated = ps.map((p) =>
      p.id === id ? { ...p, lastOpened: Date.now() } : p
    );
    saveProjects(updated);
    return updated;
  });

  document.dispatchEvent(
    new CustomEvent('reasonance:switchProject', { detail: { path: project.rootPath } })
  );
}

export function updateProjectRoot(id: string, newPath: string) {
  _projects.update((projects) => {
    const next = projects.map((p) =>
      p.id === id ? { ...p, rootPath: newPath, lastOpened: Date.now() } : p
    );
    saveProjects(next);
    return next;
  });

  const currentActive = get(activeProjectId);
  if (currentActive === id) {
    projectRoot.set(newPath);
  }
}

export function updateProjectContext(
  id: string,
  patch: Partial<Pick<Project, 'label' | 'color' | 'pinned'>>
) {
  _projects.update((projects) => {
    const next = projects.map((p) =>
      p.id === id ? { ...p, ...patch } : p
    );
    saveProjects(next);
    return next;
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// File actions (single-project compat layer — previously in files.ts)
// ─────────────────────────────────────────────────────────────────────────────

export function openFile(file: ProjectFileState) {
  openFiles.update((files) => {
    const idx = files.findIndex((f) => f.path === file.path);
    if (idx !== -1) {
      // Already open — just switch to it
      activeFilePath.set(file.path);
      return files;
    }
    activeFilePath.set(file.path);
    return [...files, file];
  });
}

export function closeFile(path: string) {
  openFiles.update((files) => {
    const remaining = files.filter((f) => f.path !== path);
    const current = get(activeFilePath);
    if (current === path) {
      const idx = files.findIndex((f) => f.path === path);
      const next = remaining[Math.min(idx, remaining.length - 1)];
      activeFilePath.set(next?.path ?? null);
    }
    return remaining;
  });
}

export function setActiveFile(path: string) {
  activeFilePath.set(path);
}

export function updateFileContent(path: string, content: string, isDirty = true) {
  openFiles.update((files) =>
    files.map((f) => (f.path === path ? { ...f, content, isDirty } : f))
  );
}

export function updateFileState(path: string, patch: Partial<ProjectFileState>) {
  openFiles.update((files) =>
    files.map((f) => (f.path === path ? { ...f, ...patch } : f))
  );
}
