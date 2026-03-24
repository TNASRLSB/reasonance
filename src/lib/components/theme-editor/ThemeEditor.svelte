<script lang="ts">
  import { get } from 'svelte/store';
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

  function handleSaveAsNew() {
    if (!currentTheme) return;
    const name = currentTheme.meta.name ?? 'custom-theme';
    const blob = new Blob([JSON.stringify(currentTheme, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${name}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function handleExport() {
    handleSaveAsNew();
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
    <div class="editor-shell" role="dialog" aria-modal="true" aria-label="Theme Editor">

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
        >Export JSON</button>
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
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: stretch;
    justify-content: flex-end;
    z-index: 500;
  }

  .editor-shell {
    display: flex;
    flex-direction: column;
    width: min(820px, 90vw);
    height: 100vh;
    background: var(--bg-primary, #0d0d0d);
    border-left: 1px solid var(--border, #2a2a2a);
    box-shadow: -8px 0 40px rgba(0, 0, 0, 0.4);
    overflow: hidden;
  }

  .top-bar {
    display: flex;
    align-items: center;
    gap: var(--space-3, 12px);
    padding: 0 var(--space-4, 16px);
    height: 40px;
    background: var(--bg-secondary, #151515);
    border-bottom: 1px solid var(--border, #2a2a2a);
    flex-shrink: 0;
  }

  .editor-title {
    font-size: 12px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-primary, #eee);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .mode-toggle {
    display: flex;
    gap: 1px;
    background: var(--bg-tertiary, #1a1a1a);
    border: 1px solid var(--border, #333);
    padding: 2px;
    flex-shrink: 0;
  }

  .mode-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #888);
    font-size: 11px;
    font-family: var(--font-ui, sans-serif);
    font-weight: 600;
    padding: 3px 10px;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .mode-btn.active {
    background: var(--accent, #4a9eff);
    color: var(--text-on-accent, #fff);
  }

  .top-actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }

  .icon-btn {
    background: transparent;
    border: 1px solid var(--border, #333);
    color: var(--text-secondary, #aaa);
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
    background: var(--bg-hover, #252525);
    color: var(--text-primary, #eee);
  }

  .icon-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .theme-name-input {
    flex: 1;
    min-width: 0;
    background: var(--bg-tertiary, #1a1a1a);
    border: 1px solid var(--border, #333);
    color: var(--text-primary, #eee);
    font-size: 12px;
    font-family: var(--font-ui, sans-serif);
    padding: 3px 8px;
    max-width: 220px;
  }

  .theme-name-input:focus {
    outline: var(--focus-ring, 2px solid #4a9eff);
    outline-offset: 1px;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #888);
    cursor: pointer;
    font-size: 16px;
    padding: 4px;
    line-height: 1;
    margin-left: auto;
  }

  .close-btn:hover {
    color: var(--text-primary, #eee);
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
    color: var(--text-muted, #888);
    font-size: 14px;
    font-family: var(--font-ui, sans-serif);
  }

  .start-btn {
    background: var(--accent, #4a9eff);
    border: none;
    color: var(--text-on-accent, #fff);
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
    background: var(--bg-secondary, #151515);
    border-top: 1px solid var(--border, #2a2a2a);
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
    border: 1px solid var(--border, #333);
    cursor: pointer;
  }

  .bar-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .cancel-btn {
    background: transparent;
    color: var(--text-secondary, #aaa);
  }

  .cancel-btn:hover {
    background: var(--bg-hover, #252525);
    color: var(--text-primary, #eee);
  }

  .save-btn {
    background: var(--accent, #4a9eff);
    border-color: var(--accent, #4a9eff);
    color: var(--text-on-accent, #fff);
  }

  .save-btn:hover:not(:disabled) {
    background: var(--accent-hover, #3a8eef);
    border-color: var(--accent-hover, #3a8eef);
  }

  .export-btn {
    background: var(--bg-tertiary, #1a1a1a);
    color: var(--text-primary, #eee);
  }

  .export-btn:hover:not(:disabled) {
    background: var(--bg-hover, #252525);
  }
</style>
