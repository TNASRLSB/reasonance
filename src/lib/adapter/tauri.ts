import { invoke } from '@tauri-apps/api/core';
import type { Adapter, FileEntry, FsEvent, PtyHandle, DiscoveredAgent, Workflow, AgentState, AgentInstance, AgentMessage, WorkflowRun } from './index';

export class TauriAdapter implements Adapter {
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
  async openExternal(path: string): Promise<void> {
    return invoke<void>('open_external', { path });
  }
  async getClipboard(): Promise<string> {
    throw new Error('Not implemented');
  }
  async setClipboard(text: string): Promise<void> {
    throw new Error('Not implemented');
  }
  async showNotification(title: string, body: string): Promise<void> {
    throw new Error('Not implemented');
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
}
