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
  import MarkdownPreview from './MarkdownPreview.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import ResponsePanel from './ResponsePanel.svelte';
  import type { Adapter } from '$lib/adapter';

  // Custom brutalist theme for editor chrome (background, gutters, cursor, selection)
  // oneDark is kept for syntax highlighting colors only
  const forgeBrutalistTheme = EditorView.theme({
    '&': {
      backgroundColor: '#0e0e0e',
      color: '#d4d4d4',
    },
    '.cm-content': {
      fontFamily: "'Atkinson Hyperlegible Mono', 'JetBrains Mono', 'Fira Code', monospace",
      fontSize: '13px',
      caretColor: '#f0f0f0',
    },
    '.cm-cursor': {
      borderLeftColor: '#f0f0f0',
    },
    '.cm-gutters': {
      backgroundColor: '#121212',
      color: '#444',
      borderRight: '2px solid #333',
    },
    '.cm-activeLineGutter': {
      backgroundColor: '#1a1a1a',
      color: '#888',
    },
    '.cm-activeLine': {
      backgroundColor: 'rgba(255, 255, 255, 0.03)',
    },
    '.cm-selectionBackground': {
      backgroundColor: 'rgba(29, 78, 216, 0.3) !important',
    },
    '&.cm-focused .cm-selectionBackground': {
      backgroundColor: 'rgba(29, 78, 216, 0.4) !important',
    },
    '.cm-selectionMatch': {
      backgroundColor: 'rgba(29, 78, 216, 0.15)',
    },
  }, { dark: true });

  const { adapter }: { adapter: Adapter } = $props();

  let container: HTMLDivElement;
  let view: EditorView | null = null;
  let readOnly = $state(true);
  let showMarkdownPreview = $state(false);

  // Context menu state
  let ctxVisible = $state(false);
  let ctxX = $state(0);
  let ctxY = $state(0);
  let ctxSelectedText = $state('');

  // Response panel state
  let responseVisible = $state(false);
  let responseContent = $state('');

  function handleContextMenu(e: MouseEvent) {
    if (!view) return;
    const sel = view.state.selection.main;
    const text = view.state.sliceDoc(sel.from, sel.to);
    if (!text.trim()) return;

    e.preventDefault();
    ctxSelectedText = text;
    ctxX = e.clientX;
    ctxY = e.clientY;
    ctxVisible = true;
  }

  function handleResponse(content: string) {
    responseContent = content;
    responseVisible = true;
  }

  // Derive current file from stores — using reactive vars
  let currentPath = $state<string | null>(null);
  let currentContent = $state<string>('');

  // Derive whether current file is markdown
  const isMarkdown = $derived(
    currentPath ? (currentPath.split('.').pop()?.toLowerCase() === 'md') : false
  );

  // Subscribe to stores
  const unsubPath = activeFilePath.subscribe((p) => {
    currentPath = p;
    // Reset preview mode when switching files
    showMarkdownPreview = false;
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
        forgeBrutalistTheme,
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
      <div class="toolbar-actions">
        {#if isMarkdown}
          <button
            class="preview-toggle"
            class:active={showMarkdownPreview}
            onclick={() => (showMarkdownPreview = !showMarkdownPreview)}
          >
            {showMarkdownPreview ? 'Code' : 'Preview'}
          </button>
        {/if}
        <button
          class="readonly-toggle"
          class:editing={!readOnly}
          onclick={() => (readOnly = !readOnly)}
        >
          {readOnly ? 'Read-only' : 'Editing'}
        </button>
      </div>
    </div>
    <div class="editor-body">
      {#if isMarkdown && showMarkdownPreview}
        <MarkdownPreview content={currentContent} />
      {:else}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="editor-cm" bind:this={container} oncontextmenu={handleContextMenu}></div>
      {/if}
      <ResponsePanel
        content={responseContent}
        visible={responseVisible}
        onClose={() => (responseVisible = false)}
      />
    </div>
  {:else}
    <div class="editor-empty">
      <p>Apri un file dal file tree</p>
      <p class="hint">Ctrl+P per cercare</p>
    </div>
  {/if}
</div>

<ContextMenu
  {adapter}
  x={ctxX}
  y={ctxY}
  visible={ctxVisible}
  selectedText={ctxSelectedText}
  onResponse={handleResponse}
  onClose={() => (ctxVisible = false)}
/>

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
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .editor-filename {
    color: var(--text-secondary);
    font-family: var(--font-mono);
  }

  .preview-toggle,
  .readonly-toggle {
    background: var(--bg-tertiary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    padding: 2px 8px;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }

  .preview-toggle:hover,
  .readonly-toggle:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
  }

  .preview-toggle.active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .readonly-toggle.editing {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .editor-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
    position: relative;
  }

  .editor-cm {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  .editor-cm :global(.cm-editor) {
    height: 100%;
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
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
  }

  .editor-empty p {
    margin: 0;
  }

  .hint {
    font-size: var(--font-size-small);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
</style>
