<script lang="ts">
  import { get } from 'svelte/store';
  import { openFiles, activeFilePath, closeFile } from '$lib/stores/files';
  import { setActiveFile } from '$lib/stores/projects';
  import { sanitizeId } from '$lib/utils/a11y';
  import { tr } from '$lib/i18n/index';
  import type { Snippet } from 'svelte';

  let { actions }: { actions?: Snippet } = $props();

  function switchTab(path: string) {
    setActiveFile(path);
  }

  async function handleClose(e: MouseEvent, path: string) {
    e.stopPropagation();
    const file = get(openFiles).find((f) => f.path === path);
    if (file?.isDirty) {
      const { ask } = await import('@tauri-apps/plugin-dialog');
      const save = await ask(
        `Save changes to "${file.name}" before closing?`,
        { title: 'Unsaved Changes', kind: 'warning', okLabel: 'Save', cancelLabel: 'Discard' }
      );
      if (save) {
        // Save before closing
        document.dispatchEvent(new CustomEvent('reasonance:save'));
        // Wait a tick for save to process
        await new Promise((r) => setTimeout(r, 50));
      }
      // If user chose Discard (save=false), close without saving
    }
    closeFile(path);
  }

  function handleKeyDown(e: KeyboardEvent, path: string, index: number) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      switchTab(path);
    } else if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
      e.preventDefault();
      const files = get(openFiles);
      const nextIndex = e.key === 'ArrowRight'
        ? (index + 1) % files.length
        : (index - 1 + files.length) % files.length;
      const tablist = (e.currentTarget as HTMLElement).closest('[role="tablist"]') as HTMLElement;
      const tabs = tablist?.querySelectorAll<HTMLElement>('[role="tab"]');
      tabs?.[nextIndex]?.focus();
    }
  }
</script>

<div class="editor-tabs">
  <div class="tabs-scroll" role="tablist">
    {#each $openFiles as file, i (file.path)}
      <div class="tab-wrapper" class:active={$activeFilePath === file.path} class:deleted={file.isDeleted} class:dirty={file.isDirty}>
        <div
          class="tab-label"
          role="tab"
          id="tab-{sanitizeId(file.path)}"
          tabindex={$activeFilePath === file.path ? 0 : -1}
          aria-selected={$activeFilePath === file.path}
          aria-controls="tabpanel-editor"
          onclick={() => switchTab(file.path)}
          onkeydown={(e) => handleKeyDown(e, file.path, i)}
        >
          <span class="tab-name">
            {#if file.isDeleted}
              <em>{file.name} {$tr('editor.deleted')}</em>
            {:else}
              {file.name}{file.isDirty ? ' ●' : ''}
            {/if}
          </span>
        </div>
        {#if file.isDirty && !file.isDeleted}
          <button
            class="tab-save"
            aria-label="Save {file.name}"
            onclick={(e) => { e.stopPropagation(); switchTab(file.path); document.dispatchEvent(new CustomEvent('reasonance:save')); }}
            title="Save"
          >&#9998;</button>
        {/if}
        <button
          class="tab-close"
          aria-label="Close {file.name}"
          onclick={(e) => handleClose(e, file.path)}
        >×</button>
      </div>
    {/each}
  </div>
  {#if actions}
    <div class="tab-actions">
      {@render actions()}
    </div>
  {/if}
</div>

<style>
  .editor-tabs {
    display: flex;
    flex-direction: row;
    align-items: stretch;
    background: var(--bg-primary);
    border-bottom: var(--border-width) solid var(--border);
    min-height: var(--interactive-min, 38px);
    flex-shrink: 0;
    font-family: var(--font-ui);
  }

  .tabs-scroll {
    display: flex;
    flex-direction: row;
    overflow-x: auto;
    flex: 1;
    min-width: 0;
    scrollbar-width: thin;
  }

  .tabs-scroll::-webkit-scrollbar {
    height: 3px;
  }

  .tabs-scroll::-webkit-scrollbar-thumb {
    background: var(--border);
  }

  .tab-actions {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
    padding: 0 var(--space-2);
    flex-shrink: 0;
    border-inline-start: 1px solid var(--border);
  }

  .tab-wrapper {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
    padding: 0 var(--space-3);
    min-width: 100px;
    max-width: 200px;
    cursor: pointer;
    border-inline-end: 2px solid var(--border);
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-muted);
    user-select: none;
    transition: background var(--transition-fast), color var(--transition-fast);
    white-space: nowrap;
    flex-shrink: 0;
    border-bottom: 2px solid transparent;
  }

  .tab-wrapper:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .tab-wrapper.active {
    background: var(--bg-primary);
    color: var(--text-primary);
    border-bottom: 2px solid var(--accent);
    font-weight: 600;
  }

  .tab-wrapper.deleted .tab-name {
    color: var(--text-secondary);
    font-style: italic;
    opacity: 0.7;
  }

  .tab-label {
    cursor: pointer;
    flex: 1;
    display: flex;
    align-items: center;
    min-width: 0;
  }

  .tab-name {
    flex: 1;
    overflow: auto;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .tab-save {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: var(--font-size-sm);
    line-height: 1;
    padding: var(--space-1) var(--space-1);
    min-width: var(--interactive-min, 24px);
    min-height: var(--interactive-min, 24px);
    border-radius: var(--radius);
    flex-shrink: 0;
    transition: color var(--transition-fast);
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0.7;
  }

  .tab-save:hover {
    color: var(--accent-text);
    opacity: 1;
  }

  .tab-close {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: var(--font-size-base);
    line-height: 1;
    padding: var(--space-1) var(--space-1);
    min-width: var(--interactive-min, 24px);
    min-height: var(--interactive-min, 24px);
    border-radius: var(--radius);
    flex-shrink: 0;
    transition: color var(--transition-fast);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .tab-close:hover {
    color: var(--danger-text);
  }
</style>
