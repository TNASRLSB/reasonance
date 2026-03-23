<script lang="ts">
  import type { AgentState } from '$lib/adapter/index';
  import { getStateColor } from '$lib/utils/state-color';
  import { isDark } from '$lib/stores/theme';

  let { id = '', label = 'Logic', kind = 'condition', rule = '', state = 'idle' as AgentState, selected = false, onselect }: {
    id?: string;
    label?: string;
    kind?: string;
    rule?: string;
    state?: AgentState;
    selected?: boolean;
    onselect?: (id: string) => void;
  } = $props();

  let borderColor = $state('');
  $effect(() => {
    const _dark = $isDark;
    borderColor = getStateColor(state);
  });
</script>

<div
  class="logic-node"
  class:selected
  style="border-color: {borderColor}"
  onclick={() => onselect?.(id)}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onselect?.(id); } }}
  role="button"
  tabindex="0"
>
  <div class="node-header">
    <span class="node-icon">&#9670;</span>
    <span class="node-label">{label}</span>
  </div>
  {#if rule}
    <div class="node-rule">{rule}</div>
  {/if}
</div>

<style>
  .logic-node {
    background: var(--bg-secondary, #1a1a1a);
    border: 2px solid var(--state-idle);
    padding: 10px 14px;
    min-width: 120px;
    font-family: var(--font-ui, sans-serif);
    cursor: pointer;
    user-select: none;
    transform: rotate(0deg);
  }
  .logic-node.selected {
    box-shadow: 0 0 0 2px var(--accent, #1d4ed8);
  }
  .node-header {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .node-icon {
    font-size: 12px;
    color: var(--warning, #ca8a04);
  }
  .node-label {
    font-weight: 700;
    font-size: 13px;
    color: var(--text-primary, #f0f0f0);
  }
  .node-rule {
    font-size: 11px;
    color: var(--text-muted, #666);
    margin-top: 4px;
    font-family: var(--font-mono, monospace);
  }
</style>
