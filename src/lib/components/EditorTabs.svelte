<script lang="ts">
  import { get } from 'svelte/store';
  import { openFiles, activeFilePath, closeFile } from '$lib/stores/files';
  import { tr } from '$lib/i18n/index';
  import type { Snippet } from 'svelte';

  let { actions }: { actions?: Snippet } = $props();

  function switchTab(path: string) {
    activeFilePath.set(path);
  }

  function handleClose(e: MouseEvent, path: string) {
    e.stopPropagation();
    const file = get(openFiles).find((f) => f.path === path);
    if (file?.isDirty) {
      const fileName = file.name;
      const ok = confirm(`Save changes to "${fileName}"?\n\nYour unsaved changes will be lost if you close without saving.\n\nClick Cancel to go back, or OK to close without saving.`);
      if (!ok) return;
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
      const tablist = (e.currentTarget as HTMLElement).parentElement;
      const tabs = tablist?.querySelectorAll<HTMLElement>('[role="tab"]');
      tabs?.[nextIndex]?.focus();
    }
  }
</script>

<div class="editor-tabs" role="tablist">
  <div class="tabs-scroll">
    {#each $openFiles as file, i (file.path)}
      <div
        class="tab"
        class:active={$activeFilePath === file.path}
        class:deleted={file.isDeleted}
        class:dirty={file.isDirty}
        role="tab"
        tabindex={$activeFilePath === file.path ? 0 : -1}
        aria-selected={$activeFilePath === file.path}
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
    border-bottom: 2px solid var(--border);
    height: 38px;
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
    gap: 6px;
    padding: 0 10px;
    flex-shrink: 0;
    border-inline-start: 1px solid var(--border);
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    min-width: 100px;
    max-width: 200px;
    cursor: pointer;
    border-inline-end: 2px solid var(--border);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted);
    user-select: none;
    transition: background 0.1s, color 0.1s;
    white-space: nowrap;
    flex-shrink: 0;
    border-bottom: 2px solid transparent;
  }

  .tab:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .tab.active {
    background: var(--bg-primary);
    color: var(--text-primary);
    border-bottom: 2px solid var(--accent);
    font-weight: 600;
  }

  .tab.deleted .tab-name {
    color: var(--text-secondary);
    font-style: italic;
    opacity: 0.7;
  }

  .tab-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .tab-save {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
    line-height: 1;
    padding: 3px 4px;
    min-width: 22px;
    min-height: 22px;
    border-radius: var(--radius);
    flex-shrink: 0;
    transition: color 0.1s;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0.7;
  }

  .tab-save:hover {
    color: var(--accent);
    opacity: 1;
  }

  .tab-close {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 5px 6px;
    min-width: 24px;
    min-height: 24px;
    border-radius: var(--radius);
    flex-shrink: 0;
    transition: color 0.1s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .tab-close:hover {
    color: var(--danger);
  }
</style>
