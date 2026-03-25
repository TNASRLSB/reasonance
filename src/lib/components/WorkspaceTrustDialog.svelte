<script lang="ts">
  import { tr } from '$lib/i18n/index';
  import type { FolderInfo } from '$lib/stores/workspace-trust';
  import type { TrustLevel } from '$lib/stores/workspace-trust';

  let { folderInfo, onDecision }: {
    folderInfo: FolderInfo;
    onDecision: (level: TrustLevel) => void;
  } = $props();

  let dialogEl = $state<HTMLElement | null>(null);

  // Focus trap: keep focus within dialog
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Tab' && dialogEl) {
      const focusable = dialogEl.querySelectorAll<HTMLElement>('button');
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
    // Escape does NOT dismiss (deliberate — force explicit choice)
  }

  // Auto-focus first button on mount
  $effect(() => {
    if (dialogEl) {
      const first = dialogEl.querySelector<HTMLElement>('button');
      first?.focus();
    }
  });
</script>

<div class="trust-overlay" role="presentation">
  <div
    class="trust-dialog"
    bind:this={dialogEl}
    role="alertdialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="trust-title"
    aria-describedby="trust-desc"
    onkeydown={handleKeydown}
  >
    <h2 id="trust-title">{$tr('trust.title')}</h2>

    <div class="folder-info">
      <span class="folder-name">{folderInfo.name}</span>
      <span class="folder-path">{folderInfo.path}</span>
      <div class="folder-meta">
        {#if folderInfo.has_git}
          <span class="meta-tag">{$tr('trust.gitPresent')}</span>
        {/if}
        <span class="meta-tag">{$tr('trust.fileCount', { count: String(folderInfo.file_count) })}</span>
      </div>
    </div>

    <p id="trust-desc" class="trust-explanation">{$tr('trust.explanation')}</p>

    <div class="trust-actions">
      <button class="trust-btn trust" onclick={() => onDecision('trusted')}>
        {$tr('trust.btnTrust')}
      </button>
      <button class="trust-btn readonly" onclick={() => onDecision('read_only')}>
        {$tr('trust.btnReadOnly')}
      </button>
      <button class="trust-btn block" onclick={() => onDecision('blocked')}>
        {$tr('trust.btnBlock')}
      </button>
    </div>
  </div>
</div>

<style>
  .trust-overlay {
    position: fixed;
    inset: 0;
    z-index: var(--layer-top);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--overlay-bg);
  }

  .trust-dialog {
    background: var(--bg-surface);
    border: 2px solid var(--border);
    padding: var(--space-5);
    max-width: 480px;
    width: 90%;
    font-family: var(--font-ui);
  }

  h2 {
    font-size: var(--font-size-base);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-primary);
    margin: 0 0 var(--space-3);
  }

  .folder-info {
    display: flex;
    flex-direction: column;
    gap: var(--stack-tight);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    margin-bottom: var(--space-3);
  }

  .folder-name {
    font-weight: 700;
    font-size: var(--font-size-small);
    color: var(--text-primary);
  }

  .folder-path {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    word-break: break-all;
  }

  .folder-meta {
    display: flex;
    gap: var(--space-2);
  }

  .meta-tag {
    font-size: var(--font-size-tiny);
    font-weight: 600;
    padding: var(--stack-tight) var(--space-1);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .trust-explanation {
    font-size: var(--font-size-small);
    color: var(--text-body);
    line-height: 1.5;
    margin: 0 0 var(--space-4);
  }

  .trust-actions {
    display: flex;
    gap: var(--space-2);
  }

  .trust-btn {
    flex: 1;
    padding: var(--space-2) var(--space-3);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: 2px solid var(--border);
    cursor: pointer;
    transition: opacity var(--transition-fast);
  }

  .trust-btn:hover { opacity: 0.85; }

  .trust-btn.trust {
    background: var(--accent-btn);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }

  .trust-btn.readonly {
    background: transparent;
    color: var(--text-body);
  }

  .trust-btn.block {
    background: transparent;
    color: var(--text-muted);
  }
</style>
