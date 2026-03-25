<script lang="ts">
  import type { AgentState } from '$lib/adapter/index';
  import { getStateColor, stateIcons } from '$lib/utils/state-color';
  import { isDark } from '$lib/stores/theme';
  import { llmConfigs } from '$lib/stores/config';

  let {
    id = '',
    label = 'Agent',
    llm = '',
    prompt = '',
    nodeState = 'idle' as AgentState,
    selected = false,
    memoryCount = 0,
    onselect,
    onchange,
  }: {
    id?: string;
    label?: string;
    llm?: string;
    prompt?: string;
    nodeState?: AgentState;
    selected?: boolean;
    memoryCount?: number;
    onselect?: (id: string) => void;
    onchange?: (field: string, value: string) => void;
  } = $props();

  let editingLabel = $state(false);
  let labelInput = $state(label);

  let borderColor = $state('');
  $effect(() => {
    const _dark = $isDark;
    borderColor = getStateColor(nodeState);
  });
  let stateIcon = $derived(stateIcons[nodeState] || '\u23F8');
  let pulsing = $derived(nodeState === 'running');

  function startEditLabel(e: MouseEvent) {
    e.stopPropagation();
    editingLabel = true;
    labelInput = label;
  }

  function commitLabel() {
    editingLabel = false;
    if (labelInput.trim() && labelInput !== label) {
      onchange?.('label', labelInput.trim());
    }
  }

  function onLabelKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') commitLabel();
    if (e.key === 'Escape') { editingLabel = false; labelInput = label; }
  }

  function handleLlmChange(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    onchange?.('llm', value);
  }

  function handlePromptInput(e: Event) {
    const value = (e.target as HTMLTextAreaElement).value;
    onchange?.('prompt', value);
  }

  function stopPropagation(e: Event) {
    e.stopPropagation();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="agent-node"
  class:selected
  class:pulsing
  style="border-color: {borderColor}"
  onclick={() => onselect?.(id)}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onselect?.(id); }}
  role="button"
  tabindex="0"
  aria-pressed={selected}
  aria-label="{label} — {llm || 'unset'}, state: {nodeState}"
>
  <div class="node-header">
    <span class="node-icon" aria-hidden="true">&#9679;</span>
    {#if editingLabel}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="label-edit"
        bind:value={labelInput}
        onblur={commitLabel}
        onkeydown={onLabelKeydown}
        onclick={stopPropagation}
        autofocus
      />
    {:else}
      <span class="node-label" ondblclick={startEditLabel} title="Double-click to rename">{label}</span>
    {/if}
    <span class="node-state" style="color: {borderColor}" aria-hidden="true">{stateIcon} {nodeState}</span>
  </div>

  <div class="node-config">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <select
      class="llm-select"
      value={llm}
      onchange={handleLlmChange}
      onclick={stopPropagation}
      onkeydown={stopPropagation}
      title="Select LLM provider"
    >
      <option value="">-- LLM --</option>
      {#each $llmConfigs as cfg}
        <option value={cfg.name.toLowerCase()}>{cfg.name}</option>
      {/each}
    </select>
    {#if memoryCount > 0}
      <span class="memory-badge" title="{memoryCount} memory entries">&#128278; {memoryCount}</span>
    {/if}
  </div>

  <textarea
    class="prompt-area"
    value={prompt}
    oninput={handlePromptInput}
    onclick={stopPropagation}
    onkeydown={stopPropagation}
    placeholder="Describe the agent's task..."
    rows="3"
    spellcheck="false"
  ></textarea>
</div>

<style>
  .agent-node {
    background: var(--bg-secondary);
    border: 2px solid var(--state-idle);
    padding: var(--space-2) var(--space-3);
    min-width: 240px;
    max-width: 320px;
    font-family: var(--font-ui, sans-serif);
    cursor: pointer;
    user-select: none;
    text-align: start;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .agent-node.selected {
    box-shadow: 0 0 0 2px var(--accent);
  }
  .agent-node.pulsing {
    animation: pulse 1.5s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }
  @media (prefers-reduced-motion: reduce) {
    .agent-node.pulsing {
      animation: none;
      border-color: var(--accent);
      box-shadow: 0 0 0 2px var(--accent);
    }
  }

  .node-header {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
  }
  .node-icon {
    font-size: var(--font-size-sm);
    color: var(--accent);
    flex-shrink: 0;
  }
  .node-label {
    font-weight: 700;
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    cursor: text;
    flex: 1;
    min-width: 0;
    overflow: auto;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .label-edit {
    font-weight: 700;
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    background: var(--bg-primary);
    border: 1px solid var(--accent);
    padding: 0 var(--space-1);
    flex: 1;
    min-width: 0;
    font-family: var(--font-ui, sans-serif);
    outline: none;
  }
  .node-state {
    font-weight: 500;
    text-transform: uppercase;
    font-size: var(--font-size-xs);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .node-config {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }
  .llm-select {
    flex: 1;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    padding: 2px var(--space-1);
    font-size: var(--font-size-sm);
    font-family: var(--font-ui, sans-serif);
    cursor: pointer;
    outline: none;
  }
  .llm-select:focus {
    border-color: var(--accent);
  }
  .memory-badge {
    font-size: var(--font-size-xs);
    color: var(--text-muted);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .prompt-area {
    width: 100%;
    background: var(--bg-primary);
    color: var(--text-body);
    border: 1px solid var(--border);
    padding: var(--space-1);
    font-size: var(--font-size-sm);
    font-family: var(--font-mono, monospace);
    resize: vertical;
    min-height: 48px;
    outline: none;
    cursor: text;
  }
  .prompt-area:focus {
    border-color: var(--accent);
  }
  .prompt-area::placeholder {
    color: var(--text-muted);
    font-style: italic;
  }
</style>
