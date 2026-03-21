<script lang="ts">
  import type { Adapter } from '$lib/adapter/index';
  import SwarmControls from './SwarmControls.svelte';
  import { currentRun, runStatus, statusSummary, nodeStates } from '$lib/stores/engine';
  import { currentWorkflow } from '$lib/stores/workflow';
  import { showSwarmCanvas } from '$lib/stores/ui';
  import type { NodeRunState } from '$lib/adapter/index';
  import { onDestroy } from 'svelte';

  let { adapter, cwd = '.' }: { adapter: Adapter; cwd?: string } = $props();

  let status = $state<string>('idle');
  let summary = $state<string>('');
  let nodes = $state<NodeRunState[]>([]);

  const unsubStatus = runStatus.subscribe((val) => { status = val; });
  const unsubSummary = statusSummary.subscribe((val) => { summary = val; });
  const unsubNodes = nodeStates.subscribe((val) => { nodes = val; });

  const stateColors: Record<string, string> = {
    idle: '#666666',
    queued: '#ca8a04',
    running: '#1d4ed8',
    success: '#16a34a',
    failed: '#dc2626',
    retrying: '#ea580c',
    fallback: '#ea580c',
    error: '#dc2626',
  };

  function openCanvas() {
    showSwarmCanvas.set(true);
  }

  onDestroy(() => { unsubStatus(); unsubSummary(); unsubNodes(); });
</script>

<div class="swarm-panel">
  <div class="panel-header">
    <span class="panel-title">Swarm</span>
    <button class="expand-btn" onclick={openCanvas} title="Open full canvas">&#x2922;</button>
  </div>

  <!-- Mini-map: colored dots for each node -->
  <div class="mini-map">
    {#each nodes as ns}
      <span
        class="node-dot"
        style="background: {stateColors[ns.state] || '#666'}"
        title="{ns.node_id}: {ns.state}"
      ></span>
    {/each}
    {#if nodes.length === 0}
      <span class="no-nodes">No workflow loaded</span>
    {/if}
  </div>

  <!-- Status -->
  <div class="status-line">{summary || 'idle'}</div>

  <!-- Controls -->
  <SwarmControls {adapter} {cwd} />
</div>

<style>
  .swarm-panel {
    padding: 10px;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-family: var(--font-ui, sans-serif);
    background: var(--bg-secondary);
  }
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .panel-title {
    font-weight: 700;
    font-size: 13px;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .expand-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    padding: 2px 6px;
    cursor: pointer;
    font-size: 14px;
  }
  .expand-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .mini-map {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 6px;
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
    font-size: 11px;
    color: var(--text-muted);
  }
  .status-line {
    font-size: 12px;
    color: var(--text-secondary);
  }
</style>
