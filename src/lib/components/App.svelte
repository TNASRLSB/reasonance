<script lang="ts">
  import Toolbar from './Toolbar.svelte';
  import StatusBar from './StatusBar.svelte';
  import { fileTreeWidth, terminalWidth } from '$lib/stores/ui';
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

  function onMouseMove(e: MouseEvent) {
    if (draggingLeft) {
      fileTreeWidth.set(Math.max(150, Math.min(500, e.clientX)));
    }
    if (draggingRight) {
      terminalWidth.set(Math.max(300, Math.min(800, window.innerWidth - e.clientX)));
    }
  }

  function onMouseUp() {
    draggingLeft = false;
    draggingRight = false;
  }
</script>

<svelte:window onmousemove={onMouseMove} onmouseup={onMouseUp} />

<div class="app-root" class:resizing={draggingLeft || draggingRight}>
  <!-- Skip links (visually hidden, visible on focus) -->
  <div class="skip-links">
    <a class="skip-link" href="#file-tree">Skip to file tree</a>
    <a class="skip-link" href="#editor">Skip to editor</a>
    <a class="skip-link" href="#terminal">Skip to terminal</a>
  </div>

  <header>
    <Toolbar {adapter} />
  </header>

  <div class="main-content">
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
            <p class="error-msg">{error?.message ?? 'Unknown error'}</p>
            <button class="error-retry" onclick={reset}>RETRY</button>
          </div>
        {/snippet}
      </svelte:boundary>
    </nav>

    <div class="divider" onmousedown={() => (draggingLeft = true)} role="separator" aria-label="Resize file tree">
      <span class="divider-handle" aria-hidden="true">···</span>
    </div>

    <main id="editor" class="panel editor">
      <svelte:boundary>
        {#if editor}
          {@render editor()}
        {:else}
          <p class="placeholder">Editor</p>
        {/if}
        {#snippet failed(error, reset)}
          <div class="panel-error">
            <p class="error-title">EDITOR ERROR</p>
            <p class="error-msg">{error?.message ?? 'Unknown error'}</p>
            <button class="error-retry" onclick={reset}>RETRY</button>
          </div>
        {/snippet}
      </svelte:boundary>
    </main>

    <div class="divider" onmousedown={() => (draggingRight = true)} role="separator" aria-label="Resize terminal">
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
            <p class="error-msg">{error?.message ?? 'Unknown error'}</p>
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
    position: absolute;
    top: 0;
    left: 0;
    z-index: 9999;
  }

  .skip-link {
    position: absolute;
    top: -100%;
    left: 0;
    background: var(--bg-primary);
    color: var(--accent-text);
    border: 2px solid var(--accent);
    padding: 8px 16px;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-decoration: none;
    white-space: nowrap;
    transition: top 0.1s;
  }

  .skip-link:focus {
    top: 0;
    outline: var(--focus-ring);
    outline-offset: var(--focus-offset);
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
    user-select: none;
  }

  .app-root.resizing :global(*) {
    user-select: none !important;
    pointer-events: none !important;
  }

  .app-root.resizing .divider {
    pointer-events: auto !important;
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
  }

  .panel.editor {
    flex: 1;
    min-width: 0;
  }

  .panel.terminal {
    flex-shrink: 0;
    background: var(--bg-surface);
    border-left: none;
  }

  .divider {
    width: 6px;
    background: var(--bg-tertiary);
    cursor: col-resize;
    flex-shrink: 0;
    transition: background 0.15s;
    position: relative;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    border-left: 1px solid var(--border);
    border-right: 1px solid var(--border);
  }

  .divider::before {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: -4px;
    right: -4px;
    z-index: 11;
    cursor: col-resize;
  }

  .divider:hover {
    background: var(--accent);
    border-color: var(--accent);
  }

  .divider-handle {
    writing-mode: vertical-lr;
    font-size: 10px;
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
    gap: 8px;
    flex: 1;
    padding: 24px;
    font-family: var(--font-ui);
  }

  .error-title {
    font-size: var(--font-size-small);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--danger);
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
    padding: 4px 16px;
    cursor: pointer;
  }

  .error-retry:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
</style>
