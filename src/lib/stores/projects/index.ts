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
