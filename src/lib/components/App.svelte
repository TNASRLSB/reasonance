<script lang="ts">
  import { onMount } from 'svelte';
  import Toolbar from './Toolbar.svelte';
  import StatusBar from './StatusBar.svelte';
  import AnalyticsDashboard from './AnalyticsDashboard.svelte';
  import { get } from 'svelte/store';
  import { fileTreeWidth, terminalWidth, analyticsDashboard } from '$lib/stores/ui';
  import { startUpdateChecker } from '$lib/updater';
  import { startLiveTracking } from '$lib/stores/analytics';
  import { llmConfigs, appSettings } from '$lib/stores/config';
  import type { Snippet } from 'svelte';
  import type { Adapter } from '$lib/adapter/index';

  let { adapter, fileTree, editor, terminal }: {
    adapter: Adapter;
    fileTree?: Snippet;
    editor?: Snippet;
    terminal?: Snippet;
  } = $props();

  let draggingLeft = $state(false);
  let draggingRight = $state(false);

  onMount(() => {
    startUpdateChecker();
    const stopTracking = startLiveTracking(adapter);
    return () => stopTracking();
  });

  function onMouseMove(e: MouseEvent) {
    if (draggingLeft) {
      const maxTree = window.innerWidth * 0.3;
      fileTreeWidth.set(Math.max(120, Math.min(maxTree, e.clientX)));
    }
    if (draggingRight) {
      const w = window.innerWidth - e.clientX;
      const maxTerm = window.innerWidth * 0.5;
      terminalWidth.set(Math.max(250, Math.min(maxTerm, w)));
    }
  }

  function onMouseUp() {
    draggingLeft = false;
    draggingRight = false;
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    // Ctrl+Shift+A → toggle analytics dashboard
    if (e.ctrlKey && e.shiftKey && e.key === 'A') {
      e.preventDefault();
      analyticsDashboard.update(v => ({ ...v, open: !v.open, focus: null }));
      return;
    }

    // Ctrl+1..9 → switch active provider
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key >= '1' && e.key <= '9') {
      const index = parseInt(e.key) - 1;
      const configs = get(llmConfigs);
      if (configs && index < configs.length) {
        e.preventDefault();
        appSettings.set({ default: configs[index].name });
      }
    }
  }

  function onDividerKeydown(e: KeyboardEvent, which: 'left' | 'right') {
    const step = e.shiftKey ? 50 : 10;
    if (which === 'left') {
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        fileTreeWidth.update(w => Math.max(120, w - step));
      } else if (e.key === 'ArrowRight') {
        e.preventDefault();
        const max = window.innerWidth * 0.3;
        fileTreeWidth.update(w => Math.min(max, w + step));
      }
    } else {
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        const max = window.innerWidth * 0.5;
        terminalWidth.update(w => Math.min(max, w + step));
      } else if (e.key === 'ArrowRight') {
        e.preventDefault();
        terminalWidth.update(w => Math.max(250, w - step));
      }
    }
  }
</script>

<svelte:window onmousemove={onMouseMove} onmouseup={onMouseUp} onkeydown={handleGlobalKeydown} />

<div class="app-root" class:resizing={draggingLeft || draggingRight}>
  {#if draggingLeft || draggingRight}
    <div class="resize-overlay"></div>
  {/if}
  <!-- Skip links (visually hidden, visible on focus) -->
  <div class="skip-links">
    <a class="skip-link" href="#file-tree">Skip to file tree</a>
    <a class="skip-link" href="#editor">Skip to editor</a>
    <a class="skip-link" href="#terminal">Skip to terminal</a>
  </div>

  <header>
    <Toolbar {adapter} />
  </header>

  <div class="main-content" data-main-content>
    <nav id="file-tree" aria-label="File explorer" class="panel file-tree" style="width: {$fileTreeWidth}px">
      <svelte:boundary>
        {#if fileTree}
          {@render fileTree()}
        {:else}
          <p class="placeholder">File Tree</p>
        {/if}
        {#snippet failed(error, reset)}
          <div class="panel-error">
            <p class="error-title">FILE TREE ERROR</p>
            <p class="error-msg">{(error as any)?.message ?? 'Unknown error'}</p>
            <button class="error-retry" onclick={reset}>RETRY</button>
          </div>
        {/snippet}
      </svelte:boundary>
    </nav>

    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="divider" onmousedown={() => (draggingLeft = true)} role="separator" aria-label="Resize file tree" tabindex="0" onkeydown={(e) => onDividerKeydown(e, 'left')}>
      <span class="divider-handle" aria-hidden="true">···</span>
    </div>

    <main id="editor" class="panel editor">
      <svelte:boundary>
        {#if $analyticsDashboard.open}
          <AnalyticsDashboard {adapter} />
        {:else if editor}
          {@render editor()}
        {:else}
          <p class="placeholder">Editor</p>
        {/if}
        {#snippet failed(error, reset)}
          <div class="panel-error">
            <p class="error-title">EDITOR ERROR</p>
            <p class="error-msg">{(error as any)?.message ?? 'Unknown error'}</p>
            <button class="error-retry" onclick={reset}>RETRY</button>
          </div>
        {/snippet}
      </svelte:boundary>
    </main>

    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="divider" onmousedown={() => (draggingRight = true)} role="separator" aria-label="Resize terminal" tabindex="0" onkeydown={(e) => onDividerKeydown(e, 'right')}>
      <span class="divider-handle" aria-hidden="true">···</span>
    </div>

    <aside id="terminal" aria-label="Terminal" class="panel terminal" style="width: {$terminalWidth}px">
      <svelte:boundary>
        {#if terminal}
          {@render terminal()}
        {:else}
          <p class="placeholder">Terminal</p>
        {/if}
        {#snippet failed(error, reset)}
          <div class="panel-error">
            <p class="error-title">TERMINAL ERROR</p>
            <p class="error-msg">{(error as any)?.message ?? 'Unknown error'}</p>
            <button class="error-retry" onclick={reset}>RETRY</button>
          </div>
        {/snippet}
      </svelte:boundary>
    </aside>
  </div>

  <footer>
    <StatusBar />
  </footer>
</div>

<style>
  .skip-links {
    position: fixed;
    top: 0;
    left: 0;
    z-index: var(--layer-top);
    pointer-events: none;
  }

  .skip-link {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
    background: var(--bg-primary);
    color: var(--accent-text);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-decoration: none;
  }

  .skip-link:focus {
    position: fixed;
    top: 4px;
    left: 4px;
    width: auto;
    height: auto;
    padding: var(--space-2) var(--space-4);
    margin: 0;
    overflow: visible;
    clip: auto;
    border: 2px solid var(--accent);
    outline: var(--focus-ring);
    outline-offset: var(--focus-offset);
    z-index: calc(var(--layer-top) + 1);
    pointer-events: auto;
  }

  header,
  footer {
    flex-shrink: 0;
  }

  .app-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
    color: var(--text-primary);
    overflow: hidden;
  }

  .app-root.resizing {
    cursor: col-resize;
  }

  .resize-overlay {
    position: fixed;
    inset: 0;
    z-index: calc(var(--layer-top) - 1);
    cursor: col-resize;
  }

  .main-content {
    display: flex;
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }

  .panel {
    overflow: hidden;
    background: var(--bg-primary);
    display: flex;
    flex-direction: column;
  }

  .panel.file-tree {
    flex-shrink: 0;
    background: var(--bg-surface);
    border-right: none;
    contain: layout style;
  }

  .panel.editor {
    flex: 1;
    min-width: 0;
    contain: layout style;
  }

  .panel.terminal {
    flex-shrink: 0;
    background: var(--bg-surface);
    border-left: none;
    contain: style;
  }

  .divider {
    width: 6px;
    background: var(--bg-tertiary);
    cursor: col-resize;
    flex-shrink: 0;
    transition: background var(--transition-fast);
    position: relative;
    z-index: var(--layer-raised);
    display: flex;
    align-items: center;
    justify-content: center;
    border-inline-start: 1px solid var(--border);
    border-inline-end: 1px solid var(--border);
  }

  .divider::before {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: -4px;
    right: -4px;
    z-index: calc(var(--layer-raised) + 1);
    cursor: col-resize;
  }

  .divider:hover {
    background: var(--accent);
    border-color: var(--accent);
  }

  .divider-handle {
    writing-mode: vertical-lr;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    pointer-events: none;
    user-select: none;
    letter-spacing: 3px;
  }

  .divider:hover .divider-handle {
    color: var(--text-primary);
  }

  .placeholder {
    color: var(--text-secondary);
    font-size: var(--font-size-small);
    padding: var(--panel-padding);
    text-align: center;
    margin-top: auto;
    margin-bottom: auto;
    align-self: center;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .panel-error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--stack-normal);
    flex: 1;
    padding: var(--space-5);
    font-family: var(--font-ui);
  }

  .error-title {
    font-size: var(--font-size-small);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--danger-text);
    margin: 0;
  }

  .error-msg {
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin: 0;
    font-family: var(--font-mono);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    word-break: break-all;
  }

  .error-retry {
    background: var(--bg-tertiary);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-primary);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: var(--space-1) var(--space-4);
    cursor: pointer;
  }

  .error-retry:hover {
    background: var(--accent-btn);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }
</style>
