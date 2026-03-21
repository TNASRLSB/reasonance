<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { MergeView } from '@codemirror/merge';
  import { EditorState } from '@codemirror/state';
  import { EditorView, basicSetup } from 'codemirror';
  import { oneDark } from '@codemirror/theme-one-dark';
  import type { Adapter } from '$lib/adapter';

  let {
    original,
    modified,
    filename,
    adapter,
    filePath,
    onAccept,
    onReject,
  }: {
    original: string;
    modified: string;
    filename: string;
    adapter: Adapter;
    filePath: string;
    onAccept: () => void;
    onReject: () => void;
  } = $props();

  let container: HTMLDivElement;
  let mergeView: MergeView | null = null;

  function buildEditorState(doc: string) {
    return EditorState.create({
      doc,
      extensions: [
        basicSetup,
        oneDark,
        EditorView.editable.of(false),
        EditorState.readOnly.of(true),
      ],
    });
  }

  onMount(() => {
    if (!container) return;
    mergeView = new MergeView({
      a: buildEditorState(original),
      b: buildEditorState(modified),
      parent: container,
    });
  });

  onDestroy(() => {
    if (mergeView) {
      mergeView.destroy();
      mergeView = null;
    }
  });

  async function handleAccept() {
    // Update shadow to new content so next watch cycle won't re-trigger
    await adapter.storeShadow(filePath, modified);
    onAccept();
  }

  async function handleReject() {
    // Write shadow content back to file, reverting the external change
    await adapter.writeFile(filePath, original);
    onReject();
  }
</script>

<div class="diff-wrapper">
  <div class="diff-toolbar">
    <span class="diff-title">Modifiche: {filename}</span>
    <div class="diff-actions">
      <button class="btn-reject" onclick={handleReject}>Rifiuta</button>
      <button class="btn-accept" onclick={handleAccept}>Accetta</button>
    </div>
  </div>
  <div class="diff-container" bind:this={container}></div>
</div>

<style>
  .diff-wrapper {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .diff-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    font-size: 12px;
  }

  .diff-title {
    color: var(--text-secondary);
    font-family: var(--font-mono, monospace);
    font-weight: 500;
  }

  .diff-actions {
    display: flex;
    gap: 8px;
  }

  .btn-accept,
  .btn-reject {
    padding: 3px 12px;
    border-radius: 4px;
    border: 1px solid transparent;
    font-size: 12px;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .btn-accept {
    background: #22c55e;
    color: #fff;
    border-color: #16a34a;
  }

  .btn-accept:hover {
    opacity: 0.85;
  }

  .btn-reject {
    background: var(--bg-primary);
    color: var(--text-secondary);
    border-color: var(--border);
  }

  .btn-reject:hover {
    color: var(--text-primary);
    border-color: #ef4444;
  }

  .diff-container {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  .diff-container :global(.cm-mergeView) {
    height: 100%;
  }

  .diff-container :global(.cm-mergeViewEditor) {
    height: 100%;
  }

  .diff-container :global(.cm-editor) {
    height: 100%;
    font-family: var(--font-mono, 'JetBrains Mono', 'Fira Code', monospace);
    font-size: 13px;
  }

  .diff-container :global(.cm-scroller) {
    overflow: auto;
    height: 100%;
  }
</style>
