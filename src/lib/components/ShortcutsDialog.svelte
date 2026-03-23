<script lang="ts">
  import { tr } from '$lib/i18n/index';
  import { shortcuts } from '$lib/data/shortcuts';
  import { trapFocus } from '$lib/utils/a11y';

  interface Props {
    visible: boolean;
    onClose: () => void;
  }

  const { visible, onClose }: Props = $props();
  let dialogEl = $state<HTMLElement | null>(null);

  $effect(() => {
    if (visible && dialogEl) {
      const destroy = trapFocus(dialogEl);
      return destroy;
    }
  });

  function handleOverlayClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('shortcuts-overlay')) {
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }

  // Group shortcuts by context
  const grouped = $derived(
    shortcuts.reduce<Record<string, typeof shortcuts>>(
      (acc, s) => {
        if (!acc[s.context]) acc[s.context] = [];
        acc[s.context].push(s);
        return acc;
      },
      {}
    )
  );
</script>

<svelte:window onkeydown={handleKeydown} />

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="shortcuts-overlay"
    onclick={handleOverlayClick}
    onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}
  >
    <div class="shortcuts-dialog" role="dialog" aria-modal="true" aria-label={$tr('shortcuts.title')} bind:this={dialogEl}>
      <div class="dialog-header">
        <span class="dialog-title">{$tr('shortcuts.title')}</span>
        <button class="close-btn" onclick={onClose} aria-label={$tr('settings.close')}>✕</button>
      </div>

      <div class="dialog-body">
        {#each Object.entries(grouped) as [ctx, items]}
          <div class="group">
            <div class="group-label">{$tr(ctx)}</div>
            {#each items as shortcut}
              <div class="shortcut-row">
                <span class="shortcut-desc">{$tr(shortcut.descriptionKey)}</span>
                <span class="shortcut-keys">
                  {#each shortcut.keys as key, i}
                    <kbd>{key}</kbd>{#if i < shortcut.keys.length - 1}<span class="plus">+</span>{/if}
                  {/each}
                </span>
              </div>
            {/each}
          </div>
        {/each}
      </div>

      <div class="dialog-footer">
        <button class="close-footer-btn" onclick={onClose}>{$tr('settings.close')}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .shortcuts-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    z-index: 2000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .shortcuts-dialog {
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    width: 480px;
    max-width: 95vw;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    font-family: var(--font-ui);
    overflow: hidden;
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px 10px;
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .dialog-title {
    font-size: 13px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-primary);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 14px;
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 0;
    font-family: var(--font-ui);
    min-width: 32px;
    min-height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .close-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .dialog-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .group-label {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--accent);
    margin-bottom: 4px;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 5px 0;
    border-bottom: 1px solid var(--border);
    gap: 12px;
  }

  .shortcut-row:last-child {
    border-bottom: none;
  }

  .shortcut-desc {
    font-size: 12px;
    color: var(--text-primary);
    flex: 1;
  }

  .shortcut-keys {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  kbd {
    display: inline-block;
    background: var(--bg-tertiary);
    border: var(--border-width) solid var(--border);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    padding: 2px 6px;
    min-width: 24px;
    text-align: center;
    white-space: nowrap;
  }

  .plus {
    font-size: 10px;
    color: var(--text-muted);
    padding: 0 1px;
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    padding: 10px 16px;
    border-top: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .close-footer-btn {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: var(--border-width) solid var(--border);
    border-radius: 0;
    padding: 4px 16px;
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
  }

  .close-footer-btn:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
    border-color: var(--text-primary);
  }
</style>
