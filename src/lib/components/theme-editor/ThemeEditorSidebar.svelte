<script lang="ts">
  let {
    sections,
    activeSection,
    onSelectSection,
    searchQuery,
    onSearch,
    onToggleJson,
  }: {
    sections: string[];
    activeSection: string;
    onSelectSection: (name: string) => void;
    searchQuery: string;
    onSearch: (query: string) => void;
    onToggleJson: () => void;
  } = $props();
</script>

<aside class="sidebar" role="navigation" aria-label="Theme editor sections">
  <div class="search-wrap">
    <input
      type="search"
      class="search-input"
      placeholder="Search variables..."
      value={searchQuery}
      oninput={(e) => onSearch((e.target as HTMLInputElement).value)}
      aria-label="Search theme variables"
    />
  </div>

  <ul class="section-list" role="listbox" aria-label="Sections">
    {#each sections as section}
      <li>
        <button
          class="section-item"
          class:active={section === activeSection}
          onclick={() => onSelectSection(section)}
          role="option"
          aria-selected={section === activeSection}
        >
          {section}
        </button>
      </li>
    {/each}
  </ul>

  <div class="sidebar-footer">
    <button class="json-toggle-btn" onclick={onToggleJson} aria-label="Toggle JSON view">
      &lt; JSON
    </button>
  </div>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    width: 180px;
    flex-shrink: 0;
    background: var(--bg-surface, #141414);
    border-right: 1px solid var(--border, #2a2a2a);
    height: 100%;
    overflow: hidden;
  }

  .search-wrap {
    padding: var(--space-2, 8px);
    border-bottom: 1px solid var(--border, #2a2a2a);
    flex-shrink: 0;
  }

  .search-input {
    width: 100%;
    font-size: 12px;
    font-family: var(--font-ui, sans-serif);
    background: var(--bg-tertiary, #1a1a1a);
    border: 1px solid var(--border, #333);
    color: var(--text-primary, #eee);
    padding: 4px 8px;
    box-sizing: border-box;
  }

  .search-input::placeholder {
    color: var(--text-muted, #666);
  }

  .search-input:focus {
    outline: var(--focus-ring, 2px solid #4a9eff);
    outline-offset: 1px;
  }

  .section-list {
    list-style: none;
    margin: 0;
    padding: var(--space-1, 4px) 0;
    flex: 1;
    overflow-y: auto;
  }

  .section-item {
    display: block;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--text-secondary, #aaa);
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    padding: 6px 12px;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background var(--transition-fast, 100ms), color var(--transition-fast, 100ms);
  }

  .section-item:hover {
    background: var(--bg-hover, #252525);
    color: var(--text-primary, #eee);
  }

  .section-item.active {
    background: var(--bg-hover, #252525);
    color: var(--accent-text, #7ac7ff);
    font-weight: 700;
    border-left: 2px solid var(--accent, #4a9eff);
    padding-left: 10px;
  }

  .sidebar-footer {
    padding: var(--space-2, 8px);
    border-top: 1px solid var(--border, #2a2a2a);
    flex-shrink: 0;
  }

  .json-toggle-btn {
    width: 100%;
    background: var(--bg-tertiary, #1a1a1a);
    border: 1px solid var(--border, #333);
    color: var(--text-muted, #888);
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    padding: 4px 8px;
    cursor: pointer;
    text-align: left;
  }

  .json-toggle-btn:hover {
    background: var(--bg-hover, #252525);
    color: var(--text-primary, #eee);
  }
</style>
