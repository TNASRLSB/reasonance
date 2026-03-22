<script lang="ts">
  import { onMount } from 'svelte';
  import { MergeView } from '@codemirror/merge';
  import { EditorState } from '@codemirror/state';
  import { EditorView, basicSetup } from 'codemirror';
  import { oneDark } from '@codemirror/theme-one-dark';
  import type { Adapter } from '$lib/adapter';
  import { isDark } from '$lib/stores/theme';
  import { tr } from '$lib/i18n/index';

  const forgeDarkDiffTheme = EditorView.theme({
    '&': { backgroundColor: '#0e0e0e', color: '#d4d4d4' },
    '.cm-gutters': { backgroundColor: '#121212', color: '#444', borderRight: '2px solid #333' },
    '.cm-activeLineGutter': { backgroundColor: '#1a1a1a' },
    '.cm-activeLine': { backgroundColor: 'rgba(255,255,255,0.03)' },
  }, { dark: true });

  const forgeLightDiffTheme = EditorView.theme({
    '&': { backgroundColor: '#fafafa', color: '#1a1a1a' },
    '.cm-gutters': { backgroundColor: '#f0f0f0', color: '#999', borderRight: '2px solid #d4d4d4' },
    '.cm-activeLineGutter': { backgroundColor: '#e5e5e5' },
    '.cm-activeLine': { backgroundColor: 'rgba(0,0,0,0.03)' },
  }, { dark: false });

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
    const themeExt = $isDark ? [oneDark, forgeDarkDiffTheme] : [forgeLightDiffTheme];
    return EditorState.create({
      doc,
      extensions: [basicSetup, ...themeExt, EditorView.editable.of(false), EditorState.readOnly.of(true)],
    });
  }

  function rebuildMergeView() {
    if (!container) return;
    if (mergeView) {
      mergeView.destroy();
      mergeView = null;
    }
    mergeView = new MergeView({
      a: buildEditorState(original),
      b: buildEditorState(modified),
      parent: container,
    });
  }

  onMount(() => {
    rebuildMergeView();
    return () => {
      if (mergeView) {
        mergeView.destroy();
        mergeView = null;
      }
    };
  });

  // Rebuild on theme change
  $effect(() => {
    const _dark = $isDark;
    if (mergeView) rebuildMergeView();
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
    <span class="diff-title">{$tr('diff.changes', { file: filename })}</span>
    <div class="diff-actions">
      <button class="btn-reject" onclick={handleReject}>&#10005; {$tr('diff.reject')}</button>
      <button class="btn-accept" onclick={handleAccept}>&#10003; {$tr('diff.accept')}</button>
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
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
    font-size: 12px;
    font-family: var(--font-ui);
  }

  .diff-title {
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-weight: 500;
  }

  .diff-actions {
    display: flex;
    gap: 8px;
  }

  .btn-accept,
  .btn-reject {
    padding: 3px 12px;
    border-radius: var(--radius);
    border: 1px solid transparent;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .btn-accept {
    background: var(--success);
    color: #fff;
    border-color: var(--success);
  }

  .btn-accept:hover {
    opacity: 0.85;
  }

  .btn-reject {
    background: var(--bg-primary);
    color: var(--danger);
    border-color: var(--danger);
  }

  .btn-reject:hover {
    background: var(--danger);
    color: #fff;
    border-color: var(--danger);
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
    font-family: var(--font-mono);
    font-size: 13px;
  }

  .diff-container :global(.cm-scroller) {
    overflow: auto;
    height: 100%;
  }
</style>
