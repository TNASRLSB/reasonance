<script lang="ts">
  import type { Adapter, GrepResult } from '$lib/adapter/index';
  import { addOpenFile, activeFilePath, pendingLine } from '$lib/stores/files';
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

  let pattern = $state('');
  let results = $state<GrepResult[]>([]);
  let searching = $state(false);
  let error = $state('');
  let searched = $state(false);
  let inputEl = $state<HTMLInputElement | null>(null);
  let dialogEl = $state<HTMLElement | null>(null);

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
    } else {
      pattern = '';
    }
  });

  $effect(() => {
    if (visible && inputEl) {
      inputEl.focus();
    }
  });

  async function runSearch() {
    const q = pattern.trim();
    if (!q) return;
    searching = true;
    error = '';
    searched = false;
    try {
      results = await adapter.grepFiles('.', q, true);
      searched = true;
    } catch (e) {
      console.error('Find in files error:', e);
      error = 'Search failed. Try a simpler pattern or check that the project folder is accessible.';
    } finally {
      searching = false;
    }
  }

  async function openResult(result: GrepResult) {
    try {
      const content = await adapter.readFile(result.path);
      const name = result.path.split('/').pop() ?? result.path;
      addOpenFile({ path: result.path, name, content, isDirty: false, isDeleted: false });
      pendingLine.set(result.line_number);
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
  function groupByFile(list: GrepResult[]): Map<string, GrepResult[]> {
    const map = new Map<string, GrepResult[]>();
    for (const r of list) {
      let arr = map.get(r.path);
      if (!arr) { arr = []; map.set(r.path, arr); }
      arr.push(r);
    }
    return map;
  }

  let grouped = $derived(groupByFile(results));
  let fileCount = $derived(grouped.size);
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
        </div>

        <div class="fif-results">
          {#each [...grouped.entries()] as [filePath, fileResults]}
            <div class="fif-file-group">
              <div class="fif-file-header">{filePath}</div>
              {#each fileResults as result}
                <button
                  class="fif-result-row"
                  onclick={() => openResult(result)}
                  aria-label="Line {result.line_number}: {result.line.trim()}"
                >
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
</style>
