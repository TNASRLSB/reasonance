<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorView, basicSetup } from 'codemirror';
  import { EditorState } from '@codemirror/state';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { javascript } from '@codemirror/lang-javascript';
  import { html } from '@codemirror/lang-html';
  import { css } from '@codemirror/lang-css';
  import { python } from '@codemirror/lang-python';
  import { rust } from '@codemirror/lang-rust';
  import { json } from '@codemirror/lang-json';
  import { markdown } from '@codemirror/lang-markdown';
  import { openFiles, activeFilePath } from '$lib/stores/files';

  let container: HTMLDivElement;
  let view: EditorView | null = null;
  let readOnly = $state(true);

  // Derive current file from stores — using reactive vars
  let currentPath = $state<string | null>(null);
  let currentContent = $state<string>('');

  // Subscribe to stores
  const unsubPath = activeFilePath.subscribe((p) => {
    currentPath = p;
  });

  const unsubFiles = openFiles.subscribe((files) => {
    if (currentPath) {
      const f = files.find((x) => x.path === currentPath);
      currentContent = f?.content ?? '';
    }
  });

  function getLang(name: string) {
    const ext = name.split('.').pop()?.toLowerCase() ?? '';
    const langs: Record<string, () => any> = {
      js: javascript,
      jsx: () => javascript({ jsx: true }),
      ts: () => javascript({ typescript: true }),
      tsx: () => javascript({ typescript: true, jsx: true }),
      html: html,
      css: css,
      py: python,
      rs: rust,
      json: json,
      md: markdown,
    };
    return langs[ext]?.() ?? [];
  }

  function buildState(content: string, fileName: string, ro: boolean) {
    const langExt = getLang(fileName);
    return EditorState.create({
      doc: content,
      extensions: [
        basicSetup,
        oneDark,
        Array.isArray(langExt) ? langExt : [langExt],
        EditorView.editable.of(!ro),
        EditorState.readOnly.of(ro),
      ],
    });
  }

  function destroyView() {
    if (view) {
      view.destroy();
      view = null;
    }
  }

  function initEditor(content: string, fileName: string) {
    if (!container) return;
    destroyView();
    const state = buildState(content, fileName, readOnly);
    view = new EditorView({ state, parent: container });
  }

  // Watch for active file changes
  $effect(() => {
    // Access reactive state to trigger effect
    const path = currentPath;
    const content = currentContent;

    if (!path || !container) {
      destroyView();
      return;
    }

    // Find file data from current openFiles value
    let fileName = path.split('/').pop() ?? path;

    // Rebuild editor when path changes
    initEditor(content, fileName);
  });

  // Watch for readOnly toggle — rebuild with same content
  $effect(() => {
    const ro = readOnly;
    if (!view || !currentPath) return;
    const doc = view.state.doc.toString();
    const fileName = currentPath.split('/').pop() ?? currentPath;
    const state = buildState(doc, fileName, ro);
    view.setState(state);
  });

  onMount(() => {
    // If there's already an active file when mounted
    if (currentPath && currentContent !== undefined) {
      const fileName = currentPath.split('/').pop() ?? currentPath;
      initEditor(currentContent, fileName);
    }
  });

  onDestroy(() => {
    unsubPath();
    unsubFiles();
    destroyView();
  });
</script>

<div class="editor-wrapper">
  {#if currentPath}
    <div class="editor-toolbar">
      <span class="editor-filename">{currentPath.split('/').pop()}</span>
      <button
        class="readonly-toggle"
        class:editing={!readOnly}
        onclick={() => (readOnly = !readOnly)}
      >
        {readOnly ? 'Read-only' : 'Editing'}
      </button>
    </div>
    <div class="editor-cm" bind:this={container}></div>
  {:else}
    <div class="editor-empty">
      <p>Apri un file dal file tree</p>
      <p class="hint">Ctrl+P per cercare</p>
    </div>
  {/if}
</div>

<style>
  .editor-wrapper {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .editor-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    font-size: 12px;
  }

  .editor-filename {
    color: var(--text-secondary);
    font-family: var(--font-mono, monospace);
  }

  .readonly-toggle {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }

  .readonly-toggle:hover {
    color: var(--text-primary);
    border-color: var(--accent);
  }

  .readonly-toggle.editing {
    color: var(--accent);
    border-color: var(--accent);
    background: rgba(var(--accent-rgb, 99, 102, 241), 0.08);
  }

  .editor-cm {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  /* Make CodeMirror fill the container */
  .editor-cm :global(.cm-editor) {
    height: 100%;
    font-family: var(--font-mono, 'JetBrains Mono', 'Fira Code', monospace);
    font-size: 13px;
  }

  .editor-cm :global(.cm-scroller) {
    overflow: auto;
    height: 100%;
  }

  .editor-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    gap: 8px;
    font-size: 14px;
  }

  .editor-empty p {
    margin: 0;
  }

  .hint {
    font-size: 12px;
    opacity: 0.6;
    font-family: var(--font-mono, monospace);
  }
</style>
