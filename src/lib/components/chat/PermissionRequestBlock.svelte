<script lang="ts">
  let { denials, onApprove, onApproveRemember }: {
    denials: unknown;
    onApprove: (tools: string[]) => void;
    onApproveRemember: (tools: string[]) => void;
  } = $props();

  let resolved = $state(false);

  let toolNames = $derived.by(() => {
    if (Array.isArray(denials)) {
      return denials.map((d: { tool_name?: string; name?: string }) =>
        d.tool_name ?? d.name ?? 'unknown'
      );
    }
    return ['unknown'];
  });

  function handleApprove() {
    resolved = true;
    onApprove(toolNames);
  }

  function handleApproveRemember() {
    resolved = true;
    onApproveRemember(toolNames);
  }
</script>

<div class="permission-request" class:resolved role="alert">
  <div class="header">PERMISSION REQUIRED</div>
  <div class="tool-list">
    {#each toolNames as tool}
      <span class="tool-name">{tool}</span>
    {/each}
  </div>
  {#if !resolved}
    <div class="actions">
      <button class="btn approve" onclick={handleApprove}>Approve</button>
      <button class="btn remember" onclick={handleApproveRemember}>Approve & Remember</button>
      <button class="btn deny" onclick={() => resolved = true}>Deny</button>
    </div>
  {:else}
    <span class="resolved-label">Resolved</span>
  {/if}
</div>

<style>
  .permission-request {
    border: 2px solid var(--warning);
    background: var(--bg-secondary);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .permission-request.resolved {
    opacity: 0.6;
    border-color: var(--border);
  }

  .header {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--warning);
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
    color: var(--text-primary);
  }

  .actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }

  .btn {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: var(--space-1) var(--space-3);
    border: var(--border-width) solid var(--border);
    cursor: pointer;
    min-height: 2rem;
    transition: background var(--transition-fast);
  }

  .btn.approve {
    background: var(--accent);
    color: var(--text-on-accent);
    border-color: var(--accent);
  }

  .btn.approve:hover {
    opacity: 0.85;
  }

  .btn.remember {
    background: transparent;
    color: var(--accent-text);
    border-color: var(--accent);
  }

  .btn.remember:hover {
    background: var(--bg-hover);
  }

  .btn.deny {
    background: transparent;
    color: var(--text-muted);
  }

  .btn.deny:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .resolved-label {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    color: var(--text-muted);
  }
</style>
