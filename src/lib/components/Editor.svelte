<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorView, basicSetup } from 'codemirror';
  import { EditorState, type Extension } from '@codemirror/state';
  import { foldGutter } from '@codemirror/language';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { getLangAsync } from '$lib/editor/languages';
  import { openFiles, activeFilePath } from '$lib/stores/files';
  import { isDark } from '$lib/stores/theme';
  import { editorTheme } from '$lib/stores/ui';
  import { editorThemes } from '$lib/editor/themes';
  import MarkdownPreview from './MarkdownPreview.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import ResponsePanel from './ResponsePanel.svelte';
  import type { Adapter } from '$lib/adapter';
  import { tr } from '$lib/i18n/index';

  // Dark theme for editor chrome
  const forgeDarkTheme = EditorView.theme({
    '&': { backgroundColor: '#0e0e0e', color: '#d4d4d4' },
    '.cm-content': {
      fontFamily: "'Atkinson Hyperlegible Mono', monospace",
      fontSize: '14px',
      caretColor: '#f0f0f0',
    },
    '.cm-cursor': { borderLeftColor: '#f0f0f0' },
    '.cm-gutters': { backgroundColor: '#121212', color: '#444', borderRight: '2px solid #333' },
    '.cm-activeLineGutter': { backgroundColor: '#1a1a1a', color: '#888' },
    '.cm-activeLine': { backgroundColor: 'rgba(255, 255, 255, 0.03)' },
    '.cm-selectionBackground': { backgroundColor: 'rgba(29, 78, 216, 0.3) !important' },
    '&.cm-focused .cm-selectionBackground': { backgroundColor: 'rgba(29, 78, 216, 0.4) !important' },
    '.cm-selectionMatch': { backgroundColor: 'rgba(29, 78, 216, 0.15)' },
  }, { dark: true });

  // Light theme for editor chrome
  const forgeLightTheme = EditorView.theme({
    '&': { backgroundColor: '#fafafa', color: '#1a1a1a' },
    '.cm-content': {
      fontFamily: "'Atkinson Hyperlegible Mono', monospace",
      fontSize: '14px',
      caretColor: '#0a0a0a',
    },
    '.cm-cursor': { borderLeftColor: '#0a0a0a' },
    '.cm-gutters': { backgroundColor: '#f0f0f0', color: '#999', borderRight: '2px solid #d4d4d4' },
    '.cm-activeLineGutter': { backgroundColor: '#e5e5e5', color: '#666' },
    '.cm-activeLine': { backgroundColor: 'rgba(0, 0, 0, 0.03)' },
    '.cm-selectionBackground': { backgroundColor: 'rgba(29, 78, 216, 0.2) !important' },
    '&.cm-focused .cm-selectionBackground': { backgroundColor: 'rgba(29, 78, 216, 0.3) !important' },
    '.cm-selectionMatch': { backgroundColor: 'rgba(29, 78, 216, 0.1)' },
  }, { dark: false });

  const { adapter }: { adapter: Adapter } = $props();

  let container: HTMLDivElement;
  let view: EditorView | null = null;
  let readOnly = $state(true);
  let showMarkdownPreview = $state(false);
  let currentLangExts: Extension[] = [];

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

  // Reactively derive content from store auto-subscriptions
  let currentContent = $derived.by(() => {
    if (!$activeFilePath) return '';
    const f = $openFiles.find((x) => x.path === $activeFilePath);
    return f?.content ?? '';
  });

  // Derive whether current file is markdown
  const isMarkdown = $derived(
    $activeFilePath ? ($activeFilePath.split('.').pop()?.toLowerCase() === 'md') : false
  );

  // Reset preview mode when switching files
  let prevPath: string | null = null;
  $effect(() => {
    if ($activeFilePath !== prevPath) {
      prevPath = $activeFilePath;
      showMarkdownPreview = false;
    }
  });

  function buildState(content: string, langExts: Extension[], ro: boolean) {
    let themeExt: Extension[];
    const selectedTheme = editorThemes[$editorTheme];
    if (selectedTheme && selectedTheme.extensions.length > 0) {
      themeExt = selectedTheme.extensions;
    } else if ($editorTheme === 'one-dark' || ($editorTheme === 'forge-dark' && $isDark)) {
      themeExt = [oneDark, forgeDarkTheme];
    } else if ($editorTheme === 'forge-light' || !$isDark) {
      themeExt = [forgeLightTheme];
    } else {
      themeExt = [oneDark, forgeDarkTheme];
    }
    return EditorState.create({
      doc: content,
      extensions: [
        basicSetup,
        foldGutter(),
        ...themeExt,
        ...langExts,
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

  async function initEditor(content: string, fileName: string) {
    if (!container) return;
    destroyView();
    currentLangExts = [];
    // Start with no language highlighting so the editor shows immediately
    const baseState = buildState(content, [], readOnly);
    view = new EditorView({ state: baseState, parent: container });
    // Load language asynchronously and apply once ready
    const langExts = await getLangAsync(fileName);
    if (!view) return; // editor may have been destroyed while awaiting
    currentLangExts = langExts;
    const state = buildState(view.state.doc.toString(), langExts, readOnly);
    view.setState(state);
  }

  // Watch for active file changes
  $effect(() => {
    const path = $activeFilePath;
    const content = currentContent;

    if (!path || !container) {
      destroyView();
      return;
    }

    let fileName = path.split('/').pop() ?? path;
    initEditor(content, fileName);
  });

  // Watch for readOnly toggle — rebuild with same content and cached language extensions
  $effect(() => {
    const ro = readOnly;
    if (!view || !$activeFilePath) return;
    const doc = view.state.doc.toString();
    const state = buildState(doc, currentLangExts, ro);
    view.setState(state);
  });

  // Watch for theme changes — rebuild editor with correct theme and cached language extensions
  $effect(() => {
    const _dark = $isDark;
    const _theme = $editorTheme;
    if (!view || !$activeFilePath) return;
    const doc = view.state.doc.toString();
    const state = buildState(doc, currentLangExts, readOnly);
    view.setState(state);
  });

  onMount(() => {
    if ($activeFilePath && currentContent !== undefined) {
      const fileName = $activeFilePath.split('/').pop() ?? $activeFilePath;
      initEditor(currentContent, fileName);
    }

    return () => {
      destroyView();
    };
  });
</script>

<div class="editor-wrapper">
  {#if $activeFilePath}
    <div class="editor-toolbar">
      <span class="editor-filename">{$activeFilePath.split('/').pop()}</span>
      <div class="toolbar-actions">
        {#if isMarkdown}
          <button
            class="preview-toggle"
            class:active={showMarkdownPreview}
            onclick={() => (showMarkdownPreview = !showMarkdownPreview)}
          >
            {showMarkdownPreview ? $tr('editor.code') : $tr('editor.preview')}
          </button>
        {/if}
        <select
          class="theme-select"
          value={$editorTheme}
          onchange={(e) => editorTheme.set((e.target as HTMLSelectElement).value)}
        >
          {#each Object.entries(editorThemes) as [key, t]}
            <option value={key}>{t.label}</option>
          {/each}
        </select>
        <button
          class="readonly-toggle"
          class:editing={!readOnly}
          onclick={() => (readOnly = !readOnly)}
        >
          {readOnly ? $tr('editor.readOnly') : $tr('editor.editing')}
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
      <p>{$tr('editor.openFile')}</p>
      <p class="hint">{$tr('editor.searchHint')}</p>
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
    color: var(--text-primary);
  }

  .readonly-toggle.editing {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--text-primary);
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
    color: var(--text-muted);
    gap: 10px;
    font-family: var(--font-ui);
    font-size: 14px;
    font-weight: 500;
  }

  .editor-empty p {
    margin: 0;
  }

  .hint {
    font-size: var(--font-size-small);
    color: var(--border);
    font-family: var(--font-mono);
    border: 1px solid var(--border);
    padding: 4px 12px;
  }

  .theme-select {
    background: var(--bg-tertiary);
    border: var(--border-width) solid var(--border);
    border-radius: 0;
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 2px 6px;
    cursor: pointer;
    outline: none;
  }

  .theme-select:focus {
    border-color: var(--accent);
  }
</style>
