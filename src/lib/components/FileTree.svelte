<script lang="ts">
  import { onMount } from 'svelte';
  import type { Adapter, FileEntry } from '$lib/adapter';
  import { getFileIcon } from '$lib/utils/icons';
  import { addOpenFile, projectRoot } from '$lib/stores/files';

  let { adapter }: { adapter: Adapter } = $props();

  let entries = $state<FileEntry[]>([]);
  let expandedDirs = $state(new Set<string>());
  let childrenCache = $state(new Map<string, FileEntry[]>());
  let clickTimer: ReturnType<typeof setTimeout> | null = null;

  let currentRoot = $derived($projectRoot || '.');

  // Reload entries when project root changes
  $effect(() => {
    const root = $projectRoot;
    if (root) {
      adapter.listDir(root).then((e) => { entries = e; });
    }
  });

  onMount(async () => {
    const root = $projectRoot || '.';
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
      try {
        const content = await adapter.readFile(entry.path);
        addOpenFile({
          path: entry.path,
          name: entry.name,
          content,
          isDirty: false,
          isDeleted: false,
        });
      } catch (err) {
        console.error('Failed to read file:', err);
      }
    }, 250);
  }

  function handleDoubleClick(entry: FileEntry) {
    adapter.openExternal(entry.path);
  }

  function handleTreeKeydown(e: KeyboardEvent) {
    const tree = e.currentTarget as HTMLElement;
    const items = Array.from(tree.querySelectorAll<HTMLElement>('[role="treeitem"]')).filter(el => el.offsetParent !== null);
    const currentIndex = items.indexOf(document.activeElement as HTMLElement);

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        if (currentIndex < items.length - 1) items[currentIndex + 1].focus();
        break;
      case 'ArrowUp':
        e.preventDefault();
        if (currentIndex > 0) items[currentIndex - 1].focus();
        break;
      case 'ArrowRight': {
        e.preventDefault();
        const btn = items[currentIndex];
        if (btn) {
          const path = btn.dataset.path;
          if (path && !expandedDirs.has(path)) {
            const entry = findEntry(path);
            if (entry?.isDir) toggleDir(entry);
          }
        }
        break;
      }
      case 'ArrowLeft': {
        e.preventDefault();
        const btn = items[currentIndex];
        if (btn) {
          const path = btn.dataset.path;
          if (path && expandedDirs.has(path)) {
            const entry = findEntry(path);
            if (entry?.isDir) toggleDir(entry);
          }
        }
        break;
      }
      case 'Home':
        e.preventDefault();
        if (items.length > 0) items[0].focus();
        break;
      case 'End':
        e.preventDefault();
        if (items.length > 0) items[items.length - 1].focus();
        break;
    }
  }

  function findEntry(path: string): FileEntry | undefined {
    function search(items: FileEntry[]): FileEntry | undefined {
      for (const item of items) {
        if (item.path === path) return item;
        if (item.isDir && childrenCache.has(item.path)) {
          const found = search(childrenCache.get(item.path)!);
          if (found) return found;
        }
      }
    }
    return search(entries);
  }

</script>

<div class="file-tree" role="tree" aria-label="File explorer" onkeydown={handleTreeKeydown}>
  <div class="tree-header">{currentRoot === '.' ? 'FILES' : currentRoot.split('/').pop()}</div>

  {#snippet renderEntries(items: FileEntry[], depth: number)}
    {#each items as entry (entry.path)}
      <button
        class="tree-item"
        class:gitignored={entry.isGitignored}
        style="padding-left: {14 + depth * 16}px"
        onclick={() => handleClick(entry)}
        role="treeitem"
        tabindex="-1"
        aria-expanded={entry.isDir ? expandedDirs.has(entry.path) : undefined}
        data-path={entry.path}
      >
        <span class="icon">{getFileIcon(entry.name, entry.isDir)}</span>
        <span class="name">{entry.name}</span>
      </button>
      {#if entry.isDir && expandedDirs.has(entry.path)}
        <div role="group">
          {@render renderEntries(childrenCache.get(entry.path) ?? [], depth + 1)}
        </div>
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
    font-family: var(--font-ui);
    background: var(--bg-surface);
  }

  .tree-header {
    font-size: var(--font-size-tiny);
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-muted);
    padding: 12px 14px 8px;
    flex-shrink: 0;
    border-bottom: 1px solid var(--border);
  }

  .tree-item {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: none;
    border: none;
    border-left: 2px solid transparent;
    color: var(--text-body);
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
    font-weight: 500;
    padding: 6px 14px;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
  }

  .tree-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-left-color: var(--accent);
  }

  .tree-item.gitignored {
    opacity: 0.5;
  }

  .tree-item.gitignored:hover {
    opacity: 0.7;
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
