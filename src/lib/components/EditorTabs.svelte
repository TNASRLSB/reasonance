<script lang="ts">
  import { openFiles, activeFilePath, closeFile } from '$lib/stores/files';

  function switchTab(path: string) {
    activeFilePath.set(path);
  }

  function handleClose(e: MouseEvent, path: string) {
    e.stopPropagation();
    closeFile(path);
  }

  function handleKeyDown(e: KeyboardEvent, path: string) {
    if (e.key === 'Enter' || e.key === ' ') {
      switchTab(path);
    }
  }
</script>

<div class="editor-tabs" role="tablist">
  {#each $openFiles as file (file.path)}
    <div
      class="tab"
      class:active={$activeFilePath === file.path}
      class:deleted={file.isDeleted}
      class:dirty={file.isDirty}
      role="tab"
      tabindex="0"
      aria-selected={$activeFilePath === file.path}
      onclick={() => switchTab(file.path)}
      onkeydown={(e) => handleKeyDown(e, file.path)}
    >
      <span class="tab-name">
        {#if file.isDeleted}
          <em>{file.name} (eliminato)</em>
        {:else}
          {file.name}{file.isDirty ? ' ●' : ''}
        {/if}
      </span>
      <button
        class="tab-close"
        aria-label="Close {file.name}"
        onclick={(e) => handleClose(e, file.path)}
      >×</button>
    </div>
  {/each}
</div>

<style>
  .editor-tabs {
    display: flex;
    flex-direction: row;
    overflow-x: auto;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    min-height: 32px;
    flex-shrink: 0;
    scrollbar-width: thin;
  }

  .editor-tabs::-webkit-scrollbar {
    height: 3px;
  }

  .editor-tabs::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 2px;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px 0 12px;
    min-width: 100px;
    max-width: 200px;
    cursor: pointer;
    border-right: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-secondary);
    user-select: none;
    transition: background 0.1s, color 0.1s;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .tab:hover {
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .tab.active {
    background: var(--bg-primary);
    color: var(--text-primary);
    border-top: 2px solid var(--accent);
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

  .tab-close {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 0 2px;
    border-radius: 3px;
    flex-shrink: 0;
    transition: color 0.1s, background 0.1s;
  }

  .tab-close:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.1);
  }
</style>
