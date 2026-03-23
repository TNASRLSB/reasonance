<script lang="ts">
  import type { ErrorSeverity } from '$lib/types/agent-event';

  let { message, severity = 'recoverable' as ErrorSeverity, code = '' }: {
    message: string;
    severity?: ErrorSeverity;
    code?: string;
  } = $props();

  const severityColors: Record<ErrorSeverity, string> = {
    recoverable: 'var(--warning)',
    degraded: 'var(--warning)',
    fatal: 'var(--danger)',
  };

  let borderColor = $derived(severityColors[severity] ?? 'var(--danger)');
</script>

<div class="error-block" style="border-inline-start-color: {borderColor};" role="alert">
  <div class="error-header">
    <span class="error-severity">{severity.toUpperCase()}</span>
    {#if code}
      <span class="error-code">{code}</span>
    {/if}
  </div>
  <div class="error-message">{message}</div>
</div>

<style>
  .error-block {
    border: var(--border-width) solid var(--border);
    border-inline-start: 4px solid var(--danger);
    background: var(--bg-secondary);
    padding: 8px 12px;
  }

  .error-header {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 4px;
  }

  .error-severity {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--danger);
  }

  .error-code {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
  }

  .error-message {
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
    color: var(--text-body);
    line-height: 1.5;
  }
</style>
