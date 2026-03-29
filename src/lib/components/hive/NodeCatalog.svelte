<script lang="ts">
  import { onMount } from 'svelte';
  import { tr } from '$lib/i18n/index';
  import type { Adapter, NodeDescriptor } from '$lib/adapter/index';

  let { adapter, onadd }: {
    adapter?: Adapter;
    onadd?: (type: 'agent' | 'resource' | 'logic') => void;
  } = $props();

  let nodeTypes = $state<NodeDescriptor[]>([]);

  // Icon characters by category (Unicode)
  const CATEGORY_ICONS: Record<string, string> = {
    agent: '\u25CF',
    resource: '\uD83D\uDCC4',
    logic: '\u25C6',
  };

  // a11y label key by type_id
  const A11Y_LABELS: Record<string, string> = {
    agent: 'a11y.addAgentNode',
    resource: 'a11y.addResourceNode',
    logic: 'a11y.addLogicNode',
  };

  const FALLBACK_TYPES: NodeDescriptor[] = [
    { type_id: 'agent', display_name: 'Agent', description: 'LLM-powered agent node', category: 'agent', config_schema: {} },
    { type_id: 'resource', display_name: 'Resource', description: 'File, folder, API, or database resource node', category: 'resource', config_schema: {} },
    { type_id: 'logic', display_name: 'Logic', description: 'Conditional branching and routing node', category: 'logic', config_schema: {} },
  ];

  onMount(async () => {
    if (!adapter) {
      nodeTypes = FALLBACK_TYPES;
      return;
    }
    try {
      nodeTypes = await adapter.getNodeTypes();
    } catch {
      nodeTypes = FALLBACK_TYPES;
    }
  });

  function addNode(typeId: string) {
    onadd?.(typeId as 'agent' | 'resource' | 'logic');
  }
</script>

<div class="node-catalog">
  {#each nodeTypes as node (node.type_id)}
    {@const a11yKey = A11Y_LABELS[node.type_id]}
    {@const icon = CATEGORY_ICONS[node.category] ?? '\u25A0'}
    <button
      class="catalog-btn {node.category}"
      onclick={() => addNode(node.type_id)}
      title={a11yKey ? $tr(a11yKey) : node.display_name}
      aria-label={a11yKey ? $tr(a11yKey) : node.display_name}
    >
      <span class="btn-icon" aria-hidden="true">{icon}</span> {node.display_name}
    </button>
  {/each}
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
