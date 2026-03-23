<script lang="ts">
  import { Marked } from 'marked';
  import DOMPurify from 'dompurify';

  let { text }: { text: string } = $props();

  // Isolated marked instance — avoids global state mutation
  const md = new Marked({ breaks: true, gfm: true, async: false });

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

  let html = $derived(DOMPurify.sanitize(md.parse(text) as string, SANITIZE_CONFIG));
</script>

<div class="text-block markdown-content">
  {@html html}
</div>

<style>
  .text-block {
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
    color: var(--text-body);
    line-height: 1.6;
    word-break: break-word;
  }

  .text-block :global(p) {
    margin: 0 0 8px 0;
  }

  .text-block :global(p:last-child) {
    margin-bottom: 0;
  }

  .text-block :global(code) {
    font-family: var(--font-mono);
    font-size: var(--font-size-code);
    background: var(--bg-tertiary);
    padding: 1px 4px;
    border: var(--border-width) solid var(--border);
  }

  .text-block :global(pre) {
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    padding: 12px;
    overflow-x: auto;
    margin: 8px 0;
  }

  .text-block :global(pre code) {
    background: none;
    padding: 0;
    border: none;
  }

  .text-block :global(a) {
    color: var(--accent-text);
    text-decoration: underline;
  }

  .text-block :global(a:hover) {
    color: var(--accent);
  }

  .text-block :global(blockquote) {
    border-inline-start: 4px solid var(--border);
    margin: 8px 0;
    padding: 4px 12px;
    color: var(--text-secondary);
  }

  .text-block :global(ul),
  .text-block :global(ol) {
    margin: 4px 0;
    padding-inline-start: 24px;
  }

  .text-block :global(table) {
    border-collapse: collapse;
    width: 100%;
    margin: 8px 0;
  }

  .text-block :global(th),
  .text-block :global(td) {
    border: var(--border-width) solid var(--border);
    padding: 4px 8px;
    text-align: start;
  }

  .text-block :global(th) {
    background: var(--bg-tertiary);
    font-weight: 700;
  }

  .text-block :global(h1),
  .text-block :global(h2),
  .text-block :global(h3),
  .text-block :global(h4) {
    font-family: var(--font-ui);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin: 12px 0 4px 0;
    color: var(--text-primary);
  }

  .text-block :global(h1) { font-size: 18px; }
  .text-block :global(h2) { font-size: 16px; }
  .text-block :global(h3) { font-size: 14px; }
  .text-block :global(h4) { font-size: 12px; }

  .text-block :global(hr) {
    border: none;
    border-top: var(--border-width) solid var(--border);
    margin: 12px 0;
  }
</style>
