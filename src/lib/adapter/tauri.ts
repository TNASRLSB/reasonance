import { invoke } from '@tauri-apps/api/core';
import type { Adapter, FileEntry, FsEvent, GrepResult, PtyHandle, DiscoveredAgent, Workflow, AgentState, AgentInstance, AgentMessage, MemoryEntry, WorkflowRun } from './index';
import type { AgentEvent, AgentEventPayload, SessionHandle, SessionSummary, ViewMode } from '$lib/types/agent-event';
import type { NegotiatedCapabilities, CliVersionInfo, VersionEntry, HealthReport } from '$lib/types/capability';
import type { ProviderAnalytics, ModelAnalytics, DailyStats, SessionMetrics, ConnectionTestStep } from '$lib/types/analytics';
import type { TrustCheckResult, TrustLevel, TrustEntry } from '$lib/stores/workspace-trust';
import type { AppState, ProjectState } from '$lib/types/app-state';

export class TauriAdapter implements Adapter {
  async setProjectRoot(path: string): Promise<void> {
    return invoke<void>('set_project_root', { path });
  }
  async readFile(path: string): Promise<string> {
    return invoke<string>('read_file', { path });
  }
  async writeFile(path: string, content: string): Promise<void> {
    return invoke<void>('write_file', { path, content });
  }
  async listDir(path: string, respectGitignore?: boolean): Promise<FileEntry[]> {
    return invoke<FileEntry[]>('list_dir', { path, respectGitignore: respectGitignore ?? true });
  }
  async watchFiles(path: string, callback: (event: FsEvent) => void): Promise<() => void> {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<FsEvent>('fs-change', (event) => { callback(event.payload); });
    await invoke('start_watching', { path });
    return unlisten;
  }
  async getGitStatus(projectRoot: string): Promise<Record<string, string>> {
    return invoke<Record<string, string>>('get_git_status', { projectRoot });
  }
  async openExternal(path: string): Promise<void> {
    return invoke<void>('open_external', { path });
  }
  async getClipboard(): Promise<string> {
    const { readText } = await import('@tauri-apps/plugin-clipboard-manager');
    return readText();
  }
  async setClipboard(text: string): Promise<void> {
    const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
    return writeText(text);
  }
  async showNotification(title: string, body: string): Promise<void> {
    const { sendNotification } = await import('@tauri-apps/plugin-notification');
    sendNotification({ title, body });
  }
  async minimizeWindow(): Promise<void> {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    return getCurrentWindow().minimize();
  }
  async maximizeWindow(): Promise<void> {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    return getCurrentWindow().toggleMaximize();
  }
  async closeWindow(): Promise<void> {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    return getCurrentWindow().close();
  }
  async startDragging(): Promise<void> {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    return getCurrentWindow().startDragging();
  }
  async onWindowClose(callback: () => Promise<void>): Promise<void> {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    getCurrentWindow().onCloseRequested(async () => { await callback(); });
  }
  async discoverLlms(): Promise<Array<{ name: string; command: string; found: boolean }>> {
    return invoke<Array<{ name: string; command: string; found: boolean }>>('discover_llms');
  }
  async grepFiles(path: string, pattern: string, respectGitignore: boolean): Promise<GrepResult[]> {
    return invoke<GrepResult[]>('grep_files', { path, pattern, respectGitignore });
  }
  async openFolderDialog(): Promise<string | null> {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({ directory: true, multiple: false });
    if (!selected) return null;
    return typeof selected === 'string' ? selected : selected[0] ?? null;
  }
  async openFileDialog(filters?: Array<{ name: string; extensions: string[] }>): Promise<string | null> {
    const { open } = await import('@tauri-apps/plugin-dialog');
    // Default to project root so the dialog doesn't open in Downloads or other unrelated folders
    let defaultPath: string | undefined;
    const { get } = await import('svelte/store');
    const { projectRoot } = await import('$lib/stores/files');
    const root = get(projectRoot);
    if (root) defaultPath = root;
    const selected = await open({ multiple: false, filters, defaultPath });
    if (!selected) return null;
    const path = typeof selected === 'string' ? selected : (selected as { path: string }).path ?? null;
    return path;
  }
  async saveFileDialog(defaultPath?: string, filters?: Array<{ name: string; extensions: string[] }>): Promise<string | null> {
    const { save } = await import('@tauri-apps/plugin-dialog');
    // Default to project root so the dialog doesn't open in Downloads or other unrelated folders
    let effectivePath = defaultPath;
    if (!effectivePath) {
      const { get } = await import('svelte/store');
      const { projectRoot } = await import('$lib/stores/files');
      const root = get(projectRoot);
      if (root) effectivePath = root;
    }
    const selected = await save({ defaultPath: effectivePath, filters });
    return selected ?? null;
  }
  async spawnProcess(command: string, args: string[], cwd: string): Promise<PtyHandle> {
    const id = await invoke<string>('spawn_process', { command, args, cwd });
    return { id };
  }
  async killProcess(id: string): Promise<void> {
    return invoke('kill_process', { id });
  }
  async resizePty(id: string, cols: number, rows: number): Promise<void> {
    return invoke('resize_pty', { id, cols: Math.floor(cols), rows: Math.floor(rows) });
  }
  async writePty(id: string, data: string): Promise<void> {
    return invoke('write_pty', { id, data });
  }
  async onPtyData(id: string, callback: (data: string) => void): Promise<() => void> {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<string>(`pty-data-${id}`, (event) => { callback(event.payload); });
    return unlisten;
  }
  async onPtyExit(id: string, callback: (code: number) => void): Promise<() => void> {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<number>(`pty-exit-${id}`, (event) => { callback(event.payload); });
    return unlisten;
  }
  async sweepPtys(): Promise<string[]> {
    return invoke<string[]>('sweep_ptys');
  }
  async readConfig(): Promise<string> {
    return invoke<string>('read_config');
  }
  async writeConfig(content: string): Promise<void> {
    return invoke('write_config', { content });
  }
  async storeShadow(path: string, content: string): Promise<void> {
    return invoke('store_shadow', { path, content });
  }
  async getShadow(path: string): Promise<string | null> {
    return invoke<string | null>('get_shadow', { path });
  }

  // Discovery
  async discoverAgents(): Promise<DiscoveredAgent[]> {
    return invoke<DiscoveredAgent[]>('discover_agents');
  }
  async getDiscoveredAgents(): Promise<DiscoveredAgent[]> {
    return invoke<DiscoveredAgent[]>('get_discovered_agents');
  }

  // Workflows
  async loadWorkflow(filePath: string): Promise<Workflow> {
    return invoke<Workflow>('load_workflow', { filePath });
  }
  async saveWorkflow(filePath: string, workflow: Workflow): Promise<void> {
    return invoke('save_workflow', { filePath, workflow });
  }
  async listWorkflows(dir: string): Promise<string[]> {
    return invoke<string[]>('list_workflows', { dir });
  }
  async deleteWorkflow(filePath: string): Promise<void> {
    return invoke('delete_workflow', { filePath });
  }
  async createWorkflow(name: string, filePath: string): Promise<Workflow> {
    return invoke<Workflow>('create_workflow', { name, filePath });
  }
  async getWorkflow(filePath: string): Promise<Workflow | null> {
    return invoke<Workflow | null>('get_workflow', { filePath });
  }
  async duplicateWorkflow(sourcePath: string, destPath: string): Promise<Workflow> {
    return invoke('duplicate_workflow', { sourcePath, destPath });
  }
  async saveToGlobal(workflowPath: string): Promise<string> {
    return invoke('save_to_global', { workflowPath });
  }
  async listGlobalWorkflows(): Promise<string[]> {
    return invoke('list_global_workflows');
  }

  // Agent Runtime
  async createAgent(nodeId: string, workflowPath: string, maxRetries: number, fallbackAgent?: string): Promise<string> {
    return invoke<string>('create_agent', { nodeId, workflowPath, maxRetries, fallbackAgent: fallbackAgent ?? null });
  }
  async transitionAgent(agentId: string, newState: AgentState): Promise<AgentState> {
    return invoke<AgentState>('transition_agent', { agentId, newState });
  }
  async setAgentPty(agentId: string, ptyId: string): Promise<void> {
    return invoke('set_agent_pty', { agentId, ptyId });
  }
  async setAgentError(agentId: string, message: string): Promise<void> {
    return invoke('set_agent_error', { agentId, message });
  }
  async getAgent(agentId: string): Promise<AgentInstance | null> {
    return invoke<AgentInstance | null>('get_agent', { agentId });
  }
  async getWorkflowAgents(workflowPath: string): Promise<AgentInstance[]> {
    return invoke<AgentInstance[]>('get_workflow_agents', { workflowPath });
  }
  async removeAgent(agentId: string): Promise<void> {
    return invoke('remove_agent', { agentId });
  }
  async stopWorkflowAgents(workflowPath: string): Promise<void> {
    return invoke('stop_workflow_agents', { workflowPath });
  }
  async sendAgentMessage(from: string, to: string, payload: unknown): Promise<void> {
    return invoke('send_agent_message', { from, to, payload });
  }
  async getAgentMessages(agentId: string): Promise<AgentMessage[]> {
    return invoke<AgentMessage[]>('get_agent_messages', { agentId });
  }
  async getAgentMemory(nodeId: string, workflowPath: string, persist = 'workflow'): Promise<MemoryEntry[]> {
    return invoke<MemoryEntry[]>('get_agent_memory', { nodeId, workflowPath, persist });
  }

  // Workflow Engine
  async playWorkflow(workflowPath: string, cwd: string): Promise<string> {
    return invoke<string>('play_workflow', { workflowPath, cwd });
  }
  async pauseWorkflow(runId: string): Promise<void> {
    return invoke('pause_workflow', { runId });
  }
  async resumeWorkflow(runId: string, workflowPath: string, cwd: string): Promise<void> {
    return invoke('resume_workflow', { runId, workflowPath, cwd });
  }
  async stopWorkflow(runId: string): Promise<void> {
    return invoke('stop_workflow', { runId });
  }
  async stepWorkflow(runId: string, workflowPath: string, cwd: string): Promise<string | null> {
    return invoke<string | null>('step_workflow', { runId, workflowPath, cwd });
  }
  async getRunStatus(runId: string): Promise<WorkflowRun | null> {
    return invoke<WorkflowRun | null>('get_run_status', { runId });
  }
  async notifyNodeCompleted(runId: string, nodeId: string, success: boolean, workflowPath: string, cwd: string): Promise<void> {
    return invoke('notify_node_completed', { runId, nodeId, success, workflowPath, cwd });
  }

  // Structured Transport
  async agentSend(prompt: string, provider: string, model?: string, sessionId?: string, cwd?: string, yolo?: boolean, allowedTools?: string[]): Promise<string> {
    return invoke<string>('agent_send', {
      request: { prompt, provider, model: model ?? null, context: [], session_id: sessionId ?? null, system_prompt: null, max_tokens: null, allowed_tools: allowedTools ?? null, cwd: cwd ?? null, yolo: yolo ?? false }
    });
  }
  async agentStop(sessionId: string): Promise<void> {
    return invoke<void>('agent_stop', { sessionId });
  }
  async agentGetEvents(sessionId: string): Promise<AgentEvent[]> {
    return invoke<AgentEvent[]>('agent_get_events', { sessionId });
  }
  async onAgentEvent(callback: (payload: AgentEventPayload) => void): Promise<() => void> {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<AgentEventPayload>('agent-event', (event) => {
      callback(event.payload);
    });
    return unlisten;
  }

  // Session Management
  async sessionCreate(provider: string, model: string): Promise<string> {
    return invoke<string>('session_create', { provider, model });
  }
  async sessionRestore(sessionId: string): Promise<SessionHandle> {
    return invoke<SessionHandle>('session_restore', { sessionId });
  }
  async sessionGetEvents(sessionId: string): Promise<AgentEvent[]> {
    return invoke<AgentEvent[]>('session_get_events', { sessionId });
  }
  async sessionList(): Promise<SessionSummary[]> {
    return invoke<SessionSummary[]>('session_list');
  }
  async sessionDelete(sessionId: string): Promise<void> {
    return invoke<void>('session_delete', { sessionId });
  }
  async sessionRename(sessionId: string, title: string): Promise<void> {
    return invoke<void>('session_rename', { sessionId, title });
  }
  async sessionFork(sessionId: string, forkEventIndex: number): Promise<string> {
    return invoke<string>('session_fork', { sessionId, forkEventIndex });
  }
  async sessionSetViewMode(sessionId: string, mode: ViewMode): Promise<void> {
    return invoke<void>('session_set_view_mode', { sessionId, mode });
  }

  // Capability & Health
  async getCapabilities(): Promise<Record<string, NegotiatedCapabilities>> {
    return invoke('get_capabilities');
  }
  async getProviderCapabilities(provider: string): Promise<NegotiatedCapabilities> {
    return invoke('get_provider_capabilities', { provider });
  }
  async getCliVersions(): Promise<CliVersionInfo[]> {
    return invoke('get_cli_versions');
  }
  async getNormalizerVersions(provider: string): Promise<VersionEntry[]> {
    return invoke('get_normalizer_versions', { provider });
  }
  async rollbackNormalizer(provider: string, versionId: string): Promise<string> {
    return invoke('rollback_normalizer', { provider, versionId });
  }
  async getHealthReport(provider: string): Promise<HealthReport> {
    return invoke('get_health_report', { provider });
  }
  async getAllHealthReports(): Promise<Record<string, HealthReport>> {
    return invoke('get_all_health_reports');
  }

  async analyticsProvider(provider: string, from?: number, to?: number): Promise<ProviderAnalytics> {
    return invoke<ProviderAnalytics>('analytics_provider', { provider, from, to });
  }
  async analyticsCompare(from?: number, to?: number): Promise<ProviderAnalytics[]> {
    return invoke<ProviderAnalytics[]>('analytics_compare', { from, to });
  }
  async analyticsModelBreakdown(provider: string, from?: number, to?: number): Promise<ModelAnalytics[]> {
    return invoke<ModelAnalytics[]>('analytics_model_breakdown', { provider, from, to });
  }
  async analyticsSession(sessionId: string): Promise<SessionMetrics | null> {
    return invoke<SessionMetrics | null>('analytics_session', { sessionId });
  }
  async analyticsDaily(provider?: string, days?: number): Promise<DailyStats[]> {
    return invoke<DailyStats[]>('analytics_daily', { provider, days });
  }
  async analyticsActive(): Promise<SessionMetrics[]> {
    return invoke<SessionMetrics[]>('analytics_active');
  }
  async testProviderConnection(provider: string): Promise<void> {
    return invoke<void>('test_provider_connection', { provider });
  }
  async onConnectionTest(callback: (step: ConnectionTestStep) => void): Promise<() => void> {
    const { listen } = await import('@tauri-apps/api/event');
    return listen<ConnectionTestStep>('connection_test_step', (event) => {
      callback(event.payload);
    });
  }
  async reloadNormalizers(): Promise<void> {
    return invoke<void>('reload_normalizers');
  }

  // Workspace Trust
  async checkWorkspaceTrust(path: string): Promise<TrustCheckResult> {
    return invoke<TrustCheckResult>('check_workspace_trust', { path });
  }

  async setWorkspaceTrust(path: string, level: TrustLevel): Promise<void> {
    return invoke<void>('set_workspace_trust', { path, level });
  }

  async revokeWorkspaceTrust(hash: string): Promise<void> {
    return invoke<void>('revoke_workspace_trust', { hash });
  }

  async listWorkspaceTrust(): Promise<TrustEntry[]> {
    return invoke<TrustEntry[]>('list_workspace_trust');
  }

  async getNormalizerConfig(provider: string): Promise<{ permission_args?: string[] } | null> {
    return invoke<{ permission_args?: string[] } | null>('get_normalizer_config', { provider });
  }

  // App State Persistence
  async getAppState(): Promise<AppState> {
    return invoke('get_app_state');
  }
  async saveAppState(state: AppState): Promise<void> {
    return invoke('save_app_state', { state });
  }
  async getProjectState(projectId: string): Promise<ProjectState> {
    return invoke('get_project_state', { projectId });
  }
  async saveProjectState(projectId: string, state: ProjectState): Promise<void> {
    return invoke('save_project_state', { projectId, state });
  }

  // File Operations (undo/trash)
  async fileOpsSetProject(path: string): Promise<void> {
    return invoke('file_ops_set_project', { path });
  }
  async fileOpsDelete(path: string): Promise<void> {
    return invoke('file_ops_delete', { path });
  }
  async fileOpsUndo(): Promise<string> {
    return invoke('file_ops_undo');
  }
  async fileOpsRecordCreate(path: string): Promise<void> {
    return invoke('file_ops_record_create', { path });
  }
  async fileOpsRecordRename(oldPath: string, newPath: string): Promise<void> {
    return invoke('file_ops_record_rename', { oldPath, newPath });
  }

  // Multi-project
  async addProject(id: string, rootPath: string, trustLevel: string): Promise<void> {
    await invoke('add_project', { id, rootPath, trustLevel });
  }

  async removeProject(id: string): Promise<void> {
    await invoke('remove_project', { id });
  }

  async setActiveProject(id: string): Promise<void> {
    await invoke('set_active_project', { id });
  }

  async getProjectRoot(projectId: string): Promise<string> {
    return invoke('get_project_root', { projectId });
  }

  async killProjectProcesses(id: string): Promise<string[]> {
    return invoke('kill_project_ptys', { projectId: id });
  }
}
