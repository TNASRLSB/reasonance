<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { removeProject, updateProjectRoot } from '$lib/stores/projects';

  let {
    projectId,
    projectLabel,
    missingPath,
    open: isOpen,
    onClose,
  }: {
    projectId: string;
    projectLabel: string;
    missingPath: string;
    open: boolean;
    onClose: () => void;
  } = $props();

  let dialogEl = $state<HTMLElement | null>(null);

  $effect(() => {
    if (isOpen && dialogEl) {
      const first = dialogEl.querySelector<HTMLElement>('button');
      first?.focus();
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
      return;
    }
    if (e.key === 'Tab' && dialogEl) {
      const focusable = Array.from(
        dialogEl.querySelectorAll<HTMLElement>('button')
      ).filter((el) => !el.disabled);
      if (focusable.length === 0) return;
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
  }

  async function handleLocate() {
    const selected = await open({ directory: true });
    if (selected) {
      updateProjectRoot(projectId, selected as string);
      onClose();
    }
  }

  function handleRemove() {
    removeProject(projectId);
    onClose();
  }
</script>

{#if isOpen}
  <div class="disconnected-overlay" role="presentation">
    <div
      class="disconnected-dialog"
      bind:this={dialogEl}
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="disconnected-title"
      aria-describedby="disconnected-desc"
      tabindex="-1"
      onkeydown={handleKeydown}
    >
      <h2 id="disconnected-title">Folder Not Found</h2>

      <div class="disconnected-info">
        <span class="project-name">{projectLabel}</span>
        <span class="project-path">{missingPath}</span>
      </div>

      <p id="disconnected-desc" class="disconnected-message">
        The folder for this project could not be found. It may have been moved,
        renamed, or deleted. You can locate the new folder or remove the project.
      </p>

      <div class="disconnected-actions">
        <button class="disconnected-btn primary" onclick={handleLocate}>
          Locate Folder...
        </button>
        <button class="disconnected-btn danger" onclick={handleRemove}>
          Remove Project
        </button>
        <button class="disconnected-btn" onclick={onClose}>
          Cancel
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .disconnected-overlay {
    position: fixed;
    inset: 0;
    z-index: var(--layer-modal);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--overlay-bg);
  }

  .disconnected-dialog {
    background: var(--bg-surface);
    border: 2px solid var(--border);
    padding: var(--space-5);
    max-width: 440px;
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

  .disconnected-info {
    display: flex;
    flex-direction: column;
    gap: var(--stack-tight);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    margin-bottom: var(--space-3);
  }

  .project-name {
    font-weight: 700;
    font-size: var(--font-size-small);
    color: var(--text-primary);
  }

  .project-path {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    word-break: break-all;
  }

  .disconnected-message {
    font-size: var(--font-size-small);
    color: var(--text-body);
    line-height: 1.5;
    margin: 0 0 var(--space-4);
  }

  .disconnected-actions {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .disconnected-btn {
    flex: 1;
    min-width: 80px;
    padding: var(--space-2) var(--space-3);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: 2px solid var(--border);
    cursor: pointer;
    background: transparent;
    color: var(--text-body);
    transition: opacity var(--transition-fast);
  }

  .disconnected-btn:hover {
    opacity: 0.85;
  }

  .disconnected-btn.primary {
    background: var(--accent-btn);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }

  .disconnected-btn.danger {
    color: var(--text-muted);
  }
</style>
