<script lang="ts">
  import type { Adapter } from '$lib/adapter/index';
  import HiveControls from './HiveControls.svelte';
  import { currentRun, runStatus, statusSummary, nodeStates, agentOutputLog } from '$lib/stores/engine';
  import type { AgentLogEntry } from '$lib/stores/engine';
  import { currentWorkflow } from '$lib/stores/workflow';
  import { showHiveCanvas } from '$lib/stores/ui';
  import type { NodeRunState } from '$lib/adapter/index';
  import { onDestroy } from 'svelte';
  import { getStateColor } from '$lib/utils/state-color';

  let { adapter, cwd = '.' }: { adapter: Adapter; cwd?: string } = $props();

  let status = $state<string>('idle');
  let summary = $state<string>('');
  let nodes = $state<NodeRunState[]>([]);

  let logLines = $state<AgentLogEntry[]>([]);

  const unsubStatus = runStatus.subscribe((val) => { status = val; });
  const unsubSummary = statusSummary.subscribe((val) => { summary = val; });
  const unsubNodes = nodeStates.subscribe((val) => { nodes = val; });
  const unsubLog = agentOutputLog.subscribe((val) => { logLines = val; });

  function openCanvas() {
    showHiveCanvas.set(true);
  }

  onDestroy(() => { unsubStatus(); unsubSummary(); unsubNodes(); unsubLog(); });
</script>

<div class="hive-panel">
  <div class="panel-header">
    <span class="panel-title">Hive</span>
    <button class="expand-btn" onclick={openCanvas} title="Open full canvas" aria-label="Open full canvas">&#x2922;</button>
  </div>

  <!-- Mini-map: colored dots for each node -->
  <div class="mini-map">
    {#each nodes as ns}
      <span
        class="node-dot"
        style="background: {getStateColor(ns.state)}"
        title="{ns.node_id}: {ns.state}"
      ></span>
    {/each}
    {#if nodes.length === 0}
      <span class="no-nodes">No workflow loaded</span>
    {/if}
  </div>

  <!-- Status -->
  <div class="status-line">{summary || 'idle'}</div>

  <!-- Live Log -->
  <div class="live-log" role="log" aria-label="Agent output log" aria-live="polite">
    {#each logLines.slice(-50) as entry}
      <div class="log-line">
        <span class="log-node">[{entry.node_id}]</span>
        <span class="log-text">{entry.line}</span>
      </div>
    {/each}
    {#if logLines.length === 0}
      <div class="log-empty">No output yet</div>
    {/if}
  </div>

  <!-- Controls -->
  <HiveControls {adapter} {cwd} />
</div>

<style>
  .hive-panel {
    padding: var(--space-2);
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--stack-normal);
    font-family: var(--font-ui, sans-serif);
    background: var(--bg-secondary);
  }
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .panel-title {
    font-weight: 800;
    font-size: var(--font-size-tiny);
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .expand-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    padding: var(--stack-tight) var(--space-1);
    cursor: pointer;
    font-size: var(--font-size-base);
  }
  .expand-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .mini-map {
    display: flex;
    flex-wrap: wrap;
    gap: var(--stack-tight);
    padding: var(--space-1);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    min-height: 30px;
    align-items: center;
  }
  .node-dot {
    width: 10px;
    height: 10px;
    display: inline-block;
  }
  .no-nodes {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }
  .status-line {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }
  .live-log {
    flex: 1;
    overflow-y: auto;
    font-family: var(--font-mono);
    font-size: 0.75rem;
    padding: 0.25rem;
    background: var(--bg-secondary);
    border-radius: 4px;
  }
  .log-line { white-space: pre-wrap; word-break: break-all; }
  .log-node { color: var(--accent); margin-right: 0.5em; }
  .log-empty { color: var(--text-muted); text-align: center; padding: 1rem; }
</style>
