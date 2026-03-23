<script lang="ts">
  import { get } from 'svelte/store';
  import type { Adapter, FileEntry } from '$lib/adapter/index';
  import { addOpenFile, projectRoot } from '$lib/stores/files';
  import { showToast } from '$lib/stores/toast';
  import { tr } from '$lib/i18n/index';
  import { trapFocus } from '$lib/utils/a11y';

  let {
    adapter,
    visible,
    onClose,
  }: {
    adapter: Adapter;
    visible: boolean;
    onClose: () => void;
  } = $props();

  let query = $state('');
  let allFiles = $state<string[]>([]);
  let matches = $state<string[]>([]);
  let selectedIndex = $state(0);
  let loading = $state(false);
  let inputEl = $state<HTMLInputElement | null>(null);
  let dialogEl = $state<HTMLElement | null>(null);

  $effect(() => {
    if (visible && dialogEl) {
      const destroy = trapFocus(dialogEl);
      return destroy;
    }
  });

  // Build flat file list by recursively listing directories (with cycle detection)
  async function buildFileList(dirPath: string, visited: Set<string> = new Set(), depth: number = 0): Promise<string[]> {
    // Prevent infinite recursion from symlink loops or extreme depth
    const MAX_DEPTH = 50;
    if (depth > MAX_DEPTH || visited.has(dirPath)) {
      return [];
    }
    visited.add(dirPath);

    let result: string[] = [];
    try {
      const entries: FileEntry[] = await adapter.listDir(dirPath, true);
      for (const entry of entries) {
        if (entry.isDir) {
          const children = await buildFileList(entry.path, visited, depth + 1);
          result = result.concat(children);
        } else {
          result.push(entry.path);
        }
      }
    } catch {
      // Skip unreadable dirs
    }
    return result;
  }

  $effect(() => {
    if (visible) {
      loadFiles();
    } else {
      query = '';
      matches = [];
      selectedIndex = 0;
    }
  });

  $effect(() => {
    if (visible && inputEl) {
      inputEl.focus();
    }
  });

  async function loadFiles() {
    if (allFiles.length > 0) {
      applyFilter();
      return;
    }
    loading = true;
    const root = get(projectRoot) || '.';
    allFiles = await buildFileList(root);
    loading = false;
    applyFilter();
  }

  function fuzzyScore(path: string, q: string): number {
    const lower = path.toLowerCase();
    const name = path.split('/').pop()?.toLowerCase() ?? lower;
    if (q === '') return 0;
    // Starts-with on filename gets highest score
    if (name.startsWith(q)) return 3;
    // Contains on filename
    if (name.includes(q)) return 2;
    // Contains on full path
    if (lower.includes(q)) return 1;
    // Fuzzy: all chars appear in order
    let idx = 0;
    for (const ch of q) {
      const found = lower.indexOf(ch, idx);
      if (found === -1) return -1;
      idx = found + 1;
    }
    return 0;
  }

  function applyFilter() {
    const q = query.toLowerCase().trim();
    if (!q) {
      matches = allFiles.slice(0, 20);
      selectedIndex = 0;
      return;
    }
    const scored = allFiles
      .map((f) => ({ f, score: fuzzyScore(f, q) }))
      .filter((x) => x.score >= 0)
      .sort((a, b) => b.score - a.score)
      .slice(0, 20)
      .map((x) => x.f);
    matches = scored;
    selectedIndex = 0;
  }

  $effect(() => {
    // Re-run filter whenever query changes
    query;
    applyFilter();
  });

  async function openFile(path: string) {
    try {
      const content = await adapter.readFile(path);
      const name = path.split('/').pop() ?? path;
      addOpenFile({ path, name, content, isDirty: false, isDeleted: false });
    } catch (e) {
      console.error('SearchPalette openFile error:', e);
      const name = path.split('/').pop() ?? path;
      showToast('error', $tr('search.errorOpen'), name);
      return;
    }
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, matches.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (matches[selectedIndex]) openFile(matches[selectedIndex]);
    }
  }

  function handleOverlayClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('palette-overlay')) onClose();
  }

  function displayPath(path: string): string {
    // Show relative path if possible
    const root = get(projectRoot) || '.';
    if (path.startsWith(root + '/')) return path.slice(root.length + 1);
    if (path.startsWith('./')) return path.slice(2);
    return path;
  }
</script>

{#if visible}
  <div class="palette-overlay" role="presentation" onclick={handleOverlayClick} onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}>
    <div class="palette" role="dialog" aria-label={$tr('search.ariaLabel')} aria-modal="true" bind:this={dialogEl}>
      <div class="palette-input-row">
        <span class="palette-icon">&#128269;</span>
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={handleKeydown}
          type="text"
          placeholder={$tr('search.placeholder')}
          class="palette-input"
          aria-label={$tr('search.inputLabel')}
          autocomplete="off"
          spellcheck="false"
        />
        {#if loading}
          <span class="palette-hint">{$tr('search.loading')}</span>
        {:else}
          <span class="palette-hint">{$tr('search.escClose')}</span>
        {/if}
        <button class="palette-close" onclick={onClose} aria-label={$tr('search.close')}>&#10005;</button>
      </div>

      {#if matches.length > 0}
        <ul class="palette-list" role="listbox">
          {#each matches as path, i}
            <li
              class="palette-item"
              class:selected={i === selectedIndex}
              role="option"
              aria-selected={i === selectedIndex}
              onclick={() => openFile(path)}
              onkeydown={(e) => { if (e.key === 'Enter') openFile(path); }}
            >
              <span class="item-name">{path.split('/').pop()}</span>
              <span class="item-dir">{displayPath(path).split('/').slice(0, -1).join('/')}</span>
            </li>
          {/each}
        </ul>
      {:else if !loading && query.trim()}
        <p class="palette-empty">{$tr('search.noMatch', { query })}</p>
      {:else if !loading}
        <p class="palette-empty">{$tr('search.hint')}</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .palette-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 2000;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 80px;
  }

  .palette {
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    width: 580px;
    max-width: 95vw;
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--font-ui);
  }

  .palette-input-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .palette-icon {
    font-size: var(--font-size-base);
    opacity: 0.6;
  }

  .palette-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-size: var(--font-size-base);
    font-family: var(--font-ui);
  }

  .palette-input::placeholder {
    color: var(--text-secondary);
  }

  .palette-hint {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .palette-list {
    list-style: none;
    margin: 0;
    padding: 4px 0;
    overflow-y: auto;
    flex: 1;
  }

  .palette-item {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 6px 14px;
    cursor: pointer;
    transition: background 0.1s;
  }

  .palette-item:hover,
  .palette-item.selected {
    background: var(--accent);
    color: var(--text-on-accent);
  }

  .palette-item.selected .item-dir,
  .palette-item:hover .item-dir {
    color: color-mix(in srgb, var(--text-on-accent) 70%, transparent);
  }

  .item-name {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
    flex-shrink: 0;
    font-family: var(--font-ui);
  }

  .item-dir {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .palette-empty {
    padding: 20px 14px;
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin: 0;
    text-align: center;
  }

  .palette-close {
    flex-shrink: 0;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    padding: 4px 6px;
    cursor: pointer;
    line-height: 1;
    min-width: 32px;
    min-height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .palette-close:hover {
    color: var(--text-primary);
  }
</style>
