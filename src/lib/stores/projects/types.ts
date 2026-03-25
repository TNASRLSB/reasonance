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
