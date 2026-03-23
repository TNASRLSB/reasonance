<script lang="ts">
  import MarkdownPreview from './MarkdownPreview.svelte';
  import { locale, tr } from '$lib/i18n/index';

  let content = $state('');
  let query = $state('');
  let contentEl = $state<HTMLElement | null>(null);

  async function loadDocs(loc: string) {
    try {
      const mod = await import(`../docs/${loc}/index.md?raw`);
      content = mod.default;
    } catch {
      try {
        const mod = await import('../docs/en/index.md?raw');
        content = mod.default;
      } catch {
        content = '# Documentation not found';
      }
    }
  }

  // OPT-08: return unsubscribe from $effect instead of onDestroy
  $effect(() => {
    return locale.subscribe((loc) => {
      loadDocs(loc);
    });
  });

  // Highlight and scroll-to-match on query change
  $effect(() => {
    const q = query.trim().toLowerCase();
    if (!contentEl) return;

    // Remove any existing highlights
    contentEl.querySelectorAll('mark.help-highlight').forEach((m) => {
      const parent = m.parentNode;
      if (parent) {
        parent.replaceChild(document.createTextNode(m.textContent ?? ''), m);
        parent.normalize();
      }
    });

    if (!q) return;

    // Walk text nodes and highlight
    let firstMatch: Element | null = null;
    const walker = document.createTreeWalker(contentEl, NodeFilter.SHOW_TEXT);
    const nodesToProcess: Text[] = [];
    let node: Node | null;
    while ((node = walker.nextNode())) {
      nodesToProcess.push(node as Text);
    }

    for (const textNode of nodesToProcess) {
      const text = textNode.textContent ?? '';
      const lowerText = text.toLowerCase();
      const idx = lowerText.indexOf(q);
      if (idx === -1) continue;

      const mark = document.createElement('mark');
      mark.className = 'help-highlight';
      mark.textContent = text.slice(idx, idx + q.length);

      const after = textNode.splitText(idx);
      after.textContent = text.slice(idx + q.length);
      textNode.parentNode?.insertBefore(mark, after);

      if (!firstMatch) firstMatch = mark;
    }

    if (firstMatch) {
      firstMatch.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  });

  // Derived: does the current content contain the query?
  let hasResults = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return true;
    return content.toLowerCase().includes(q);
  });
</script>

<div class="help-panel">
  <div class="help-search-bar">
    <input
      type="search"
      class="help-search-input"
      bind:value={query}
      placeholder={$tr('help.search.placeholder')}
      aria-label={$tr('help.search.placeholder')}
    />
  </div>
  {#if !hasResults && query.trim()}
    <p class="help-no-results">{$tr('help.search.noResults', { query: query.trim() })}</p>
  {/if}
  <div class="help-content" bind:this={contentEl}>
    <MarkdownPreview {content} />
  </div>
</div>

<style>
  .help-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .help-search-bar {
    flex-shrink: 0;
    padding: var(--space-2) var(--space-4);
    border-bottom: var(--border-width) solid var(--border);
    background: var(--bg-surface);
  }

  .help-search-input {
    width: 100%;
    background: var(--bg-tertiary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    padding: var(--space-1) var(--space-2);
    outline: none;
  }

  .help-search-input:focus {
    outline: var(--focus-ring);
    outline-offset: var(--focus-offset);
  }

  .help-no-results {
    padding: var(--space-2) var(--space-4);
    font-size: var(--font-size-small);
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .help-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-5) var(--space-6);
  }

  .help-content :global(mark.help-highlight) {
    background: var(--warning);
    color: var(--text-primary);
    border-radius: 2px;
    padding: 0 var(--space-1);
  }
</style>
