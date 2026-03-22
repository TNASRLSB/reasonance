<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { AgentEvent } from '$lib/types/agent-event';

  let { events, role, forkIndex, onRetry, onFork, children }: {
    events: AgentEvent[];
    role: 'user' | 'agent';
    forkIndex?: number;
    onRetry?: (text: string) => void;
    onFork?: (eventIndex: number) => void;
    children: Snippet;
  } = $props();

  let showActions = $state(false);
  let copied = $state(false);

  // Extract plain text from all events in this group for copy
  function getPlainText(): string {
    return events
      .map((e) => {
        if (e.content.type === 'text') return e.content.value;
        if (e.content.type === 'code') return e.content.source;
        return '';
      })
      .filter(Boolean)
      .join('\n\n');
  }

  function handleCopy() {
    navigator.clipboard.writeText(getPlainText()).then(() => {
      copied = true;
      setTimeout(() => { copied = false; }, 2000);
    });
  }
</script>

<div
  class="actionable-message"
  onmouseenter={() => showActions = true}
  onmouseleave={() => { showActions = false; copied = false; }}
  onfocusin={() => showActions = true}
  onfocusout={(e) => { if (!e.currentTarget.contains(e.relatedTarget as Node)) { showActions = false; copied = false; } }}
  role="group"
>
  {@render children()}

  {#if showActions && role === 'agent'}
    <div class="action-bar">
      <button class="action-btn" onclick={handleCopy} aria-label="Copy message">
        {copied ? 'COPIED' : 'COPY'}
      </button>
      {#if onRetry}
        <button class="action-btn" onclick={() => onRetry(getPlainText())} aria-label="Retry">
          RETRY
        </button>
      {/if}
      {#if onFork}
        <button class="action-btn" onclick={() => onFork(forkIndex ?? 0)} aria-label="Fork from here">
          FORK
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .actionable-message {
    position: relative;
  }

  .action-bar {
    display: flex;
    gap: 4px;
    padding-top: 4px;
  }

  .action-btn {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    background: transparent;
    border: var(--border-width) solid var(--border);
    padding: 1px 6px;
    cursor: pointer;
  }

  .action-btn:hover {
    color: var(--text-primary);
    border-color: var(--accent);
  }
</style>
