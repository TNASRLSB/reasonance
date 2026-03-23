<script lang="ts">
  import type { AgentState } from '$lib/adapter/index';
  import { getStateColor, stateIcons } from '$lib/utils/state-color';
  import { isDark } from '$lib/stores/theme';

  let { id = '', label = 'Agent', llm = '', state = 'idle' as AgentState, selected = false, onselect }: {
    id?: string;
    label?: string;
    llm?: string;
    state?: AgentState;
    selected?: boolean;
    onselect?: (id: string) => void;
  } = $props();

  let borderColor = $state('');
  $effect(() => {
    const _dark = $isDark; // track theme changes
    borderColor = getStateColor(state);
  });
  let stateIcon = $derived(stateIcons[state] || '⏸');
  let pulsing = $derived(state === 'running');
</script>

<button
  class="agent-node"
  class:selected
  class:pulsing
  style="border-color: {borderColor}"
  onclick={() => onselect?.(id)}
  aria-pressed={selected}
  aria-label="{label} — {llm || 'unset'}, state: {state}"
>
  <div class="node-header">
    <span class="node-icon" aria-hidden="true">&#9679;</span>
    <span class="node-label">{label}</span>
  </div>
  <div class="node-meta">
    <span class="node-llm">{llm || 'unset'}</span>
    <span class="node-state" style="color: {borderColor}" aria-hidden="true"><span class="state-icon" aria-hidden="true">{stateIcon}</span> {state}</span>
  </div>
</button>

<style>
  .agent-node {
    background: var(--bg-secondary, #1a1a1a);
    border: 2px solid var(--state-idle);
    padding: var(--space-2) var(--space-3);
    min-width: 140px;
    font-family: var(--font-ui, sans-serif);
    cursor: pointer;
    user-select: none;
    text-align: start;
  }
  .agent-node.selected {
    box-shadow: 0 0 0 2px var(--accent, #1d4ed8);
  }
  .agent-node.pulsing {
    animation: pulse 1.5s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }
  .node-header {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
    margin-bottom: var(--stack-tight);
  }
  .node-icon {
    font-size: var(--font-size-sm);
    color: var(--accent, #1d4ed8);
  }
  .node-label {
    font-weight: 700;
    font-size: var(--font-size-sm);
    color: var(--text-primary, #f0f0f0);
  }
  .node-meta {
    display: flex;
    justify-content: space-between;
    font-size: var(--font-size-sm);
    color: var(--text-secondary, #a3a3a3);
  }
  .node-state {
    font-weight: 500;
    text-transform: uppercase;
    font-size: var(--font-size-sm);
  }
</style>
