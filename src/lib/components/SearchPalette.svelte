<script lang="ts">
  import { get } from 'svelte/store';
  import type { Adapter, FileEntry } from '$lib/adapter/index';
  import { addOpenFile, projectRoot } from '$lib/stores/files';
  import { showToast } from '$lib/stores/toast';
  import { showSettings } from '$lib/stores/ui';
  import { tr } from '$lib/i18n/index';
  import { trapFocus } from '$lib/utils/a11y';
  import { checkForUpdate } from '$lib/updater';

  let {
    adapter,
    visible,
    onClose,
  }: {
    adapter: Adapter;
    visible: boolean;
    onClose: () => void;
  } = $props();

  // ── Command definitions ──────────────────────────────────────────────────

  interface Command {
    id: string;
    label: string;
    shortcut?: string;
    category: string;
    execute: () => void;
  }

  function dispatch(name: string) {
    document.dispatchEvent(new CustomEvent(name));
  }

  const commands: Command[] = [
    { id: 'settings', label: 'Open Settings', shortcut: 'Ctrl+,', category: 'General', execute: () => showSettings.set(true) },
    { id: 'help', label: 'Documentation', shortcut: 'F1', category: 'General', execute: () => dispatch('reasonance:help') },
    { id: 'shortcuts', label: 'Keyboard Shortcuts', shortcut: 'Ctrl+/', category: 'General', execute: () => dispatch('reasonance:shortcuts') },
    { id: 'about', label: 'About Reasonance', category: 'General', execute: () => dispatch('reasonance:about') },
    { id: 'save', label: 'Save File', shortcut: 'Ctrl+S', category: 'File', execute: () => dispatch('reasonance:save') },
    { id: 'saveAll', label: 'Save All Files', shortcut: 'Ctrl+Shift+S', category: 'File', execute: () => dispatch('reasonance:saveAll') },
    { id: 'openFolder', label: 'Open Folder', category: 'File', execute: () => dispatch('reasonance:openFolder') },
    { id: 'closeFile', label: 'Close File', shortcut: 'Ctrl+W', category: 'File', execute: () => dispatch('reasonance:closeFile') },
    { id: 'findInFiles', label: 'Find in Files', shortcut: 'Ctrl+Shift+F', category: 'Search', execute: () => dispatch('reasonance:findInFiles') },
    { id: 'newTerminal', label: 'New LLM Session', category: 'Terminal', execute: () => dispatch('reasonance:newTerminal') },
    { id: 'closeTerminal', label: 'Close Terminal', category: 'Terminal', execute: () => dispatch('reasonance:closeTerminal') },
    { id: 'detectLLMs', label: 'Detect LLMs', category: 'Terminal', execute: () => dispatch('reasonance:detectLLMs') },
    { id: 'toggleFilePanel', label: 'Toggle File Panel', category: 'View', execute: () => dispatch('reasonance:toggleFilePanel') },
    { id: 'toggleTerminalPanel', label: 'Toggle Terminal Panel', category: 'View', execute: () => dispatch('reasonance:toggleTerminalPanel') },
    { id: 'zoomIn', label: 'Zoom In', shortcut: 'Ctrl++', category: 'View', execute: () => dispatch('reasonance:zoomIn') },
    { id: 'zoomOut', label: 'Zoom Out', shortcut: 'Ctrl+-', category: 'View', execute: () => dispatch('reasonance:zoomOut') },
    { id: 'sessions', label: 'Session History', shortcut: 'Ctrl+Shift+H', category: 'General', execute: () => dispatch('reasonance:sessions') },
    { id: 'analytics', label: 'Toggle Analytics', shortcut: 'Ctrl+Shift+A', category: 'View', execute: () => dispatch('reasonance:analytics') },
    { id: 'checkUpdate', label: 'Check for Updates', category: 'General', execute: () => { checkForUpdate(true); } },
  ];

  // ── Result types ─────────────────────────────────────────────────────────

  interface ResultItem {
    type: 'command' | 'file';
    label: string;
    detail: string;
    shortcut?: string;
    path?: string;
    command?: Command;
  }

  let query = $state('');
  let allFiles = $state<string[]>([]);
  let results = $state<ResultItem[]>([]);
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

  // Build flat file list recursively
  async function buildFileList(dirPath: string, visited: Set<string> = new Set(), depth = 0): Promise<string[]> {
    const MAX_DEPTH = 50;
    if (depth > MAX_DEPTH || visited.has(dirPath)) return [];
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
    } catch { /* skip unreadable dirs */ }
    return result;
  }

  $effect(() => {
    if (visible) {
      loadFiles();
    } else {
      query = '';
      results = [];
      selectedIndex = 0;
    }
  });

  $effect(() => {
    if (visible && inputEl) inputEl.focus();
  });

  async function loadFiles() {
    if (allFiles.length > 0) { applyFilter(); return; }
    loading = true;
    const root = get(projectRoot) || '.';
    allFiles = await buildFileList(root);
    loading = false;
    applyFilter();
  }

  function fuzzyMatch(text: string, q: string): number {
    const lower = text.toLowerCase();
    if (q === '') return 0;
    if (lower.startsWith(q)) return 3;
    if (lower.includes(q)) return 2;
    // Fuzzy: all chars in order
    let idx = 0;
    for (const ch of q) {
      const found = lower.indexOf(ch, idx);
      if (found === -1) return -1;
      idx = found + 1;
    }
    return 1;
  }

  function applyFilter() {
    const raw = query.trim();
    const isCommandMode = raw.startsWith('>');
    const q = (isCommandMode ? raw.slice(1) : raw).toLowerCase().trim();

    const items: ResultItem[] = [];

    // Search commands
    for (const cmd of commands) {
      const score = fuzzyMatch(cmd.label, q);
      if (score >= 0) {
        items.push({
          type: 'command',
          label: cmd.label,
          detail: cmd.category,
          shortcut: cmd.shortcut,
          command: cmd,
        });
      }
    }

    // Search files (unless in command-only mode)
    if (!isCommandMode) {
      const fileResults: { item: ResultItem; score: number }[] = [];
      for (const path of allFiles) {
        const name = path.split('/').pop() ?? path;
        const score = fuzzyMatch(name, q);
        if (score >= 0) {
          fileResults.push({
            item: {
              type: 'file',
              label: name,
              detail: displayPath(path),
              path,
            },
            score,
          });
        }
      }
      fileResults.sort((a, b) => b.score - a.score);
      // Interleave: commands first, then files
      items.push(...fileResults.slice(0, 20).map((r) => r.item));
    }

    results = items.slice(0, 30);
    selectedIndex = 0;
  }

  $effect(() => { query; applyFilter(); });

  async function selectItem(item: ResultItem) {
    if (item.type === 'command' && item.command) {
      onClose();
      // Defer execution so the palette closes first
      setTimeout(() => item.command!.execute(), 50);
    } else if (item.type === 'file' && item.path) {
      try {
        const content = await adapter.readFile(item.path);
        addOpenFile({ path: item.path, name: item.label, content, isDirty: false, isDeleted: false });
      } catch (e) {
        const msg = (e as Error)?.message ?? String(e);
        if (msg.includes('non-UTF-8') || msg.includes('binary')) {
          showToast('info', 'Binary file', `Opening "${item.label}" externally`);
          adapter.openExternal(item.path);
        } else {
          showToast('error', $tr('search.errorOpen'), item.label);
          return;
        }
      }
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { onClose(); return; }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (results[selectedIndex]) selectItem(results[selectedIndex]);
    }
  }

  function handleOverlayClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('palette-overlay')) onClose();
  }

  function displayPath(path: string): string {
    const root = get(projectRoot) || '.';
    if (path.startsWith(root + '/')) return path.slice(root.length + 1);
    if (path.startsWith('./')) return path.slice(2);
    return path;
  }
</script>

{#if visible}
  <div class="palette-overlay" role="presentation" onclick={handleOverlayClick} onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}>
    <div class="palette" role="dialog" aria-label="Command palette" aria-modal="true" bind:this={dialogEl}>
      <div class="palette-input-row">
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={handleKeydown}
          type="text"
          placeholder={$tr('a11y.commandPalettePlaceholder')}
          class="palette-input"
          aria-label={$tr('a11y.search')}
          autocomplete="off"
          spellcheck="false"
        />
        {#if loading}
          <span class="palette-hint">Loading...</span>
        {/if}
      </div>

      {#if results.length > 0}
        <ul class="palette-list" role="listbox">
          {#each results as item, i}
            <li
              class="palette-item"
              class:selected={i === selectedIndex}
              class:command={item.type === 'command'}
              role="option"
              aria-selected={i === selectedIndex}
              onclick={() => selectItem(item)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') selectItem(item); }}
            >
              <span class="item-icon">{item.type === 'command' ? '>' : ''}</span>
              <span class="item-label">{item.label}</span>
              <span class="item-detail">{item.detail}</span>
              {#if item.shortcut}
                <kbd class="item-shortcut">{item.shortcut}</kbd>
              {/if}
            </li>
          {/each}
        </ul>
      {:else if !loading && query.trim()}
        <p class="palette-empty">No results for "{query}"</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .palette-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    z-index: var(--layer-modal);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 60px;
  }

  .palette {
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    width: 560px;
    max-width: 95vw;
    max-height: 50vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--font-ui);
  }

  .palette-input-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
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
    font-size: var(--font-size-tiny);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .palette-list {
    list-style: none;
    margin: 0;
    padding: var(--space-1) 0;
    overflow-y: auto;
    flex: 1;
  }

  .palette-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    cursor: pointer;
    transition: background var(--transition-fast);
    min-height: 28px;
  }

  .palette-item:hover,
  .palette-item.selected {
    background: var(--accent-btn);
    color: var(--text-on-accent);
  }

  .item-icon {
    width: 14px;
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    opacity: 0.6;
    text-align: center;
  }

  .item-label {
    font-size: var(--font-size-small);
    font-weight: 600;
    flex-shrink: 0;
  }

  .item-detail {
    font-size: var(--font-size-tiny);
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .palette-item.selected .item-detail,
  .palette-item:hover .item-detail {
    color: color-mix(in srgb, var(--text-on-accent) 60%, transparent);
  }

  .item-shortcut {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    padding: 0 var(--space-1);
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    color: var(--text-secondary);
    flex-shrink: 0;
    line-height: 1.6;
  }

  .palette-item.selected .item-shortcut,
  .palette-item:hover .item-shortcut {
    border-color: color-mix(in srgb, var(--text-on-accent) 30%, transparent);
    background: transparent;
    color: color-mix(in srgb, var(--text-on-accent) 80%, transparent);
  }

  .palette-empty {
    padding: var(--space-4) var(--space-3);
    font-size: var(--font-size-small);
    color: var(--text-secondary);
    margin: 0;
    text-align: center;
  }
</style>
