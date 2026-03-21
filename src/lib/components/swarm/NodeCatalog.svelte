<script lang="ts">
  import { currentWorkflow, workflowDirty } from '$lib/stores/workflow';
  import { get } from 'svelte/store';

  let { onadd }: {
    onadd?: (type: 'agent' | 'resource' | 'logic') => void;
  } = $props();

  function addNode(type: 'agent' | 'resource' | 'logic') {
    onadd?.(type);
  }
</script>

<div class="node-catalog">
  <button class="catalog-btn agent" onclick={() => addNode('agent')} title="Add Agent Node">
    <span class="btn-icon">&#9679;</span> Agent
  </button>
  <button class="catalog-btn resource" onclick={() => addNode('resource')} title="Add Resource Node">
    <span class="btn-icon">&#128196;</span> Resource
  </button>
  <button class="catalog-btn logic" onclick={() => addNode('logic')} title="Add Logic Node">
    <span class="btn-icon">&#9670;</span> Logic
  </button>
</div>

<style>
  .node-catalog {
    display: flex;
    gap: 4px;
  }
  .catalog-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: var(--border-width) solid var(--border);
    padding: 4px 10px;
    font-size: 12px;
    font-family: var(--font-ui);
    cursor: pointer;
  }
  .catalog-btn:hover {
    background: var(--bg-hover);
  }
  .btn-icon {
    font-size: 10px;
  }
  .catalog-btn.agent .btn-icon { color: var(--accent); }
  .catalog-btn.resource .btn-icon { color: var(--success); }
  .catalog-btn.logic .btn-icon { color: var(--warning); }
</style>
