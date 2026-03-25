<script lang="ts">
  import { onMount } from 'svelte';
  import type { Adapter, FileEntry } from '$lib/adapter';
  import { getFileIcon } from '$lib/utils/icons';
  import { addOpenFile, projectRoot, activeFilePath } from '$lib/stores/files';
  import { showToast } from '$lib/stores/toast';
  import { tr } from '$lib/i18n/index';

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

  // Listen for filesystem changes (dispatched by +page.svelte watcher)
  $effect(() => {
    function handleFsChange(e: Event) {
      const { type, path } = (e as CustomEvent).detail as { type: string; path: string };
      if (type !== 'create' && type !== 'remove') return;
      const parentDir = path.substring(0, path.lastIndexOf('/')) || currentRoot;
      // Refresh the parent directory if it's currently visible
      adapter.listDir(parentDir).then((refreshed) => {
        if (parentDir === currentRoot) {
          entries = refreshed;
        } else if (childrenCache.has(parentDir)) {
          childrenCache.set(parentDir, refreshed);
          childrenCache = new Map(childrenCache);
        }
      }).catch(() => { /* parent dir may no longer exist */ });
    }
    document.addEventListener('reasonance:fsChange', handleFsChange);
    return () => document.removeEventListener('reasonance:fsChange', handleFsChange);
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
        const msg = (err as Error)?.message ?? String(err);
        // Binary files: open externally instead of showing error
        if (msg.includes('non-UTF-8') || msg.includes('binary')) {
          showToast('info', 'Binary file', `Opening "${entry.name}" with default application`);
          adapter.openExternal(entry.path);
        } else {
          console.error('Failed to read file:', err);
          showToast('error', 'Failed to read file', msg);
        }
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
      case 'Enter':
      case ' ': {
        e.preventDefault();
        const btn = items[currentIndex];
        if (btn) {
          const path = btn.dataset.path;
          if (path) {
            const entry = findEntry(path);
            if (entry) handleClick(entry);
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

  // Context menu state
  let ctxVisible = $state(false);
  let ctxX = $state(0);
  let ctxY = $state(0);
  let ctxTargetDir = $state('');
  let inlineInput = $state<{ parentDir: string; type: 'file' | 'folder' } | null>(null);
  let inlineValue = $state('');

  function handleContextMenu(e: MouseEvent, entry: FileEntry) {
    e.preventDefault();
    e.stopPropagation();
    ctxX = e.clientX;
    ctxY = e.clientY;
    ctxTargetDir = entry.isDir ? entry.path : entry.path.substring(0, entry.path.lastIndexOf('/'));
    ctxVisible = true;
  }

  function handleTreeContextMenu(e: MouseEvent) {
    e.preventDefault();
    ctxX = e.clientX;
    ctxY = e.clientY;
    ctxTargetDir = currentRoot;
    ctxVisible = true;
  }

  function startInlineCreate(type: 'file' | 'folder') {
    ctxVisible = false;
    inlineInput = { parentDir: ctxTargetDir, type };
    inlineValue = '';
    // Expand the parent directory so the input is visible
    if (ctxTargetDir !== currentRoot && !expandedDirs.has(ctxTargetDir)) {
      expandedDirs.add(ctxTargetDir);
      expandedDirs = new Set(expandedDirs);
    }
  }

  async function commitInlineCreate() {
    if (!inlineInput || !inlineValue.trim()) {
      inlineInput = null;
      return;
    }
    const fullPath = `${inlineInput.parentDir}/${inlineValue.trim()}`;
    try {
      if (inlineInput.type === 'folder') {
        // Create folder by writing a placeholder and relying on the backend
        // The adapter has no createDir, so we write a file inside the dir
        // Actually, we can use writeFile to create the directory path
        await adapter.writeFile(`${fullPath}/.keep`, '');
      } else {
        await adapter.writeFile(fullPath, '');
        addOpenFile({ path: fullPath, name: inlineValue.trim(), content: '', isDirty: false, isDeleted: false });
      }
      // Refresh the parent directory listing
      const parentDir = inlineInput.parentDir;
      const refreshed = await adapter.listDir(parentDir);
      if (parentDir === currentRoot) {
        entries = refreshed;
      } else {
        childrenCache.set(parentDir, refreshed);
        childrenCache = new Map(childrenCache);
      }
    } catch (err) {
      showToast('error', 'Creation failed', (err as Error)?.message ?? String(err));
    }
    inlineInput = null;
  }

  function cancelInlineCreate() {
    inlineInput = null;
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

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="file-tree" oncontextmenu={handleTreeContextMenu}>
  <div class="tree-header">
    <span>{currentRoot === '.' ? 'FILES' : currentRoot.split('/').pop()}</span>
    <span class="tree-header-actions">
      <button class="tree-action-btn" title={$tr('a11y.newFile')} aria-label={$tr('a11y.newFile')} onclick={() => { ctxTargetDir = currentRoot; startInlineCreate('file'); }}>+</button>
      <button class="tree-action-btn" title={$tr('a11y.newFolder')} aria-label={$tr('a11y.newFolder')} onclick={() => { ctxTargetDir = currentRoot; startInlineCreate('folder'); }}>&#128193;</button>
    </span>
  </div>

  <div class="tree-scroll" role="tree" tabindex="-1" aria-label={$tr('a11y.fileExplorer')} onkeydown={handleTreeKeydown}>
    {#snippet renderEntries(items: FileEntry[], depth: number)}
      {#each items as entry, idx (entry.path)}
        <button
          class="tree-item"
          class:gitignored={entry.isGitignored}
          class:active={!entry.isDir && $activeFilePath === entry.path}
          style="padding-inline-start: {14 + depth * 16}px"
          onclick={() => handleClick(entry)}
          oncontextmenu={(e) => handleContextMenu(e, entry)}
          role="treeitem"
          tabindex={depth === 0 && idx === 0 ? 0 : -1}
          aria-selected={!entry.isDir && $activeFilePath === entry.path}
          aria-expanded={entry.isDir ? expandedDirs.has(entry.path) : undefined}
          data-path={entry.path}
        >
          <span class="icon">{getFileIcon(entry.name, entry.isDir)}</span>
          <span class="name">{entry.name}</span>
        </button>
        {#if entry.isDir && expandedDirs.has(entry.path)}
          <div role="group">
            {@render renderEntries(childrenCache.get(entry.path) ?? [], depth + 1)}
            {#if inlineInput && inlineInput.parentDir === entry.path}
              <div class="inline-input-row" style="padding-inline-start: {14 + (depth + 1) * 16}px">
                <span class="icon">{inlineInput.type === 'folder' ? '\ud83d\udcc1' : '\ud83d\udcc4'}</span>
                <input
                  class="inline-input"
                  type="text"
                  bind:value={inlineValue}
                  placeholder={inlineInput.type === 'folder' ? 'folder name' : 'file name'}
                  onkeydown={(e) => { if (e.key === 'Enter') commitInlineCreate(); if (e.key === 'Escape') cancelInlineCreate(); }}
                  onblur={commitInlineCreate}
                />
              </div>
            {/if}
          </div>
        {/if}
      {/each}
    {/snippet}

    {@render renderEntries(entries, 0)}

    {#if inlineInput && inlineInput.parentDir === currentRoot}
      <div class="inline-input-row" style="padding-inline-start: 14px">
        <span class="icon">{inlineInput.type === 'folder' ? '\ud83d\udcc1' : '\ud83d\udcc4'}</span>
        <input
          class="inline-input"
          type="text"
          bind:value={inlineValue}
          placeholder={inlineInput.type === 'folder' ? 'folder name' : 'file name'}
          onkeydown={(e) => { if (e.key === 'Enter') commitInlineCreate(); if (e.key === 'Escape') cancelInlineCreate(); }}
          onblur={commitInlineCreate}
        />
      </div>
    {/if}
  </div>
</div>

{#if ctxVisible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="ctx-overlay" onclick={() => { ctxVisible = false; }} onkeydown={(e) => { if (e.key === 'Escape') ctxVisible = false; }} oncontextmenu={(e) => { e.preventDefault(); ctxVisible = false; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="ctx-menu" role="menu" tabindex="-1" style="left: {ctxX}px; top: {ctxY}px" onclick={(e) => e.stopPropagation()}>
      <button class="ctx-item" onclick={() => startInlineCreate('file')}>New File</button>
      <button class="ctx-item" onclick={() => startInlineCreate('folder')}>New Folder</button>
    </div>
  </div>
{/if}

<style>
  .file-tree {
    height: 100%;
    display: flex;
    flex-direction: column;
    font-family: var(--font-ui);
    background: var(--bg-surface);
    overflow: hidden;
    min-height: 0;
  }

  .tree-header {
    font-size: var(--font-size-tiny);
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-primary);
    padding: 0 var(--space-3);
    height: 38px;
    flex-shrink: 0;
    border-bottom: var(--border-width) solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .tree-scroll {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .tree-item {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    background: none;
    border: none;
    border-inline-start: 2px solid transparent;
    color: var(--text-body);
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
    font-weight: 500;
    padding: var(--space-1) var(--space-3);
    text-align: start;
    cursor: pointer;
    white-space: nowrap;
    overflow: auto;
    transition: background var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }

  .tree-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-inline-start-color: var(--accent);
  }

  .tree-item.active {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-inline-start-color: var(--accent);
    font-weight: 600;
  }

  .tree-item.gitignored {
    opacity: 0.5;
  }

  .tree-item.gitignored:hover {
    opacity: 0.7;
  }

  .icon {
    flex-shrink: 0;
    font-size: var(--font-size-base);
    line-height: 1;
  }

  .name {
    overflow: auto;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tree-header-actions {
    display: flex;
    gap: var(--stack-tight);
  }

  .tree-action-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: var(--font-size-sm);
    cursor: pointer;
    padding: 0 var(--space-1);
    line-height: 1;
    min-width: var(--interactive-min, 24px);
  }

  .tree-action-btn:hover {
    color: var(--text-primary);
  }

  .inline-input-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--stack-tight) var(--space-3);
  }

  .inline-input {
    flex: 1;
    background: var(--bg-secondary);
    border: 1px solid var(--accent);
    color: var(--text-primary);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    padding: var(--stack-tight) var(--space-1);
    outline: none;
    min-width: 0;
  }

  .ctx-overlay {
    position: fixed;
    inset: 0;
    z-index: var(--layer-dropdown);
  }

  .ctx-menu {
    position: fixed;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    z-index: calc(var(--layer-dropdown) + 1);
    min-width: 140px;
    padding: var(--space-1) 0;
  }

  .ctx-item {
    display: block;
    width: 100%;
    padding: var(--space-1) var(--space-3);
    text-align: start;
    background: none;
    border: none;
    color: var(--text-body);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    cursor: pointer;
  }

  .ctx-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
</style>
