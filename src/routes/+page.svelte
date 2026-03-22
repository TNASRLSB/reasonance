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
  import { initTheme, themeMode } from '$lib/stores/theme';
  import { openFiles, activeFilePath, addOpenFile, projectRoot, recentProjects, addRecentProject } from '$lib/stores/files';
  import { llmConfigs, appSettings } from '$lib/stores/config';
  import { showSettings, fontFamily, fontSize, enhancedReadability } from '$lib/stores/ui';
  import { terminalTabs } from '$lib/stores/terminals';
  import { registerKeybinding, initKeybindings } from '$lib/utils/keybindings';
  import Toast from '$lib/components/Toast.svelte';
  import { showToast } from '$lib/stores/toast';
  import SwarmCanvas from '$lib/components/swarm/SwarmCanvas.svelte';
  import { showSwarmCanvas } from '$lib/stores/ui';
  import { initI18n, locale, loadLocale, t } from '$lib/i18n/index';
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { parse } from 'smol-toml';
  import { parseLlmConfig } from '$lib/utils/config-parser';
  import { invoke } from '@tauri-apps/api/core';
  import { load } from '@tauri-apps/plugin-store';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/plugin-dialog';
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
  let swarmVisible = $state(false);
  const unsubSwarm = showSwarmCanvas.subscribe((val) => { swarmVisible = val; });
  let unsubEnhanced: () => void;

  // Reactive projectRoot for passing as cwd
  let currentProjectRoot = $state('.');
  const unsubRoot = projectRoot.subscribe((v) => { currentProjectRoot = v || '.'; });

  // Cleanup array for event listeners
  const cleanups: Array<() => void> = [];

  // ── Open Folder / Switch Project ─────────────────────────────────────────

  async function openFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      await switchProject(typeof selected === 'string' ? selected : selected[0]);
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

  // ── Session persistence ──────────────────────────────────────────────────
  // Saved as: openFiles[], activeFile, theme, fontFamily, fontSize, terminalTabs[], projectRoot, recentProjects

  async function saveSession() {
    try {
      const store = await load('session.json', { autoSave: false });

      const currentFiles = get(openFiles);
      await store.set('openFiles', currentFiles.map((f) => f.path));
      await store.set('activeFile', get(activeFilePath));
      await store.set('theme', get(themeMode));
      await store.set('fontFamily', get(fontFamily));
      await store.set('fontSize', get(fontSize));
      await store.set('enhancedReadability', get(enhancedReadability));
      await store.set('projectRoot', get(projectRoot));
      await store.set('recentProjects', get(recentProjects));

      const currentLocale = get(locale);
      await store.set('locale', currentLocale);

      // Terminal tabs: only metadata (llmName, instance count) — PTY sessions can't survive restart
      const tabs = get(terminalTabs);
      await store.set(
        'terminalTabs',
        tabs.map((t) => ({ llmName: t.llmName, instanceCount: t.instances.length }))
      );

      await store.save();
    } catch {
      // Session save failures are non-fatal
    }
  }

  async function restoreSession() {
    try {
      const store = await load('session.json', { autoSave: false });

      // Restore theme
      const savedTheme = await store.get<string>('theme');
      if (savedTheme && ['light', 'dark', 'system'].includes(savedTheme)) {
        themeMode.set(savedTheme as import('$lib/stores/theme').ThemeMode);
      }

      // Restore font settings
      const savedFontFamily = await store.get<string>('fontFamily');
      if (savedFontFamily) fontFamily.set(savedFontFamily);

      const savedFontSize = await store.get<number>('fontSize');
      if (savedFontSize && savedFontSize > 0) fontSize.set(savedFontSize);

      const savedEnhancedReadability = await store.get<boolean>('enhancedReadability');
      if (savedEnhancedReadability !== null && savedEnhancedReadability !== undefined) {
        enhancedReadability.set(savedEnhancedReadability);
      }

      // Restore locale
      const savedLocale = await store.get<string>('locale');
      if (savedLocale && ['en','it','de','es','fr','pt','zh','hi','ar'].includes(savedLocale)) {
        await loadLocale(savedLocale as any);
        locale.set(savedLocale as any);
      }

      // Restore project root and recent projects
      const savedRoot = await store.get<string>('projectRoot');
      if (savedRoot) {
        projectRoot.set(savedRoot);
        showWelcome = false;
        try { await adapter.setProjectRoot(savedRoot); } catch { /* non-fatal */ }
      }

      const savedRecent = await store.get<string[]>('recentProjects');
      if (savedRecent) recentProjects.set(savedRecent);

      // Restore open files
      const savedPaths = await store.get<string[]>('openFiles');
      const savedActive = await store.get<string | null>('activeFile');

      if (savedPaths && savedPaths.length > 0) {
        for (const path of savedPaths) {
          try {
            const content = await adapter.readFile(path);
            const name = path.split('/').pop() ?? path;
            addOpenFile({ path, name, content, isDirty: false, isDeleted: false });
          } catch {
            // File may have been deleted since last session — skip silently
          }
        }
      }

      // Set active file after all are loaded (addOpenFile already sets the last one)
      if (savedActive) {
        activeFilePath.set(savedActive);
      }
    } catch {
      // Restore failures are non-fatal — start fresh
    }
  }

  // Track known open file paths so we can store shadows when new files are opened
  let knownPaths = new Set<string>();

  // Subscribe to openFiles to store shadow on first open
  const unsubFiles = openFiles.subscribe(async (files) => {
    for (const file of files) {
      if (!knownPaths.has(file.path)) {
        knownPaths.add(file.path);
        try {
          await adapter.storeShadow(file.path, file.content);
        } catch {
          // Shadow store failures are non-fatal
        }
      }
    }
  });

  async function loadInitialConfig() {
    try {
      const raw = await adapter.readConfig();
      if (!raw || !raw.trim()) return;

      const parsed = parse(raw) as {
        settings?: { default?: string; context_menu_llm?: string };
        llm?: Array<Record<string, unknown>>;
      };

      const rawLlms = parsed.llm ?? [];
      llmConfigs.set(parseLlmConfig(rawLlms));

      const s = parsed.settings ?? {};
      appSettings.set({
        default: s.default !== undefined ? String(s.default) : undefined,
        contextMenuLlm:
          s.context_menu_llm !== undefined ? String(s.context_menu_llm) : undefined,
      });

      // Apply persisted app settings from config if present
      // (font family/size and theme are managed by stores; no TOML fields for them yet)
    } catch (err) {
      // Config load failures are non-fatal — continue with defaults
      showToast('error', 'Config parse error', String(err));
    }
  }

  onMount(async () => {
    initTheme();

    // Initialize i18n before restoring session
    await initI18n();

    // Enhanced Readability mode — toggle CSS class on root element
    unsubEnhanced = enhancedReadability.subscribe((on) => {
      document.documentElement.classList.toggle('enhanced-readability', on);
    });

    // Restore persisted session state before anything else
    await restoreSession();

    await loadInitialConfig();

    // Auto-discover installed LLM CLIs if none configured
    {
      const configs = get(llmConfigs);
      if (configs.length === 0) {
        try {
          const discovered = await invoke<Array<{ name: string; command: string; found: boolean }>>(
            'discover_llms'
          );
          const found = discovered.filter((d) => d.found);
          if (found.length > 0) {
            const newConfigs: import('$lib/stores/config').LlmConfig[] = found.map((d) => ({
              name: d.name,
              type: 'cli' as const,
              command: d.command,
              args: [],
              yoloFlag: d.command === 'claude' ? '--dangerously-skip-permissions' : undefined,
              imageMode: 'path' as const,
            }));
            // If Ollama was found, also add it as an API-type LLM (OpenAI-compatible)
            if (found.some((d) => d.command === 'ollama')) {
              newConfigs.push({
                name: 'Ollama (API)',
                type: 'api',
                provider: 'openai',
                endpoint: 'http://localhost:11434/v1',
                model: 'llama3',
                imageMode: 'none',
              });
            }
            llmConfigs.set(newConfigs);

            // Persist discovered LLMs to TOML so Settings can find them
            try {
              const { stringify } = await import('smol-toml');
              const tomlObj: Record<string, unknown> = {
                settings: { default: '', context_menu_llm: '' },
                llm: newConfigs.map((l) => {
                  const entry: Record<string, unknown> = { name: l.name, type: l.type };
                  if (l.type === 'cli') {
                    entry.command = l.command ?? '';
                    entry.args = l.args ?? [];
                    if (l.yoloFlag) entry.yolo_flag = l.yoloFlag;
                    entry.image_mode = l.imageMode ?? 'path';
                  } else {
                    entry.provider = l.provider ?? '';
                    entry.model = l.model ?? '';
                    if (l.endpoint) entry.endpoint = l.endpoint;
                  }
                  return entry;
                }),
              };
              await adapter.writeConfig(stringify(tomlObj));
            } catch { /* non-fatal */ }

            showToast(
              'success',
              t('toast.llmFound'),
              t('toast.detected', { names: found.map((d) => d.name).join(', ') })
            );
          }
        } catch {
          // Silently ignore discovery errors
        }
      }
    }

    // Listen for window close to save session state
    const appWindow = getCurrentWindow();
    appWindow.onCloseRequested(async () => {
      await saveSession();
    });

    // Register global keyboard shortcuts
    registerKeybinding('ctrl+p', () => { showSearchPalette = true; });
    registerKeybinding('ctrl+shift+f', () => { showFindInFiles = true; });
    registerKeybinding('ctrl+,', () => showSettings.set(true));
    registerKeybinding('f1', () => { showHelp = !showHelp; });
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

    // Start watching the project directory for external changes
    const { get } = await import('svelte/store');
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
    unsubFiles();
    unsubEnhanced();
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
  <WelcomeScreen onOpenFolder={openFolder} onSelectProject={switchProject} />
{:else}
  <App {adapter}>
    {#snippet fileTree()}
      <FileTree {adapter} />
    {/snippet}

    {#snippet editor()}
      <EditorTabs />
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
        <Editor {adapter} />
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
</style>
