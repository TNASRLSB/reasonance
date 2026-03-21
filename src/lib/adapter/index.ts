export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  modified: number; // unix timestamp ms
}

export interface PtyHandle {
  id: string;
}

export interface Adapter {
  // Filesystem
  readFile(path: string): Promise<string>;
  writeFile(path: string, content: string): Promise<void>;
  listDir(path: string, respectGitignore?: boolean): Promise<FileEntry[]>;
  watchFiles(path: string, callback: (event: FsEvent) => void): Promise<() => void>;

  // System
  openExternal(path: string): Promise<void>;
  getClipboard(): Promise<string>;
  setClipboard(text: string): Promise<void>;
  showNotification(title: string, body: string): Promise<void>;

  // PTY
  spawnProcess(command: string, args: string[], cwd: string): Promise<PtyHandle>;
  killProcess(id: string): Promise<void>;
  resizePty(id: string, cols: number, rows: number): Promise<void>;
  writePty(id: string, data: string): Promise<void>;
  onPtyData(id: string, callback: (data: string) => void): Promise<() => void>;
  onPtyExit(id: string, callback: (code: number) => void): Promise<() => void>;

  // Config
  readConfig(): Promise<string>;
  writeConfig(content: string): Promise<void>;

  // Shadow copies
  storeShadow(path: string, content: string): Promise<void>;
  getShadow(path: string): Promise<string | null>;

  // Discovery
  discoverAgents(): Promise<DiscoveredAgent[]>;
  getDiscoveredAgents(): Promise<DiscoveredAgent[]>;

  // Workflows
  loadWorkflow(filePath: string): Promise<Workflow>;
  saveWorkflow(filePath: string, workflow: Workflow): Promise<void>;
  listWorkflows(dir: string): Promise<string[]>;
  deleteWorkflow(filePath: string): Promise<void>;
  createWorkflow(name: string, filePath: string): Promise<Workflow>;
  getWorkflow(filePath: string): Promise<Workflow | null>;
  duplicateWorkflow(sourcePath: string, destPath: string): Promise<Workflow>;
  saveToGlobal(workflowPath: string): Promise<string>;
  listGlobalWorkflows(): Promise<string[]>;

  // Agent Runtime
  createAgent(nodeId: string, workflowPath: string, maxRetries: number, fallbackAgent?: string): Promise<string>;
  transitionAgent(agentId: string, newState: AgentState): Promise<AgentState>;
  setAgentPty(agentId: string, ptyId: string): Promise<void>;
  setAgentError(agentId: string, message: string): Promise<void>;
  getAgent(agentId: string): Promise<AgentInstance | null>;
  getWorkflowAgents(workflowPath: string): Promise<AgentInstance[]>;
  removeAgent(agentId: string): Promise<void>;
  stopWorkflowAgents(workflowPath: string): Promise<void>;
  sendAgentMessage(from: string, to: string, payload: unknown): Promise<void>;
  getAgentMessages(agentId: string): Promise<AgentMessage[]>;

  // Workflow Engine
  playWorkflow(workflowPath: string, cwd: string): Promise<string>;
  pauseWorkflow(runId: string): Promise<void>;
  resumeWorkflow(runId: string, workflowPath: string, cwd: string): Promise<void>;
  stopWorkflow(runId: string): Promise<void>;
  stepWorkflow(runId: string, workflowPath: string, cwd: string): Promise<string | null>;
  getRunStatus(runId: string): Promise<WorkflowRun | null>;
  notifyNodeCompleted(runId: string, nodeId: string, success: boolean, workflowPath: string, cwd: string): Promise<void>;
}

export interface FsEvent {
  type: 'create' | 'modify' | 'remove';
  path: string;
}

// === Agent Swarm Types ===

export interface CapabilityProfile {
  read_file: boolean;
  write_file: boolean;
  execute_command: boolean;
  web_search: boolean;
  image_input: boolean;
  long_context: boolean;
}

export interface DiscoveredAgent {
  name: string;
  source: 'cli' | 'api' | 'manual';
  command: string | null;
  endpoint: string | null;
  models: string[];
  capabilities: CapabilityProfile;
  max_context: number | null;
  available: boolean;
}

export type AgentState = 'idle' | 'queued' | 'running' | 'success' | 'failed' | 'retrying' | 'fallback' | 'error';

export interface AgentInstance {
  id: string;
  node_id: string;
  workflow_path: string;
  state: AgentState;
  pty_id: string | null;
  retry_count: number;
  max_retries: number;
  fallback_agent: string | null;
  started_at: string | null;
  finished_at: string | null;
  error_message: string | null;
}

export interface AgentMessage {
  from: string;
  to: string;
  payload: unknown;
  timestamp: string;
}

export interface WorkflowNode {
  id: string;
  type: 'agent' | 'resource' | 'logic';
  label: string;
  config: Record<string, unknown>;
  position: { x: number; y: number };
}

export interface WorkflowEdge {
  id: string;
  from: string;
  to: string;
  label?: string;
}

export interface WorkflowSettings {
  max_concurrent_agents: number;
  default_retry: number;
  timeout: number;
}

export interface Workflow {
  name: string;
  version: string;
  description?: string;
  created?: string;
  modified?: string;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  settings: WorkflowSettings;
}

// === Workflow Engine Types ===

export type RunStatus = 'idle' | 'running' | 'paused' | 'completed' | 'failed' | 'stopped';

export interface NodeRunState {
  node_id: string;
  agent_id: string | null;
  state: AgentState;
}

export interface WorkflowRun {
  id: string;
  workflow_path: string;
  status: RunStatus;
  node_states: Record<string, NodeRunState>;
  started_at: string | null;
  finished_at: string | null;
}
