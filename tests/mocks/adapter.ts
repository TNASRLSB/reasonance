import type {
  Adapter,
  FileEntry,
  GrepResult,
  PtyHandle,
  FsEvent,
  DiscoveredAgent,
  Workflow,
  AgentInstance,
  AgentState,
  AgentMessage,
  MemoryEntry,
  WorkflowRun,
} from '$lib/adapter/index';
import type { AgentEvent, AgentEventPayload, SessionHandle, SessionSummary, ViewMode } from '$lib/types/agent-event';
import type { NegotiatedCapabilities, CliVersionInfo, VersionEntry, HealthReport } from '$lib/types/capability';
import type { ProviderAnalytics, ModelAnalytics, DailyStats, SessionMetrics, ConnectionTestStep } from '$lib/types/analytics';
import type { TrustCheckResult, TrustEntry } from '$lib/stores/workspace-trust';
import type { AppState, ProjectState } from '$lib/types/app-state';

let _ptyIdCounter = 0;
let _agentIdCounter = 0;
let _runIdCounter = 0;

export function createMockAdapter(overrides?: Partial<Adapter>): Adapter {
  const files = new Map<string, string>();
  const shadows = new Map<string, string>();
  let clipboard = '';
  let config = '';

  const agents = new Map<string, AgentInstance>();
  const agentMessages = new Map<string, AgentMessage[]>();
  const workflows = new Map<string, Workflow>();
  const runs = new Map<string, WorkflowRun>();

  const defaultWorkflow = (): Workflow => ({
    name: 'Untitled',
    version: '1.0.0',
    schemaVersion: 1,
    nodes: [],
    edges: [],
    settings: { max_concurrent_agents: 4, default_retry: 3, timeout: 300, permissionLevel: 'supervised' },
  });

  const base: Adapter = {
    // Filesystem
    setProjectRoot(_path: string) {
      return Promise.resolve();
    },
    readFile(path: string) {
      return Promise.resolve(files.get(path) ?? '');
    },
    writeFile(path: string, content: string) {
      files.set(path, content);
      return Promise.resolve();
    },
    listDir(_path: string, _respectGitignore?: boolean): Promise<FileEntry[]> {
      return Promise.resolve([]);
    },
    watchFiles(_path: string, _callback: (event: FsEvent) => void) {
      return Promise.resolve(() => {});
    },
    getGitStatus(_projectRoot: string): Promise<Record<string, string>> {
      return Promise.resolve({});
    },

    // System
    openExternal(_path: string) {
      return Promise.resolve();
    },
    getClipboard() {
      return Promise.resolve(clipboard);
    },
    setClipboard(text: string) {
      clipboard = text;
      return Promise.resolve();
    },
    showNotification(_title: string, _body: string) {
      return Promise.resolve();
    },
    minimizeWindow() {
      return Promise.resolve();
    },
    maximizeWindow() {
      return Promise.resolve();
    },
    closeWindow() {
      return Promise.resolve();
    },
    startDragging() {
      return Promise.resolve();
    },
    onWindowClose(_callback: () => Promise<void>) {
      return Promise.resolve();
    },
    discoverLlms(): Promise<Array<{ name: string; command: string; found: boolean }>> {
      return Promise.resolve([]);
    },
    grepFiles(_path: string, _pattern: string, _respectGitignore: boolean): Promise<GrepResult[]> {
      return Promise.resolve([]);
    },
    openFolderDialog(): Promise<string | null> {
      return Promise.resolve(null);
    },
    openFileDialog(_filters?: Array<{ name: string; extensions: string[] }>): Promise<string | null> {
      return Promise.resolve(null);
    },
    saveFileDialog(_defaultPath?: string, _filters?: Array<{ name: string; extensions: string[] }>): Promise<string | null> {
      return Promise.resolve(null);
    },

    // PTY
    spawnProcess(_command: string, _args: string[], _cwd: string): Promise<PtyHandle> {
      const id = `pty-${++_ptyIdCounter}`;
      return Promise.resolve({ id });
    },
    killProcess(_id: string) {
      return Promise.resolve();
    },
    resizePty(_id: string, _cols: number, _rows: number) {
      return Promise.resolve();
    },
    writePty(_id: string, _data: string) {
      return Promise.resolve();
    },
    onPtyData(_id: string, _callback: (data: string) => void) {
      return Promise.resolve(() => {});
    },
    onPtyExit(_id: string, _callback: (code: number) => void) {
      return Promise.resolve(() => {});
    },
    sweepPtys() {
      return Promise.resolve([]);
    },

    // Config
    readConfig() {
      return Promise.resolve(config);
    },
    writeConfig(content: string) {
      config = content;
      return Promise.resolve();
    },

    // Shadow copies
    storeShadow(path: string, content: string) {
      shadows.set(path, content);
      return Promise.resolve();
    },
    getShadow(path: string) {
      return Promise.resolve(shadows.get(path) ?? null);
    },

    // Discovery
    discoverAgents(): Promise<DiscoveredAgent[]> {
      return Promise.resolve([]);
    },
    getDiscoveredAgents(): Promise<DiscoveredAgent[]> {
      return Promise.resolve([]);
    },

    // Workflows
    loadWorkflow(filePath: string) {
      return Promise.resolve(workflows.get(filePath) ?? defaultWorkflow());
    },
    saveWorkflow(filePath: string, workflow: Workflow) {
      workflows.set(filePath, workflow);
      return Promise.resolve();
    },
    listWorkflows(_dir: string): Promise<string[]> {
      return Promise.resolve([]);
    },
    deleteWorkflow(filePath: string) {
      workflows.delete(filePath);
      return Promise.resolve();
    },
    createWorkflow(name: string, filePath: string) {
      const wf: Workflow = { ...defaultWorkflow(), name };
      workflows.set(filePath, wf);
      return Promise.resolve(wf);
    },
    getWorkflow(filePath: string) {
      return Promise.resolve(workflows.get(filePath) ?? null);
    },
    duplicateWorkflow(sourcePath: string, destPath: string) {
      const source = workflows.get(sourcePath) ?? defaultWorkflow();
      const copy = { ...source };
      workflows.set(destPath, copy);
      return Promise.resolve(copy);
    },
    saveToGlobal(_workflowPath: string) {
      return Promise.resolve('');
    },
    listGlobalWorkflows(): Promise<string[]> {
      return Promise.resolve([]);
    },

    // Agent Runtime
    createAgent(nodeId: string, workflowPath: string, maxRetries: number, fallbackAgent?: string) {
      const id = `agent-${++_agentIdCounter}`;
      const instance: AgentInstance = {
        id,
        node_id: nodeId,
        workflow_path: workflowPath,
        state: 'idle',
        pty_id: null,
        retry_count: 0,
        max_retries: maxRetries,
        fallback_agent: fallbackAgent ?? null,
        started_at: null,
        finished_at: null,
        error_message: null,
        output_buffer: [],
      };
      agents.set(id, instance);
      agentMessages.set(id, []);
      return Promise.resolve(id);
    },
    transitionAgent(agentId: string, newState: AgentState) {
      const agent = agents.get(agentId);
      if (agent) {
        agent.state = newState;
        agents.set(agentId, agent);
      }
      return Promise.resolve(newState);
    },
    setAgentPty(agentId: string, ptyId: string) {
      const agent = agents.get(agentId);
      if (agent) {
        agent.pty_id = ptyId;
        agents.set(agentId, agent);
      }
      return Promise.resolve();
    },
    setAgentError(agentId: string, message: string) {
      const agent = agents.get(agentId);
      if (agent) {
        agent.error_message = message;
        agent.state = 'error';
        agents.set(agentId, agent);
      }
      return Promise.resolve();
    },
    getAgent(agentId: string) {
      return Promise.resolve(agents.get(agentId) ?? null);
    },
    getWorkflowAgents(workflowPath: string): Promise<AgentInstance[]> {
      const result: AgentInstance[] = [];
      for (const agent of agents.values()) {
        if (agent.workflow_path === workflowPath) result.push(agent);
      }
      return Promise.resolve(result);
    },
    removeAgent(agentId: string) {
      agents.delete(agentId);
      agentMessages.delete(agentId);
      return Promise.resolve();
    },
    stopWorkflowAgents(workflowPath: string) {
      for (const [id, agent] of agents.entries()) {
        if (agent.workflow_path === workflowPath) {
          agent.state = 'idle';
          agents.set(id, agent);
        }
      }
      return Promise.resolve();
    },
    sendAgentMessage(from: string, to: string, payload: unknown) {
      const msg: AgentMessage = { from, to, payload, timestamp: new Date().toISOString() };
      const existing = agentMessages.get(to) ?? [];
      existing.push(msg);
      agentMessages.set(to, existing);
      return Promise.resolve();
    },
    getAgentMessages(agentId: string): Promise<AgentMessage[]> {
      return Promise.resolve(agentMessages.get(agentId) ?? []);
    },
    getAgentMemory(): Promise<any[]> {
      return Promise.resolve([]);
    },

    // Workflow Engine
    playWorkflow(_workflowPath: string, _cwd: string) {
      const id = `run-${++_runIdCounter}`;
      const run: WorkflowRun = {
        id,
        workflow_path: _workflowPath,
        status: 'running',
        node_states: {},
        started_at: new Date().toISOString(),
        finished_at: null,
      };
      runs.set(id, run);
      return Promise.resolve(id);
    },
    pauseWorkflow(runId: string) {
      const run = runs.get(runId);
      if (run) { run.status = 'paused'; runs.set(runId, run); }
      return Promise.resolve();
    },
    resumeWorkflow(runId: string, _workflowPath: string, _cwd: string) {
      const run = runs.get(runId);
      if (run) { run.status = 'running'; runs.set(runId, run); }
      return Promise.resolve();
    },
    stopWorkflow(runId: string) {
      const run = runs.get(runId);
      if (run) {
        run.status = 'stopped';
        run.finished_at = new Date().toISOString();
        runs.set(runId, run);
      }
      return Promise.resolve();
    },
    stepWorkflow(_runId: string, _workflowPath: string, _cwd: string): Promise<string | null> {
      return Promise.resolve(null);
    },
    getRunStatus(runId: string): Promise<WorkflowRun | null> {
      return Promise.resolve(runs.get(runId) ?? null);
    },
    notifyNodeCompleted(runId: string, nodeId: string, success: boolean, _workflowPath: string, _cwd: string) {
      const run = runs.get(runId);
      if (run) {
        run.node_states[nodeId] = {
          node_id: nodeId,
          agent_id: null,
          state: success ? 'success' : 'failed',
        };
        runs.set(runId, run);
      }
      return Promise.resolve();
    },

    // Structured Transport
    agentSend(_prompt: string, _provider: string, _model?: string, _sessionId?: string, _cwd?: string, _yolo?: boolean, _allowedTools?: string[]): Promise<string> {
      return Promise.resolve('mock-session-id');
    },
    agentStop(_sessionId: string): Promise<void> {
      return Promise.resolve();
    },
    agentGetEvents(_sessionId: string): Promise<AgentEvent[]> {
      return Promise.resolve([]);
    },
    onAgentEvent(_callback: (payload: AgentEventPayload) => void): Promise<() => void> {
      return Promise.resolve(() => {});
    },

    // Session Management
    sessionCreate(_provider: string, _model: string): Promise<string> {
      return Promise.resolve('mock-session-id');
    },
    sessionRestore(_sessionId: string): Promise<SessionHandle> {
      return Promise.resolve({
        id: 'mock-session-id',
        provider: 'mock',
        model: 'mock-model',
        cli_session_id: null,
        status: 'idle',
        title: 'Mock Session',
        created_at: Date.now(),
        last_active_at: Date.now(),
        event_count: 0,
        view_mode: 'chat',
        source: 'user',
        forked_from: null,
      });
    },
    sessionGetEvents(_sessionId: string): Promise<AgentEvent[]> {
      return Promise.resolve([]);
    },
    sessionList(): Promise<SessionSummary[]> {
      return Promise.resolve([]);
    },
    sessionDelete(_sessionId: string): Promise<void> {
      return Promise.resolve();
    },
    sessionRename(_sessionId: string, _title: string): Promise<void> {
      return Promise.resolve();
    },
    sessionFork(_sessionId: string, _forkEventIndex: number): Promise<string> {
      return Promise.resolve('mock-forked-session-id');
    },
    sessionSetViewMode(_sessionId: string, _mode: ViewMode): Promise<void> {
      return Promise.resolve();
    },

    // Capability & Health
    getCapabilities(): Promise<Record<string, NegotiatedCapabilities>> {
      return Promise.resolve({});
    },
    getProviderCapabilities(_provider: string): Promise<NegotiatedCapabilities> {
      return Promise.resolve({
        provider: _provider,
        cli_version: '0.0.0',
        cli_mode: 'basic_print',
        features: {},
        negotiated_at: Date.now(),
      });
    },
    getCliVersions(): Promise<CliVersionInfo[]> {
      return Promise.resolve([]);
    },
    getNormalizerVersions(_provider: string): Promise<VersionEntry[]> {
      return Promise.resolve([]);
    },
    rollbackNormalizer(_provider: string, _versionId: string): Promise<string> {
      return Promise.resolve('');
    },
    getHealthReport(_provider: string): Promise<HealthReport> {
      return Promise.resolve({
        provider: _provider,
        status: { type: 'healthy' },
        results: [],
        capabilities_confirmed: [],
        capabilities_missing: [],
        capabilities_broken: [],
        tested_at: Date.now(),
        cli_version: '0.0.0',
      });
    },
    getAllHealthReports(): Promise<Record<string, HealthReport>> {
      return Promise.resolve({});
    },

    // Analytics
    analyticsProvider(_provider: string, _from?: number, _to?: number): Promise<ProviderAnalytics> {
      return Promise.resolve({
        provider: _provider, total_sessions: 0, total_input_tokens: 0, total_output_tokens: 0,
        total_cache_creation_tokens: 0, total_cache_read_tokens: 0, cache_hit_rate: 0,
        total_errors: 0, recovered_errors: 0, error_rate: 0, avg_duration_ms: 0,
        avg_tokens_per_second: 0, most_used_model: '', total_tool_invocations: 0, total_cost_usd: null,
      });
    },
    analyticsCompare(_from?: number, _to?: number): Promise<ProviderAnalytics[]> {
      return Promise.resolve([]);
    },
    analyticsModelBreakdown(_provider: string, _from?: number, _to?: number): Promise<ModelAnalytics[]> {
      return Promise.resolve([]);
    },
    analyticsSession(_sessionId: string): Promise<SessionMetrics | null> {
      return Promise.resolve(null);
    },
    analyticsDaily(_provider?: string, _days?: number): Promise<DailyStats[]> {
      return Promise.resolve([]);
    },
    analyticsActive(): Promise<SessionMetrics[]> {
      return Promise.resolve([]);
    },

    // Provider management
    testProviderConnection(_provider: string): Promise<void> {
      return Promise.resolve();
    },
    onConnectionTest(_callback: (step: ConnectionTestStep) => void): Promise<() => void> {
      return Promise.resolve(() => {});
    },
    reloadNormalizers(): Promise<void> {
      return Promise.resolve();
    },

    // Workspace Trust
    checkWorkspaceTrust(_path: string): Promise<TrustCheckResult> {
      return Promise.resolve({ level: null, needs_prompt: false, folder_info: null, rename_hint: null });
    },
    setWorkspaceTrust(_path: string, _level: import('$lib/stores/workspace-trust').TrustLevel): Promise<void> {
      return Promise.resolve();
    },
    revokeWorkspaceTrust(_hash: string): Promise<void> {
      return Promise.resolve();
    },
    listWorkspaceTrust(): Promise<TrustEntry[]> {
      return Promise.resolve([]);
    },
    getNormalizerConfig(_provider: string): Promise<{ permission_args?: string[] } | null> {
      return Promise.resolve(null);
    },

    // File Operations (undo/trash)
    fileOpsSetProject(_path: string): Promise<void> { return Promise.resolve(); },
    fileOpsDelete(_path: string): Promise<void> { return Promise.resolve(); },
    fileOpsUndo(): Promise<string> { return Promise.resolve('Undo: restored file'); },
    fileOpsRecordCreate(_path: string): Promise<void> { return Promise.resolve(); },
    fileOpsRecordRename(_oldPath: string, _newPath: string): Promise<void> { return Promise.resolve(); },

    // App State Persistence
    getAppState(): Promise<AppState> {
      return Promise.resolve({
        last_active_project_id: null,
        recent_projects: [],
        window_state: null,
      });
    },
    saveAppState(_state: AppState): Promise<void> {
      return Promise.resolve();
    },
    getSetting(_key: string): Promise<unknown> {
      return Promise.resolve(null);
    },
    setSetting(_key: string, _value: unknown, _layer?: string): Promise<void> {
      return Promise.resolve();
    },
    getAllSettings(): Promise<Record<string, unknown>> {
      return Promise.resolve({});
    },
    reloadSettings(): Promise<void> {
      return Promise.resolve();
    },
    getProjectState(_projectId: string): Promise<ProjectState> {
      return Promise.resolve({
        active_session_id: null,
        open_files: [],
        active_file_path: null,
        panel_layout: null,
        last_model_used: null,
      });
    },
    saveProjectState(_projectId: string, _state: ProjectState): Promise<void> {
      return Promise.resolve();
    },

    // Permissions
    async recordPermissionDecision() {},
    async lookupPermissionDecision() { return null; },
    async clearPermissionSession() {},

    // Agent Memory v2
    memoryAdd(_entry: MemoryEntry): Promise<string> {
      return Promise.resolve('mock-memory-id');
    },
    memorySearch(_query: string, _scope: string, _scopeId?: string, _limit?: number): Promise<MemoryEntry[]> {
      return Promise.resolve([]);
    },
    memoryList(_scope: string, _scopeId?: string, _sort?: string, _limit?: number, _offset?: number): Promise<MemoryEntry[]> {
      return Promise.resolve([]);
    },
    memoryGet(_id: string): Promise<MemoryEntry | null> {
      return Promise.resolve(null);
    },

    // Batching (pass-through in mock — just runs the callback)
    async batch<T extends unknown[]>(
      fn: (ctx: Adapter) => [...{ [K in keyof T]: Promise<T[K]> }],
    ): Promise<T> {
      return Promise.all(fn(base)) as Promise<T>;
    },
  };

  return { ...base, ...overrides };
}
