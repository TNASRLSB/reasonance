<script lang="ts">
  import Toolbar from './Toolbar.svelte';
  import StatusBar from './StatusBar.svelte';
  import { fileTreeWidth, terminalWidth } from '$lib/stores/ui';
  import type { Snippet } from 'svelte';

  let { fileTree, editor, terminal }: {
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
  <Toolbar />

  <div class="main-content">
    <div class="panel file-tree" style="width: {$fileTreeWidth}px">
      {#if fileTree}
        {@render fileTree()}
      {:else}
        <p class="placeholder">File Tree</p>
      {/if}
    </div>

    <div class="divider" onmousedown={() => (draggingLeft = true)} role="separator"></div>

    <div class="panel editor">
      {#if editor}
        {@render editor()}
      {:else}
        <p class="placeholder">Editor</p>
      {/if}
    </div>

    <div class="divider" onmousedown={() => (draggingRight = true)} role="separator"></div>

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
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
  }

  .panel.editor {
    flex: 1;
    min-width: 0;
  }

  .panel.terminal {
    flex-shrink: 0;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
  }

  .divider {
    width: 4px;
    background: var(--border);
    cursor: col-resize;
    flex-shrink: 0;
    transition: background 0.15s;
    position: relative;
    z-index: 10;
  }

  .divider:hover {
    background: var(--accent);
  }

  .placeholder {
    color: var(--text-secondary);
    font-size: 12px;
    padding: 16px;
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
