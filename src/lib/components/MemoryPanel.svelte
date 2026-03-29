<script lang="ts">
  import type { Adapter, MemoryEntry } from '$lib/adapter/index';
  import { showToast } from '$lib/stores/toast';
  import { tr } from '$lib/i18n/index';
  import { onMount } from 'svelte';

  let { adapter, projectId, nodeId, visible, onClose }: {
    adapter: Adapter;
    projectId?: string;
    nodeId?: string;
    visible: boolean;
    onClose: () => void;
  } = $props();

  type Scope = 'node' | 'project' | 'global';
  type SortMode = 'recency' | 'importance' | 'relevance';

  let query = $state('');
  let scope = $state<Scope>('global');
  let sort = $state<SortMode>('recency');
  let scopeInitialized = false;
  let entries = $state<MemoryEntry[]>([]);
  let selectedEntry = $state<MemoryEntry | null>(null);
  let loading = $state(false);
  let offset = $state(0);
  let hasMore = $state(false);
  const PAGE_SIZE = 50;

  // Debounce timer for search
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  function scopeId(): string | undefined {
    if (scope === 'node') return nodeId;
    if (scope === 'project') return projectId;
    return undefined;
  }

  async function loadEntries(append = false) {
    loading = true;
    try {
      let results: MemoryEntry[];
      if (query.trim()) {
        results = await adapter.memorySearch(query.trim(), scope, scopeId(), PAGE_SIZE);
        hasMore = false; // search doesn't support offset
      } else {
        const currentOffset = append ? offset : 0;
        results = await adapter.memoryList(scope, scopeId(), sort, PAGE_SIZE, currentOffset);
        hasMore = results.length === PAGE_SIZE;
      }
      if (append) {
        entries = [...entries, ...results];
      } else {
        entries = results;
        offset = 0;
      }
    } catch (e) {
      showToast('error', 'Failed to load memories', String(e));
    }
    loading = false;
  }

  function loadMore() {
    offset += PAGE_SIZE;
    loadEntries(true);
  }

  function onSearchInput() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      sort = query.trim() ? 'relevance' : 'recency';
      loadEntries();
    }, 300);
  }

  function changeScope(newScope: Scope) {
    scope = newScope;
    selectedEntry = null;
    loadEntries();
  }

  function changeSort(newSort: SortMode) {
    sort = newSort;
    loadEntries();
  }

  function selectEntry(entry: MemoryEntry) {
    selectedEntry = entry;
  }

  function backToList() {
    selectedEntry = null;
  }

  function formatTimestamp(ts: string): string {
    try {
      return new Intl.DateTimeFormat(undefined, {
        month: 'short', day: 'numeric',
        hour: '2-digit', minute: '2-digit',
      }).format(new Date(ts));
    } catch {
      return ts;
    }
  }

  function truncate(text: string, maxLen: number): string {
    if (!text || text.length <= maxLen) return text || '';
    return text.slice(0, maxLen) + '...';
  }

  function parseTags(tags: string | undefined): string[] {
    if (!tags) return [];
    return tags.split(',').map(t => t.trim()).filter(Boolean);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (selectedEntry) { selectedEntry = null; }
      else { onClose(); }
    }
  }

  $effect(() => {
    if (visible) {
      if (!scopeInitialized) {
        scope = nodeId ? 'node' : projectId ? 'project' : 'global';
        scopeInitialized = true;
      }
      loadEntries();
    } else {
      scopeInitialized = false;
    }
  });
</script>

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="memory-overlay" onclick={(e) => { if ((e.target as HTMLElement).classList.contains('memory-overlay')) onClose(); }} onkeydown={handleKeydown}>
    <div class="memory-panel" role="dialog" aria-label={$tr('a11y.memoryPanel')} aria-modal="true">
      <div class="panel-header">
        <h2 class="panel-title">{$tr('memory.title')}</h2>
        <button class="panel-close" onclick={onClose} aria-label="Close">&#10005;</button>
      </div>

      <!-- Scope tabs -->
      <div class="scope-bar" role="tablist" aria-label="Memory scope">
        <button
          class="scope-tab"
          class:active={scope === 'node'}
          role="tab"
          aria-selected={scope === 'node'}
          disabled={!nodeId}
          onclick={() => changeScope('node')}
        >{$tr('memory.scopeNode')}</button>
        <button
          class="scope-tab"
          class:active={scope === 'project'}
          role="tab"
          aria-selected={scope === 'project'}
          disabled={!projectId}
          onclick={() => changeScope('project')}
        >{$tr('memory.scopeProject')}</button>
        <button
          class="scope-tab"
          class:active={scope === 'global'}
          role="tab"
          aria-selected={scope === 'global'}
          onclick={() => changeScope('global')}
        >{$tr('memory.scopeGlobal')}</button>
      </div>

      <!-- Search + sort -->
      <div class="search-row">
        <input
          type="text"
          class="search-input"
          bind:value={query}
          oninput={onSearchInput}
          placeholder={$tr('memory.search')}
          aria-label={$tr('a11y.memorySearch')}
        />
        <select
          class="sort-select"
          value={sort}
          onchange={(e) => changeSort((e.target as HTMLSelectElement).value as SortMode)}
          aria-label="Sort order"
        >
          <option value="recency">{$tr('memory.sortRecency')}</option>
          <option value="importance">{$tr('memory.sortImportance')}</option>
          {#if query.trim()}
            <option value="relevance">{$tr('memory.sortRelevance')}</option>
          {/if}
        </select>
      </div>

      <!-- Content area -->
      <div class="panel-body">
        {#if selectedEntry}
          <!-- Detail view -->
          <div class="detail-view">
            <button class="back-btn" onclick={backToList}>&larr; {$tr('memory.detail.back')}</button>

            <div class="detail-header">
              <span class="detail-timestamp">{formatTimestamp(selectedEntry.timestamp)}</span>
              <span class="outcome-badge" class:success={selectedEntry.outcome === 'success'} class:failed={selectedEntry.outcome !== 'success'}>{selectedEntry.outcome}</span>
              {#if selectedEntry.importance !== undefined}
                <span class="importance-score" title="Importance">{selectedEntry.importance.toFixed(1)}</span>
              {/if}
            </div>

            {#if selectedEntry.id}
              <div class="detail-section">
                <div class="detail-label">ID</div>
                <div class="detail-value mono">{selectedEntry.id}</div>
              </div>
            {/if}

            {#if selectedEntry.node_id}
              <div class="detail-section">
                <div class="detail-label">Node</div>
                <div class="detail-value mono">{selectedEntry.node_id}</div>
              </div>
            {/if}

            <div class="detail-section">
              <div class="detail-label">{$tr('memory.detail.input')}</div>
              <div class="detail-value">{selectedEntry.input_summary}</div>
            </div>

            <div class="detail-section">
              <div class="detail-label">{$tr('memory.detail.output')}</div>
              <div class="detail-value">{selectedEntry.output_summary}</div>
            </div>

            {#if selectedEntry.tags}
              <div class="detail-section">
                <div class="detail-label">{$tr('memory.detail.tags')}</div>
                <div class="tags-row">
                  {#each parseTags(selectedEntry.tags) as tag}
                    <span class="tag">{tag}</span>
                  {/each}
                </div>
              </div>
            {/if}

            {#if selectedEntry.context}
              <div class="detail-section">
                <div class="detail-label">{$tr('memory.detail.context')}</div>
                <pre class="detail-json">{JSON.stringify(selectedEntry.context, null, 2)}</pre>
              </div>
            {/if}
          </div>
        {:else if loading && entries.length === 0}
          <div class="panel-empty">{$tr('memory.loading')}</div>
        {:else if entries.length === 0}
          <div class="panel-empty">{$tr('memory.noResults')}</div>
        {:else}
          <!-- Results list -->
          <ul class="entry-list" role="list" aria-label={$tr('a11y.memoryResults')}>
            {#each entries as entry, i (entry.id ?? `${entry.run_id}-${i}`)}
              <li class="entry-item">
                <button class="entry-btn" onclick={() => selectEntry(entry)}>
                  <div class="entry-top">
                    <span class="entry-timestamp">{formatTimestamp(entry.timestamp)}</span>
                    <span class="outcome-badge" class:success={entry.outcome === 'success'} class:failed={entry.outcome !== 'success'}>{entry.outcome}</span>
                    {#if entry.importance !== undefined}
                      <span class="importance-score" title="Importance">{entry.importance.toFixed(1)}</span>
                    {/if}
                  </div>
                  <div class="entry-summary input-summary">{truncate(entry.input_summary, 120)}</div>
                  <div class="entry-summary output-summary">{truncate(entry.output_summary, 80)}</div>
                </button>
              </li>
            {/each}
          </ul>

          {#if hasMore}
            <div class="load-more-row">
              <button class="load-more-btn" onclick={loadMore} disabled={loading}>
                {loading ? $tr('memory.loading') : $tr('memory.loadMore')}
              </button>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .memory-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    z-index: var(--layer-modal);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 60px;
  }

  .memory-panel {
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    width: 600px;
    max-width: 95vw;
    max-height: 75vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--font-ui);
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .panel-title {
    font-size: var(--font-size-tiny);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-primary);
    margin: 0;
  }

  .panel-close {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    cursor: pointer;
    min-width: 32px;
    min-height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .panel-close:hover {
    color: var(--text-primary);
  }

  /* Scope tabs */
  .scope-bar {
    display: flex;
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .scope-tab {
    flex: 1;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: var(--space-2) var(--space-2);
    cursor: pointer;
    transition: color var(--transition-fast), border-color var(--transition-fast);
  }

  .scope-tab:hover:not(:disabled) {
    color: var(--text-primary);
  }

  .scope-tab.active {
    color: var(--accent-text);
    border-bottom-color: var(--accent);
  }

  .scope-tab:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  /* Search row */
  .search-row {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    color: var(--text-primary);
    padding: var(--space-1) var(--space-2);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .search-input:focus {
    border-color: var(--accent);
    outline: none;
  }

  .sort-select {
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    color: var(--text-primary);
    padding: var(--space-1) var(--space-2);
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    cursor: pointer;
  }

  .sort-select:focus {
    border-color: var(--accent);
    outline: none;
  }

  /* Body */
  .panel-body {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .panel-empty {
    padding: var(--space-5) var(--space-4);
    text-align: center;
    color: var(--text-muted);
    font-size: var(--font-size-small);
  }

  /* Entry list */
  .entry-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .entry-item {
    border-bottom: 1px solid var(--bg-tertiary);
  }

  .entry-item:hover {
    background: var(--bg-hover);
  }

  .entry-btn {
    display: block;
    width: 100%;
    background: transparent;
    border: none;
    text-align: start;
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    font-family: var(--font-ui);
    color: var(--text-primary);
  }

  .entry-btn:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-offset, -2px);
  }

  .entry-top {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--stack-tight);
  }

  .entry-timestamp {
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    font-weight: 600;
  }

  .outcome-badge {
    font-size: var(--font-size-tiny);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0 var(--space-1);
  }

  .outcome-badge.success {
    color: var(--success-text);
  }

  .outcome-badge.failed {
    color: var(--danger-text);
  }

  .importance-score {
    font-size: var(--font-size-tiny);
    font-family: var(--font-mono, monospace);
    color: var(--accent-text);
    font-weight: 700;
    margin-inline-start: auto;
  }

  .entry-summary {
    font-size: var(--font-size-small);
    line-height: 1.35;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .input-summary {
    color: var(--text-primary);
    font-weight: 600;
  }

  .output-summary {
    color: var(--text-secondary);
    margin-top: 1px;
  }

  /* Load more */
  .load-more-row {
    padding: var(--space-2) var(--space-3);
    text-align: center;
  }

  .load-more-btn {
    background: var(--bg-tertiary);
    border: var(--border-width) solid var(--border);
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: var(--space-1) var(--space-3);
    cursor: pointer;
  }

  .load-more-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .load-more-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Detail view */
  .detail-view {
    padding: var(--space-3);
  }

  .back-btn {
    background: transparent;
    border: none;
    color: var(--accent-text);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    cursor: pointer;
    padding: var(--space-1) 0;
    margin-bottom: var(--space-2);
  }

  .back-btn:hover {
    text-decoration: underline;
  }

  .detail-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
    padding-bottom: var(--space-2);
    border-bottom: var(--border-width) solid var(--border);
  }

  .detail-timestamp {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    font-weight: 600;
  }

  .detail-section {
    margin-bottom: var(--space-3);
  }

  .detail-label {
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    text-transform: uppercase;
    font-weight: 700;
    letter-spacing: 0.06em;
    margin-bottom: var(--stack-tight);
  }

  .detail-value {
    font-size: var(--font-size-small);
    color: var(--text-primary);
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .detail-value.mono {
    font-family: var(--font-mono, monospace);
    font-size: var(--font-size-tiny);
  }

  .tags-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .tag {
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    padding: 1px var(--space-1);
  }

  .detail-json {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    padding: var(--space-2);
    font-family: var(--font-mono, monospace);
    font-size: var(--font-size-tiny);
    color: var(--text-body);
    overflow-x: auto;
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
  }
</style>
