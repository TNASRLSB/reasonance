<script lang="ts">
  import { get } from 'svelte/store';
  import { invoke } from '@tauri-apps/api/core';
  import { activeTheme, setPreviewVariable } from '$lib/stores/theme';
  import { validateTheme } from '$lib/engine/theme-validator';
  import { extractVariables } from '$lib/engine/theme-engine';
  import type { ThemeFile } from '$lib/engine/theme-types';
  import { THEME_SECTIONS } from '$lib/engine/theme-types';
  import ThemeEditorSidebar from './ThemeEditorSidebar.svelte';
  import ThemeEditorSection from './ThemeEditorSection.svelte';
  import ThemeJsonView from './ThemeJsonView.svelte';
  import ThemeStartDialog from './ThemeStartDialog.svelte';
  import ModifierSections from './ModifierSections.svelte';

  let {
    open,
    onClose,
  }: {
    open: boolean;
    onClose: () => void;
  } = $props();

  type EditorMode = 'theme' | 'modifier';

  let editorMode = $state<EditorMode>('theme');
  let currentTheme = $state<ThemeFile | null>(null);
  let showStartDialog = $state(false);
  let showJson = $state(false);
  let activeSection = $state('colors');
  let searchQuery = $state('');
  let saveError = $state<string | null>(null);
  let copyConfirm = $state(false);
  let dragOver = $state(false);
  let importError = $state<string | null>(null);

  // Undo/redo stacks
  let undoStack = $state<ThemeFile[]>([]);
  let redoStack = $state<ThemeFile[]>([]);

  // Sections visible in the sidebar (only sections that exist in theme)
  let availableSections = $derived(
    currentTheme
      ? (THEME_SECTIONS as readonly string[]).filter((s) => currentTheme![s as keyof ThemeFile] !== undefined)
      : []
  );

  // Filtered entries when searching
  let filteredVariables = $derived<Record<string, string | number>>(() => {
    if (!currentTheme || !activeSection) return {};
    const section = currentTheme[activeSection as keyof ThemeFile] as Record<string, string | number> | undefined;
    if (!section) return {};
    if (!searchQuery.trim()) return section;
    const q = searchQuery.toLowerCase();
    return Object.fromEntries(
      Object.entries(section).filter(([k, v]) =>
        k.toLowerCase().includes(q) || String(v).toLowerCase().includes(q)
      )
    );
  });

  function pushUndo(prev: ThemeFile) {
    undoStack = [...undoStack.slice(-50), prev];
    redoStack = [];
  }

  function undo() {
    if (undoStack.length === 0 || !currentTheme) return;
    const prev = undoStack[undoStack.length - 1];
    redoStack = [currentTheme, ...redoStack.slice(0, 50)];
    undoStack = undoStack.slice(0, -1);
    currentTheme = prev;
  }

  function redo() {
    if (redoStack.length === 0 || !currentTheme) return;
    const next = redoStack[0];
    undoStack = [...undoStack.slice(-50), currentTheme];
    redoStack = redoStack.slice(1);
    currentTheme = next;
  }

  function updateVariable(section: string, key: string, value: string | number) {
    if (!currentTheme) return;
    pushUndo(currentTheme);
    const updated = structuredClone(currentTheme);
    if (!updated[section as keyof ThemeFile]) {
      (updated as any)[section] = {};
    }
    (updated[section as keyof ThemeFile] as Record<string, string | number>)[key] = value;
    currentTheme = updated;

    // Live preview
    setPreviewVariable(key, String(value));
  }

  function updateTheme(theme: ThemeFile) {
    if (!currentTheme) return;
    pushUndo(currentTheme);
    currentTheme = theme;

    // Reapply all previewed variables
    const vars = extractVariables(theme);
    for (const [k, v] of Object.entries(vars)) {
      setPreviewVariable(k, String(v));
    }
  }

  function handleStartSelect(theme: ThemeFile, name: string) {
    currentTheme = theme;
    if (!currentTheme.meta.name) currentTheme.meta.name = name;
    showStartDialog = false;
    undoStack = [];
    redoStack = [];
    editorMode = theme.meta.type === 'modifier' ? 'modifier' : 'theme';
    activeSection = availableSections[0] ?? 'colors';
  }

  function handleCancel() {
    // Restore original theme variables
    const original = get(activeTheme);
    const vars = extractVariables(original);
    for (const [k, v] of Object.entries(vars)) {
      setPreviewVariable(k, String(v));
    }
    onClose();
  }

  async function handleSaveAsNew() {
    if (!currentTheme) return;
    saveError = null;
    const result = validateTheme(currentTheme);
    if (!result.valid) {
      saveError = result.errors.join('\n');
      return;
    }
    const name = currentTheme.meta.name ?? 'custom-theme';
    const content = JSON.stringify(currentTheme, null, 2);
    try {
      await invoke('save_user_theme', { name, content });
    } catch (err) {
      saveError = String(err);
    }
  }

  async function handleExport() {
    if (!currentTheme) return;
    saveError = null;
    const result = validateTheme(currentTheme);
    if (!result.valid) {
      saveError = result.errors.join('\n');
      return;
    }
    const json = JSON.stringify(currentTheme, null, 2);
    try {
      await navigator.clipboard.writeText(json);
      copyConfirm = true;
      setTimeout(() => { copyConfirm = false; }, 2000);
    } catch (err) {
      saveError = `Clipboard error: ${String(err)}`;
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    dragOver = true;
  }

  function handleDragLeave() {
    dragOver = false;
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    importError = null;
    const file = e.dataTransfer?.files?.[0];
    if (!file) return;
    if (!file.name.endsWith('.json')) {
      importError = 'Only .json files are supported';
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const parsed = JSON.parse(reader.result as string);
        const result = validateTheme(parsed);
        if (!result.valid) {
          importError = result.errors.join('\n');
          return;
        }
        const theme = parsed as ThemeFile;
        if (currentTheme) pushUndo(currentTheme);
        currentTheme = theme;
        undoStack = [];
        redoStack = [];
        editorMode = theme.meta.type === 'modifier' ? 'modifier' : 'theme';
        activeSection = availableSections[0] ?? 'colors';
        const vars = extractVariables(theme);
        for (const [k, v] of Object.entries(vars)) {
          setPreviewVariable(k, String(v));
        }
      } catch {
        importError = 'Invalid JSON file';
      }
    };
    reader.readAsText(file);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') handleCancel();
    if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) { e.preventDefault(); undo(); }
    if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || (e.key === 'z' && e.shiftKey))) { e.preventDefault(); redo(); }
  }

  $effect(() => {
    if (open && !currentTheme) {
      showStartDialog = true;
    }
  });
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="editor-overlay" onkeydown={handleKeydown} role="presentation">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="editor-shell"
      class:drag-over={dragOver}
      role="dialog"
      aria-modal="true"
      aria-label="Theme Editor"
      ondragover={handleDragOver}
      ondragleave={handleDragLeave}
      ondrop={handleDrop}
    >

      <!-- Top bar -->
      <div class="top-bar">
        <span class="editor-title">Theme Editor</span>

        <div class="mode-toggle" role="group" aria-label="Editor mode">
          <button
            class="mode-btn"
            class:active={editorMode === 'theme'}
            onclick={() => { editorMode = 'theme'; }}
          >Theme</button>
          <button
            class="mode-btn"
            class:active={editorMode === 'modifier'}
            onclick={() => { editorMode = 'modifier'; }}
          >Modifier</button>
        </div>

        <div class="top-actions">
          <button
            class="icon-btn"
            onclick={undo}
            disabled={undoStack.length === 0}
            aria-label="Undo"
            title="Undo (Ctrl+Z)"
          >↩</button>
          <button
            class="icon-btn"
            onclick={redo}
            disabled={redoStack.length === 0}
            aria-label="Redo"
            title="Redo (Ctrl+Y)"
          >↪</button>
          <button
            class="icon-btn"
            onclick={() => { showStartDialog = true; }}
            aria-label="Change starting theme"
            title="Change starting theme"
          >⊕</button>
        </div>

        {#if currentTheme}
          <input
            type="text"
            class="theme-name-input"
            value={currentTheme.meta.name}
            oninput={(e) => {
              if (!currentTheme) return;
              const updated = structuredClone(currentTheme);
              updated.meta.name = (e.target as HTMLInputElement).value;
              currentTheme = updated;
            }}
            aria-label="Theme name"
            placeholder="Theme name"
          />
        {/if}

        <button class="close-btn" onclick={handleCancel} aria-label="Close theme editor">✕</button>
      </div>

      <!-- Main area -->
      <div class="editor-body">
        {#if currentTheme}
          {#if showJson}
            <ThemeJsonView
              theme={currentTheme}
              onUpdate={updateTheme}
            />
          {:else if editorMode === 'modifier'}
            <div class="modifier-scroll">
              <ModifierSections theme={currentTheme} onUpdate={updateTheme} />
            </div>
          {:else}
            <ThemeEditorSidebar
              sections={availableSections}
              activeSection={activeSection}
              onSelectSection={(s) => { activeSection = s; searchQuery = ''; }}
              searchQuery={searchQuery}
              onSearch={(q) => { searchQuery = q; }}
              onToggleJson={() => { showJson = !showJson; }}
            />
            <div class="section-scroll">
              <ThemeEditorSection
                sectionName={activeSection}
                variables={filteredVariables()}
                onUpdate={(key, value) => updateVariable(activeSection, key, value)}
              />
            </div>
          {/if}
        {:else}
          <div class="empty-state">
            <p>No theme loaded. Choose a starting point.</p>
            <button class="start-btn" onclick={() => { showStartDialog = true; }}>Choose Theme</button>
          </div>
        {/if}
      </div>

      <!-- Bottom bar -->
      <div class="bottom-bar">
        <button class="bar-btn cancel-btn" onclick={handleCancel}>Cancel</button>
        {#if importError}
          <span class="feedback-msg error-msg" role="alert">{importError}</span>
        {/if}
        {#if saveError}
          <span class="feedback-msg error-msg" role="alert">{saveError}</span>
        {/if}
        <div class="spacer"></div>
        <button
          class="bar-btn save-btn"
          onclick={handleSaveAsNew}
          disabled={!currentTheme}
        >Save as New</button>
        <button
          class="bar-btn export-btn"
          onclick={handleExport}
          disabled={!currentTheme}
        >{copyConfirm ? 'Copied!' : 'Export JSON'}</button>
      </div>
    </div>
  </div>
{/if}

<ThemeStartDialog
  open={showStartDialog}
  onSelect={handleStartSelect}
  onClose={() => { showStartDialog = false; if (!currentTheme) onClose(); }}
/>

<style>
  .editor-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    display: flex;
    align-items: stretch;
    justify-content: flex-end;
    z-index: var(--layer-overlay);
  }

  .editor-shell {
    display: flex;
    flex-direction: column;
    width: min(820px, 90vw);
    height: 100vh;
    background: var(--bg-primary);
    border-left: 1px solid var(--border);
    box-shadow: var(--shadow-overlay);
    overflow: hidden;
  }

  .top-bar {
    display: flex;
    align-items: center;
    gap: var(--space-3, 12px);
    padding: 0 var(--space-4, 16px);
    height: 40px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .editor-title {
    font-size: 12px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-primary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .mode-toggle {
    display: flex;
    gap: 1px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    padding: 2px;
    flex-shrink: 0;
  }

  .mode-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 11px;
    font-family: var(--font-ui, sans-serif);
    font-weight: 600;
    padding: 3px 10px;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .mode-btn.active {
    background: var(--accent-btn);
    color: var(--text-on-accent);
  }

  .top-actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }

  .icon-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 14px;
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
  }

  .icon-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .icon-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .theme-name-input {
    flex: 1;
    min-width: 0;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-ui, sans-serif);
    padding: 3px 8px;
    max-width: 220px;
  }

  .theme-name-input:focus {
    outline: var(--focus-ring);
    outline-offset: 1px;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 16px;
    padding: 4px;
    line-height: 1;
    margin-left: auto;
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .editor-body {
    display: flex;
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }

  .section-scroll,
  .modifier-scroll {
    flex: 1;
    overflow-y: auto;
    min-width: 0;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-4, 16px);
    color: var(--text-muted);
    font-size: 14px;
    font-family: var(--font-ui, sans-serif);
  }

  .start-btn {
    background: var(--accent-btn);
    border: none;
    color: var(--text-on-accent);
    font-size: 13px;
    font-family: var(--font-ui, sans-serif);
    font-weight: 700;
    padding: 8px 20px;
    cursor: pointer;
  }

  .bottom-bar {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
    padding: var(--space-2, 8px) var(--space-4, 16px);
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .spacer {
    flex: 1;
  }

  .bar-btn {
    font-size: 12px;
    font-family: var(--font-ui, sans-serif);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 5px 16px;
    border: 1px solid var(--border);
    cursor: pointer;
  }

  .bar-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .cancel-btn {
    background: transparent;
    color: var(--text-secondary);
  }

  .cancel-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .save-btn {
    background: var(--accent-btn);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }

  .save-btn:hover:not(:disabled) {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .export-btn {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .export-btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .editor-shell.drag-over {
    outline: 2px dashed var(--accent);
    outline-offset: -4px;
  }

  .feedback-msg {
    font-size: 11px;
    font-family: var(--font-ui, sans-serif);
    max-width: 300px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 1;
  }

  .error-msg {
    color: var(--danger);
  }
</style>
