<script lang="ts">
  let { denials, locked = false }: {
    denials: unknown;
    locked?: boolean;
  } = $props();

  let toolNames = $derived.by(() => {
    if (Array.isArray(denials)) {
      return denials.map((d: { tool_name?: string; name?: string }) =>
        d.tool_name ?? d.name ?? 'unknown'
      );
    }
    return ['unknown'];
  });
</script>

<div class="permission-denial" role="status">
  <div class="header">TOOL USE DENIED</div>
  <div class="tool-list">
    {#each toolNames as tool}
      <span class="tool-name">{tool}</span>
    {/each}
  </div>
  <p class="hint">
    {#if locked}
      Permission level is LOCKED. Change to ASK or YOLO in Settings to allow tools.
    {:else}
      This provider doesn't support selective tool approval. Switch to YOLO to allow all tools.
    {/if}
  </p>
</div>

<style>
  .permission-denial {
    border: 2px solid var(--text-muted);
    background: var(--bg-secondary);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    opacity: 0.8;
  }

  .header {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .tool-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .tool-name {
    font-family: var(--font-mono);
    font-size: var(--font-size-small);
    padding: var(--stack-tight) var(--space-1);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .hint {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin: 0;
  }
</style>
