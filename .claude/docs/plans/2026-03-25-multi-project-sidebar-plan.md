# Multi-Project Sidebar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add multi-project support with a sidebar project switcher, isolated per-project state, and background PTY persistence.

**Architecture:** Namespace layer pattern (A+) — a flat `Map<string, ProjectContext>` store with derived stores that expose only the active project's data. Components import the same store API as today. 3-phase migration: shim layer → multi-project store → sidebar UI.

**Tech Stack:** Svelte 5, TypeScript, Tauri v2 (Rust), xterm.js, CodeMirror 6, CSS custom properties (theme system).

**Spec:** `.claude/docs/specs/2026-03-25-multi-project-sidebar-design.md`

---

## File Structure

### New files

| File | Responsibility |
|------|----------------|
| `src/lib/stores/projects/types.ts` | `ProjectContext`, `ProjectFileState`, `ProjectSummary` interfaces |
| `src/lib/stores/projects/registry.ts` | `projects` store (Map), `activeProjectId`, add/remove/switch actions |
| `src/lib/stores/projects/namespace.ts` | Derived stores (projectRoot, openFiles, etc.) + action wrappers |
| `src/lib/stores/projects/persistence.ts` | Session v2 read/write/migrate, atomic save |
| `src/lib/stores/projects/sidebar.ts` | `projectSummaries`, `projectStatuses` derived stores |
| `src/lib/components/project/ProjectSidebar.svelte` | Sidebar column: tabs, drag-drop, indicators |
| `src/lib/components/project/ProjectQuickSwitcher.svelte` | `Ctrl+Shift+E` overlay with fuzzy search |
| `src/lib/components/project/ProjectAddMenu.svelte` | "+" menu: browse, recents |
| `src/lib/components/project/ProjectDisconnectedDialog.svelte` | Missing folder dialog |
| `src-tauri/src/project_manager.rs` | Rust: multi-project state, watcher manager |
| `src-tauri/src/session_io.rs` | Rust: atomic session.json write/read |

### Modified files

| File | What changes |
|------|-------------|
| `src/lib/stores/files.ts` | Exports become re-exports from namespace layer |
| `src/lib/stores/terminals.ts` | `TerminalInstance` gets `projectId` field; exports become re-exports |
| `src/lib/stores/agent-session.ts` | `AgentSessionState` gets `projectId` field |
| `src/lib/stores/workspace-trust.ts` | Trust becomes per-project path |
| `src/lib/stores/ui.ts` | Add `sidebarCollapsed` persisted store |
| `src/lib/stores/index.ts` | Create if needed — re-export consumer API (optional, components can import from `$lib/stores/files` or `$lib/stores/projects` directly) |
| `src/lib/components/App.svelte` | Add ProjectSidebar column, skip-link, keyboard handlers |
| `src/lib/components/MenuBar.svelte` | Populate Recent submenu, add Close Project |
| `src/lib/components/TerminalManager.svelte` | Import from namespace, filter by project |
| `src/lib/components/chat/ChatView.svelte` | Pass `projectId` to `upsertSession()` |
| `src/lib/adapter/tauri.ts` | New methods: addProject, removeProject, setActiveProject, projectId on file ops |
| `src/lib/utils/session.ts` | Replace plugin-store with custom persistence; v1→v2 migration |
| `src/routes/+page.svelte` | Multi-project startup, updated openFolder handler |
| `src-tauri/src/lib.rs` | Register project_manager, session_io; update managed state; CLI args |
| `src-tauri/src/pty_manager.rs` | Add `project_id` to PtyInstance; `kill_project_ptys()` |
| `src/lib/themes/reasonance-dark.json` | Add sidebar CSS variables |
| `src/lib/themes/reasonance-light.json` | Add sidebar CSS variables |
| `src/lib/themes/theme-schema.json` | Add sidebar variables to required sections; schemaVersion 2 |
| `src/lib/engine/theme-validator.ts` | CURRENT_SCHEMA_VERSION → 2; v1 migration |
| `src/lib/engine/theme-engine.ts` | v1→v2 theme migration on load |
| `src/lib/engine/fallback-theme.ts` | Add sidebar variable defaults |
| `src/lib/themes/enhanced-readability.json` | Sidebar spacing overrides |
| `src/lib/themes/_high-contrast.json` | Sidebar contrast overrides |
| `src/lib/themes/_reduced-motion.json` | Disable sidebar animations |
| `src/lib/components/theme-editor/ThemeStartDialog.svelte` | Template includes sidebar section |

---

## Phase 1: Shim Layer (zero visible changes)

Goal: replace direct store usage with namespace layer, keeping single-project behavior identical.

---

### Task 1: Project types

**Files:**
- Create: `src/lib/stores/projects/types.ts`

- [ ] **Step 1: Create types file**

```typescript
// src/lib/stores/projects/types.ts
import type { TrustLevel } from '$lib/stores/workspace-trust';

export interface ProjectFileState {
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
  isDeleted: boolean;
  scrollPosition: { line: number; col: number };
  cursorPosition: { line: number; col: number };
}

export interface ProjectContext {
  id: string;
  rootPath: string;
  label: string;
  color: string;
  sortOrder: number;
  addedAt: number;
  pinned: boolean;
  trustLevel: TrustLevel;
  openFiles: ProjectFileState[];
  activeFilePath: string | null;
  terminalInstances: TerminalInstanceMeta[];
  activeTerminalId: string | null;
  agentSessionIds: string[];
  fileTreeState: {
    collapsed: boolean;
    expandedDirs: string[];
  };
  gitState: {
    branch: string | null;
    hasChanges: boolean;
    remote: string | null;
  } | null;
}

export interface TerminalInstanceMeta {
  id: string;
  provider: string;
  label?: string;
  modelName?: string;
  apiOnly?: boolean;
  projectId: string;
  // Runtime metadata (updated frequently by normalizer)
  contextPercent?: number;
  tokenCount?: string;
  activeMode?: string;
  messagesLeft?: number;
  resetTimer?: string;
  progressState?: number;
  progressValue?: number;
}

export interface ProjectSummary {
  id: string;
  label: string;
  color: string;
  rootPath: string;
  sortOrder: number;
  pinned: boolean;
  isActive: boolean;
  hasUnsavedChanges: boolean;
  activeTerminals: number;
  hasRunningAgent: boolean;
}

export interface RecentProject {
  path: string;
  label: string;
  color: string;
  lastOpened: number;
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npx tsc --noEmit 2>&1 | head -20`
Note: tsc may show pre-existing errors from other files — focus on errors in `projects/types.ts` only.

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/projects/types.ts
git commit -m "feat(multi-project): add project context type definitions"
```

---

### Task 2: Project registry store

**Files:**
- Create: `src/lib/stores/projects/registry.ts`
- Read: `src/lib/stores/files.ts` (lines 3-14 for OpenFile, projectRoot)
- Read: `src/lib/stores/workspace-trust.ts` (line 3 for TrustLevel)

- [ ] **Step 1: Create registry store**

```typescript
// src/lib/stores/projects/registry.ts
import { writable, get } from 'svelte/store';
import type { ProjectContext, ProjectFileState, RecentProject } from './types';

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

  // If removing active, switch to next
  if (current === id) {
    const all = [...get(projects).values()]
      .filter(p => p.id !== id)
      .sort((a, b) => a.sortOrder - b.sortOrder);
    activeProjectId.set(all.length > 0 ? all[0].id : null);
  }

  // Save to recents before removing
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
```

- [ ] **Step 2: Verify compiles**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npx tsc --noEmit 2>&1 | head -20`

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/projects/registry.ts
git commit -m "feat(multi-project): add project registry store with lifecycle actions"
```

---

### Task 3: Namespace layer

**Files:**
- Create: `src/lib/stores/projects/namespace.ts`
- Read: `src/lib/stores/files.ts` (current exports to replicate)
- Read: `src/lib/stores/terminals.ts` (current exports to replicate)

- [ ] **Step 1: Create namespace with derived stores and action wrappers**

```typescript
// src/lib/stores/projects/namespace.ts
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

    // Count occurrences of each name
    for (const inst of $instances) {
      const name = inst.modelName ?? inst.provider;
      nameCount.set(name, (nameCount.get(name) ?? 0) + 1);
    }

    // Assign labels
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
```

- [ ] **Step 2: Verify compiles**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npx tsc --noEmit 2>&1 | head -20`

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/projects/namespace.ts
git commit -m "feat(multi-project): add namespace layer with derived stores and action wrappers"
```

---

### Task 4: Sidebar derived stores

**Files:**
- Create: `src/lib/stores/projects/sidebar.ts`
- Read: `src/lib/stores/agent-session.ts` (line 20 for agentSessions store)

- [ ] **Step 1: Create sidebar stores**

```typescript
// src/lib/stores/projects/sidebar.ts
import { derived, get, type Readable } from 'svelte/store';
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
        // Pinned first, then by sortOrder
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
```

- [ ] **Step 2: Verify compiles**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npx tsc --noEmit 2>&1 | head -20`

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/projects/sidebar.ts
git commit -m "feat(multi-project): add sidebar summary and status derived stores"
```

---

### Task 5: Re-export index

**Files:**
- Create: `src/lib/stores/projects/index.ts`

- [ ] **Step 1: Create index file**

```typescript
// src/lib/stores/projects/index.ts

// Types
export type {
  ProjectContext,
  ProjectFileState,
  ProjectSummary,
  TerminalInstanceMeta,
  RecentProject,
} from './types';

// Registry (project lifecycle)
export {
  projects,
  activeProjectId,
  recentProjectsList,
  addProject,
  removeProject,
  switchProject,
  reorderProject,
  updateProjectContext,
} from './registry';

// Namespace (consumer-facing stores + actions)
export {
  projectRoot,
  openFiles,
  activeFilePath,
  terminalInstances,
  activeTerminalId,
  projectTrustLevel,
  computedLabels,
  openFile,
  closeFile,
  setActiveFile,
  updateFileContent,
  updateFileState,
  addTerminalInstance,
  removeTerminalInstance,
  updateTerminalInstance,
  setActiveTerminal,
  addAgentSession,
} from './namespace';

// Sidebar
export {
  projectSummaries,
  projectStatuses,
} from './sidebar';
export type { ProjectStatus } from './sidebar';
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/stores/projects/index.ts
git commit -m "feat(multi-project): add projects store index with re-exports"
```

---

### Task 6: Wire shim into files.ts

**Files:**
- Modify: `src/lib/stores/files.ts` (entire file — 45 lines)

This is the critical shim step. The existing `files.ts` exports (`projectRoot`, `openFiles`, `activeFilePath`, etc.) become re-exports from the namespace layer. Callers don't change.

- [ ] **Step 1: Read current files.ts**

Read `src/lib/stores/files.ts` to capture exact current exports and ensure nothing is missed.

- [ ] **Step 2: Rewrite files.ts as shim**

Replace `src/lib/stores/files.ts` contents. Keep the `OpenFile` interface for backward compatibility but mark deprecated. Re-export everything from the projects namespace.

```typescript
// src/lib/stores/files.ts
// SHIM: This file re-exports from the projects namespace layer.
// Components importing from here continue to work unchanged.

import { writable } from 'svelte/store';

// Legacy type — use ProjectFileState from projects instead
/** @deprecated Use ProjectFileState from '$lib/stores/projects' */
export interface OpenFile {
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
  isDeleted: boolean;
}

// Re-export from namespace layer
export {
  projectRoot,
  openFiles,
  activeFilePath,
  openFile as addOpenFile,
  closeFile,
} from './projects';

export {
  recentProjectsList as recentProjects,
} from './projects';

// These cursor stores remain global (not per-project — they track the current editor cursor)
export const pendingLine = writable<number | null>(null);
export const cursorLine = writable<number>(1);
export const cursorCol = writable<number>(1);
```

- [ ] **Step 3: Verify the app still builds**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -20`

Note: This will likely show type errors in components that use `openFiles` or `projectRoot` as `Writable` (calling `.set()`). List those errors — they're the components that need migration in the next task.

- [ ] **Step 4: Commit (even if build errors)**

```bash
git add src/lib/stores/files.ts
git commit -m "feat(multi-project): shim files.ts to re-export from namespace layer"
```

---

### Task 7: Wire shim into terminals.ts

**Files:**
- Modify: `src/lib/stores/terminals.ts` (73 lines)

- [ ] **Step 1: Read current terminals.ts**

Read `src/lib/stores/terminals.ts` to capture exact exports.

- [ ] **Step 2: Rewrite terminals.ts as shim**

```typescript
// src/lib/stores/terminals.ts
// SHIM: Re-exports from projects namespace layer.

// Legacy type — kept for backward compat
export { type TerminalInstanceMeta as TerminalInstance } from './projects';

// Re-export from namespace
export {
  terminalInstances,
  activeTerminalId as activeInstanceId,  // alias for backward compat
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
```

**IMPORTANT:** `activeInstanceId` is now `Readable` (was `Writable`). All `.set()` calls on `activeInstanceId` must be migrated to `setActiveTerminal()` in Task 8. Known locations: `TerminalManager.svelte` lines 195, 234, 236, 257, 261, 264, 267, 356.

- [ ] **Step 3: Verify build**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -30`

- [ ] **Step 4: Commit**

```bash
git add src/lib/stores/terminals.ts
git commit -m "feat(multi-project): shim terminals.ts to re-export from namespace layer"
```

---

### Task 8: Fix all Writable → Readable compile errors

**Files:**
- Modify: `src/routes/+page.svelte` (lines 142-150 — switchProject, lines 135-140 — openFolder)
- Modify: Any other files that call `.set()` on projectRoot, openFiles, or terminalInstances directly

This is the breakage point. Components that called `.set()` on stores that are now `Readable` must use action functions instead.

- [ ] **Step 1: Run build and capture all errors**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | grep "error TS"`

- [ ] **Step 2: Fix component errors**

Migration patterns:

| Old pattern | New pattern | Import from |
|-------------|-------------|-------------|
| `projectRoot.set(path)` | `addProject(path)` | `$lib/stores/projects` |
| `openFiles.set([])` | (remove — per-project now) | — |
| `openFiles.update(...)` | `openFile()` / `closeFile()` / `updateFileContent()` | `$lib/stores/projects` |
| `terminalInstances.set([])` | (remove — per-project now) | — |
| `terminalInstances.update(...)` | `addTerminalInstance()` / `removeTerminalInstance()` / `updateTerminalInstance()` | `$lib/stores/projects` |
| `activeFilePath.set(path)` | `setActiveFile(path)` | `$lib/stores/projects` |
| `activeInstanceId.set(id)` | `setActiveTerminal(id)` | `$lib/stores/projects` |

**Known affected components** (from codebase grep):
- `src/routes/+page.svelte` — `projectRoot.set()`, `openFiles` manipulation
- `src/lib/components/TerminalManager.svelte` — `activeInstanceId.set()` at ~6 locations
- `src/lib/components/Editor.svelte` — `openFiles.update()` at line ~163
- `src/lib/components/EditorTabs.svelte` — `activeFilePath.set()` at line ~11

Apply fixes one file at a time.

- [ ] **Step 3: Fix test file errors**

Test files also use `.set()` for setup. Tests should import the **writable stores directly from the registry** (not from the namespace shim) for test setup purposes:

```typescript
// In test files:
import { projects, activeProjectId } from '$lib/stores/projects/registry';
// Set up test state by writing directly to the registry
```

**Known affected test files** (~49 calls to fix):
- `tests/unit/stores/files.test.ts`
- `tests/unit/stores/terminals.test.ts`
- `tests/unit/components/FileTree.test.ts`
- `tests/unit/components/StatusBar.test.ts`
- `tests/unit/components/EditorTabs.test.ts`
- `tests/a11y/keyboard.test.ts`
- `tests/a11y/aria.test.ts`
- `tests/unit/components/ContextMenu.test.ts`

- [ ] **Step 4: Handle addRecentProject migration**

The current `files.ts` exports `addRecentProject()` which is used in `+page.svelte`. This is no longer needed — the `addProject()` and `removeProject()` functions in the registry handle recent projects automatically. Remove the import and calls to `addRecentProject()`.

- [ ] **Step 5: Verify full build passes**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -10`

Expected: Build succeeds with zero TS errors.

- [ ] **Step 6: Run tests**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm test 2>&1 | tail -30`

Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "fix(multi-project): migrate all Writable callers to namespace actions"
```

---

### Task 9: Session persistence v2

**Files:**
- Create: `src/lib/stores/projects/persistence.ts`
- Create: `src-tauri/src/session_io.rs`
- Modify: `src/lib/utils/session.ts` (entire file — 154 lines)
- Modify: `src-tauri/src/lib.rs` (register new commands)

- [ ] **Step 1: Create Rust session I/O**

```rust
// src-tauri/src/session_io.rs
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

fn session_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("app data dir")
}

#[tauri::command]
pub fn save_session(app: AppHandle, data: String) -> Result<(), String> {
    let dir = session_dir(&app);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let path = dir.join("session.json");
    let tmp = dir.join("session.tmp.json");
    let backup = dir.join("session.backup.json");

    // Write to temp
    fs::write(&tmp, &data).map_err(|e| e.to_string())?;

    // Backup current
    if path.exists() {
        let _ = fs::copy(&path, &backup);
    }

    // Atomic rename
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn load_session(app: AppHandle) -> Result<String, String> {
    let dir = session_dir(&app);
    let path = dir.join("session.json");
    let backup = dir.join("session.backup.json");

    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(data) => Ok(data),
            Err(_) => {
                // Try backup
                if backup.exists() {
                    fs::read_to_string(&backup).map_err(|e| e.to_string())
                } else {
                    Ok("{}".to_string())
                }
            }
        }
    } else if backup.exists() {
        fs::read_to_string(&backup).map_err(|e| e.to_string())
    } else {
        Ok("{}".to_string())
    }
}
```

- [ ] **Step 2: Register in lib.rs**

Add `mod session_io;` to module declarations and add commands to the invoke handler.

- [ ] **Step 3: Create frontend persistence module**

```typescript
// src/lib/stores/projects/persistence.ts
import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { projects, activeProjectId, recentProjectsList, addProject, updateProjectContext } from './registry';
import type { ProjectContext, RecentProject } from './types';

interface SessionV2 {
  version: 2;
  activeProjectId: string | null;
  sidebarCollapsed: boolean;
  recentProjects: RecentProject[];
  projects: Record<string, ProjectContext>;
}

interface SessionV1 {
  projectRoot?: string;
  openFiles?: Array<{ path: string; name: string; content: string; isDirty: boolean; isDeleted: boolean }>;
  activeFile?: string;
  recentProjects?: string[];
  terminalInstances?: Array<{ provider: string; label: string; modelName: string | null }>;
}

export async function saveSessionV2(sidebarCollapsed: boolean): Promise<void> {
  const data: SessionV2 = {
    version: 2,
    activeProjectId: get(activeProjectId),
    sidebarCollapsed,
    recentProjects: get(recentProjectsList),
    projects: Object.fromEntries(get(projects)),
  };
  await invoke('save_session', { data: JSON.stringify(data) });
}

export async function restoreSessionV2(): Promise<{ sidebarCollapsed: boolean }> {
  const raw = await invoke<string>('load_session');
  if (!raw || raw === '{}') return { sidebarCollapsed: false };

  let parsed: any;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return { sidebarCollapsed: false };
  }

  if (parsed.version === 2) {
    return restoreV2(parsed as SessionV2);
  } else {
    return migrateV1(parsed as SessionV1);
  }
}

function restoreV2(data: SessionV2): { sidebarCollapsed: boolean } {
  const map = new Map<string, ProjectContext>();
  for (const [id, ctx] of Object.entries(data.projects)) {
    map.set(id, ctx);
  }
  projects.set(map);
  activeProjectId.set(data.activeProjectId);
  recentProjectsList.set(data.recentProjects ?? []);
  return { sidebarCollapsed: data.sidebarCollapsed };
}

function migrateV1(data: SessionV1): { sidebarCollapsed: boolean } {
  if (data.projectRoot) {
    const ctx = addProject(data.projectRoot);

    // Migrate open files
    if (data.openFiles?.length) {
      updateProjectContext(ctx.id, {
        openFiles: data.openFiles.map(f => ({
          ...f,
          scrollPosition: { line: 0, col: 0 },
          cursorPosition: { line: 0, col: 0 },
        })),
        activeFilePath: data.activeFile ?? null,
      });
    }
  }

  // Migrate recent projects
  if (data.recentProjects?.length) {
    recentProjectsList.set(
      data.recentProjects.map(path => ({
        path,
        label: path.split('/').pop() ?? path,
        color: '#6B7280',
        lastOpened: 0,
      }))
    );
  }

  return { sidebarCollapsed: false };
}
```

- [ ] **Step 4: Update session.ts to use new persistence**

Read current `src/lib/utils/session.ts`, then replace `saveSession()` and `restoreSession()` to call `saveSessionV2()` and `restoreSessionV2()` from the persistence module.

- [ ] **Step 5: Verify build and manual test**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -10`

Test manually: open app, open a project, close app, reopen — state should restore.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/session_io.rs src-tauri/src/lib.rs src/lib/stores/projects/persistence.ts src/lib/utils/session.ts
git commit -m "feat(multi-project): session persistence v2 with atomic writes and v1 migration"
```

---

### Task 10: Verify Phase 1 — app works identically

- [ ] **Step 1: Build**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -10`

- [ ] **Step 2: Run Tauri dev**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && cargo tauri dev 2>&1`

- [ ] **Step 3: Manual verification checklist**

Test each item:
- Open app, FileTree shows files
- Open a file, editor shows it
- Create a terminal, it works
- Close and reopen app, state is restored
- Check that session.json in app data dir has `"version": 2`

- [ ] **Step 4: Commit any fixes**

```bash
git add -A
git commit -m "fix(multi-project): phase 1 stabilization fixes"
```

---

## Phase 2: Backend + Multi-Project Store (single-project UI, multi-project capable)

Goal: backend supports multiple projects, store can hold N projects, but UI still shows one.

---

### Task 11: Rust project manager

**Files:**
- Create: `src-tauri/src/project_manager.rs`
- Modify: `src-tauri/src/lib.rs` (register module, managed state)
- Modify: `src-tauri/src/pty_manager.rs` (add project_id to PtyInstance)

- [ ] **Step 1: Create project_manager.rs**

Implement:
- `ProjectsState` — `HashMap<String, ProjectState>` with root path, trust level
- `ActiveProjectState` — `Option<String>`
- Tauri commands: `add_project`, `remove_project`, `set_active_project`, `get_git_state`
- Watcher management: active = real-time, background = none (defer multi-watcher to Phase 3)

- [ ] **Step 2: Update pty_manager.rs**

Add `project_id: String` field to `PtyInstance`. Add `kill_project_ptys(project_id)` command.

- [ ] **Step 3: Register in lib.rs**

Add module, managed state, and commands.

- [ ] **Step 4: Verify Rust compiles**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && cargo build 2>&1 | tail -20`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/project_manager.rs src-tauri/src/pty_manager.rs src-tauri/src/lib.rs
git commit -m "feat(multi-project): Rust project manager with multi-project state"
```

---

### Task 12: Update adapter interface

**Files:**
- Modify: `src/lib/adapter/tauri.ts`

- [ ] **Step 1: Read current adapter**

Read `src/lib/adapter/tauri.ts` to understand current interface.

- [ ] **Step 2: Add project-aware methods**

Add to the adapter:
- `addProject(id, rootPath, trustLevel)` → invokes `add_project`
- `removeProject(id)` → invokes `remove_project`
- `setActiveProject(id)` → invokes `set_active_project`
- `killProjectProcesses(id)` → invokes `kill_project_ptys`
- `getGitState(projectId)` → invokes `get_git_state`
- `cleanupOrphanProcesses()` → invokes `cleanup_orphan_processes`

Keep existing methods for backward compat; update `spawnProcess` to accept `projectId`.

- [ ] **Step 3: Verify build**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -10`

- [ ] **Step 4: Commit**

```bash
git add src/lib/adapter/tauri.ts
git commit -m "feat(multi-project): extend adapter with project-aware methods"
```

---

### Task 13: Agent session tagging

**Files:**
- Modify: `src/lib/stores/agent-session.ts` (line 4-17 — add projectId to interface)
- Modify: `src/lib/components/chat/ChatView.svelte` (line ~62 — pass projectId to upsertSession)

- [ ] **Step 1: Add projectId to AgentSessionState**

In `agent-session.ts`, add `projectId: string` to the `AgentSessionState` interface (after line ~17).

- [ ] **Step 2: Update ChatView to pass projectId**

In `ChatView.svelte`, where `upsertSession()` is called (~line 62), add:
```typescript
projectId: get(activeProjectId) ?? '',
```
Import `activeProjectId` from `$lib/stores/projects`.

- [ ] **Step 3: Update namespace addAgentSession**

In `namespace.ts`, the `addAgentSession()` function already handles this. Verify it's called from ChatView when a new session starts.

- [ ] **Step 4: Verify build**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -10`

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/agent-session.ts src/lib/components/chat/ChatView.svelte
git commit -m "feat(multi-project): tag agent sessions with projectId"
```

---

### Task 14: Verify Phase 2

- [ ] **Step 1: Build and run**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && cargo tauri dev 2>&1`

- [ ] **Step 2: Manual verification**

Same as Phase 1 checklist plus:
- Session.json shows project structure
- Multiple calls to `addProject()` from console add entries to the Map
- `switchProject()` from console changes derived store values

- [ ] **Step 3: Commit fixes**

```bash
git add -A
git commit -m "fix(multi-project): phase 2 stabilization fixes"
```

---

## Phase 3: Sidebar UI (feature complete)

Goal: full multi-project experience with sidebar, switching, drag-drop, keyboard shortcuts.

---

### Task 15: Theme system updates

**Files:**
- Modify: `src/lib/themes/reasonance-dark.json`
- Modify: `src/lib/themes/reasonance-light.json`
- Modify: `src/lib/themes/theme-schema.json`
- Modify: `src/lib/engine/theme-validator.ts` (line 8 — CURRENT_SCHEMA_VERSION)
- Modify: `src/lib/engine/theme-engine.ts` (add v1→v2 migration)
- Modify: `src/lib/engine/fallback-theme.ts`
- Modify: `src/lib/themes/enhanced-readability.json`
- Modify: `src/lib/themes/_high-contrast.json`
- Modify: `src/lib/themes/_reduced-motion.json`
- Modify: `src/lib/components/theme-editor/ThemeStartDialog.svelte`

- [ ] **Step 1: Read all theme files**

Read each file to understand current structure before modifying.

- [ ] **Step 2: Add sidebar variables to dark and light themes**

Add to the appropriate sections of each theme JSON:
- `colors`: `--sidebar-bg`, `--sidebar-bg-hover`, `--sidebar-border`, `--sidebar-tab-active-accent`, `--sidebar-tab-active-bg`, `--sidebar-tab-text`, `--sidebar-tab-text-active`
- `states`: `--sidebar-indicator-running`, `--sidebar-indicator-idle`, `--sidebar-indicator-unsaved`, `--sidebar-indicator-error`, `--sidebar-indicator-pulse`
- `ui-states`: `--sidebar-dropzone-bg`, `--sidebar-dropzone-border`, `--sidebar-separator`, `--sidebar-badge-bg`, `--sidebar-badge-text`
- `layout`: `--sidebar-width`
- `transitions`: `--sidebar-transition-speed`

Use values consistent with the existing theme palette.

- [ ] **Step 3: Update theme-schema.json**

Add the sidebar variables to the required properties in each section. Bump `schemaVersion` const to `2`.

- [ ] **Step 4: Update theme-validator.ts**

Change `CURRENT_SCHEMA_VERSION` from `1` to `2`.

- [ ] **Step 5: Add v1→v2 migration to theme-engine.ts**

When loading a theme with `schemaVersion: 1`, inject sidebar variable defaults and bump to 2.

- [ ] **Step 6: Update fallback-theme.ts**

Add sidebar variable hardcoded defaults.

- [ ] **Step 7: Update modifiers**

- `enhanced-readability.json`: sidebar spacing overrides if needed
- `_high-contrast.json`: stronger sidebar borders and contrast
- `_reduced-motion.json`: `--sidebar-indicator-pulse: none`, `--sidebar-transition-speed: 0ms`

- [ ] **Step 8: Update ThemeStartDialog.svelte**

Ensure the empty theme template and clone function include sidebar section.

- [ ] **Step 9: Verify build**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -10`

- [ ] **Step 10: Commit**

```bash
git add src/lib/themes/ src/lib/engine/ src/lib/components/theme-editor/ThemeStartDialog.svelte
git commit -m "feat(multi-project): add sidebar CSS variables to theme system (schema v2)"
```

---

### Task 16: ProjectSidebar component

**Files:**
- Create: `src/lib/components/project/ProjectSidebar.svelte`

- [ ] **Step 1: Create the component**

Implement the sidebar per the spec:
- 48px fixed width column
- Vertical tab list with project initials, color indicators
- Active project: accent border left
- Status indicators: pulsing dot (agent running), static dot (terminals), yellow dot (unsaved)
- "+" button at bottom
- Right-click context menu (rename, color, close)
- Middle-click to close
- Hover tooltip with full path + git branch
- Drop zone for drag-and-drop folders
- WCAG: `role="tablist"`, `aria-orientation="vertical"`, keyboard nav (arrows, Enter)
- Auto-hide when only 1 project
- All colors from theme CSS variables (`--sidebar-*`)

- [ ] **Step 2: Verify Tauri drag-drop config**

Check `src-tauri/tauri.conf.json` — ensure `windows[0].dragDropEnabled` is `true` (default in Tauri v2). If not present, add it.

- [ ] **Step 3: Verify compiles**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -10`

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/project/ProjectSidebar.svelte
git commit -m "feat(multi-project): add ProjectSidebar component"
```

---

### Task 17: ProjectAddMenu component

**Files:**
- Create: `src/lib/components/project/ProjectAddMenu.svelte`

- [ ] **Step 1: Create the component**

Dropdown menu from the "+" button:
- "Open Folder..." → file picker dialog
- Divider
- Recent projects list (from `recentProjectsList` store)
- Each recent: label, path, click → `addProject(path)`

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/project/ProjectAddMenu.svelte
git commit -m "feat(multi-project): add ProjectAddMenu component"
```

---

### Task 18: ProjectQuickSwitcher component

**Files:**
- Create: `src/lib/components/project/ProjectQuickSwitcher.svelte`

- [ ] **Step 1: Create the component**

Overlay triggered by `Ctrl+Shift+E`:
- Text input with fuzzy search
- List of matching projects (from `projectSummaries`)
- Arrow keys to navigate, Enter to select
- Esc to close
- Accessible: `role="dialog"`, focus trap

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/project/ProjectQuickSwitcher.svelte
git commit -m "feat(multi-project): add ProjectQuickSwitcher overlay"
```

---

### Task 19: ProjectDisconnectedDialog component

**Files:**
- Create: `src/lib/components/project/ProjectDisconnectedDialog.svelte`

- [ ] **Step 1: Create the component**

Dialog shown when a project's folder is missing:
- Shows project label and missing path
- Two buttons: "Locate Folder..." (file picker) and "Remove Project"
- Used at boot and at runtime (file watcher disconnect)

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/project/ProjectDisconnectedDialog.svelte
git commit -m "feat(multi-project): add ProjectDisconnectedDialog component"
```

---

### Task 20: Integrate sidebar into App.svelte

**Files:**
- Modify: `src/lib/components/App.svelte` (lines 96-176 — layout)

- [ ] **Step 1: Read current App.svelte layout**

Read `src/lib/components/App.svelte` lines 90-180.

- [ ] **Step 2: Add ProjectSidebar to layout**

Insert the sidebar before the FileTree in the main-content flex row:

```svelte
<div class="main-content">
  <ProjectSidebar {adapter} />
  <!-- existing: file tree, divider, editor, divider, terminal -->
</div>
```

- [ ] **Step 3: Add skip-link**

Add `<a href="#project-sidebar" class="skip-link">Skip to projects</a>` to the skip links section (~line 96).

- [ ] **Step 4: Add keyboard shortcuts**

In the keyboard handler section (~line 47):
- `Alt+1..9` → switch to project N
- `Ctrl+Shift+E` → open ProjectQuickSwitcher
- `Ctrl+B` → toggle sidebar collapsed

- [ ] **Step 5: Add window title reactivity**

```svelte
<script>
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { projectRoot } from '$lib/stores/projects';

  // NOTE: If the project uses Svelte 5 runes, use $effect instead of $:
  // Check +page.svelte for which syntax the project uses and match it.
  $: if ($projectRoot) {
    const label = $projectRoot.split('/').pop() ?? 'Reasonance';
    getCurrentWindow().setTitle(`${label} — Reasonance`);
  }
</script>
```

- [ ] **Step 6: Verify build**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -10`

- [ ] **Step 7: Commit**

```bash
git add src/lib/components/App.svelte
git commit -m "feat(multi-project): integrate sidebar into App layout with shortcuts"
```

---

### Task 21: Update MenuBar

**Files:**
- Modify: `src/lib/components/MenuBar.svelte` (lines 48-67 — File menu)

- [ ] **Step 1: Read current MenuBar**

Read lines 47-70 of MenuBar.svelte.

- [ ] **Step 2: Update File menu**

- "Open Folder" → dispatches event that calls `addProject()` (not replace)
- "Recent" submenu → populated from `recentProjectsList` store
- Add "Close Project" item → dispatches `reasonance:closeProject`

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/MenuBar.svelte
git commit -m "feat(multi-project): update MenuBar with Recent projects and Close Project"
```

---

### Task 22: Update +page.svelte startup

**Files:**
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Read current +page.svelte**

Read the full file.

- [ ] **Step 2: Update startup flow**

Replace `restoreSession()` with `restoreSessionV2()`. Update `openFolder()` to call `addProject()`. Remove the old `switchProject()` function. Add event listener for `cli-open-project` (from Tauri single-instance plugin). Add `cleanupOrphanProcesses()` on mount.

- [ ] **Step 3: Update save-on-unmount**

Replace `saveSession()` with `saveSessionV2()`.

- [ ] **Step 4: Verify build**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -10`

- [ ] **Step 5: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat(multi-project): update page startup for multi-project lifecycle"
```

---

### Task 23: Update TerminalManager for project scoping

**Files:**
- Modify: `src/lib/components/TerminalManager.svelte`

- [ ] **Step 1: Read TerminalManager**

Read the key sections: imports, addInstance, closeInstance, tab rendering.

- [ ] **Step 2: Update imports**

Switch imports from `$lib/stores/terminals` to `$lib/stores/projects` (or keep shimmed imports from terminals.ts). Verify that `terminalInstances` and `computedLabels` are already project-scoped via the namespace.

- [ ] **Step 3: Hide background terminals instead of destroying**

When the active project changes, terminals of the previous project should be hidden (`display: none`) not destroyed. This preserves xterm.js buffers.

Add a wrapper that checks if each terminal's `projectId` matches `$activeProjectId`:

```svelte
{#each $terminalInstances as instance}
  <div style:display={instance.projectId === $activeProjectId ? 'contents' : 'none'}>
    <!-- existing terminal rendering -->
  </div>
{/each}
```

Wait — actually, since `$terminalInstances` is already project-scoped (from namespace), we need a different approach for buffer preservation. We need ALL terminal instances (across projects) to remain mounted but hidden. This requires accessing the raw `projects` map for terminal rendering only.

Alternative approach: keep a global `allTerminalInstances` derived that returns ALL instances, render them all, but only show the active project's. The namespace's `terminalInstances` is used for the tab bar, the global list for the render area.

- [ ] **Step 4: Verify build**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && npm run build 2>&1 | tail -10`

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/TerminalManager.svelte
git commit -m "feat(multi-project): scope terminal tabs per project, preserve background buffers"
```

---

### Task 24: CLI argument handling

**Files:**
- Modify: `src-tauri/src/lib.rs` (lines ~93-98 — single-instance plugin)

- [ ] **Step 1: Read current single-instance setup**

Read lines 90-100 of lib.rs.

- [ ] **Step 2: Wire CLI args**

Update the single-instance plugin closure to emit `cli-open-project` event when a path argument is provided:

```rust
.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
    if args.len() > 1 {
        if let Some(path) = args.get(1) {
            let _ = app.emit("cli-open-project", path.as_str());
        }
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_focus();
    }
}))
```

- [ ] **Step 3: Verify Rust compiles**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && cargo build 2>&1 | tail -10`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(multi-project): wire CLI path argument to open project"
```

---

### Task 25: End-to-end verification

- [ ] **Step 1: Full build**

Run: `cd /home/uh1/VIBEPROJECTS/REASONANCE && cargo tauri build 2>&1 | tail -20`

- [ ] **Step 2: Manual test matrix**

| Test | Expected |
|------|----------|
| Open app, FileTree shows files | Pass |
| Open file, editor shows it | Pass |
| Create terminal, works | Pass |
| Close/reopen, state restored | Pass |
| Drag folder to sidebar | New project tab appears |
| Click "+" → browse → select folder | New project tab appears |
| Click project tab | Workspace switches (FileTree, Editor, Terminal) |
| Previous project's terminal keeps running | Visible output when switching back |
| `Alt+2` shortcut | Switches to 2nd project |
| `Ctrl+Shift+E` | Quick switcher opens |
| Right-click tab → Close | Project closes with confirmation if unsaved |
| Middle-click tab | Same as close |
| Close last project | Empty state with drop zone |
| session.json v1 migration | Old session loads correctly |
| Theme: sidebar uses CSS variables | Sidebar matches theme |
| Theme editor: sidebar section visible | Can edit sidebar colors |
| High contrast modifier | Sidebar borders enhanced |
| Reduced motion | No pulse animation |

- [ ] **Step 3: Commit final fixes**

```bash
git add -A
git commit -m "fix(multi-project): end-to-end stabilization"
```

---

## Post-Implementation

### Task 26: Update registry

**Files:**
- Modify: `.claude/docs/registry.md`

- [ ] **Step 1: Add new components to registry**

Add ProjectSidebar, ProjectQuickSwitcher, ProjectAddMenu, ProjectDisconnectedDialog to the Components section. Add new key functions. Update data flows.

- [ ] **Step 2: Commit**

```bash
git add .claude/docs/registry.md
git commit -m "docs: update registry with multi-project components"
```

---

## Deferred to v2 (tracked, not forgotten)

These spec requirements are intentionally deferred to reduce scope of the initial implementation:

1. **Multi-watcher with active/background strategy** — For v1, only the active project gets a file watcher (same as today). Background projects don't get real-time file updates. The spec's `WatcherManager` with active=real-time / background=polling will be a follow-up.

2. **fs-change event routing with project_id** — Depends on multi-watcher. For v1, `fs-change` events only fire for the active project.

3. **ResourceMonitor and `getResourceUsage()`** — Monitoring is nice-to-have for v1. No user-facing impact.

4. **Orphan PID tracking file (`reasonance.pids`)** — For v1, use the simpler approach: on startup, kill any PTY processes that were tracked in session.json but are no longer needed. Full PID file tracking is a follow-up.

5. **HIVE workflow projectId tagging** — No `HiveWorkflow` interface exists yet in the codebase. Will be added when HIVE workflows are formalized.

6. **Session save debounce** — For v1, save on project switch and on app close. Debounced auto-save (every 5s after change) is a follow-up.

## Verification: components that auto-migrate via shim

These components use `projectRoot` as read-only (`$projectRoot` in templates) and should work without changes via the namespace shim. **Verify during Task 10 (Phase 1 verification):**

- `SearchPalette.svelte` — scopes search to `$projectRoot`
- `FindInFiles.svelte` — scopes search to `$projectRoot`
- `StatusBar.svelte` — displays file/cursor info
- `FileTree.svelte` — reads `$projectRoot` for tree rendering

If any of these write to stores directly, they'll show compile errors in Task 8.

---

## Notes for Implementor

1. **Phase 1 is the hardest phase.** The shim layer requires finding and fixing every place that writes directly to stores. Build errors are your guide — fix them all before moving on.

2. **Terminal buffer preservation (Task 23)** is the trickiest UI task. You need ALL terminals mounted in the DOM but only the active project's visible. This means the namespace layer needs to expose an `allTerminalInstances` derived (not project-scoped) specifically for the render area, while the tab bar uses the project-scoped `terminalInstances`.

3. **Theme changes (Task 15)** must match existing patterns exactly. Read the current theme files carefully. Don't invent new section names — add variables to existing sections.

4. **The v1→v2 migration (Task 9)** must be tested with the current session.json format. Save a copy of a working session.json before starting, and verify the migration produces the correct v2 structure.

5. **Don't skip Phase 2.** It's tempting to jump from Phase 1 to Phase 3, but Phase 2 ensures the Rust backend is ready before the UI demands it.
