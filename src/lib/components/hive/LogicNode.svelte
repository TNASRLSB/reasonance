<script lang="ts">
  import type { AgentState } from '$lib/adapter/index';
  import { getStateColor, stateIcons } from '$lib/utils/state-color';
  import { isDark } from '$lib/stores/theme';

  let {
    id = '',
    label = 'Logic',
    kind = 'condition',
    rule = '',
    nodeState = 'idle' as AgentState,
    selected = false,
    onselect,
    onchange,
  }: {
    id?: string;
    label?: string;
    kind?: string;
    rule?: string;
    nodeState?: AgentState;
    selected?: boolean;
    onselect?: (id: string) => void;
    onchange?: (field: string, value: string) => void;
  } = $props();

  const logicKinds = ['condition', 'switch', 'loop'] as const;
  const kindIcons: Record<string, string> = {
    condition: '\u2753',
    switch: '\u{1F500}',
    loop: '\u{1F501}',
  };

  let editingLabel = $state(false);
  let labelInput = $state('');
  $effect(() => { if (!editingLabel) labelInput = label; });

  let borderColor = $state('');
  $effect(() => {
    const _dark = $isDark;
    borderColor = getStateColor(nodeState);
  });

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

  function handleKindChange(newKind: string) {
    onchange?.('kind', newKind);
  }

  function handleRuleInput(e: Event) {
    const value = (e.target as HTMLTextAreaElement).value;
    onchange?.('rule', value);
  }

  function stopPropagation(e: Event) {
    e.stopPropagation();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="logic-node"
  class:selected
  style="border-color: {borderColor}"
  onclick={() => onselect?.(id)}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onselect?.(id); }}
  role="button"
  tabindex="0"
  aria-pressed={selected}
  aria-label="{label}{rule ? ': ' + rule : ''}, state: {nodeState}"
>
  <div class="node-header">
    <span class="node-icon" aria-hidden="true">&#9670;</span>
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
    <span class="node-state" style="color: {borderColor}" aria-hidden="true">{stateIcons[nodeState] || '\u23F8'} {nodeState}</span>
  </div>

  <div class="kind-row">
    {#each logicKinds as k}
      <button
        class="kind-btn"
        class:active={kind === k}
        onclick={(e) => { e.stopPropagation(); handleKindChange(k); }}
        title={k}
      >{kindIcons[k]} {k}</button>
    {/each}
  </div>

  <textarea
    class="rule-area"
    value={rule}
    oninput={handleRuleInput}
    onclick={stopPropagation}
    onkeydown={stopPropagation}
    placeholder={kind === 'condition' ? 'output.contains("error")' : kind === 'switch' ? 'match output { ... }' : 'while condition { ... }'}
    rows="2"
    spellcheck="false"
  ></textarea>
</div>

<style>
  .logic-node {
    background: var(--bg-secondary);
    border: 2px solid var(--state-idle);
    padding: var(--space-2) var(--space-3);
    min-width: 220px;
    max-width: 300px;
    font-family: var(--font-ui, sans-serif);
    cursor: pointer;
    user-select: none;
    text-align: start;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .logic-node.selected {
    box-shadow: 0 0 0 2px var(--accent);
  }

  .node-header {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
  }
  .node-icon {
    font-size: var(--font-size-sm);
    color: var(--warning);
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

  .kind-row {
    display: flex;
    gap: 2px;
  }
  .kind-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    padding: 2px 6px;
    font-size: var(--font-size-xs);
    cursor: pointer;
    color: var(--text-secondary);
    font-family: var(--font-ui, sans-serif);
    opacity: 0.6;
  }
  .kind-btn.active {
    opacity: 1;
    border-color: var(--warning);
    background: var(--bg-primary);
    color: var(--text-primary);
  }
  .kind-btn:hover {
    opacity: 0.8;
  }

  .rule-area {
    width: 100%;
    background: var(--bg-primary);
    color: var(--text-body);
    border: 1px solid var(--border);
    padding: var(--space-1);
    font-size: var(--font-size-sm);
    font-family: var(--font-mono, monospace);
    resize: vertical;
    min-height: 36px;
    outline: none;
    cursor: text;
  }
  .rule-area:focus {
    border-color: var(--warning);
  }
  .rule-area::placeholder {
    color: var(--text-muted);
    font-style: italic;
  }
</style>
