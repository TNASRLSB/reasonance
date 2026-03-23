<script lang="ts">
  import type { AgentState } from '$lib/adapter/index';

  let { id = '', label = 'Agent', llm = '', state = 'idle' as AgentState, selected = false, onselect }: {
    id?: string;
    label?: string;
    llm?: string;
    state?: AgentState;
    selected?: boolean;
    onselect?: (id: string) => void;
  } = $props();

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

  let borderColor = $derived(stateColors[state] || '#666666');
  let pulsing = $derived(state === 'running');
</script>

<div
  class="agent-node"
  class:selected
  class:pulsing
  style="border-color: {borderColor}"
  onclick={() => onselect?.(id)}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onselect?.(id); } }}
  role="button"
  tabindex="0"
>
  <div class="node-header">
    <span class="node-icon">&#9679;</span>
    <span class="node-label">{label}</span>
  </div>
  <div class="node-meta">
    <span class="node-llm">{llm || 'unset'}</span>
    <span class="node-state" style="color: {borderColor}">{state}</span>
  </div>
</div>

<style>
  .agent-node {
    background: var(--bg-secondary, #1a1a1a);
    border: 2px solid #666;
    padding: 10px 14px;
    min-width: 140px;
    font-family: var(--font-ui, sans-serif);
    cursor: pointer;
    user-select: none;
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
    gap: 6px;
    margin-bottom: 4px;
  }
  .node-icon {
    font-size: 10px;
    color: var(--accent, #1d4ed8);
  }
  .node-label {
    font-weight: 700;
    font-size: 13px;
    color: var(--text-primary, #f0f0f0);
  }
  .node-meta {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-secondary, #a3a3a3);
  }
  .node-state {
    font-weight: 500;
    text-transform: uppercase;
    font-size: 10px;
  }
</style>
