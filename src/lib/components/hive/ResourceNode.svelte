<script lang="ts">
  let {
    id = '',
    label = 'Resource',
    kind = 'folder',
    path = '',
    access = 'read',
    selected = false,
    onselect,
    onchange,
  }: {
    id?: string;
    label?: string;
    kind?: string;
    path?: string;
    access?: string;
    selected?: boolean;
    onselect?: (id: string) => void;
    onchange?: (field: string, value: string) => void;
  } = $props();

  const kinds = ['file', 'folder', 'api', 'database'] as const;
  const kindIcons: Record<string, string> = {
    folder: '\u{1F4C1}',
    file: '\u{1F4C4}',
    api: '\u{1F310}',
    database: '\u{1F5C3}',
  };

  let editingLabel = $state(false);
  let labelInput = $state(label);

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

  function handlePathInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    onchange?.('path', value);
  }

  function cycleAccess() {
    const modes = ['read', 'write', 'read_write'];
    const next = modes[(modes.indexOf(access) + 1) % modes.length];
    onchange?.('access', next);
  }

  function stopPropagation(e: Event) {
    e.stopPropagation();
  }

  const accessLabels: Record<string, string> = {
    read: 'R',
    write: 'W',
    read_write: 'RW',
  };
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="resource-node"
  class:selected
  onclick={() => onselect?.(id)}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onselect?.(id); }}
  role="button"
  tabindex="0"
  aria-pressed={selected}
  aria-label="{label}{path ? ': ' + path : ''}"
>
  <div class="node-header">
    <span class="node-icon" aria-hidden="true">{kindIcons[kind] || '\u{1F4C4}'}</span>
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
    <button
      class="access-badge"
      onclick={(e) => { e.stopPropagation(); cycleAccess(); }}
      title="Access: {access} (click to cycle)"
    >{accessLabels[access] || 'R'}</button>
  </div>

  <div class="kind-row">
    {#each kinds as k}
      <button
        class="kind-btn"
        class:active={kind === k}
        onclick={(e) => { e.stopPropagation(); handleKindChange(k); }}
        title={k}
      >{kindIcons[k]}</button>
    {/each}
  </div>

  <input
    class="path-input"
    type="text"
    value={path}
    oninput={handlePathInput}
    onclick={stopPropagation}
    onkeydown={stopPropagation}
    placeholder="path/to/resource..."
    spellcheck="false"
  />
</div>

<style>
  .resource-node {
    background: var(--bg-secondary);
    border: 2px solid var(--success);
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
  .resource-node.selected {
    box-shadow: 0 0 0 2px var(--accent);
  }

  .node-header {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
  }
  .node-icon { font-size: var(--font-size-base); flex-shrink: 0; }
  .node-label {
    font-weight: 700;
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    cursor: text;
    flex: 1;
    min-width: 0;
    overflow: hidden;
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
  .access-badge {
    font-size: var(--font-size-xs);
    font-weight: 700;
    color: var(--text-secondary);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    padding: 1px 4px;
    cursor: pointer;
    font-family: var(--font-mono, monospace);
    flex-shrink: 0;
  }
  .access-badge:hover {
    border-color: var(--accent);
  }

  .kind-row {
    display: flex;
    gap: 2px;
  }
  .kind-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    padding: 2px 6px;
    font-size: var(--font-size-sm);
    cursor: pointer;
    opacity: 0.5;
  }
  .kind-btn.active {
    opacity: 1;
    border-color: var(--success);
    background: var(--bg-primary);
  }
  .kind-btn:hover {
    opacity: 0.8;
  }

  .path-input {
    width: 100%;
    background: var(--bg-primary);
    color: var(--text-body);
    border: 1px solid var(--border);
    padding: 2px var(--space-1);
    font-size: var(--font-size-sm);
    font-family: var(--font-mono, monospace);
    outline: none;
  }
  .path-input:focus {
    border-color: var(--accent);
  }
  .path-input::placeholder {
    color: var(--text-muted);
    font-style: italic;
  }
</style>
