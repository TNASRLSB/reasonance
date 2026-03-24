<script lang="ts">
  import { get } from 'svelte/store';
  import { activeTheme } from '$lib/stores/theme';
  import type { ThemeFile } from '$lib/engine/theme-types';
  import { FALLBACK_THEME } from '$lib/engine/fallback-theme';

  let {
    open,
    onSelect,
    onClose,
  }: {
    open: boolean;
    onSelect: (theme: ThemeFile, name: string) => void;
    onClose: () => void;
  } = $props();

  const EMPTY_THEME: ThemeFile = {
    meta: {
      name: 'New Theme',
      type: 'theme',
      colorScheme: 'dark',
      schemaVersion: 1,
    },
    colors: {},
    hues: {},
    states: {},
    'ui-states': {},
    typography: {},
    spacing: {},
    borders: {},
    focus: {},
    transitions: {},
    layout: {},
    layers: {},
  };

  async function selectBuiltin(name: string) {
    try {
      const loader =
        name === 'reasonance-dark'
          ? () => import('$lib/themes/reasonance-dark.json').then((m) => m.default as ThemeFile)
          : () => import('$lib/themes/reasonance-light.json').then((m) => m.default as ThemeFile);
      const theme = await loader();
      onSelect(structuredClone(theme), name);
    } catch {
      onClose();
    }
  }

  function selectEmpty() {
    onSelect(structuredClone(EMPTY_THEME), 'new-theme');
  }

  function cloneCurrent() {
    const current = get(activeTheme);
    onSelect(structuredClone(current), 'custom-theme');
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={onClose} onkeydown={handleKeydown} role="presentation">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Choose starting theme"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="dialog-header">
        <h2 class="dialog-title">Start From...</h2>
        <button class="close-btn" onclick={onClose} aria-label="Close dialog">✕</button>
      </div>

      <div class="options">
        <button class="option-card" onclick={() => selectBuiltin('reasonance-dark')}>
          <span class="option-swatch dark-swatch"></span>
          <div class="option-info">
            <span class="option-name">Reasonance Dark</span>
            <span class="option-desc">Built-in dark theme</span>
          </div>
        </button>

        <button class="option-card" onclick={() => selectBuiltin('reasonance-light')}>
          <span class="option-swatch light-swatch"></span>
          <div class="option-info">
            <span class="option-name">Reasonance Light</span>
            <span class="option-desc">Built-in light theme</span>
          </div>
        </button>

        <button class="option-card" onclick={cloneCurrent}>
          <span class="option-swatch clone-swatch">⎘</span>
          <div class="option-info">
            <span class="option-name">Clone from Current</span>
            <span class="option-desc">Start with the active theme</span>
          </div>
        </button>

        <button class="option-card" onclick={selectEmpty}>
          <span class="option-swatch empty-swatch">+</span>
          <div class="option-info">
            <span class="option-name">Empty Theme</span>
            <span class="option-desc">Blank slate with all sections</span>
          </div>
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--bg-secondary, #1a1a1a);
    border: 1px solid var(--border, #2a2a2a);
    width: 400px;
    max-width: 95vw;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4, 16px);
    border-bottom: 1px solid var(--border, #2a2a2a);
  }

  .dialog-title {
    font-size: var(--font-size-base, 14px);
    font-weight: 700;
    color: var(--text-primary, #eee);
    margin: 0;
    font-family: var(--font-ui, sans-serif);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #888);
    cursor: pointer;
    font-size: 14px;
    padding: 4px;
    line-height: 1;
  }

  .close-btn:hover {
    color: var(--text-primary, #eee);
  }

  .options {
    display: flex;
    flex-direction: column;
    padding: var(--space-3, 12px);
    gap: var(--space-2, 8px);
  }

  .option-card {
    display: flex;
    align-items: center;
    gap: var(--space-3, 12px);
    background: var(--bg-tertiary, #1e1e1e);
    border: 1px solid var(--border, #333);
    color: var(--text-primary, #eee);
    padding: var(--space-3, 12px);
    cursor: pointer;
    text-align: left;
    transition: border-color var(--transition-fast, 100ms), background var(--transition-fast, 100ms);
  }

  .option-card:hover {
    border-color: var(--accent, #4a9eff);
    background: var(--bg-hover, #252525);
  }

  .option-swatch {
    width: 40px;
    height: 40px;
    border-radius: 4px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 18px;
    font-weight: bold;
    border: 1px solid var(--border, #444);
  }

  .dark-swatch {
    background: #0d0d0d;
  }

  .light-swatch {
    background: #f5f5f5;
  }

  .clone-swatch {
    background: var(--bg-surface, #141414);
    color: var(--text-secondary, #aaa);
  }

  .empty-swatch {
    background: var(--bg-surface, #141414);
    color: var(--accent-text, #7ac7ff);
    border-color: var(--accent, #4a9eff);
  }

  .option-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .option-name {
    font-size: var(--font-size-base, 14px);
    font-weight: 600;
    font-family: var(--font-ui, sans-serif);
  }

  .option-desc {
    font-size: 12px;
    color: var(--text-muted, #888);
    font-family: var(--font-ui, sans-serif);
  }
</style>
