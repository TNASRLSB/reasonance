import type { AgentEvent, AgentEventPayload, SessionHandle, SessionSummary, ViewMode } from '$lib/types/agent-event';
import type { NegotiatedCapabilities, CliVersionInfo, VersionEntry, HealthReport } from '$lib/types/capability';
import type { ProviderAnalytics, ModelAnalytics, DailyStats, SessionMetrics, ConnectionTestStep } from '$lib/types/analytics';
import type { TrustCheckResult, TrustLevel, TrustEntry } from '$lib/stores/workspace-trust';
import type { AppState, ProjectState } from '$lib/types/app-state';

export interface ImageAttachment {
  data: string;      // base64 (no data: prefix)
  mimeType: string;  // image/png, image/jpeg, etc.
  name: string;
}

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
  // Batching
  batch<T extends unknown[]>(fn: (ctx: Adapter) => [...{ [K in keyof T]: Promise<T[K]> }]): Promise<T>;

  // Filesystem
  setProjectRoot(path: string): Promise<void>;
  readFile(path: string, signal?: AbortSignal): Promise<string>;
  writeFile(path: string, content: string): Promise<void>;
  listDir(path: string, respectGitignore?: boolean, signal?: AbortSignal): Promise<FileEntry[]>;
  watchFiles(path: string, callback: (event: FsEvent) => void): Promise<() => void>;
  getGitStatus(projectRoot: string, signal?: AbortSignal): Promise<Record<string, string>>;

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
  /** Remove backend entries for PTYs whose process has already exited. Returns swept IDs. */
  sweepPtys(): Promise<string[]>;
  /** Reconnect a dead PTY by killing the old entry and spawning a fresh process. Returns the new PTY ID. */
  reconnectPty(ptyId: string, command: string, args: string[], cwd: string): Promise<string>;
  /** Kill all active PTY processes. Returns number killed. */
  killAllPtys(): Promise<number>;

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
  getAgentMemory(nodeId: string, workflowPath: string, persist?: string): Promise<MemoryEntry[]>;

  // Workflow Engine
  playWorkflow(workflowPath: string, cwd: string): Promise<string>;
  pauseWorkflow(runId: string): Promise<void>;
  resumeWorkflow(runId: string, workflowPath: string, cwd: string): Promise<void>;
  stopWorkflow(runId: string): Promise<void>;
  stepWorkflow(runId: string, workflowPath: string, cwd: string): Promise<string | null>;
  getRunStatus(runId: string): Promise<WorkflowRun | null>;
  notifyNodeCompleted(runId: string, nodeId: string, success: boolean, workflowPath: string, cwd: string): Promise<void>;

  // Structured Transport
  agentSend(prompt: string, provider: string, model?: string, sessionId?: string, cwd?: string, yolo?: boolean, allowedTools?: string[], images?: ImageAttachment[]): Promise<string>;
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
  healNormalizer(provider: string): Promise<{ status: 'fixed' | 'failed'; message: string }>;

  // Workspace Trust
  checkWorkspaceTrust(path: string): Promise<TrustCheckResult>;
  setWorkspaceTrust(path: string, level: TrustLevel): Promise<void>;
  revokeWorkspaceTrust(hash: string): Promise<void>;
  listWorkspaceTrust(): Promise<TrustEntry[]>;
  getNormalizerConfig(provider: string): Promise<{
    binary: string;
    programmatic_args: string[];
    resume_args: string[];
    permission_args: string[];
    image_mode: string | null;
    transport_mode: string | null;
    rules_count: number;
  } | null>;

  // --- App State Persistence ---
  getAppState(): Promise<AppState>;
  saveAppState(state: AppState): Promise<void>;
  getProjectState(projectId: string): Promise<ProjectState>;
  saveProjectState(projectId: string, state: ProjectState): Promise<void>;

  // --- Model Slots ---
  getModelForSlot(provider: string, slot: string): Promise<string | null>;
  setModelSlot(provider: string, slot: string, model: string): Promise<void>;
  listModelSlots(provider: string): Promise<Array<[string, string | null]>>;

  // --- Layered Settings ---
  getSetting(key: string): Promise<unknown>;
  setSetting(key: string, value: unknown, layer?: string): Promise<void>;
  getAllSettings(): Promise<Record<string, unknown>>;
  reloadSettings(): Promise<void>;

  // --- File Operations (undo/trash) ---
  fileOpsSetProject(path: string): Promise<void>;
  fileOpsDelete(path: string): Promise<void>;
  fileOpsUndo(): Promise<string>;
  fileOpsRecordCreate(path: string): Promise<void>;
  fileOpsRecordRename(oldPath: string, newPath: string): Promise<void>;
  fileOpsMove(oldPath: string, newPath: string): Promise<void>;

  // Permissions
  recordPermissionDecision(sessionId: string, toolName: string, action: string, scope: string): Promise<void>;
  lookupPermissionDecision(sessionId: string, toolName: string): Promise<unknown>;
  clearPermissionSession(sessionId: string): Promise<void>;

  // Agent Memory v2
  memoryAdd(entry: MemoryEntry): Promise<string>;
  memorySearch(query: string, scope: string, scopeId?: string, limit?: number): Promise<MemoryEntry[]>;
  memoryList(scope: string, scopeId?: string, sort?: string, limit?: number, offset?: number): Promise<MemoryEntry[]>;
  memoryGet(id: string): Promise<MemoryEntry | null>;

  // Agent Communications (CommsBus)
  commsPublish(from: string, channel: CommsChannelType, payload: unknown, replyTo?: string, ttlSecs?: number): Promise<string>;
  commsGetMessages(nodeId: string, sinceId?: string): Promise<CommsMessage[]>;
  commsGetTopicMessages(topic: string, sinceId?: string): Promise<CommsMessage[]>;
  commsGetBroadcastMessages(workflowId: string, sinceId?: string): Promise<CommsMessage[]>;
  commsSweep(): Promise<number>;
  commsClearWorkflow(workflowId: string): Promise<void>;

  // Node Registry
  getNodeTypes(): Promise<NodeDescriptor[]>;
}

export interface GrepResult {
  path: string;
  line_number: number;
  line: string;
}

// === Agent Communications (CommsBus) Types ===

export type CommsChannelType =
  | { type: 'Direct'; value: { target_id: string } }
  | { type: 'Broadcast'; value: { workflow_id: string } }
  | { type: 'Topic'; value: { name: string } };

export interface CommsMessage {
  id: string;
  from: string;
  channel: CommsChannelType;
  payload: unknown;
  timestamp: string;
  reply_to: string | null;
  ttl_secs: number | null;
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

export interface MemoryEntry {
  id?: string;
  node_id?: string;
  project_id?: string | null;
  session_id?: string | null;
  run_id: string;
  timestamp: string;
  input_summary: string;
  output_summary: string;
  outcome: string;
  importance?: number;
  tags?: string;
  context: unknown;
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

// === Node Registry Types ===

export interface NodeDescriptor {
  type_id: string;
  display_name: string;
  description: string;
  category: string;
  config_schema: unknown;
}
