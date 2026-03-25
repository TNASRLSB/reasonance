<script lang="ts">
  import { currentWorkflow, workflowDirty } from '$lib/stores/workflow';
  import { get } from 'svelte/store';
  import { tr } from '$lib/i18n/index';

  let { onadd }: {
    onadd?: (type: 'agent' | 'resource' | 'logic') => void;
  } = $props();

  function addNode(type: 'agent' | 'resource' | 'logic') {
    onadd?.(type);
  }
</script>

<div class="node-catalog">
  <button class="catalog-btn agent" onclick={() => addNode('agent')} title={$tr('a11y.addAgentNode')} aria-label={$tr('a11y.addAgentNode')}>
    <span class="btn-icon" aria-hidden="true">&#9679;</span> Agent
  </button>
  <button class="catalog-btn resource" onclick={() => addNode('resource')} title={$tr('a11y.addResourceNode')} aria-label={$tr('a11y.addResourceNode')}>
    <span class="btn-icon" aria-hidden="true">&#128196;</span> Resource
  </button>
  <button class="catalog-btn logic" onclick={() => addNode('logic')} title={$tr('a11y.addLogicNode')} aria-label={$tr('a11y.addLogicNode')}>
    <span class="btn-icon" aria-hidden="true">&#9670;</span> Logic
  </button>
</div>

<style>
  .node-catalog {
    display: flex;
    gap: var(--stack-tight);
  }
  .catalog-btn {
    display: flex;
    align-items: center;
    gap: var(--stack-tight);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: var(--border-width) solid var(--border);
    padding: var(--space-1) var(--space-2);
    font-size: var(--font-size-sm);
    font-family: var(--font-ui);
    cursor: pointer;
  }
  .catalog-btn:hover {
    background: var(--bg-hover);
  }
  .btn-icon {
    font-size: var(--font-size-sm);
  }
  .catalog-btn.agent .btn-icon { color: var(--accent-text); }
  .catalog-btn.resource .btn-icon { color: var(--success-text); }
  .catalog-btn.logic .btn-icon { color: var(--warning-text); }
</style>
