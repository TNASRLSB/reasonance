<script lang="ts">
  import type { SessionStatus } from '$lib/types/agent-event';

  let { provider, model, status, streaming = false, tokenCount = 0, currentSpeed = 0, elapsed = 0 }: {
    provider: string;
    model: string;
    status: SessionStatus;
    streaming?: boolean;
    tokenCount?: number;
    currentSpeed?: number;
    elapsed?: number;
  } = $props();

  let statusLabel = $derived.by(() => {
    if (typeof status === 'string') return status;
    if ('error' in status) return `error (${status.error.severity})`;
    return 'unknown';
  });

  let statusClass = $derived.by(() => {
    if (streaming) return 'streaming';
    if (typeof status === 'string') return status;
    return 'error';
  });

  let elapsedDisplay = $derived.by(() => {
    if (elapsed < 1000) return `${elapsed}ms`;
    return `${(elapsed / 1000).toFixed(1)}s`;
  });
</script>

<div class="chat-header">
  <div class="header-left">
    <span class="provider">{provider}</span>
    <span class="separator">/</span>
    <span class="model">{model}</span>
  </div>

  <div class="header-right">
    {#if streaming}
      <span class="metric">{currentSpeed.toFixed(1)} tok/s</span>
      <span class="metric">{elapsedDisplay}</span>
    {/if}
    {#if tokenCount > 0}
      <span class="metric">{tokenCount.toLocaleString()} tok</span>
    {/if}
    <span class="status {statusClass}">{streaming ? 'streaming' : statusLabel}</span>
  </div>
</div>

<style>
  .chat-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 16px;
    border-bottom: var(--border-width) solid var(--border);
    background: var(--bg-tertiary);
    min-height: 32px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .provider {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-primary);
  }

  .separator {
    color: var(--text-muted);
    font-size: var(--font-size-tiny);
  }

  .model {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-secondary);
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .metric {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
  }

  .status {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 1px 6px;
    border: var(--border-width) solid var(--border);
  }

  .status.active,
  .status.streaming {
    color: var(--success);
    border-color: var(--success);
  }

  .status.terminated {
    color: var(--text-muted);
    border-color: var(--border);
  }

  .status.error {
    color: var(--danger);
    border-color: var(--danger);
  }

  .status.idle,
  .status.resumable {
    color: var(--text-secondary);
    border-color: var(--border);
  }
</style>
