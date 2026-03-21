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

<div class="app-root">
  <Toolbar {adapter} />

  <div class="main-content">
    <div class="panel file-tree" style="width: {$fileTreeWidth}px">
      {#if fileTree}
        {@render fileTree()}
      {:else}
        <p class="placeholder">File Tree</p>
      {/if}
    </div>

    <div class="divider" onmousedown={() => (draggingLeft = true)} role="separator" aria-label="Resize file tree">
      <span class="divider-handle" aria-hidden="true">···</span>
    </div>

    <div class="panel editor">
      {#if editor}
        {@render editor()}
      {:else}
        <p class="placeholder">Editor</p>
      {/if}
    </div>

    <div class="divider" onmousedown={() => (draggingRight = true)} role="separator" aria-label="Resize terminal">
      <span class="divider-handle" aria-hidden="true">···</span>
    </div>

    <div class="panel terminal" style="width: {$terminalWidth}px">
      {#if terminal}
        {@render terminal()}
      {:else}
        <p class="placeholder">Terminal</p>
      {/if}
    </div>
  </div>

  <StatusBar />
</div>

<style>
  .app-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
    color: var(--text-primary);
    overflow: hidden;
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
</style>
