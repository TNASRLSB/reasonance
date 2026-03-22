<script lang="ts">
  import { Marked } from 'marked';
  import { markedHighlight } from 'marked-highlight';
  import hljs from 'highlight.js';
  import 'highlight.js/styles/github-dark.css';

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

  const rendered = $derived(markedInstance.parse(content) as string);
</script>

<div class="markdown-preview">
  {@html rendered}
</div>

<style>
  .markdown-preview {
    flex: 1;
    overflow: auto;
    padding: 24px 32px;
    color: var(--text-primary, #e2e8f0);
    font-family: var(--font-ui);
    font-size: 15px;
    line-height: 1.7;
    max-width: 860px;
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
    font-size: 2em;
    border-bottom: 1px solid var(--border, #334155);
    padding-bottom: 0.3em;
  }

  .markdown-preview :global(h2) {
    font-size: 1.5em;
    border-bottom: 1px solid var(--border, #334155);
    padding-bottom: 0.2em;
  }

  .markdown-preview :global(h3) {
    font-size: 1.25em;
  }

  .markdown-preview :global(p) {
    margin: 0.75em 0;
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
    color: #e879f9;
    padding: 0.15em 0.4em;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.875em;
  }

  .markdown-preview :global(pre) {
    background: #0d1117;
    border: 1px solid var(--border, #334155);
    border-radius: 8px;
    padding: 16px;
    overflow: auto;
    margin: 1em 0;
  }

  .markdown-preview :global(pre code) {
    background: transparent;
    color: inherit;
    padding: 0;
    border-radius: 0;
    font-size: 0.875em;
  }

  .markdown-preview :global(blockquote) {
    border-left: 4px solid var(--accent, #6366f1);
    margin: 1em 0;
    padding: 0.5em 1em;
    background: var(--bg-secondary, #1e293b);
    border-radius: 0 6px 6px 0;
    color: var(--text-secondary, #94a3b8);
  }

  .markdown-preview :global(blockquote p) {
    margin: 0;
  }

  .markdown-preview :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 1em 0;
    font-size: 0.9em;
  }

  .markdown-preview :global(th),
  .markdown-preview :global(td) {
    border: 1px solid var(--border, #334155);
    padding: 8px 12px;
    text-align: left;
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
    padding-left: 1.5em;
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
    border-radius: 6px;
  }
</style>
