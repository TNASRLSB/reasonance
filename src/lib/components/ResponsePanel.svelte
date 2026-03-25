<script lang="ts">
  import { Marked } from 'marked';
  import { markedHighlight } from 'marked-highlight';
  import hljs from 'highlight.js';
  import 'highlight.js/styles/github-dark.css';
  import DOMPurify from 'dompurify';
  import { tr } from '$lib/i18n/index';
  import AnalyticsBar from './AnalyticsBar.svelte';
  import { analyticsDashboard } from '$lib/stores/ui';
  import type { Adapter } from '$lib/adapter/index';
  import { trapFocus } from '$lib/utils/a11y';

  interface Props {
    content: string;
    visible: boolean;
    onClose: () => void;
    adapter?: Adapter;
  }

  const { content, visible, onClose, adapter }: Props = $props();

  const markedInstance = new Marked(
    markedHighlight({
      langPrefix: 'hljs language-',
      highlight(code: string, lang: string) {
        if (lang && hljs.getLanguage(lang)) {
          return hljs.highlight(code, { language: lang }).value;
        }
        return hljs.highlightAuto(code).value;
      },
    }),
    { gfm: true, breaks: true }
  );

  const SANITIZE_CONFIG = {
    ALLOWED_TAGS: [
      'p', 'br', 'strong', 'em', 'b', 'i', 'u', 'del', 's',
      'a', 'code', 'pre', 'span',
      'ul', 'ol', 'li',
      'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
      'blockquote', 'hr', 'img',
      'table', 'thead', 'tbody', 'tr', 'th', 'td',
      'div', 'sub', 'sup', 'mark', 'abbr',
    ],
    ALLOWED_ATTR: ['href', 'src', 'alt', 'title', 'class', 'id', 'target', 'rel', 'colspan', 'rowspan'],
    ALLOW_DATA_ATTR: false,
  };

  const rendered = $derived(DOMPurify.sanitize(markedInstance.parse(content) as string, SANITIZE_CONFIG));

  let panelEl = $state<HTMLElement | null>(null);
  let destroyTrap: (() => void) | null = null;

  $effect(() => {
    if (visible && panelEl) {
      destroyTrap = trapFocus(panelEl);
    } else {
      destroyTrap?.();
      destroyTrap = null;
    }
    return () => {
      destroyTrap?.();
      destroyTrap = null;
    };
  });

  function handlePanelKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="response-panel" bind:this={panelEl} role="dialog" tabindex="-1" aria-modal="true" aria-label={$tr('response.title')} onkeydown={handlePanelKeydown}>
    <div class="response-header">
      <span class="response-title">{$tr('response.title')}</span>
      <button class="close-btn" onclick={onClose} title={$tr('response.close')} aria-label={$tr('response.close')}>✕</button>
    </div>
    <div class="response-body">
      <div class="markdown-preview">
        {@html rendered}
      </div>
    </div>
    <AnalyticsBar
      {adapter}
      onOpenDashboard={() => analyticsDashboard.set({ open: true, focus: null })}
    />
  </div>
{/if}

<style>
  .response-panel {
    position: absolute;
    top: 0;
    inset-inline-end: 0;
    width: 420px;
    height: 100%;
    background: var(--bg-secondary);
    border-inline-start: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    z-index: var(--layer-raised);
    box-shadow: var(--shadow-overlay);
  }

  .response-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
    background: var(--bg-primary);
  }

  .response-title {
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
    cursor: pointer;
    font-size: var(--font-size-base);
    padding: var(--space-1) var(--space-1);
    border-radius: var(--radius);
    line-height: 1;
    min-width: 32px;
    min-height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color var(--transition-fast), background var(--transition-fast);
  }

  .close-btn:hover {
    color: var(--text-primary);
    background: var(--highlight-hover);
  }

  .response-body {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  .markdown-preview {
    padding: var(--space-5) var(--space-5);
    color: var(--text-primary);
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
    line-height: var(--line-height-base);
    max-width: var(--measure);
  }

  .markdown-preview :global(h1),
  .markdown-preview :global(h2),
  .markdown-preview :global(h3),
  .markdown-preview :global(h4),
  .markdown-preview :global(h5),
  .markdown-preview :global(h6) {
    color: var(--text-primary);
    margin-top: 1.25em;
    margin-bottom: 0.4em;
    font-weight: 600;
    line-height: 1.3;
  }

  .markdown-preview :global(h1) { font-size: 1.6em; border-bottom: 1px solid var(--border); padding-bottom: 0.25em; }
  .markdown-preview :global(h2) { font-size: 1.3em; border-bottom: 1px solid var(--border); padding-bottom: 0.2em; }
  .markdown-preview :global(h3) { font-size: var(--font-size-md); line-height: var(--line-height-md); }

  .markdown-preview :global(p) { margin: 0.65em 0; }
  .markdown-preview :global(p + p) { margin-top: var(--paragraph-spacing); }

  .markdown-preview :global(a) { color: var(--accent-text); text-decoration: underline; }
  .markdown-preview :global(a:hover) { text-decoration: underline; }

  .markdown-preview :global(code) {
    background: var(--bg-primary);
    color: var(--code-accent);
    padding: 0.15em 0.4em;
    border-radius: var(--radius);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }

  .markdown-preview :global(pre) {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-3);
    overflow: auto;
    margin: 0.75em 0;
  }

  .markdown-preview :global(pre code) {
    background: transparent;
    color: inherit;
    padding: 0;
    border-radius: var(--radius);
    font-size: var(--font-size-sm);
  }

  .markdown-preview :global(blockquote) {
    border-inline-start: 3px solid var(--accent);
    margin: 0.75em 0;
    padding: 0.4em 0.9em;
    background: var(--bg-primary);
    border-radius: 0 4px 4px 0;
    color: var(--text-secondary);
  }

  .markdown-preview :global(blockquote p) { margin: 0; }

  .markdown-preview :global(ul),
  .markdown-preview :global(ol) { padding-inline-start: 1.4em; margin: 0.6em 0; }

  .markdown-preview :global(li) { margin: 0.25em 0; }

  .markdown-preview :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 0.75em 0;
    font-size: var(--font-size-sm);
  }

  .markdown-preview :global(th),
  .markdown-preview :global(td) {
    border: 1px solid var(--border);
    padding: var(--space-1) var(--space-2);
    text-align: start;
  }

  .markdown-preview :global(th) {
    background: var(--bg-primary);
    font-weight: 600;
  }

  .markdown-preview :global(hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 1.5em 0;
  }

  .markdown-preview :global(img) {
    max-width: 100%;
    border-radius: 4px;
  }
</style>
