<script lang="ts">
  import { onMount, tick } from 'svelte';
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
  let readAbortController: AbortController | null = null;
  let gitAbortController: AbortController | null = null;

  let currentRoot = $derived($projectRoot || '.');

  // --- Git status state ---
  let gitStatuses = $state<Record<string, string>>({});
  let gitRefreshTimer: ReturnType<typeof setTimeout> | null = null;

  async function refreshGitStatus() {
    // Cancel any in-flight git status request (e.g. rapid project root changes)
    gitAbortController?.abort();
    gitAbortController = new AbortController();
    const signal = gitAbortController.signal;
    try {
      const statuses = await adapter.getGitStatus(currentRoot, signal);
      gitStatuses = statuses;
    } catch (err) {
      if (err instanceof DOMException && err.name === 'AbortError') return;
      // non-git project or git not available — ignore
      gitStatuses = {};
    }
  }

  function scheduleGitRefresh() {
    if (gitRefreshTimer) clearTimeout(gitRefreshTimer);
    gitRefreshTimer = setTimeout(refreshGitStatus, 2000);
  }

  function getGitStatus(path: string): string {
    const relativePath = path.replace(currentRoot + '/', '');
    return gitStatuses[relativePath] || '';
  }

  function getGitStatusLetter(path: string): string {
    const status = getGitStatus(path);
    switch (status) {
      case 'modified': return 'M';
      case 'added': return 'A';
      case 'deleted': return 'D';
      case 'renamed': return 'R';
      case 'untracked': return 'U';
      case 'conflicted': return 'C';
      default: return '';
    }
  }

  // --- Virtual scroll constants ---
  const ROW_HEIGHT = 28;
  const BUFFER_ITEMS = 20;

  // --- Virtual scroll state ---
  let scrollTop = $state(0);
  let containerHeight = $state(400);
  let focusedIndex = $state(0);

  // --- Flat tree representation for virtual scrolling ---
  interface FlatItem {
    entry: FileEntry;      // the first (display) entry — used for name, gitignored, icon
    foldedEntry: FileEntry; // the last entry in the fold chain — used for expand/collapse
    displayName: string;   // folded path like "src/main/java" or just entry.name
    depth: number;
    expanded: boolean;
    isDir: boolean;
  }

  // Directories that break the auto-fold chain
  const FOLD_BREAK_DIRS = new Set(['.git', '.svn', '.hg', 'node_modules', '.yarn', '.pnpm']);

  /**
   * Walk a single-child directory chain and return the final entry + joined display path.
   * Stops when a dir has != 1 child, the child is a file, or the child is a hidden/break dir.
   */
  function getFoldedPath(
    entry: FileEntry,
    cache: Map<string, FileEntry[]>
  ): { displayName: string; foldedEntry: FileEntry } {
    let current = entry;
    const parts = [current.name];

    while (true) {
      const children = cache.get(current.path);
      if (!children || children.length !== 1 || !children[0].isDir) break;
      const child = children[0];
      // Don't fold through hidden / special dirs
      if (FOLD_BREAK_DIRS.has(child.name) || child.name.startsWith('.')) break;
      current = child;
      parts.push(current.name);
    }

    return { displayName: parts.join('/'), foldedEntry: current };
  }

  function flattenTree(
    items: FileEntry[],
    expanded: Set<string>,
    cache: Map<string, FileEntry[]>,
    depth: number = 0
  ): FlatItem[] {
    const result: FlatItem[] = [];
    for (const entry of items) {
      if (!entry.isDir) {
        result.push({ entry, foldedEntry: entry, displayName: entry.name, depth, expanded: false, isDir: false });
        continue;
      }

      // Attempt to fold single-child directory chains
      const { displayName, foldedEntry } = getFoldedPath(entry, cache);
      const isExpanded = expanded.has(foldedEntry.path);

      result.push({ entry, foldedEntry, displayName, depth, expanded: isExpanded, isDir: true });

      if (isExpanded) {
        const children = cache.get(foldedEntry.path) || [];
        result.push(...flattenTree(children, expanded, cache, depth + 1));
      }
    }
    return result;
  }

  let flatItems = $derived(flattenTree(entries, expandedDirs, childrenCache));
  let totalHeight = $derived(flatItems.length * ROW_HEIGHT);

  let visibleStart = $derived(Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER_ITEMS));
  let visibleEnd = $derived(Math.min(flatItems.length, Math.ceil((scrollTop + containerHeight) / ROW_HEIGHT) + BUFFER_ITEMS));
  let visibleItems = $derived(flatItems.slice(visibleStart, visibleEnd));
  let offsetY = $derived(visibleStart * ROW_HEIGHT);

  // Reload entries when project root changes
  $effect(() => {
    const root = $projectRoot;
    if (root) {
      // Reset caches on project switch
      childrenCache = new Map();
      expandedDirs = new Set();
      focusedIndex = 0;
      adapter.listDir(root).then((e) => { entries = e; });
      refreshGitStatus();
    }
  });

  onMount(async () => {
    const root = $projectRoot || '.';
    entries = await adapter.listDir(root);
    refreshGitStatus();
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
      scheduleGitRefresh();
    }
    document.addEventListener('reasonance:fsChange', handleFsChange);
    return () => document.removeEventListener('reasonance:fsChange', handleFsChange);
  });

  // Scroll focused item into view on keyboard navigation
  $effect(() => {
    const idx = focusedIndex;
    void idx; // track reactivity
    tick().then(() => {
      const item = flatItems[idx];
      if (!item) return;
      const el = document.querySelector(`[data-path="${CSS.escape(item.foldedEntry.path)}"]`) as HTMLElement | null;
      el?.scrollIntoView({ block: 'nearest' });
      el?.focus();
    });
  });

  async function toggleDir(entry: FileEntry) {
    if (expandedDirs.has(entry.path)) {
      expandedDirs.delete(entry.path);
      expandedDirs = new Set(expandedDirs);
    } else {
      // Load and walk the single-child chain so getFoldedPath can fold correctly
      let current = entry;
      const newCache = new Map(childrenCache);
      while (true) {
        if (!newCache.has(current.path)) {
          const children = await adapter.listDir(current.path);
          newCache.set(current.path, children);
        }
        const children = newCache.get(current.path)!;
        // Stop walking if: not a single-child dir, child is a file, or child is a hidden/break dir
        if (
          children.length !== 1 ||
          !children[0].isDir ||
          FOLD_BREAK_DIRS.has(children[0].name) ||
          children[0].name.startsWith('.')
        ) break;
        current = children[0];
      }
      childrenCache = newCache;
      expandedDirs.add(current.path);
      expandedDirs = new Set(expandedDirs);
    }
  }

  function handleClick(entry: FileEntry, foldedEntry?: FileEntry) {
    if (entry.isDir) {
      toggleDir(foldedEntry ?? entry);
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
      // Cancel any previous in-flight readFile (e.g. user clicks a different file before the first loads)
      readAbortController?.abort();
      readAbortController = new AbortController();
      const signal = readAbortController.signal;
      try {
        const content = await adapter.readFile(entry.path, signal);
        addOpenFile(entry.path, content);
      } catch (err) {
        if (err instanceof DOMException && err.name === 'AbortError') return;
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

  function handleItemKeydown(e: KeyboardEvent, flatIndex: number) {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        focusedIndex = Math.min(flatIndex + 1, flatItems.length - 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        focusedIndex = Math.max(flatIndex - 1, 0);
        break;
      case 'ArrowRight': {
        e.preventDefault();
        const item = flatItems[flatIndex];
        if (item?.isDir && !item.expanded) {
          toggleDir(item.foldedEntry);
        }
        break;
      }
      case 'ArrowLeft': {
        e.preventDefault();
        const item = flatItems[flatIndex];
        if (item?.isDir && item.expanded) {
          toggleDir(item.foldedEntry);
        }
        break;
      }
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (flatItems[flatIndex]) handleClick(flatItems[flatIndex].entry, flatItems[flatIndex].foldedEntry);
        break;
      case 'Home':
        e.preventDefault();
        focusedIndex = 0;
        break;
      case 'End':
        e.preventDefault();
        focusedIndex = flatItems.length - 1;
        break;
      case 'Delete': {
        e.preventDefault();
        const item = flatItems[flatIndex];
        if (item && !item.entry.isDir) {
          deleteFile(item.entry);
        }
        break;
      }
    }
  }

  // Context menu state
  let ctxVisible = $state(false);
  let ctxX = $state(0);
  let ctxY = $state(0);
  let ctxTargetDir = $state('');
  let ctxEntry = $state<FileEntry | null>(null);
  let inlineInput = $state<{ parentDir: string; type: 'file' | 'folder' } | null>(null);
  let inlineValue = $state('');

  function handleContextMenu(e: MouseEvent, entry: FileEntry) {
    e.preventDefault();
    e.stopPropagation();
    ctxX = e.clientX;
    ctxY = e.clientY;
    ctxTargetDir = entry.isDir ? entry.path : entry.path.substring(0, entry.path.lastIndexOf('/'));
    ctxEntry = entry;
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
        await adapter.writeFile(`${fullPath}/.keep`, '');
      } else {
        await adapter.writeFile(fullPath, '');
        addOpenFile(fullPath, '');
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

  async function deleteFile(entry: FileEntry) {
    if (entry.isDir) return;
    const name = entry.name;
    try {
      await adapter.fileOpsDelete(entry.path);
      const { closeFile } = await import('$lib/stores/projects/namespace');
      closeFile(entry.path);
      showToast('info', $tr('fileTree.deleted', { name }), '', 8000, {
        actions: [{
          label: $tr('fileTree.undo'),
          onClick: async () => {
            try {
              await adapter.fileOpsUndo();
              showToast('success', $tr('fileTree.undone', { name }));
            } catch (e) {
              showToast('error', 'Undo failed', String(e));
            }
          },
        }],
      });
    } catch (e) {
      showToast('error', 'Delete failed', String(e));
    }
    ctxVisible = false;
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

  /**
   * Determine the inline input insert position within the flat list.
   * Returns the flat index AFTER which the inline input should appear,
   * or -1 if it should appear at the end of the root list.
   */
  function getInlineInputFlatIndex(): number | null {
    if (!inlineInput) return null;
    if (inlineInput.parentDir === currentRoot) {
      // After the last root-level item (and all its children)
      return flatItems.length - 1;
    }
    // Find the parent dir in the flat list (match against foldedEntry.path for folded dirs)
    for (let i = 0; i < flatItems.length; i++) {
      if (flatItems[i].foldedEntry.path === inlineInput.parentDir) {
        // Find the last child of this directory
        const parentDepth = flatItems[i].depth;
        let lastChild = i;
        for (let j = i + 1; j < flatItems.length; j++) {
          if (flatItems[j].depth > parentDepth) {
            lastChild = j;
          } else {
            break;
          }
        }
        return lastChild;
      }
    }
    return null;
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

  <div
    class="tree-scroll"
    role="tree"
    tabindex="-1"
    aria-label={$tr('a11y.fileExplorer')}
    onscroll={(e) => { scrollTop = e.currentTarget.scrollTop; }}
    bind:clientHeight={containerHeight}
  >
    <div class="filetree-spacer" style="height: {totalHeight}px; position: relative;">
      <div class="filetree-viewport" style="transform: translateY({offsetY}px); will-change: transform;">
        {#each visibleItems as item, i (item.entry.path)}
          {@const flatIndex = visibleStart + i}
          {@const inlineIdx = getInlineInputFlatIndex()}
          <button
            class="tree-item"
            class:gitignored={item.entry.isGitignored}
            class:active={!item.isDir && $activeFilePath === item.entry.path}
            style="padding-inline-start: {14 + item.depth * 16}px; height: {ROW_HEIGHT}px;"
            onclick={() => handleClick(item.entry, item.foldedEntry)}
            oncontextmenu={(e) => handleContextMenu(e, item.foldedEntry)}
            onkeydown={(e) => handleItemKeydown(e, flatIndex)}
            role="treeitem"
            tabindex={flatIndex === focusedIndex ? 0 : -1}
            aria-selected={!item.isDir && $activeFilePath === item.entry.path}
            aria-expanded={item.isDir ? item.expanded : undefined}
            aria-level={item.depth + 1}
            data-path={item.foldedEntry.path}
          >
            <span class="icon">{getFileIcon(item.entry.name, item.isDir)}</span>
            <span class="name">{item.displayName}</span>
            {#if !item.isDir && getGitStatusLetter(item.entry.path)}
              <span class="git-status git-{getGitStatus(item.entry.path)}" aria-label="git: {getGitStatus(item.entry.path)}">{getGitStatusLetter(item.entry.path)}</span>
            {/if}
          </button>
          {#if inlineInput && inlineIdx === flatIndex}
            <div class="inline-input-row" style="padding-inline-start: {14 + (item.depth + (inlineInput.parentDir === item.foldedEntry.path ? 1 : 0)) * 16}px; height: {ROW_HEIGHT}px;">
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
        {/each}
        {#if inlineInput && inlineInput.parentDir === currentRoot && flatItems.length === 0}
          <div class="inline-input-row" style="padding-inline-start: 14px; height: {ROW_HEIGHT}px;">
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
  </div>
</div>

{#if ctxVisible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="ctx-overlay" onclick={() => { ctxVisible = false; }} onkeydown={(e) => { if (e.key === 'Escape') ctxVisible = false; }} oncontextmenu={(e) => { e.preventDefault(); ctxVisible = false; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="ctx-menu" role="menu" tabindex="-1" style="left: {ctxX}px; top: {ctxY}px" onclick={(e) => e.stopPropagation()}>
      <button class="ctx-item" onclick={() => startInlineCreate('file')}>{$tr('fileTree.newFile')}</button>
      <button class="ctx-item" onclick={() => startInlineCreate('folder')}>{$tr('fileTree.newFolder')}</button>
      {#if ctxEntry && !ctxEntry.isDir}
        <button class="ctx-item ctx-item--danger" onclick={() => ctxEntry && deleteFile(ctxEntry)}>{$tr('fileTree.delete')}</button>
      {/if}
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

  .filetree-spacer {
    overflow: hidden;
  }

  .filetree-viewport {
    will-change: transform;
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
    padding: 0 var(--space-3);
    text-align: start;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    box-sizing: border-box;
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

  .git-status {
    font-size: 0.7rem;
    margin-left: auto;
    font-weight: 600;
    flex-shrink: 0;
  }

  .git-modified {
    color: #e5c07b;
  }

  .git-added {
    color: #98c379;
  }

  .git-deleted {
    color: #e06c75;
  }

  .git-renamed {
    color: #61afef;
  }

  .git-untracked {
    color: #888;
  }

  .git-conflicted {
    color: #e06c75;
    font-weight: 800;
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

  .ctx-item--danger {
    color: var(--error);
  }

  .ctx-item--danger:hover {
    background: var(--error);
    color: var(--bg-surface);
  }
</style>
