<script lang="ts">
  let { language = '', source = '' }: { language?: string; source?: string } = $props();

  let copied = $state(false);

  function copyCode() {
    navigator.clipboard.writeText(source).then(() => {
      copied = true;
      setTimeout(() => { copied = false; }, 2000);
    });
  }
</script>

<div class="code-block">
  <div class="code-header">
    <span class="code-lang">{language || 'code'}</span>
    <button class="copy-btn" onclick={copyCode} aria-label="Copy code">
      {copied ? 'Copied' : 'Copy'}
    </button>
  </div>
  <pre class="code-content"><code>{source}</code></pre>
</div>

<style>
  .code-block {
    border: var(--border-width) solid var(--border);
    background: var(--bg-primary);
    overflow: hidden;
  }

  .code-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-1) var(--space-3);
    background: var(--bg-tertiary);
    border-bottom: var(--border-width) solid var(--border);
  }

  .code-lang {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 700;
  }

  .copy-btn {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    background: transparent;
    border: var(--border-width) solid var(--border);
    padding: var(--stack-tight) var(--space-2);
    cursor: pointer;
  }

  .copy-btn:hover {
    color: var(--text-primary);
    border-color: var(--accent);
  }

  .code-content {
    margin: 0;
    padding: var(--inset-component);
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: var(--font-size-code);
    color: var(--text-body);
    line-height: 1.5;
  }

  .code-content code {
    font-family: inherit;
  }
</style>
