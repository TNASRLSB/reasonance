<script lang="ts">
  import App from '$lib/components/App.svelte';
  import FileTree from '$lib/components/FileTree.svelte';
  import EditorTabs from '$lib/components/EditorTabs.svelte';
  import Editor from '$lib/components/Editor.svelte';
  import DiffView from '$lib/components/DiffView.svelte';
  import HelpPanel from '$lib/components/HelpPanel.svelte';
  import TerminalManager from '$lib/components/TerminalManager.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import SearchPalette from '$lib/components/SearchPalette.svelte';
  import FindInFiles from '$lib/components/FindInFiles.svelte';
  import WelcomeScreen from '$lib/components/WelcomeScreen.svelte';
  import { TauriAdapter } from '$lib/adapter/tauri';
  import { initTheme } from '$lib/stores/theme';
  import { openFiles, activeFilePath, projectRoot, addRecentProject } from '$lib/stores/files';
  import { showSettings, enhancedReadability, showSwarmCanvas } from '$lib/stores/ui';
  import { initI18n, tr } from '$lib/i18n/index';
  import { registerKeybinding, initKeybindings } from '$lib/utils/keybindings';
  import Toast from '$lib/components/Toast.svelte';
  import { showToast } from '$lib/stores/toast';
  import SwarmCanvas from '$lib/components/swarm/SwarmCanvas.svelte';
  import ShortcutsDialog from '$lib/components/ShortcutsDialog.svelte';
  import { saveSession, restoreSession, initShadowTracking } from '$lib/utils/session';
  import { loadInitialConfig, discoverAndApplyLlms } from '$lib/utils/config-bootstrap';
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import '../app.css';

  interface DiffState {
    path: string;
    original: string;
    modified: string;
    filename: string;
  }

  const adapter = new TauriAdapter();

  let diffState = $state<DiffState | null>(null);
  let unwatchFiles: (() => void) | null = null;
  let showSearchPalette = $state(false);
  let showFindInFiles = $state(false);
  let showWelcome = $state(true);
  let showHelp = $state(false);
  let showAbout = $state(false);
  let showShortcuts = $state(false);
  let editorReadOnly = $state(true);
  let showMarkdownPreview = $state(false);
  let swarmVisible = $state(false);
  const unsubSwarm = showSwarmCanvas.subscribe((val) => { swarmVisible = val; });
  let unsubEnhanced: () => void;

  // Reactive projectRoot for passing as cwd
  let currentProjectRoot = $state('.');
  const unsubRoot = projectRoot.subscribe((v) => { currentProjectRoot = v || '.'; });

  // Editor toolbar state
  const isMarkdown = $derived(
    $activeFilePath ? ($activeFilePath.split('.').pop()?.toLowerCase() === 'md') : false
  );

  // Reset preview when switching files
  let prevEditorPath: string | null = null;
  $effect(() => {
    if ($activeFilePath !== prevEditorPath) {
      prevEditorPath = $activeFilePath;
      showMarkdownPreview = false;
    }
  });

  // Cleanup array for event listeners
  const cleanups: Array<() => void> = [];

  // ── Open Folder / Switch Project ─────────────────────────────────────────

  async function openFolder() {
    const selected = await adapter.openFolderDialog();
    if (selected) {
      await switchProject(selected);
    }
  }

  async function switchProject(path: string) {
    openFiles.set([]);
    activeFilePath.set(null);
    projectRoot.set(path);
    addRecentProject(path);
    showWelcome = false;
    try { await adapter.setProjectRoot(path); } catch { /* non-fatal */ }

    // Restart file watcher for new directory
    if (unwatchFiles) unwatchFiles();
    unwatchFiles = await adapter.watchFiles(path, async (event) => {
      const currentFiles = get(openFiles);
      const openFile = currentFiles.find((f) => f.path === event.path);
      if (!openFile) return;

      if (event.type === 'remove') {
        openFiles.update((all) =>
          all.map((f) => (f.path === event.path ? { ...f, isDeleted: true } : f))
        );
        showToast('warning', 'File deleted', event.path.split('/').pop() ?? event.path);
        return;
      }

      if (event.type === 'modify') {
        if (diffState && diffState.path === event.path) return;
        try {
          const [newContent, shadow] = await Promise.all([
            adapter.readFile(event.path),
            adapter.getShadow(event.path),
          ]);
          if (shadow === null) return;
          if (newContent === shadow) return;
          diffState = {
            path: event.path,
            original: shadow,
            modified: newContent,
            filename: event.path.split('/').pop() ?? event.path,
          };
          activeFilePath.set(event.path);
        } catch { /* non-fatal */ }
      }
    });
  }

  // Shadow tracking subscription — initialised in onMount, cleaned up in onDestroy
  let unsubFiles: () => void;

  onMount(async () => {
    initTheme();

    // Initialize i18n before restoring session
    await initI18n();

    // Enhanced Readability mode — toggle CSS class on root element
    unsubEnhanced = enhancedReadability.subscribe((on) => {
      document.documentElement.classList.toggle('enhanced-readability', on);
    });

    // Start shadow tracking before restoring session (so restored files get shadows)
    unsubFiles = initShadowTracking(adapter);

    // Restore persisted session state before anything else
    await restoreSession(adapter, () => { showWelcome = false; });

    // Load TOML config then auto-discover LLM CLIs if none configured
    await loadInitialConfig(adapter);
    await discoverAndApplyLlms(adapter);

    // Listen for window close to save session state
    await adapter.onWindowClose(saveSession);

    // Register global keyboard shortcuts
    registerKeybinding('ctrl+p', () => { showSearchPalette = true; });
    registerKeybinding('ctrl+shift+f', () => { showFindInFiles = true; });
    registerKeybinding('ctrl+,', () => showSettings.set(true));
    registerKeybinding('f1', () => { showHelp = !showHelp; });
    registerKeybinding('ctrl+/', () => { showShortcuts = true; });
    initKeybindings();

    // Listen for openFolder custom event from MenuBar
    const handleOpenFolder = () => openFolder();
    window.addEventListener('reasonance:openFolder', handleOpenFolder);
    cleanups.push(() => window.removeEventListener('reasonance:openFolder', handleOpenFolder));

    // Listen for help custom event from MenuBar
    const handleOpenHelp = () => { showHelp = true; };
    document.addEventListener('reasonance:help', handleOpenHelp);
    cleanups.push(() => document.removeEventListener('reasonance:help', handleOpenHelp));

    // Listen for about custom event from MenuBar
    const handleAbout = () => { showAbout = true; };
    document.addEventListener('reasonance:about', handleAbout);
    cleanups.push(() => document.removeEventListener('reasonance:about', handleAbout));

    // Listen for shortcuts custom event from MenuBar
    const handleShortcuts = () => { showShortcuts = true; };
    document.addEventListener('reasonance:shortcuts', handleShortcuts);
    cleanups.push(() => document.removeEventListener('reasonance:shortcuts', handleShortcuts));

    // Start watching the project directory for external changes
    const root = get(projectRoot) || '.';
    unwatchFiles = await adapter.watchFiles(root, async (event) => {
      const currentFiles = get(openFiles);
      const openFile = currentFiles.find((f) => f.path === event.path);

      if (!openFile) return;

      if (event.type === 'remove') {
        // Mark the file as deleted in the store
        openFiles.update((all) =>
          all.map((f) => (f.path === event.path ? { ...f, isDeleted: true } : f))
        );
        showToast('warning', 'File deleted', event.path.split('/').pop() ?? event.path);
        return;
      }

      if (event.type === 'modify') {
        // Avoid showing diff if already showing one for this path
        if (diffState && diffState.path === event.path) return;

        try {
          const [newContent, shadow] = await Promise.all([
            adapter.readFile(event.path),
            adapter.getShadow(event.path),
          ]);

          if (shadow === null) return; // No shadow means we don't track this file
          if (newContent === shadow) return; // No actual change

          diffState = {
            path: event.path,
            original: shadow,
            modified: newContent,
            filename: event.path.split('/').pop() ?? event.path,
          };

          // Switch to the changed file's tab
          activeFilePath.set(event.path);
        } catch {
          // Read failures are non-fatal
        }
      }
    });
  });

  onDestroy(() => {
    unsubFiles?.();
    unsubEnhanced?.();
    unsubSwarm();
    unsubRoot();
    cleanups.forEach((fn) => fn());
    if (unwatchFiles) unwatchFiles();
  });

  function handleAccept() {
    if (!diffState) return;
    const path = diffState.path;
    const newContent = diffState.modified;
    // Update the open file's content in the store
    openFiles.update((files) =>
      files.map((f) => (f.path === path ? { ...f, content: newContent, isDirty: false } : f))
    );
    diffState = null;
  }

  function handleReject() {
    if (!diffState) return;
    // File has been reverted on disk by DiffView; update store content to match original
    const path = diffState.path;
    const originalContent = diffState.original;
    openFiles.update((files) =>
      files.map((f) => (f.path === path ? { ...f, content: originalContent, isDirty: false } : f))
    );
    diffState = null;
  }
</script>

{#if showWelcome}
  <WelcomeScreen {adapter} onOpenFolder={openFolder} onSelectProject={switchProject} />
{:else}
  <App {adapter}>
    {#snippet fileTree()}
      <FileTree {adapter} />
    {/snippet}

    {#snippet editor()}
      <EditorTabs>
        {#snippet actions()}
          {#if $activeFilePath}
            {#if isMarkdown}
              <button
                class="editor-action"
                class:active={showMarkdownPreview}
                onclick={() => (showMarkdownPreview = !showMarkdownPreview)}
              >
                {showMarkdownPreview ? $tr('editor.code') : $tr('editor.preview')}
              </button>
            {/if}
            <button
              class="editor-action"
              class:active={!editorReadOnly}
              onclick={() => (editorReadOnly = !editorReadOnly)}
            >
              {editorReadOnly ? $tr('editor.readOnly') : $tr('editor.editing')}
            </button>
          {/if}
        {/snippet}
      </EditorTabs>
      {#if showHelp}
        <HelpPanel />
      {:else if diffState}
        <DiffView
          original={diffState.original}
          modified={diffState.modified}
          filename={diffState.filename}
          {adapter}
          filePath={diffState.path}
          onAccept={handleAccept}
          onReject={handleReject}
        />
      {:else}
        <Editor {adapter} readOnly={editorReadOnly} {showMarkdownPreview} />
      {/if}
    {/snippet}

    {#snippet terminal()}
      <TerminalManager {adapter} cwd={currentProjectRoot} />
    {/snippet}
  </App>
{/if}

{#if $showSettings}
  <Settings
    {adapter}
    visible={$showSettings}
    onClose={() => showSettings.set(false)}
  />
{/if}

<SearchPalette
  {adapter}
  visible={showSearchPalette}
  onClose={() => (showSearchPalette = false)}
/>

<FindInFiles
  {adapter}
  visible={showFindInFiles}
  onClose={() => (showFindInFiles = false)}
/>

{#if swarmVisible}
  <div class="swarm-overlay">
    <SwarmCanvas {adapter} />
  </div>
{/if}

{#if showAbout}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="about-overlay" onclick={() => showAbout = false} onkeydown={(e) => { if (e.key === 'Escape') showAbout = false; }}>
    <div class="about-dialog" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <div class="about-logo">REASONANCE</div>
      <div class="about-subtitle">IDE for Vibecoders</div>
      <div class="about-version">v0.1.0</div>
      <div class="about-stack">Tauri v2 + Svelte 5 + CodeMirror 6</div>
      <div class="about-license">MIT License</div>
      <button class="about-close" onclick={() => showAbout = false}>OK</button>
    </div>
  </div>
{/if}

<ShortcutsDialog visible={showShortcuts} onClose={() => { showShortcuts = false; }} />

<Toast />

<style>
  .swarm-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 1000;
    background: var(--bg-primary);
  }

  .about-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    z-index: 2000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .about-dialog {
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    padding: 32px 40px;
    text-align: center;
    font-family: var(--font-ui);
    min-width: 280px;
  }

  .about-logo {
    font-size: 22px;
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: -0.02em;
    margin-bottom: 4px;
  }

  .about-subtitle {
    font-size: var(--font-size-base);
    color: var(--text-secondary);
    margin-bottom: 16px;
  }

  .about-version {
    font-size: var(--font-size-small);
    color: var(--accent);
    font-weight: 700;
    margin-bottom: 4px;
  }

  .about-stack {
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .about-license {
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin-bottom: 20px;
  }

  .about-close {
    background: var(--accent);
    border: none;
    color: #fff;
    padding: 6px 24px;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    cursor: pointer;
  }

  .about-close:hover {
    opacity: 0.85;
  }

  .editor-action {
    background: var(--bg-tertiary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    padding: 2px 8px;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
    white-space: nowrap;
  }

  .editor-action:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
  }

  .editor-action.active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

</style>
