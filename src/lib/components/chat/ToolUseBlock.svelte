<script lang="ts">
  import type { AgentEvent } from '$lib/types/agent-event';
  import TextBlock from './TextBlock.svelte';

  let { event, result }: {
    event: AgentEvent;
    result?: AgentEvent;
  } = $props();

  let expanded = $state(false);

  let toolName = $derived(event.metadata.tool_name ?? 'unknown');

  // Format JSON input for display
  let inputDisplay = $derived.by(() => {
    if (event.content.type === 'json') {
      try {
        return JSON.stringify(event.content.value, null, 2);
      } catch {
        return String(event.content.value);
      }
    }
    if (event.content.type === 'text') return event.content.value;
    return '';
  });

  let resultText = $derived.by(() => {
    if (!result) return null;
    if (result.content.type === 'text') return result.content.value;
    return JSON.stringify(result.content, null, 2);
  });
</script>

<div class="tool-use-block">
  <button class="tool-header" onclick={() => expanded = !expanded}>
    <span class="tool-icon">⚙</span>
    <span class="tool-name">{toolName}</span>
    {#if result}
      <span class="tool-status done">✓</span>
    {:else}
      <span class="tool-status pending">…</span>
    {/if}
    <span class="tool-toggle">{expanded ? '▾' : '▸'}</span>
  </button>

  {#if expanded}
    <div class="tool-content">
      {#if inputDisplay}
        <div class="tool-section">
          <div class="tool-section-label">INPUT</div>
          <pre class="tool-json">{inputDisplay}</pre>
        </div>
      {/if}
      {#if resultText}
        <div class="tool-section">
          <div class="tool-section-label">RESULT</div>
          <div class="tool-result">
            <TextBlock text={resultText} />
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .tool-use-block {
    border: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
  }

  .tool-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-1) var(--space-3);
    background: var(--bg-tertiary);
    border: none;
    border-bottom: var(--border-width) solid var(--border);
    cursor: pointer;
    color: var(--text-secondary);
  }

  .tool-header:hover {
    color: var(--text-primary);
  }

  .tool-icon {
    font-size: var(--font-size-small);
  }

  .tool-name {
    font-family: var(--font-mono);
    font-size: var(--font-size-small);
    font-weight: 700;
    flex: 1;
    text-align: start;
  }

  .tool-status {
    font-size: var(--font-size-tiny);
  }

  .tool-status.done {
    color: var(--success-text);
  }

  .tool-status.pending {
    color: var(--text-muted);
  }

  .tool-toggle {
    font-size: var(--font-size-small);
  }

  .tool-content {
    padding: var(--space-2) var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--stack-normal);
  }

  .tool-section-label {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    margin-bottom: var(--stack-tight);
  }

  .tool-json {
    font-family: var(--font-mono);
    font-size: var(--font-size-code);
    color: var(--text-body);
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    padding: var(--space-2);
    margin: 0;
    overflow-x: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .tool-result {
    padding: var(--space-1) 0;
  }
</style>
