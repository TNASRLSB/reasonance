<script lang="ts">
  import { toasts, dismissToast, pauseToastTimer, resumeToastTimer } from '$lib/stores/toast';

  const borderColors: Record<string, string> = {
    error: 'var(--danger)',
    warning: 'var(--warning)',
    success: 'var(--success)',
    info: 'var(--accent)',
    update: 'var(--accent)',
  };

  const labels: Record<string, string> = {
    error: 'ERROR',
    warning: 'WARNING',
    success: 'SUCCESS',
    info: 'INFO',
    update: 'UPDATE',
  };

  const icons: Record<string, string> = {
    error: '✕',
    warning: '⚠',
    success: '✓',
    info: 'ℹ',
    update: '↑',
  };
</script>

<div class="toast-container" aria-live="polite" aria-atomic="false">
  {#each $toasts as toast (toast.id)}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="toast"
      style="border-inline-start-color: {borderColors[toast.type] ?? borderColors.info}"
      role="alert"
      tabindex="0"
      onfocus={() => pauseToastTimer(toast.id)}
      onblur={() => resumeToastTimer(toast.id)}
      onmouseenter={() => pauseToastTimer(toast.id)}
      onmouseleave={() => resumeToastTimer(toast.id)}
    >
      <div class="toast-row">
        <div class="toast-icon" style="color: {borderColors[toast.type] ?? borderColors.info}">
          {icons[toast.type] ?? icons.info}
          <span class="toast-label">{labels[toast.type] ?? labels.info}</span>
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
      {#if toast.progress !== undefined}
        <div class="toast-progress">
          <div class="toast-progress-bar" style="width: {toast.progress}%"></div>
        </div>
      {/if}
      {#if toast.actions?.length}
        <div class="toast-actions">
          {#each toast.actions as action}
            <button class="toast-action-btn" onclick={action.onClick}>
              {action.label}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 1.25rem;
    inset-inline-end: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    z-index: 9999;
    pointer-events: none;
  }

  .toast {
    display: flex;
    flex-direction: column;
    padding: var(--space-3) var(--space-4);
    min-width: 280px;
    max-width: 420px;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    border-inline-start: 4px solid transparent;
    border-radius: var(--radius);
    pointer-events: all;
    font-family: var(--font-ui);
    animation: slide-in 0.2s ease-out;
  }

  .toast-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
  }

  .toast-icon {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
    font-size: var(--font-size-sm);
    font-weight: bold;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .toast-label {
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .toast-content {
    flex: 1;
    min-width: 0;
  }

  .toast-title {
    font-size: var(--font-size-base);
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.3;
  }

  .toast-body {
    font-size: var(--font-size-small);
    color: var(--text-secondary);
    margin-top: var(--space-1);
    line-height: 1.4;
    word-break: break-word;
  }

  .toast-dismiss {
    flex-shrink: 0;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    font-size: var(--font-size-base);
    line-height: 1;
    padding: var(--space-1);
    margin-top: -1px;
    opacity: 0.6;
    transition: opacity 0.15s;
    min-width: 32px;
    min-height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .toast-dismiss:hover {
    opacity: 1;
    color: var(--text-primary);
  }

  .toast-progress {
    width: 100%;
    height: 3px;
    background: var(--bg-secondary, #333);
    margin-top: var(--space-2);
  }
  .toast-progress-bar {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease;
  }
  .toast-actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .toast-action-btn {
    background: transparent;
    border: 1px solid var(--accent);
    color: var(--accent);
    padding: var(--space-1) var(--space-3);
    cursor: pointer;
    font-family: inherit;
    font-size: var(--font-size-sm);
  }
  .toast-action-btn:hover {
    background: var(--accent);
    color: var(--bg-primary, #000);
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
