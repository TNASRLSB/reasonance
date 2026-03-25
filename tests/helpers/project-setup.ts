/**
 * Shared test helper for setting up project state via the registry layer.
 *
 * Since stores like openFiles, activeFilePath, projectRoot, terminalInstances,
 * and activeInstanceId are now Readable (derived from the projects registry),
 * tests must set state through the registry writable stores.
 */
import { projects, activeProjectId } from '$lib/stores/projects/registry';
import type { ProjectContext, TerminalInstanceMeta } from '$lib/stores/projects';

const TEST_PROJECT_ID = 'test-project';

export interface SetupProjectOptions {
  id?: string;
  rootPath?: string;
  openFiles?: Array<{
    path: string;
    name?: string;
    content?: string;
    isDirty?: boolean;
    isDeleted?: boolean;
  }>;
  activeFilePath?: string | null;
  terminalInstances?: TerminalInstanceMeta[];
  activeTerminalId?: string | null;
}

/**
 * Creates a test project in the registry and sets it as active.
 * Returns the project ID for further reference.
 */
export function setupTestProject(opts: SetupProjectOptions = {}): string {
  const id = opts.id ?? TEST_PROJECT_ID;

  const ctx: ProjectContext = {
    id,
    rootPath: opts.rootPath ?? '/test',
    label: (opts.rootPath ?? '/test').split('/').pop() ?? 'test',
    color: '#4A9EFF',
    sortOrder: 0,
    addedAt: Date.now(),
    pinned: false,
    trustLevel: 'trusted',
    openFiles: (opts.openFiles ?? []).map(f => ({
      path: f.path,
      name: f.name ?? f.path.split('/').pop() ?? f.path,
      content: f.content ?? '',
      isDirty: f.isDirty ?? false,
      isDeleted: f.isDeleted ?? false,
      scrollPosition: { line: 0, col: 0 },
      cursorPosition: { line: 0, col: 0 },
    })),
    activeFilePath: opts.activeFilePath ?? null,
    terminalInstances: opts.terminalInstances ?? [],
    activeTerminalId: opts.activeTerminalId ?? null,
    agentSessionIds: [],
    fileTreeState: { collapsed: false, expandedDirs: [] },
    gitState: null,
  };

  projects.set(new Map([[id, ctx]]));
  activeProjectId.set(id);
  return id;
}

/**
 * Resets the registry to empty state (no projects, no active project).
 */
export function resetProjectState(): void {
  projects.set(new Map());
  activeProjectId.set(null);
}

export { TEST_PROJECT_ID };
