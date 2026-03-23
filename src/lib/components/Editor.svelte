<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorView, basicSetup } from 'codemirror';
  import { EditorState, EditorSelection, type Extension } from '@codemirror/state';
  import { foldGutter } from '@codemirror/language';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { getLangAsync } from '$lib/editor/languages';
  import { openFiles, activeFilePath, pendingLine, cursorLine, cursorCol } from '$lib/stores/files';
  import { isDark } from '$lib/stores/theme';
  import { editorTheme, fontFamily, fontSize } from '$lib/stores/ui';
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

  let { adapter, readOnly = true, showMarkdownPreview = false }: {
    adapter: Adapter;
    readOnly?: boolean;
    showMarkdownPreview?: boolean;
  } = $props();

  let container = $state<HTMLDivElement | null>(null);
  let wrapper = $state<HTMLDivElement | null>(null);
  let view: EditorView | null = null;
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

  // Extension that tracks cursor position changes
  function buildCursorTracker(): Extension {
    return EditorView.updateListener.of((update) => {
      if (update.selectionSet || update.docChanged) {
        const pos = update.state.selection.main.head;
        const line = update.state.doc.lineAt(pos);
        cursorLine.set(line.number);
        cursorCol.set(pos - line.from + 1);
      }
    });
  }

  function buildFontExt(): Extension {
    return EditorView.theme({
      '.cm-content': { fontFamily: $fontFamily, fontSize: `${$fontSize}px` },
      '.cm-gutters': { fontFamily: $fontFamily },
    });
  }

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
        EditorView.lineWrapping,
        ...themeExt,
        buildFontExt(),
        ...langExts,
        buildCursorTracker(),
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

  // Watch for active file changes — debounced to prevent rapid teardown/recreation
  let editorSwitchTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    const path = $activeFilePath;
    const content = currentContent;

    if (editorSwitchTimer) {
      clearTimeout(editorSwitchTimer);
      editorSwitchTimer = null;
    }

    if (!path || !container) {
      destroyView();
      return;
    }

    const fileName = path.split('/').pop() ?? path;
    editorSwitchTimer = setTimeout(() => {
      editorSwitchTimer = null;
      initEditor(content, fileName);
    }, 75);
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

  // Watch for font changes — rebuild editor with new font settings
  $effect(() => {
    const _ff = $fontFamily;
    const _fs = $fontSize;
    if (!view || !$activeFilePath) return;
    const doc = view.state.doc.toString();
    const state = buildState(doc, currentLangExts, readOnly);
    view.setState(state);
  });

  // Watch for pendingLine — scroll editor to the requested line
  $effect(() => {
    const line = $pendingLine;
    if (line == null || !view) return;
    // Clear immediately so we don't re-trigger
    pendingLine.set(null);
    try {
      const lineInfo = view.state.doc.line(Math.min(line, view.state.doc.lines));
      view.dispatch({
        selection: EditorSelection.cursor(lineInfo.from),
        scrollIntoView: true,
      });
    } catch {
      // Line out of range — ignore
    }
  });

  onMount(() => {
    if ($activeFilePath && currentContent !== undefined) {
      const fileName = $activeFilePath.split('/').pop() ?? $activeFilePath;
      initEditor(currentContent, fileName);
    }

    // ResizeObserver on wrapper to force CM6 relayout on panel resize
    const ro = new ResizeObserver(() => {
      if (view) view.requestMeasure();
    });
    if (wrapper) ro.observe(wrapper);

    return () => {
      ro.disconnect();
      destroyView();
    };
  });
</script>

<div class="editor-wrapper" bind:this={wrapper}>
  {#if $activeFilePath}
    <div class="editor-body">
      {#if isMarkdown && showMarkdownPreview}
        <MarkdownPreview content={currentContent} />
      {/if}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="editor-cm"
        class:hidden={isMarkdown && showMarkdownPreview}
        bind:this={container}
        oncontextmenu={handleContextMenu}
      ></div>
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
    overflow: hidden;
    min-height: 0;
    min-width: 0;
  }

  .editor-cm.hidden {
    display: none;
  }

  .editor-cm :global(.cm-editor) {
    height: 100%;
    max-height: 100%;
  }

  .editor-cm :global(.cm-scroller) {
    overflow: auto;
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

</style>
