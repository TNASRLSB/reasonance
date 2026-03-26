<script lang="ts">
  import { showToast } from '$lib/stores/toast';
  import type { Snippet } from 'svelte';

  let { children, name = 'panel' }: { children: Snippet; name?: string } = $props();

  let errorCount = $state(0);
  let lastErrorTime = $state(0);

  const MAX_RETRIES = 3;
  const WINDOW_MS = 30_000;

  function handleError(error: unknown, reset: () => void) {
    const now = Date.now();
    if (now - lastErrorTime > WINDOW_MS) {
      errorCount = 0;
    }
    errorCount++;
    lastErrorTime = now;

    const msg = error instanceof Error ? error.message : String(error);
    console.error(`[ErrorBoundary:${name}]`, error);
    showToast('error', `${name} encountered an error`, msg);
  }
</script>

<svelte:boundary
  onerror={(error, reset) => {
    handleError(error, reset);
  }}
>
  {@render children()}

  {#snippet failed(error, reset)}
    <div class="error-boundary-fallback" role="alert" aria-live="assertive">
      <div class="error-boundary-content">
        <h3>Something went wrong in {name}</h3>
        <p class="error-message">{error instanceof Error ? error.message : String(error)}</p>
        {#if errorCount >= MAX_RETRIES}
          <p class="error-persistent">This error keeps recurring. Try restarting the app.</p>
        {:else}
          <button class="error-retry" onclick={reset}>Try again</button>
        {/if}
      </div>
    </div>
  {/snippet}
</svelte:boundary>

<style>
  .error-boundary-fallback {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 100px;
    padding: 1rem;
    background: var(--surface-1, #1e1e1e);
    color: var(--text-primary, #ccc);
    border: 1px solid var(--danger, #dc3545);
    border-radius: 4px;
  }

  .error-boundary-content {
    text-align: center;
    max-width: 400px;
  }

  .error-boundary-content h3 {
    margin: 0 0 0.5rem;
    font-size: 1rem;
    color: var(--danger, #dc3545);
  }

  .error-message {
    font-size: 0.875rem;
    opacity: 0.7;
    margin: 0.5rem 0;
    word-break: break-word;
  }

  .error-persistent {
    color: var(--warning, #f0ad4e);
    font-size: 0.875rem;
  }

  .error-retry {
    margin-top: 0.5rem;
    padding: 0.5rem 1rem;
    background: var(--accent, #4a9eff);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.875rem;
  }

  .error-retry:hover {
    opacity: 0.9;
  }

  .error-retry:focus-visible {
    outline: 2px solid var(--focus-ring, #4a9eff);
    outline-offset: 2px;
  }
</style>
