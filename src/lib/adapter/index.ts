import type { AgentEvent, AgentEventPayload, SessionHandle, SessionSummary, ViewMode } from '$lib/types/agent-event';
import type { NegotiatedCapabilities, CliVersionInfo, VersionEntry, HealthReport } from '$lib/types/capability';
import type { ProviderAnalytics, ModelAnalytics, DailyStats, SessionMetrics, ConnectionTestStep } from '$lib/types/analytics';
import type { TrustCheckResult, TrustLevel, TrustEntry } from '$lib/stores/workspace-trust';

export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  modified: number; // unix timestamp ms
  isGitignored: boolean;
}

export interface PtyHandle {
  id: string;
}

export interface Adapter {
  // Filesystem
  setProjectRoot(path: string): Promise<void>;
  readFile(path: string): Promise<string>;
  writeFile(path: string, content: string): Promise<void>;
  listDir(path: string, respectGitignore?: boolean): Promise<FileEntry[]>;
  watchFiles(path: string, callback: (event: FsEvent) => void): Promise<() => void>;

  // System
  openExternal(path: string): Promise<void>;
  getClipboard(): Promise<string>;
  setClipboard(text: string): Promise<void>;
  showNotification(title: string, body: string): Promise<void>;
  minimizeWindow(): Promise<void>;
  maximizeWindow(): Promise<void>;
  closeWindow(): Promise<void>;
  startDragging(): Promise<void>;
  onWindowClose(callback: () => Promise<void>): Promise<void>;

  // Discovery
  discoverLlms(): Promise<Array<{ name: string; command: string; found: boolean }>>;
  grepFiles(path: string, pattern: string, respectGitignore: boolean): Promise<GrepResult[]>;

  // Dialogs
  openFolderDialog(): Promise<string | null>;
  openFileDialog(filters?: Array<{ name: string; extensions: string[] }>): Promise<string | null>;
  saveFileDialog(defaultPath?: string, filters?: Array<{ name: string; extensions: string[] }>): Promise<string | null>;

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

  // Agent Discovery
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

  // Structured Transport
  agentSend(prompt: string, provider: string, model?: string, sessionId?: string, cwd?: string, yolo?: boolean, allowedTools?: string[]): Promise<string>;
  agentStop(sessionId: string): Promise<void>;
  agentGetEvents(sessionId: string): Promise<AgentEvent[]>;
  onAgentEvent(callback: (payload: AgentEventPayload) => void): Promise<() => void>;

  // Session Management
  sessionCreate(provider: string, model: string): Promise<string>;
  sessionRestore(sessionId: string): Promise<SessionHandle>;
  sessionGetEvents(sessionId: string): Promise<AgentEvent[]>;
  sessionList(): Promise<SessionSummary[]>;
  sessionDelete(sessionId: string): Promise<void>;
  sessionRename(sessionId: string, title: string): Promise<void>;
  sessionFork(sessionId: string, forkEventIndex: number): Promise<string>;
  sessionSetViewMode(sessionId: string, mode: ViewMode): Promise<void>;

  // Capability & health commands
  getCapabilities(): Promise<Record<string, NegotiatedCapabilities>>;
  getProviderCapabilities(provider: string): Promise<NegotiatedCapabilities>;
  getCliVersions(): Promise<CliVersionInfo[]>;
  getNormalizerVersions(provider: string): Promise<VersionEntry[]>;
  rollbackNormalizer(provider: string, versionId: string): Promise<string>;
  getHealthReport(provider: string): Promise<HealthReport>;
  getAllHealthReports(): Promise<Record<string, HealthReport>>;

  // Analytics
  analyticsProvider(provider: string, from?: number, to?: number): Promise<ProviderAnalytics>;
  analyticsCompare(from?: number, to?: number): Promise<ProviderAnalytics[]>;
  analyticsModelBreakdown(provider: string, from?: number, to?: number): Promise<ModelAnalytics[]>;
  analyticsSession(sessionId: string): Promise<SessionMetrics | null>;
  analyticsDaily(provider?: string, days?: number): Promise<DailyStats[]>;
  analyticsActive(): Promise<SessionMetrics[]>;

  // Provider management
  testProviderConnection(provider: string): Promise<void>;
  onConnectionTest(callback: (step: ConnectionTestStep) => void): Promise<() => void>;
  reloadNormalizers(): Promise<void>;

  // Workspace Trust
  checkWorkspaceTrust(path: string): Promise<TrustCheckResult>;
  setWorkspaceTrust(path: string, level: TrustLevel): Promise<void>;
  revokeWorkspaceTrust(hash: string): Promise<void>;
  listWorkspaceTrust(): Promise<TrustEntry[]>;
  getNormalizerConfig(provider: string): Promise<{ permission_args?: string[] } | null>;
}

export interface GrepResult {
  path: string;
  line_number: number;
  line: string;
}

export interface FsEvent {
  type: 'create' | 'modify' | 'remove';
  path: string;
}

// === Agent Hive Types ===

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

export type AgentState = 'idle' | 'queued' | 'running' | 'success' | 'failed' | 'retrying' | 'fallback' | 'error' | 'skipped';

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
  output_buffer: string[];
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
  id?: string;
  from: string;
  to: string;
  label?: string;
}

export interface MemoryConfig {
  enabled: boolean;
  maxEntries: number;
  persist: 'none' | 'workflow' | 'global';
}

export interface WorkflowSettings {
  max_concurrent_agents: number;
  default_retry: number;
  timeout: number;
  permissionLevel: 'supervised' | 'trusted' | 'dry-run';
}

export interface Workflow {
  name: string;
  version: string;
  schemaVersion: number;
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
