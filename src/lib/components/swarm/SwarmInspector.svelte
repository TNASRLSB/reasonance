<script lang="ts">
  import { selectedNodeId } from '$lib/stores/ui';
  import { currentWorkflow } from '$lib/stores/workflow';
  import type { WorkflowNode } from '$lib/adapter/index';
  import { onDestroy } from 'svelte';
  import { get } from 'svelte/store';

  let node = $state<WorkflowNode | null>(null);
  let showJson = $state(false);
  let selId = $state<string | null>(null);

  const unsubSel = selectedNodeId.subscribe((id) => {
    selId = id;
    const wf = get(currentWorkflow);
    if (id && wf) {
      node = wf.nodes.find((n) => n.id === id) ?? null;
    } else {
      node = null;
    }
  });

  const unsubWf = currentWorkflow.subscribe((wf) => {
    if (selId && wf) {
      node = wf.nodes.find((n) => n.id === selId) ?? null;
    } else {
      node = null;
    }
  });

  onDestroy(() => { unsubSel(); unsubWf(); });
</script>

{#if node}
  <div class="inspector">
    <div class="inspector-header">
      <h3>{node.label}</h3>
      <span class="node-type">{node.type}</span>
    </div>

    <div class="inspector-section">
      <div class="section-label">ID</div>
      <div class="section-value mono">{node.id}</div>
    </div>

    <div class="inspector-section">
      <div class="section-label">Position</div>
      <div class="section-value">x: {node.position.x.toFixed(0)}, y: {node.position.y.toFixed(0)}</div>
    </div>

    <div class="inspector-section">
      <button class="toggle-json" onclick={() => showJson = !showJson}>
        {showJson ? 'Hide' : 'Show'} JSON
      </button>
      {#if showJson}
        <pre class="json-raw">{JSON.stringify(node.config, null, 2)}</pre>
      {/if}
    </div>
  </div>
{:else}
  <div class="inspector empty">
    <p>Select a node to inspect</p>
  </div>
{/if}

<style>
  .inspector {
    padding: 12px;
    font-family: var(--font-ui, sans-serif);
    height: 100%;
    overflow-y: auto;
  }
  .inspector.empty {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .inspector-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    padding-bottom: 8px;
    border-bottom: var(--border-width) solid var(--border);
  }
  .inspector-header h3 {
    margin: 0;
    font-size: 14px;
    color: var(--text-primary);
  }
  .node-type {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--text-secondary);
    padding: 2px 6px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
  }
  .inspector-section {
    margin-bottom: 10px;
  }
  .section-label {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    margin-bottom: 2px;
  }
  .section-value {
    font-size: 13px;
    color: var(--text-primary);
  }
  .section-value.mono {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
  }
  .toggle-json {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    padding: 3px 8px;
    font-size: 11px;
    cursor: pointer;
    font-family: var(--font-ui);
  }
  .toggle-json:hover {
    background: var(--bg-hover);
  }
  .json-raw {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    padding: 8px;
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    color: var(--text-body);
    margin-top: 6px;
    overflow-x: auto;
    white-space: pre-wrap;
  }
</style>
