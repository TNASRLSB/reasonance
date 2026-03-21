<script lang="ts">
  import { toasts, dismissToast } from '$lib/stores/toast';

  const borderColors: Record<string, string> = {
    error: '#ef4444',
    warning: '#f59e0b',
    success: '#22c55e',
    info: '#3b82f6',
  };

  const icons: Record<string, string> = {
    error: '✕',
    warning: '⚠',
    success: '✓',
    info: 'ℹ',
  };
</script>

<div class="toast-container" aria-live="polite" aria-atomic="false">
  {#each $toasts as toast (toast.id)}
    <div
      class="toast"
      style="border-left-color: {borderColors[toast.type] ?? borderColors.info}"
      role="alert"
    >
      <div class="toast-icon" style="color: {borderColors[toast.type] ?? borderColors.info}">
        {icons[toast.type] ?? icons.info}
      </div>
      <div class="toast-content">
        <div class="toast-title">{toast.title}</div>
        {#if toast.body}
          <div class="toast-body">{toast.body}</div>
        {/if}
      </div>
      <button
        class="toast-dismiss"
        onclick={() => dismissToast(toast.id)}
        aria-label="Dismiss notification"
      >×</button>
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 1.25rem;
    right: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    z-index: 9999;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 0.625rem;
    padding: 0.75rem 1rem;
    min-width: 280px;
    max-width: 420px;
    background: var(--bg-secondary, #1e1e2e);
    border: 1px solid var(--border, #313244);
    border-left: 4px solid transparent;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    pointer-events: all;
    animation: slide-in 0.2s ease-out;
  }

  .toast-icon {
    font-size: 0.875rem;
    font-weight: bold;
    flex-shrink: 0;
    margin-top: 1px;
    width: 1rem;
    text-align: center;
  }

  .toast-content {
    flex: 1;
    min-width: 0;
  }

  .toast-title {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--fg-primary, #cdd6f4);
    line-height: 1.3;
  }

  .toast-body {
    font-size: 0.75rem;
    color: var(--fg-secondary, #a6adc8);
    margin-top: 0.25rem;
    line-height: 1.4;
    word-break: break-word;
  }

  .toast-dismiss {
    flex-shrink: 0;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--fg-secondary, #a6adc8);
    font-size: 1rem;
    line-height: 1;
    padding: 0;
    margin-top: -1px;
    opacity: 0.6;
    transition: opacity 0.15s;
  }

  .toast-dismiss:hover {
    opacity: 1;
  }

  @keyframes slide-in {
    from {
      transform: translateX(calc(100% + 1.25rem));
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }
</style>
