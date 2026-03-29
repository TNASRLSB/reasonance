<script lang="ts">
  import type { Adapter, GrepResult } from '$lib/adapter/index';
  import { addOpenFile, pendingLine, activeFilePath, activeEditorView, pendingAnchorIndex } from '$lib/stores/files';
  import { tr } from '$lib/i18n/index';
  import { trapFocus } from '$lib/utils/a11y';
  import { setAnchors } from '$lib/editor/search-anchors';
  import { get } from 'svelte/store';

  interface AnchoredResult extends GrepResult {
    stale: boolean;
    /** Index into the searchAnchorsField positions array for this result's file. */
    anchorIndex: number;
  }

  let {
    adapter,
    visible,
    onClose,
  }: {
    adapter: Adapter;
    visible: boolean;
    onClose: () => void;
  } = $props();

  let pattern = $state('');
  let results = $state<AnchoredResult[]>([]);
  let searching = $state(false);
  let error = $state('');
  let searched = $state(false);
  let inputEl = $state<HTMLInputElement | null>(null);
  let dialogEl = $state<HTMLElement | null>(null);

  // Tracks the mtime (ms) of each file at search time: path → mtime
  let searchMtimes = $state<Record<string, number>>({});

  $effect(() => {
    if (visible && dialogEl) {
      const destroy = trapFocus(dialogEl);
      return destroy;
    }
  });

  $effect(() => {
    if (visible) {
      // Reset state when opened
      results = [];
      error = '';
      searched = false;
      searchMtimes = {};
    } else {
      pattern = '';
    }
  });

  $effect(() => {
    if (visible && inputEl) {
      inputEl.focus();
    }
  });

  /**
   * For each unique parent directory in the result paths, call listDir once
   * and return a map of filePath → mtime (ms). Falls back silently on error.
   */
  async function fetchMtimes(paths: string[]): Promise<Record<string, number>> {
    const mtimeMap: Record<string, number> = {};
    // Group paths by parent directory to batch listDir calls
    const byDir = new Map<string, string[]>();
    for (const p of paths) {
      const slash = p.lastIndexOf('/');
      const dir = slash > 0 ? p.slice(0, slash) : '.';
      let arr = byDir.get(dir);
      if (!arr) { arr = []; byDir.set(dir, arr); }
      arr.push(p);
    }
    await Promise.all(
      [...byDir.entries()].map(async ([dir, filePaths]) => {
        try {
          const entries = await adapter.listDir(dir, false);
          for (const entry of entries) {
            if (filePaths.includes(entry.path)) {
              mtimeMap[entry.path] = entry.modified;
            }
          }
        } catch {
          // Non-fatal — leave those files without mtime data
        }
      })
    );
    return mtimeMap;
  }

  /**
   * Convert a 1-based line number to an absolute document offset using the
   * active CodeMirror EditorView, if the file is already open in the editor.
   * Returns null if the view is not available or the line is out of range.
   */
  function lineToPos(view: import('@codemirror/view').EditorView, lineNumber: number): number | null {
    try {
      const lineInfo = view.state.doc.line(Math.min(lineNumber, view.state.doc.lines));
      return lineInfo.from;
    } catch {
      return null;
    }
  }

  /**
   * If the currently active file is the searched file, dispatch setAnchors
   * for those results so the Editor's StateField tracks them through edits.
   */
  function maybeDispatchAnchors(filePath: string, fileResults: AnchoredResult[]): void {
    const view = get(activeEditorView);
    const currentPath = get(activeFilePath);
    if (!view || currentPath !== filePath) return;
    const positions = fileResults.map((r) => lineToPos(view, r.line_number) ?? 0);
    view.dispatch({ effects: setAnchors.of(positions) });
  }

  async function runSearch() {
    const q = pattern.trim();
    if (!q) return;
    searching = true;
    error = '';
    searched = false;
    results = [];
    searchMtimes = {};
    try {
      const raw = await adapter.grepFiles('.', q, true);
      // Collect unique paths and fetch their mtimes before storing results
      const uniquePaths = [...new Set(raw.map((r) => r.path))];
      const mtimes = await fetchMtimes(uniquePaths);
      searchMtimes = mtimes;

      // Assign per-file anchor indices (each file has its own index sequence)
      const fileIndexCounters = new Map<string, number>();
      const mapped: AnchoredResult[] = raw.map((r) => {
        const count = fileIndexCounters.get(r.path) ?? 0;
        fileIndexCounters.set(r.path, count + 1);
        return { ...r, stale: false, anchorIndex: count };
      });
      results = mapped;
      searched = true;

      // For any file already open in the editor, push anchor positions now
      const grouped = groupByFile(mapped);
      const currentPath = get(activeFilePath);
      if (currentPath && grouped.has(currentPath)) {
        maybeDispatchAnchors(currentPath, grouped.get(currentPath)!);
      }
    } catch (e) {
      console.error('Find in files error:', e);
      error = 'Search failed. Try a simpler pattern or check that the project folder is accessible.';
    } finally {
      searching = false;
    }
  }

  async function openResult(result: AnchoredResult) {
    // Before opening, do a fresh mtime check to update stale state
    try {
      const slash = result.path.lastIndexOf('/');
      const dir = slash > 0 ? result.path.slice(0, slash) : '.';
      const entries = await adapter.listDir(dir, false);
      const entry = entries.find((e) => e.path === result.path);
      if (entry) {
        const searchMtime = searchMtimes[result.path];
        if (searchMtime !== undefined && entry.modified > searchMtime) {
          // Mark all results for this file as stale
          results = results.map((r) =>
            r.path === result.path ? { ...r, stale: true } : r
          );
          // Don't navigate — let user see the stale state
          return;
        }
      }
    } catch {
      // Non-fatal — proceed with navigation
    }

    try {
      const content = await adapter.readFile(result.path);
      const fileResults = results.filter((r) => r.path === result.path);
      const isAlreadyOpen = get(activeFilePath) === result.path;

      addOpenFile(result.path, content);

      // W3.5: If the file was already open, the editor already has anchors set.
      // If it's newly opened, we need to dispatch anchors after the editor
      // initialises — we do so reactively. For now also set pendingLine as
      // fallback so navigation works even if the editor hasn't yet processed
      // the anchor dispatch.
      if (isAlreadyOpen) {
        // Dispatch fresh anchors (editor view is live)
        maybeDispatchAnchors(result.path, fileResults);
        pendingAnchorIndex.set(result.anchorIndex);
      } else {
        // File is being opened — fall back to line-based navigation since the
        // editor hasn't mounted yet; anchors will be set on next search open.
        pendingLine.set(result.line_number);
      }
    } catch {
      // Non-fatal
    }
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
      return;
    }
    if (e.key === 'Enter') {
      runSearch();
    }
  }

  function handleOverlayClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('fif-overlay')) onClose();
  }

  // Group results by file for display
  function groupByFile(list: AnchoredResult[]): Map<string, AnchoredResult[]> {
    const map = new Map<string, AnchoredResult[]>();
    for (const r of list) {
      let arr = map.get(r.path);
      if (!arr) { arr = []; map.set(r.path, arr); }
      arr.push(r);
    }
    return map;
  }

  let grouped = $derived(groupByFile(results));
  let fileCount = $derived(grouped.size);
  let hasStaleResults = $derived(results.some((r) => r.stale));
</script>

{#if visible}
  <div class="fif-overlay" role="presentation" onclick={handleOverlayClick} onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}>
    <div class="fif-panel" role="dialog" aria-label={$tr('fif.ariaLabel')} aria-modal="true" bind:this={dialogEl}>
      <div class="fif-header">
        <span class="fif-title">{$tr('fif.title')}</span>
        <button class="close-btn" onclick={onClose} aria-label={$tr('settings.close')}>&#10005;</button>
      </div>

      <div class="fif-input-row">
        <input
          bind:this={inputEl}
          bind:value={pattern}
          onkeydown={handleKeydown}
          type="text"
          placeholder={$tr('fif.placeholder')}
          class="fif-input"
          aria-label={$tr('fif.placeholder')}
          autocomplete="off"
          spellcheck="false"
        />
        <button class="search-btn" onclick={runSearch} disabled={searching || !pattern.trim()}>
          {searching ? $tr('fif.searching') : $tr('fif.search')}
        </button>
      </div>

      {#if error}
        <p class="fif-error">{$tr('fif.error')}</p>
      {/if}

      {#if searched && results.length === 0}
        <p class="fif-empty">{$tr('fif.noResults', { pattern })}</p>
      {/if}

      {#if results.length > 0}
        <div class="fif-summary">
          {$tr('fif.summary', { count: String(results.length), files: String(fileCount) })}
          {#if hasStaleResults}
            <button class="re-search" onclick={runSearch} disabled={searching}>
              Re-search to refresh
            </button>
          {/if}
        </div>

        <div class="fif-results">
          {#each [...grouped.entries()] as [filePath, fileResults]}
            <div class="fif-file-group">
              <div class="fif-file-header">{filePath}</div>
              {#each fileResults as result}
                <button
                  class="fif-result-row"
                  class:stale={result.stale}
                  onclick={() => openResult(result)}
                  aria-label="Line {result.line_number}: {result.line.trim()}{result.stale ? ' (stale — file modified since search)' : ''}"
                >
                  {#if result.stale}
                    <span class="stale-badge" aria-hidden="true" title="File modified since search">stale</span>
                  {/if}
                  <span class="fif-line-num" aria-hidden="true">{result.line_number}</span>
                  <span class="fif-line-text" aria-hidden="true">{result.line.trim()}</span>
                </button>
              {/each}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .fif-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    z-index: var(--layer-modal);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 80px;
  }

  .fif-panel {
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    width: 640px;
    max-width: 95vw;
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--font-ui);
  }

  .fif-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .fif-title {
    font-size: var(--font-size-tiny);
    font-weight: 800;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: var(--font-size-base);
    cursor: pointer;
    padding: var(--stack-tight) var(--space-1);
    border-radius: var(--radius);
    min-width: 32px;
    min-height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .close-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .fif-input-row {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .fif-input {
    flex: 1;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-family: inherit;
    padding: var(--space-1) var(--space-2);
    outline: none;
  }

  .fif-input:focus {
    border-color: var(--accent);
  }

  .search-btn {
    background: var(--accent-btn);
    color: var(--text-on-accent);
    border: var(--border-width) solid var(--accent);
    border-radius: var(--radius);
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-size-sm);
    cursor: pointer;
    flex-shrink: 0;
    transition: opacity var(--transition-fast);
  }

  .search-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .search-btn:not(:disabled):hover {
    opacity: 0.85;
  }

  .fif-error {
    padding: var(--space-2) var(--space-3);
    font-size: var(--font-size-sm);
    color: var(--danger);
    margin: 0;
    flex-shrink: 0;
  }

  .fif-empty {
    padding: var(--space-5) var(--space-3);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin: 0;
    text-align: center;
  }

  .fif-summary {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .fif-results {
    flex: 1;
    overflow-y: auto;
  }

  .fif-file-group {
    border-bottom: 1px solid var(--border);
  }

  .fif-file-header {
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--accent-text);
    background: var(--bg-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fif-result-row {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    cursor: pointer;
    transition: background var(--transition-fast);
    width: 100%;
    text-align: start;
    background: none;
    border: none;
    color: inherit;
    font-family: inherit;
  }

  .fif-result-row:hover {
    background: var(--accent-btn);
  }

  .fif-result-row:hover .fif-line-num,
  .fif-result-row:hover .fif-line-text {
    color: var(--text-on-accent);
  }

  .fif-line-num {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
    min-width: 28px;
    text-align: end;
    flex-shrink: 0;
  }

  .fif-line-text {
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    font-family: var(--font-mono);
    overflow: auto;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fif-result-row.stale {
    opacity: 0.5;
  }

  .stale-badge {
    font-size: 0.65rem;
    color: var(--warning-text, #c8902a);
    background: var(--warning-bg, rgba(240, 173, 78, 0.1));
    padding: 0 4px;
    border-radius: 2px;
    margin-right: 4px;
    flex-shrink: 0;
    font-family: var(--font-ui);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .re-search {
    background: none;
    border: var(--border-width) solid var(--warning-text, #c8902a);
    border-radius: var(--radius);
    color: var(--warning-text, #c8902a);
    font-size: var(--font-size-tiny, 0.7rem);
    padding: 2px var(--space-2);
    cursor: pointer;
    transition: opacity var(--transition-fast);
    flex-shrink: 0;
    font-family: var(--font-ui);
  }

  .re-search:hover:not(:disabled) {
    opacity: 0.75;
  }

  .re-search:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
