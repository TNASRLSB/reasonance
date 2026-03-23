<script lang="ts">
  let { id = '', label = 'Resource', kind = 'folder', path = '', selected = false, onselect }: {
    id?: string;
    label?: string;
    kind?: string;
    path?: string;
    selected?: boolean;
    onselect?: (id: string) => void;
  } = $props();

  const kindIcons: Record<string, string> = {
    folder: '\u{1F4C1}',
    file: '\u{1F4C4}',
    api: '\u{1F310}',
    database: '\u{1F5C3}',
  };
</script>

<button
  class="resource-node"
  class:selected
  onclick={() => onselect?.(id)}
  aria-pressed={selected}
  aria-label="{label}{path ? ': ' + path : ''}"
>
  <div class="node-header">
    <span class="node-icon" aria-hidden="true">{kindIcons[kind] || '\u{1F4C4}'}</span>
    <span class="node-label">{label}</span>
  </div>
  {#if path}
    <div class="node-path">{path}</div>
  {/if}
</button>

<style>
  .resource-node {
    background: var(--bg-secondary, #1a1a1a);
    border: 2px solid var(--success, #16a34a);
    padding: var(--space-2) var(--space-3);
    min-width: 120px;
    font-family: var(--font-ui, sans-serif);
    cursor: pointer;
    user-select: none;
    text-align: start;
  }
  .resource-node.selected {
    box-shadow: 0 0 0 2px var(--accent, #1d4ed8);
  }
  .node-header {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
  }
  .node-icon { font-size: var(--font-size-base); }
  .node-label {
    font-weight: 700;
    font-size: var(--font-size-sm);
    color: var(--text-primary, #f0f0f0);
  }
  .node-path {
    font-size: var(--font-size-sm);
    color: var(--text-muted, #666);
    margin-top: var(--stack-tight);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 160px;
  }
</style>
