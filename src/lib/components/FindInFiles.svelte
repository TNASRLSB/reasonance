<script lang="ts">
  import type { Adapter, GrepResult } from '$lib/adapter/index';
  import { addOpenFile, activeFilePath } from '$lib/stores/files';
  import { tr } from '$lib/i18n/index';

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
      // Note: jumping to line would require editor line API; for now just open the file
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
  <div class="fif-overlay" role="button" tabindex="-1" onclick={handleOverlayClick} onkeydown={(e) => { if (e.key === 'Escape') handleOverlayClick(); }}>
    <div class="fif-panel" role="dialog" aria-label={$tr('fif.ariaLabel')} aria-modal="true">
      <div class="fif-header">
        <span class="fif-title">{$tr('fif.title')}</span>
        <button class="close-btn" onclick={onClose} aria-label="Close">&#10005;</button>
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
                <div
                  class="fif-result-row"
                  role="button"
                  tabindex="0"
                  onclick={() => openResult(result)}
                  onkeydown={(e) => e.key === 'Enter' && openResult(result)}
                >
                  <span class="fif-line-num">{result.line_number}</span>
                  <span class="fif-line-text">{result.line.trim()}</span>
                </div>
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
    background: rgba(0, 0, 0, 0.5);
    z-index: 2000;
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
    padding: 10px 14px 8px;
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .fif-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 14px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: var(--radius);
  }

  .close-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .fif-input-row {
    display: flex;
    gap: 8px;
    padding: 10px 14px;
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .fif-input {
    flex: 1;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    padding: 6px 10px;
    outline: none;
  }

  .fif-input:focus {
    border-color: var(--accent);
  }

  .search-btn {
    background: var(--accent);
    color: #fff;
    border: var(--border-width) solid var(--accent);
    border-radius: var(--radius);
    padding: 6px 14px;
    font-size: 12px;
    cursor: pointer;
    flex-shrink: 0;
    transition: opacity 0.15s;
  }

  .search-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .search-btn:not(:disabled):hover {
    opacity: 0.85;
  }

  .fif-error {
    padding: 8px 14px;
    font-size: 12px;
    color: var(--danger, #e74c3c);
    margin: 0;
    flex-shrink: 0;
  }

  .fif-empty {
    padding: 20px 14px;
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0;
    text-align: center;
  }

  .fif-summary {
    padding: 6px 14px;
    font-size: 11px;
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
    padding: 6px 14px;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    background: var(--bg-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fif-result-row {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 4px 14px;
    cursor: pointer;
    transition: background 0.1s;
  }

  .fif-result-row:hover {
    background: var(--accent);
  }

  .fif-result-row:hover .fif-line-num,
  .fif-result-row:hover .fif-line-text {
    color: #fff;
  }

  .fif-line-num {
    font-size: 11px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
    min-width: 28px;
    text-align: right;
    flex-shrink: 0;
  }

  .fif-line-text {
    font-size: 12px;
    color: var(--text-primary);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
