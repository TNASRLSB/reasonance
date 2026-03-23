<script lang="ts">
  import { Marked } from 'marked';
  import { markedHighlight } from 'marked-highlight';
  import hljs from 'highlight.js';
  import 'highlight.js/styles/github-dark.css';
  import DOMPurify from 'dompurify';

  const { content }: { content: string } = $props();

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
</script>

<div class="markdown-preview">
  {@html rendered}
</div>

<style>
  .markdown-preview {
    flex: 1;
    overflow: auto;
    padding: var(--space-5) var(--space-6);
    color: var(--text-primary, #e2e8f0);
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
    line-height: var(--line-height-base);
    max-width: var(--measure);
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }

  .markdown-preview :global(h1),
  .markdown-preview :global(h2),
  .markdown-preview :global(h3),
  .markdown-preview :global(h4),
  .markdown-preview :global(h5),
  .markdown-preview :global(h6) {
    color: var(--text-primary, #f1f5f9);
    margin-top: 1.5em;
    margin-bottom: 0.5em;
    font-weight: 600;
    line-height: 1.3;
  }

  .markdown-preview :global(h1) {
    font-size: var(--font-size-hero);
    line-height: var(--line-height-hero);
    font-weight: var(--font-weight-hero);
    border-bottom: 1px solid var(--border, #334155);
    padding-bottom: 0.3em;
  }

  .markdown-preview :global(h2) {
    font-size: var(--font-size-lg);
    line-height: var(--line-height-lg);
    font-weight: var(--font-weight-lg);
    border-bottom: 1px solid var(--border, #334155);
    padding-bottom: 0.2em;
  }

  .markdown-preview :global(h3) {
    font-size: var(--font-size-md);
    line-height: var(--line-height-md);
    font-weight: var(--font-weight-md);
  }

  .markdown-preview :global(p) {
    margin: 0.75em 0;
  }

  .markdown-preview :global(p + p) {
    margin-top: var(--paragraph-spacing);
  }

  .markdown-preview :global(a) {
    color: var(--accent, #6366f1);
    text-decoration: none;
  }

  .markdown-preview :global(a:hover) {
    text-decoration: underline;
  }

  .markdown-preview :global(code) {
    background: var(--bg-secondary, #1e293b);
    color: var(--code-accent);
    padding: 0.15em 0.4em;
    border-radius: 0;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }

  .markdown-preview :global(pre) {
    background: var(--code-bg);
    border: 1px solid var(--border, #334155);
    border-radius: 0;
    padding: var(--inset-section);
    overflow: auto;
    margin: 1em 0;
  }

  .markdown-preview :global(pre code) {
    background: transparent;
    color: inherit;
    padding: 0;
    border-radius: 0;
    font-size: var(--font-size-sm);
  }

  .markdown-preview :global(blockquote) {
    border-inline-start: 4px solid var(--accent, #6366f1);
    margin: 1em 0;
    padding: 0.5em 1em;
    background: var(--bg-secondary, #1e293b);
    border-radius: 0;
    color: var(--text-secondary, #94a3b8);
  }

  .markdown-preview :global(blockquote p) {
    margin: 0;
  }

  .markdown-preview :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 1em 0;
    font-size: var(--font-size-sm);
  }

  .markdown-preview :global(th),
  .markdown-preview :global(td) {
    border: 1px solid var(--border, #334155);
    padding: var(--space-2) var(--space-3);
    text-align: start;
  }

  .markdown-preview :global(th) {
    background: var(--bg-secondary, #1e293b);
    color: var(--text-primary, #f1f5f9);
    font-weight: 600;
  }

  .markdown-preview :global(tr:nth-child(even)) {
    background: rgba(255, 255, 255, 0.02);
  }

  .markdown-preview :global(ul),
  .markdown-preview :global(ol) {
    padding-inline-start: 1.5em;
    margin: 0.75em 0;
  }

  .markdown-preview :global(li) {
    margin: 0.3em 0;
  }

  .markdown-preview :global(hr) {
    border: none;
    border-top: 1px solid var(--border, #334155);
    margin: 2em 0;
  }

  .markdown-preview :global(img) {
    max-width: 100%;
    border-radius: 0;
  }
</style>
