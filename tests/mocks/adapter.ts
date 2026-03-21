import type {
  Adapter,
  FileEntry,
  PtyHandle,
  FsEvent,
  DiscoveredAgent,
  Workflow,
  AgentInstance,
  AgentState,
  AgentMessage,
  WorkflowRun,
} from '$lib/adapter/index';

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
    nodes: [],
    edges: [],
    settings: { max_concurrent_agents: 4, default_retry: 3, timeout: 300 },
  });

  const base: Adapter = {
    // Filesystem
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
  };

  return { ...base, ...overrides };
}
