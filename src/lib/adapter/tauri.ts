import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';
import type { Adapter, FileEntry, FsEvent, GrepResult, PtyHandle, DiscoveredAgent, Workflow, AgentState, AgentInstance, AgentMessage, MemoryEntry, WorkflowRun, CommsChannelType, CommsMessage, NodeDescriptor, ImageAttachment } from './index';
import type { AgentEvent, AgentEventPayload, SessionHandle, SessionSummary, ViewMode } from '$lib/types/agent-event';
import type { NegotiatedCapabilities, CliVersionInfo, VersionEntry, HealthReport } from '$lib/types/capability';
import type { ProviderAnalytics, ModelAnalytics, DailyStats, SessionMetrics, ConnectionTestStep } from '$lib/types/analytics';
import type { TrustCheckResult, TrustLevel, TrustEntry } from '$lib/stores/workspace-trust';
import type { AppState, ProjectState } from '$lib/types/app-state';
import { batchSchemas, AgentEventSchema } from './batch-schemas';

interface PendingCall {
  command: string;
  args: Record<string, unknown>;
  resolve: (value: unknown) => void;
  reject: (error: unknown) => void;
  signal?: AbortSignal;
}

interface BatchCallResult {
  ok: unknown;
  err: unknown;
}

export class TauriAdapter implements Adapter {
  private queue: PendingCall[] = [];
  private flushScheduled = false;

  /**
   * Queue an IPC call for batched execution on the next microtask.
   */
  private enqueue<T>(command: string, args: Record<string, unknown>, signal?: AbortSignal): Promise<T> {
    return new Promise<T>((resolve, reject) => {
      if (signal?.aborted) {
        reject(signal.reason ?? new DOMException('Aborted', 'AbortError'));
        return;
      }

      const entry: PendingCall = {
        command,
        args,
        resolve: resolve as (value: unknown) => void,
        reject,
        signal,
      };

      if (signal) {
        const onAbort = () => {
          entry.reject(signal.reason ?? new DOMException('Aborted', 'AbortError'));
        };
        signal.addEventListener('abort', onAbort, { once: true });
      }

      this.queue.push(entry);

      if (!this.flushScheduled) {
        this.flushScheduled = true;
        queueMicrotask(() => this.flush());
      }
    });
  }

  /**
   * Flush the pending queue: deduplicate, send a single batch_invoke, distribute results.
   */
  private async flush(): Promise<void> {
    this.flushScheduled = false;
    const pending = this.queue;
    this.queue = [];
    if (pending.length === 0) return;

    // Filter out already-aborted entries
    const live = pending.filter((p) => !p.signal?.aborted);
    if (live.length === 0) return;

    // Deduplicate by (command, JSON.stringify(args))
    const keyMap = new Map<string, { command: string; args: Record<string, unknown>; entries: PendingCall[] }>();
    for (const entry of live) {
      const key = entry.command + '\0' + JSON.stringify(entry.args);
      const group = keyMap.get(key);
      if (group) {
        group.entries.push(entry);
      } else {
        keyMap.set(key, { command: entry.command, args: entry.args, entries: [entry] });
      }
    }

    const calls = [...keyMap.values()].map(({ command, args }) => ({ command, args }));
    const groups = [...keyMap.values()];

    const t0 = performance.now();

    try {
      const results = await invoke<BatchCallResult[]>('batch_invoke', { calls });
      const elapsed = performance.now() - t0;

      if (import.meta.env.DEV) {
        console.debug(
          `[batch] ${calls.length} calls (${live.length} queued, ${live.length - calls.length} deduped) in ${elapsed.toFixed(1)}ms`,
        );
      }

      for (let i = 0; i < groups.length; i++) {
        const result = results[i];
        const { entries } = groups[i];
        if (result.err !== null && result.err !== undefined) {
          for (const e of entries) {
            if (!e.signal?.aborted) e.reject(result.err);
          }
        } else {
          // Validate via Zod if a schema exists for this command
          const schema = batchSchemas[groups[i].command];
          let value = result.ok;
          if (schema) {
            const parsed = schema.safeParse(value);
            if (!parsed.success) {
              for (const e of entries) {
                if (!e.signal?.aborted) e.reject(parsed.error);
              }
              continue;
            }
            value = parsed.data;
          }
          for (const e of entries) {
            if (!e.signal?.aborted) e.resolve(value);
          }
        }
      }
    } catch (err) {
      // Total failure — reject everything
      for (const group of groups) {
        for (const e of group.entries) e.reject(err);
      }
    }
  }

  /**
   * Explicit batch API: captures calls made inside `fn` and flushes them as one batch.
   */
  async batch<T extends unknown[]>(
    fn: (ctx: Adapter) => [...{ [K in keyof T]: Promise<T[K]> }],
  ): Promise<T> {
    const saved = this.queue;
    this.queue = [];

    const promises = fn(this);

    const captured = this.queue;
    this.queue = saved;

    // Flush captured batch directly (bypass microtask scheduling)
    if (captured.length > 0) {
      const prev = this.queue;
      this.queue = captured;
      await this.flush();
      this.queue = prev;
    }

    return Promise.all(promises) as Promise<T>;
  }

  async setProjectRoot(path: string): Promise<void> {
    return this.enqueue('set_project_root', { path });
  }
  async readFile(path: string, signal?: AbortSignal): Promise<string> {
    return this.enqueue('read_file', { path }, signal);
  }
  async writeFile(path: string, content: string): Promise<void> {
    return this.enqueue('write_file', { path, content });
  }
  async listDir(path: string, respectGitignore?: boolean, signal?: AbortSignal): Promise<FileEntry[]> {
    return this.enqueue('list_dir', { path, respectGitignore: respectGitignore ?? true }, signal);
  }
  async watchFiles(path: string, callback: (event: FsEvent) => void): Promise<() => void> {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<FsEvent>('fs-change', (event) => { callback(event.payload); });
    // Non-batchable: sets up persistent watcher — inline Zod validation
    const result = await invoke('start_watching', { path });
    z.null().parse(result);
    return unlisten;
  }
  async getGitStatus(projectRoot: string, signal?: AbortSignal): Promise<Record<string, string>> {
    return this.enqueue('get_git_status', { projectRoot }, signal);
  }
  async openExternal(path: string): Promise<void> {
    return this.enqueue('open_external', { path });
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
    return this.enqueue('discover_llms', {});
  }
  async grepFiles(path: string, pattern: string, respectGitignore: boolean): Promise<GrepResult[]> {
    return this.enqueue('grep_files', { path, pattern, respectGitignore });
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
    const id: string = await this.enqueue('spawn_process', { command, args, cwd });
    return { id };
  }
  async killProcess(id: string): Promise<void> {
    return this.enqueue('kill_process', { id });
  }
  async resizePty(id: string, cols: number, rows: number): Promise<void> {
    return this.enqueue('resize_pty', { id, cols: Math.floor(cols), rows: Math.floor(rows) });
  }
  async writePty(id: string, data: string): Promise<void> {
    return this.enqueue('write_pty', { id, data });
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
    return this.enqueue('sweep_ptys', {});
  }
  async reconnectPty(ptyId: string, command: string, args: string[], cwd: string): Promise<string> {
    return this.enqueue('reconnect_pty', { ptyId, command, args, cwd });
  }
  async killAllPtys(): Promise<number> {
    return this.enqueue('kill_all_ptys', {});
  }
  async readConfig(): Promise<string> {
    return this.enqueue('read_config', {});
  }
  async writeConfig(content: string): Promise<void> {
    return this.enqueue('write_config', { content });
  }
  async storeShadow(path: string, content: string): Promise<void> {
    return this.enqueue('store_shadow', { path, content });
  }
  async getShadow(path: string): Promise<string | null> {
    return this.enqueue('get_shadow', { path });
  }

  // Discovery
  async discoverAgents(): Promise<DiscoveredAgent[]> {
    return this.enqueue('discover_agents', {});
  }
  async getDiscoveredAgents(): Promise<DiscoveredAgent[]> {
    return this.enqueue('get_discovered_agents', {});
  }

  // Workflows
  async loadWorkflow(filePath: string): Promise<Workflow> {
    return this.enqueue('load_workflow', { filePath });
  }
  async saveWorkflow(filePath: string, workflow: Workflow): Promise<void> {
    return this.enqueue('save_workflow', { filePath, workflow });
  }
  async listWorkflows(dir: string): Promise<string[]> {
    return this.enqueue('list_workflows', { dir });
  }
  async deleteWorkflow(filePath: string): Promise<void> {
    return this.enqueue('delete_workflow', { filePath });
  }
  async createWorkflow(name: string, filePath: string): Promise<Workflow> {
    return this.enqueue('create_workflow', { name, filePath });
  }
  async getWorkflow(filePath: string): Promise<Workflow | null> {
    return this.enqueue('get_workflow', { filePath });
  }
  async duplicateWorkflow(sourcePath: string, destPath: string): Promise<Workflow> {
    return this.enqueue('duplicate_workflow', { sourcePath, destPath });
  }
  async saveToGlobal(workflowPath: string): Promise<string> {
    return this.enqueue('save_to_global', { workflowPath });
  }
  async listGlobalWorkflows(): Promise<string[]> {
    return this.enqueue('list_global_workflows', {});
  }

  // Agent Runtime
  async createAgent(nodeId: string, workflowPath: string, maxRetries: number, fallbackAgent?: string): Promise<string> {
    return this.enqueue('create_agent', { nodeId, workflowPath, maxRetries, fallbackAgent: fallbackAgent ?? null });
  }
  async transitionAgent(agentId: string, newState: AgentState): Promise<AgentState> {
    return this.enqueue('transition_agent', { agentId, newState });
  }
  async setAgentPty(agentId: string, ptyId: string): Promise<void> {
    return this.enqueue('set_agent_pty', { agentId, ptyId });
  }
  async setAgentError(agentId: string, message: string): Promise<void> {
    return this.enqueue('set_agent_error', { agentId, message });
  }
  async getAgent(agentId: string): Promise<AgentInstance | null> {
    return this.enqueue('get_agent', { agentId });
  }
  async getWorkflowAgents(workflowPath: string): Promise<AgentInstance[]> {
    return this.enqueue('get_workflow_agents', { workflowPath });
  }
  async removeAgent(agentId: string): Promise<void> {
    return this.enqueue('remove_agent', { agentId });
  }
  async stopWorkflowAgents(workflowPath: string): Promise<void> {
    return this.enqueue('stop_workflow_agents', { workflowPath });
  }
  async sendAgentMessage(from: string, to: string, payload: unknown): Promise<void> {
    return this.enqueue('send_agent_message', { from, to, payload });
  }
  async getAgentMessages(agentId: string): Promise<AgentMessage[]> {
    return this.enqueue('get_agent_messages', { agentId });
  }
  async getAgentMemory(nodeId: string, workflowPath: string, persist = 'workflow'): Promise<MemoryEntry[]> {
    return this.enqueue('get_agent_memory', { nodeId, workflowPath, persist });
  }

  // Workflow Engine
  async playWorkflow(workflowPath: string, cwd: string): Promise<string> {
    return this.enqueue('play_workflow', { workflowPath, cwd });
  }
  async pauseWorkflow(runId: string): Promise<void> {
    return this.enqueue('pause_workflow', { runId });
  }
  async resumeWorkflow(runId: string, workflowPath: string, cwd: string): Promise<void> {
    return this.enqueue('resume_workflow', { runId, workflowPath, cwd });
  }
  async stopWorkflow(runId: string): Promise<void> {
    return this.enqueue('stop_workflow', { runId });
  }
  async stepWorkflow(runId: string, workflowPath: string, cwd: string): Promise<string | null> {
    return this.enqueue('step_workflow', { runId, workflowPath, cwd });
  }
  async getRunStatus(runId: string): Promise<WorkflowRun | null> {
    return this.enqueue('get_run_status', { runId });
  }
  async notifyNodeCompleted(runId: string, nodeId: string, success: boolean, workflowPath: string, cwd: string): Promise<void> {
    return this.enqueue('notify_node_completed', { runId, nodeId, success, workflowPath, cwd });
  }

  // Structured Transport (non-batchable: long-running/streaming — inline Zod validation)
  async agentSend(prompt: string, provider: string, model?: string, sessionId?: string, cwd?: string, yolo?: boolean, allowedTools?: string[], images?: ImageAttachment[]): Promise<string> {
    const result = await invoke('agent_send', {
      request: {
        prompt, provider, model: model ?? null, context: [],
        session_id: sessionId ?? null, system_prompt: null,
        max_tokens: null, allowed_tools: allowedTools ?? null,
        cwd: cwd ?? null, yolo: yolo ?? false,
        images: (images ?? []).map(img => ({
          data: img.data, mime_type: img.mimeType, name: img.name,
        })),
      }
    });
    return z.string().parse(result);
  }
  async agentStop(sessionId: string): Promise<void> {
    const result = await invoke('agent_stop', { sessionId });
    z.null().parse(result);
  }
  async agentGetEvents(sessionId: string): Promise<AgentEvent[]> {
    const result = await invoke('agent_get_events', { sessionId });
    return z.array(AgentEventSchema).parse(result) as AgentEvent[];
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
    return this.enqueue('session_create', { provider, model });
  }
  async sessionRestore(sessionId: string): Promise<SessionHandle> {
    return this.enqueue('session_restore', { sessionId });
  }
  async sessionGetEvents(sessionId: string): Promise<AgentEvent[]> {
    return this.enqueue('session_get_events', { sessionId });
  }
  async sessionList(): Promise<SessionSummary[]> {
    return this.enqueue('session_list', {});
  }
  async sessionDelete(sessionId: string): Promise<void> {
    return this.enqueue('session_delete', { sessionId });
  }
  async sessionRename(sessionId: string, title: string): Promise<void> {
    return this.enqueue('session_rename', { sessionId, title });
  }
  async sessionFork(sessionId: string, forkEventIndex: number): Promise<string> {
    return this.enqueue('session_fork', { sessionId, forkEventIndex });
  }
  async sessionSetViewMode(sessionId: string, mode: ViewMode): Promise<void> {
    return this.enqueue('session_set_view_mode', { sessionId, mode });
  }

  // Capability & Health
  async getCapabilities(): Promise<Record<string, NegotiatedCapabilities>> {
    return this.enqueue('get_capabilities', {});
  }
  async getProviderCapabilities(provider: string): Promise<NegotiatedCapabilities> {
    return this.enqueue('get_provider_capabilities', { provider });
  }
  async getCliVersions(): Promise<CliVersionInfo[]> {
    return this.enqueue('get_cli_versions', {});
  }
  async getNormalizerVersions(provider: string): Promise<VersionEntry[]> {
    return this.enqueue('get_normalizer_versions', { provider });
  }
  async rollbackNormalizer(provider: string, versionId: string): Promise<string> {
    return this.enqueue('rollback_normalizer', { provider, versionId });
  }
  async getHealthReport(provider: string): Promise<HealthReport> {
    return this.enqueue('get_health_report', { provider });
  }
  async getAllHealthReports(): Promise<Record<string, HealthReport>> {
    return this.enqueue('get_all_health_reports', {});
  }

  async analyticsProvider(provider: string, from?: number, to?: number): Promise<ProviderAnalytics> {
    return this.enqueue('analytics_provider', { provider, from, to });
  }
  async analyticsCompare(from?: number, to?: number): Promise<ProviderAnalytics[]> {
    return this.enqueue('analytics_compare', { from, to });
  }
  async analyticsModelBreakdown(provider: string, from?: number, to?: number): Promise<ModelAnalytics[]> {
    return this.enqueue('analytics_model_breakdown', { provider, from, to });
  }
  async analyticsSession(sessionId: string): Promise<SessionMetrics | null> {
    return this.enqueue('analytics_session', { sessionId });
  }
  async analyticsDaily(provider?: string, days?: number): Promise<DailyStats[]> {
    return this.enqueue('analytics_daily', { provider, days });
  }
  async analyticsActive(): Promise<SessionMetrics[]> {
    return this.enqueue('analytics_active', {});
  }
  // Non-batchable: long-running, emits step events via Tauri event system
  async testProviderConnection(provider: string): Promise<void> {
    const result = await invoke('test_provider_connection', { provider });
    z.null().parse(result);
  }
  async onConnectionTest(callback: (step: ConnectionTestStep) => void): Promise<() => void> {
    const { listen } = await import('@tauri-apps/api/event');
    return listen<ConnectionTestStep>('connection_test_step', (event) => {
      callback(event.payload);
    });
  }
  // Non-batchable: reloads all normalizers from disk, runs health checks
  async reloadNormalizers(): Promise<void> {
    const result = await invoke('reload_normalizers');
    z.null().parse(result);
  }
  async healNormalizer(provider: string): Promise<{ status: 'fixed' | 'failed'; message: string }> {
    return this.enqueue('heal_normalizer', { provider });
  }

  // Workspace Trust
  async checkWorkspaceTrust(path: string): Promise<TrustCheckResult> {
    return this.enqueue('check_workspace_trust', { path });
  }

  async setWorkspaceTrust(path: string, level: TrustLevel): Promise<void> {
    return this.enqueue('set_workspace_trust', { path, level });
  }

  async revokeWorkspaceTrust(hash: string): Promise<void> {
    return this.enqueue('revoke_workspace_trust', { hash });
  }

  async listWorkspaceTrust(): Promise<TrustEntry[]> {
    return this.enqueue('list_workspace_trust', {});
  }

  async getNormalizerConfig(provider: string): Promise<{ permission_args?: string[] } | null> {
    return this.enqueue('get_normalizer_config', { provider });
  }

  // App State Persistence
  async getAppState(): Promise<AppState> {
    return this.enqueue('get_app_state', {});
  }
  async saveAppState(state: AppState): Promise<void> {
    return this.enqueue('save_app_state', { state });
  }
  async getProjectState(projectId: string): Promise<ProjectState> {
    return this.enqueue('get_project_state', { projectId });
  }
  async saveProjectState(projectId: string, state: ProjectState): Promise<void> {
    return this.enqueue('save_project_state', { projectId, state });
  }

  // File Operations (undo/trash)
  async fileOpsSetProject(path: string): Promise<void> {
    return this.enqueue('file_ops_set_project', { path });
  }
  async fileOpsDelete(path: string): Promise<void> {
    return this.enqueue('file_ops_delete', { path });
  }
  async fileOpsUndo(): Promise<string> {
    return this.enqueue('file_ops_undo', {});
  }
  async fileOpsRecordCreate(path: string): Promise<void> {
    return this.enqueue('file_ops_record_create', { path });
  }
  async fileOpsRecordRename(oldPath: string, newPath: string): Promise<void> {
    return this.enqueue('file_ops_record_rename', { oldPath, newPath });
  }
  async fileOpsMove(oldPath: string, newPath: string): Promise<void> {
    return this.enqueue('file_ops_move', { oldPath, newPath });
  }

  // Permissions
  async recordPermissionDecision(sessionId: string, toolName: string, action: string, scope: string): Promise<void> {
    return this.enqueue('record_permission_decision', { sessionId, toolName, action, scope }) as Promise<void>;
  }
  async lookupPermissionDecision(sessionId: string, toolName: string): Promise<unknown> {
    return this.enqueue('lookup_permission_decision', { sessionId, toolName }) as Promise<unknown>;
  }
  async clearPermissionSession(sessionId: string): Promise<void> {
    return this.enqueue('clear_permission_session', { sessionId }) as Promise<void>;
  }

  // Agent Memory v2
  async memoryAdd(entry: MemoryEntry): Promise<string> {
    return this.enqueue('memory_add_entry', { entry });
  }
  async memorySearch(query: string, scope: string, scopeId?: string, limit?: number): Promise<MemoryEntry[]> {
    return this.enqueue('memory_search', { query, scope, scopeId, limit: limit ?? 20 });
  }
  async memoryList(scope: string, scopeId?: string, sort?: string, limit?: number, offset?: number): Promise<MemoryEntry[]> {
    return this.enqueue('memory_list', { scope, scopeId, sort: sort ?? 'recency', limit: limit ?? 50, offset: offset ?? 0 });
  }
  async memoryGet(id: string): Promise<MemoryEntry | null> {
    return this.enqueue('memory_get', { id });
  }

  // Agent Communications (CommsBus)
  async commsPublish(from: string, channel: CommsChannelType, payload: unknown, replyTo?: string, ttlSecs?: number): Promise<string> {
    return this.enqueue('agent_publish_message', {
      from,
      channel,
      payload,
      replyTo: replyTo ?? null,
      ttlSecs: ttlSecs ?? null,
    });
  }
  async commsGetMessages(nodeId: string, sinceId?: string): Promise<CommsMessage[]> {
    return this.enqueue('agent_get_messages', { nodeId, sinceId: sinceId ?? null });
  }
  async commsGetTopicMessages(topic: string, sinceId?: string): Promise<CommsMessage[]> {
    return this.enqueue('agent_get_topic_messages', { topic, sinceId: sinceId ?? null });
  }
  async commsGetBroadcastMessages(workflowId: string, sinceId?: string): Promise<CommsMessage[]> {
    return this.enqueue('agent_get_broadcast_messages', { workflowId, sinceId: sinceId ?? null });
  }
  async commsSweep(): Promise<number> {
    return this.enqueue('agent_sweep_messages', {});
  }
  async commsClearWorkflow(workflowId: string): Promise<void> {
    return this.enqueue('agent_clear_workflow_messages', { workflowId });
  }

  // Multi-project
  async addProject(id: string, rootPath: string, trustLevel: string): Promise<void> {
    return this.enqueue('add_project', { id, rootPath, trustLevel });
  }

  // Model Slots
  async getModelForSlot(provider: string, slot: string): Promise<string | null> {
    return this.enqueue('get_model_for_slot', { provider, slot });
  }
  async setModelSlot(provider: string, slot: string, model: string): Promise<void> {
    return this.enqueue('set_model_slot', { provider, slot, model });
  }
  async listModelSlots(provider: string): Promise<Array<[string, string | null]>> {
    return this.enqueue('list_model_slots', { provider });
  }

  async getSetting(key: string): Promise<unknown> {
    return this.enqueue('get_setting', { key });
  }

  async setSetting(key: string, value: unknown, layer?: string): Promise<void> {
    return this.enqueue('set_setting', { key, value, layer: layer ?? 'user' });
  }

  async getAllSettings(): Promise<Record<string, unknown>> {
    return this.enqueue('get_all_settings', {});
  }

  async reloadSettings(): Promise<void> {
    return this.enqueue('reload_settings', {});
  }

  async removeProject(id: string): Promise<void> {
    return this.enqueue('remove_project', { id });
  }

  async setActiveProject(id: string): Promise<void> {
    return this.enqueue('set_active_project', { id });
  }

  async getProjectRoot(projectId: string): Promise<string> {
    return this.enqueue('get_project_root', { projectId });
  }

  async killProjectProcesses(id: string): Promise<string[]> {
    return this.enqueue('kill_project_ptys', { projectId: id });
  }

  // Node Registry
  async getNodeTypes(): Promise<NodeDescriptor[]> {
    return this.enqueue('get_node_types', {});
  }
}
