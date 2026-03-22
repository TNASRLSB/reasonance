<script lang="ts">
  import TextBlock from './TextBlock.svelte';

  let { text }: { text: string } = $props();

  let expanded = $state(false);

  let preview = $derived(text.length > 120 ? text.slice(0, 120) + '…' : text);
</script>

<div class="thinking-block">
  <button class="thinking-header" onclick={() => expanded = !expanded}>
    <span class="thinking-label">THINKING</span>
    <span class="thinking-toggle">{expanded ? '▾' : '▸'}</span>
  </button>
  {#if expanded}
    <div class="thinking-content">
      <TextBlock {text} />
    </div>
  {:else}
    <div class="thinking-preview">{preview}</div>
  {/if}
</div>

<style>
  .thinking-block {
    border: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
  }

  .thinking-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 6px 12px;
    background: var(--bg-tertiary);
    border: none;
    border-bottom: var(--border-width) solid var(--border);
    cursor: pointer;
    color: var(--text-secondary);
  }

  .thinking-header:hover {
    color: var(--text-primary);
  }

  .thinking-label {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .thinking-toggle {
    font-size: var(--font-size-small);
  }

  .thinking-preview {
    padding: 8px 12px;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    color: var(--text-muted);
    font-style: italic;
  }

  .thinking-content {
    padding: 12px;
  }
</style>
