<script lang="ts">
  import MarkdownPreview from './MarkdownPreview.svelte';
  import { locale } from '$lib/i18n/index';
  import { onDestroy } from 'svelte';

  let content = $state('');

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

  const unsubLocale = locale.subscribe((loc) => {
    loadDocs(loc);
  });

  onDestroy(() => unsubLocale());
</script>

<div class="help-panel">
  <div class="help-content">
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

  .help-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px 32px;
  }
</style>
