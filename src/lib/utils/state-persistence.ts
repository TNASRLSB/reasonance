import type { AppState, ProjectState, PanelLayout, TerminalState } from '$lib/types/app-state';
import type { Adapter } from '$lib/adapter/index';

export interface FileStateInput {
  path: string;
  cursorLine?: number;
  cursorCol?: number;
  scrollOffset?: number;
}

export interface RecentProjectInput {
  path: string;
  label: string;
  lastOpened: number;
}

export function gatherAppState(
  activeProjectId: string | null,
  recentProjects: RecentProjectInput[],
): AppState {
  return {
    last_active_project_id: activeProjectId,
    recent_projects: recentProjects.map(p => ({
      path: p.path,
      label: p.label,
      last_opened: p.lastOpened,
    })),
    window_state: null,
  };
}

export function gatherProjectState(
  openFiles: FileStateInput[],
  activeFilePath: string | null,
  panelLayout: PanelLayout | null,
  lastModelUsed: string | null,
  activeSessionId: string | null,
  terminals: TerminalState[] = [],
): ProjectState {
  return {
    active_session_id: activeSessionId,
    open_files: openFiles.map(f => ({
      path: f.path,
      cursor_line: f.cursorLine ?? 0,
      cursor_column: f.cursorCol ?? 0,
      scroll_offset: f.scrollOffset ?? 0,
    })),
    active_file_path: activeFilePath,
    panel_layout: panelLayout,
    last_model_used: lastModelUsed,
    terminals,
  };
}

export async function saveAllState(
  adapter: Adapter,
  activeProjectId: string | null,
  recentProjects: RecentProjectInput[],
  projectStateGatherer: () => ProjectState | null,
): Promise<void> {
  const appState = gatherAppState(activeProjectId, recentProjects);
  await adapter.saveAppState(appState);

  if (activeProjectId) {
    const projectState = projectStateGatherer();
    if (projectState) {
      await adapter.saveProjectState(activeProjectId, projectState);
    }
  }
}

export async function loadAppState(adapter: Adapter): Promise<AppState> {
  try {
    return await adapter.getAppState();
  } catch {
    return { last_active_project_id: null, recent_projects: [], window_state: null };
  }
}

export async function loadProjectState(adapter: Adapter, projectId: string): Promise<ProjectState> {
  try {
    const state = await adapter.getProjectState(projectId);
    // Ensure terminals array is always present (backward compat with older saved state)
    return { ...state, terminals: state.terminals ?? [] };
  } catch {
    return {
      active_session_id: null,
      open_files: [],
      active_file_path: null,
      panel_layout: null,
      last_model_used: null,
      terminals: [],
    };
  }
}
