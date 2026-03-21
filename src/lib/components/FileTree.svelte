<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import type { Adapter, FileEntry } from '$lib/adapter';
  import { getFileIcon } from '$lib/utils/icons';
  import { addOpenFile, projectRoot } from '$lib/stores/files';

  let { adapter }: { adapter: Adapter } = $props();

  let entries = $state<FileEntry[]>([]);
  let expandedDirs = $state(new Set<string>());
  let childrenCache = $state(new Map<string, FileEntry[]>());
  let clickTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    const root = get(projectRoot) || '.';
    entries = await adapter.listDir(root);
  });

  async function toggleDir(entry: FileEntry) {
    if (expandedDirs.has(entry.path)) {
      expandedDirs.delete(entry.path);
      expandedDirs = new Set(expandedDirs);
    } else {
      if (!childrenCache.has(entry.path)) {
        const children = await adapter.listDir(entry.path);
        childrenCache.set(entry.path, children);
        childrenCache = new Map(childrenCache);
      }
      expandedDirs.add(entry.path);
      expandedDirs = new Set(expandedDirs);
    }
  }

  function handleClick(entry: FileEntry) {
    if (entry.isDir) {
      toggleDir(entry);
      return;
    }
    if (clickTimer) {
      clearTimeout(clickTimer);
      clickTimer = null;
      handleDoubleClick(entry);
      return;
    }
    clickTimer = setTimeout(async () => {
      clickTimer = null;
      const content = await adapter.readFile(entry.path);
      addOpenFile({
        path: entry.path,
        name: entry.name,
        content,
        isDirty: false,
        isDeleted: false,
      });
    }, 250);
  }

  function handleDoubleClick(entry: FileEntry) {
    adapter.openExternal(entry.path);
  }
</script>

<div class="file-tree">
  <div class="tree-header">FILES</div>

  {#snippet renderEntries(items: FileEntry[], depth: number)}
    {#each items as entry (entry.path)}
      <button
        class="tree-item"
        style="padding-left: {12 + depth * 16}px"
        onclick={() => handleClick(entry)}
      >
        <span class="icon">{getFileIcon(entry.name, entry.isDir)}</span>
        <span class="name">{entry.name}</span>
      </button>
      {#if entry.isDir && expandedDirs.has(entry.path)}
        {@render renderEntries(childrenCache.get(entry.path) ?? [], depth + 1)}
      {/if}
    {/each}
  {/snippet}

  {@render renderEntries(entries, 0)}
</div>

<style>
  .file-tree {
    height: 100%;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .tree-header {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-secondary);
    padding: 10px 12px 6px;
    flex-shrink: 0;
  }

  .tree-item {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 6px;
    width: 100%;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    padding-top: 3px;
    padding-bottom: 3px;
    padding-right: 8px;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
  }

  .tree-item:hover {
    background: var(--bg-hover, rgba(255, 255, 255, 0.06));
  }

  .icon {
    flex-shrink: 0;
    font-size: 14px;
    line-height: 1;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
