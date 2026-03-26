<script lang="ts">
  import App from '$lib/components/App.svelte';
  import FileTree from '$lib/components/FileTree.svelte';
  import EditorTabs from '$lib/components/EditorTabs.svelte';
  import Editor from '$lib/components/Editor.svelte';
  import DiffView from '$lib/components/DiffView.svelte';
  import HelpPanel from '$lib/components/HelpPanel.svelte';
  import TerminalManager from '$lib/components/TerminalManager.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import ThemeEditor from '$lib/components/theme-editor/ThemeEditor.svelte';
  import SearchPalette from '$lib/components/SearchPalette.svelte';
  import FindInFiles from '$lib/components/FindInFiles.svelte';
  import WelcomeScreen from '$lib/components/WelcomeScreen.svelte';
  import { TauriAdapter } from '$lib/adapter/tauri';
  import { initThemeEngine } from '$lib/stores/theme';
  import { openFiles, activeFilePath, projectRoot } from '$lib/stores/files';
  import { addProject, removeProject, activeProjectId, setActiveFile, updateFileContent, updateFileState } from '$lib/stores/projects';
  import { showSettings, enhancedReadability, showHiveCanvas, showThemeEditor } from '$lib/stores/ui';
  import { activeInstance } from '$lib/stores/terminals';
  import { llmConfigs } from '$lib/stores/config';
  import { initI18n, tr } from '$lib/i18n/index';
  import { registerKeybinding, initKeybindings } from '$lib/utils/keybindings';
  import { sanitizeId } from '$lib/utils/a11y';
  import { showToast } from '$lib/stores/toast';
  import { appAnnouncer } from '$lib/utils/a11y-announcer';
  import HiveCanvas from '$lib/components/hive/HiveCanvas.svelte';
  import ShortcutsDialog from '$lib/components/ShortcutsDialog.svelte';
  import SessionPanel from '$lib/components/SessionPanel.svelte';
  import { saveSession, restoreSession, initShadowTracking } from '$lib/utils/session';
  import { loadInitialConfig, discoverAndApplyLlms } from '$lib/utils/config-bootstrap';
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { attachConsole } from '@tauri-apps/plugin-log';
  import { listen } from '@tauri-apps/api/event';
  import '../app.css';

  function isActiveSessionYolo(): boolean {
    const inst = get(activeInstance);
    if (!inst) return false;
    const config = get(llmConfigs).find((c) => c.name === inst.provider);
    return config?.permissionLevel === 'yolo';
  }

  interface DiffState {
    path: string;
    original: string;
    modified: string;
    filename: string;
  }

  const adapter = new TauriAdapter();

  let diffState = $state<(DiffState & { isUserSave?: boolean }) | null>(null);
  let unwatchFiles: (() => void) | null = null;
  let showSearchPalette = $state(false);
  let showFindInFiles = $state(false);
  let showWelcome = $state(true);
  let showHelp = $state(false);
  let showAbout = $state(false);
  let showShortcuts = $state(false);
  let showSessions = $state(false);
  let editorReadOnly = $state(false);
  let showMarkdownPreview = $state(false);
  let hiveVisible = $state(false);
  const unsubHive = showHiveCanvas.subscribe((val) => { hiveVisible = val; });
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

  // ── Save File ─────────────────────────────────────────────────────────────

  async function saveActiveFile() {
    const path = get(activeFilePath);
    if (!path) return;
    const files = get(openFiles);
    const file = files.find((f) => f.path === path);
    if (!file || !file.isDirty) return;

    // Get original content (shadow) to show diff
    const shadow = await adapter.getShadow(path);
    const original = shadow ?? '';

    if (original === file.content) {
      // No actual change vs saved — just clear dirty flag
      updateFileContent(path, file.content, false);
      return;
    }

    // Show diff for user to review before saving
    diffState = {
      path,
      original,
      modified: file.content,
      filename: file.name,
      isUserSave: true,
    };
  }

  async function saveAllFiles() {
    const files = get(openFiles).filter((f) => f.isDirty);
    for (const file of files) {
      try {
        await adapter.writeFile(file.path, file.content);
        await adapter.storeShadow(file.path, file.content);
      } catch (err) {
        showToast('error', 'Save failed', `${file.name}: ${err}`);
      }
    }
    for (const file of files) {
      updateFileContent(file.path, file.content, false);
    }
  }

  // ── File Watcher ──────────────────────────────────────────────────────────

  async function setupFileWatcher(root: string): Promise<(() => void) | undefined> {
    return adapter.watchFiles(root, async (event) => {
      // Notify FileTree about filesystem changes
      if (event.type === 'create' || event.type === 'remove') {
        document.dispatchEvent(new CustomEvent('reasonance:fsChange', { detail: event }));
      }

      const currentFiles = get(openFiles);
      const openFile = currentFiles.find((f) => f.path === event.path);
      if (!openFile) return;

      if (event.type === 'remove') {
        updateFileState(event.path, { isDeleted: true });
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

          if (isActiveSessionYolo()) {
            // AUTO/YOLO: accept changes silently
            await adapter.storeShadow(event.path, newContent);
            updateFileContent(event.path, newContent, false);
          } else {
            diffState = {
              path: event.path,
              original: shadow,
              modified: newContent,
              filename: event.path.split('/').pop() ?? event.path,
            };
            setActiveFile(event.path);
          }
        } catch { /* non-fatal */ }
      }
    });
  }

  // ── Open Folder / Switch Project ─────────────────────────────────────────

  async function openFolder() {
    const selected = await adapter.openFolderDialog();
    if (selected) {
      addProject(selected);
      const currentId = get(activeProjectId);
      if (currentId) { try { await adapter.addProject(currentId, selected, 'trusted'); } catch { /* non-fatal */ } }
      try { await adapter.setProjectRoot(selected); } catch { /* non-fatal */ }
      await switchProject(selected);
    }
  }

  async function switchProject(path: string) {
    addProject(path);
    showWelcome = false;
    try { await adapter.setProjectRoot(path); } catch { /* non-fatal */ }

    // Restart file watcher for new directory
    if (unwatchFiles) unwatchFiles();
    unwatchFiles = await setupFileWatcher(path) ?? null;
  }

  // Shadow tracking subscription — initialised in onMount, cleaned up in onDestroy
  let unsubFiles: () => void;

  onMount(async () => {
    // Attach console to Tauri log plugin — forwards console.* to Rust backend + log file
    const detachConsole = await attachConsole();
    console.info('[Reasonance] Frontend log bridge attached');

    await initThemeEngine();

    // Initialize i18n before restoring session
    await initI18n();

    // Mount app-level screen reader announcer
    appAnnouncer.mount(document.body);

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
    registerKeybinding('ctrl+shift+h', () => { showSessions = true; });
    registerKeybinding('ctrl+s', () => saveActiveFile());
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

    // Listen for save events from MenuBar / EditorTabs
    const handleSave = () => saveActiveFile();
    document.addEventListener('reasonance:save', handleSave);
    cleanups.push(() => document.removeEventListener('reasonance:save', handleSave));

    const handleSaveAll = () => saveAllFiles();
    document.addEventListener('reasonance:saveAll', handleSaveAll);
    cleanups.push(() => document.removeEventListener('reasonance:saveAll', handleSaveAll));

    // Listen for close-project custom event
    const handleCloseProject = async () => {
      const id = get(activeProjectId);
      if (!id) return;
      try { await adapter.killProjectProcesses(id); } catch { /* non-fatal */ }
      try { await adapter.removeProject(id); } catch { /* non-fatal */ }
      removeProject(id);
    };
    document.addEventListener('reasonance:closeProject', handleCloseProject);
    cleanups.push(() => document.removeEventListener('reasonance:closeProject', handleCloseProject));

    // Listen for CLI-initiated project open
    const unlistenCli = await listen<string>('cli-open-project', (event) => {
      addProject(event.payload);
    });
    cleanups.push(unlistenCli);

    // Start watching the project directory for external changes
    const root = get(projectRoot);
    if (root) {
      unwatchFiles = await setupFileWatcher(root) ?? null;
    } // end if (root)
  });

  onDestroy(() => {
    unsubFiles?.();
    unsubEnhanced?.();
    unsubHive();
    unsubRoot();
    appAnnouncer.destroy();
    cleanups.forEach((fn) => fn());
    if (unwatchFiles) unwatchFiles();
  });

  async function handleAccept() {
    if (!diffState) return;
    const path = diffState.path;
    const newContent = diffState.modified;

    // If this is a user-initiated save, write the file to disk
    if (diffState.isUserSave) {
      try {
        await adapter.writeFile(path, newContent);
        await adapter.storeShadow(path, newContent);
      } catch (err) {
        showToast('error', 'Save failed', String(err));
        diffState = null;
        return;
      }
      const fileName = path.split('/').pop() ?? path;
      appAnnouncer.announce(`File ${fileName} saved`);
    }

    // Update the open file's content in the store
    updateFileContent(path, newContent, false);
    diffState = null;
  }

  function handleReject() {
    if (!diffState) return;
    // File has been reverted on disk by DiffView; update store content to match original
    const path = diffState.path;
    const originalContent = diffState.original;
    updateFileContent(path, originalContent, false);
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
      {#if hiveVisible}
        <HiveCanvas {adapter} />
      {:else}
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
        <div role="tabpanel" id="tabpanel-editor" aria-labelledby={$activeFilePath ? `tab-${sanitizeId($activeFilePath)}` : undefined}>
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
        </div>
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

<ThemeEditor
  open={$showThemeEditor}
  onClose={() => showThemeEditor.set(false)}
/>

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


{#if showAbout}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="about-overlay" onclick={() => showAbout = false} onkeydown={(e) => { if (e.key === 'Escape') showAbout = false; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="about-dialog" role="dialog" tabindex="-1" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <svg class="about-icon" viewBox="0 0 250.44 250.44" aria-hidden="true">
        <g transform="translate(25.222,12.197)"><g transform="translate(0,-40.497)">
          <path d="m 0,235.4972 1,-0.02 1,-0.02 1,-0.02 1,-0.03 1,-0.02 1,-0.03 1,-0.02 1,-0.03 1,-0.03 1,-0.02 1,-0.03 1,-0.03 1,-0.03 1,-0.04 1,-0.03 1,-0.03 1,-0.04 1,-0.04 1,-0.03 1,-0.04 1,-0.04 1,-0.05 1,-0.04 1,-0.05 1,-0.05 1,-0.05 1,-0.05 1,-0.05 1,-0.06 1,-0.06 1,-0.06 1,-0.06 1,-0.07 1,-0.07 1,-0.07 1,-0.08 1,-0.08 1,-0.08 1,-0.09 1,-0.09 1,-0.1 1,-0.1 1,-0.11 1,-0.11 1,-0.12 1,-0.12 1,-0.14 1,-0.14 1,-0.14 1,-0.16 1,-0.16 1,-0.18 1,-0.19 1,-0.19 1,-0.22 1,-0.22 1,-0.24 1,-0.26 1,-0.27 1,-0.3 1,-0.31 1,-0.34 1,-0.37 1,-0.39 1,-0.43 1,-0.47 1,-0.5 1,-0.55 1,-0.59 1,-0.65 1,-0.72 1,-0.78 1,-0.87 1,-0.95 1,-1.06 1,-1.17 1,-1.31 1,-1.46 1,-1.64 1,-1.85 1,-2.09 1,-2.37 1,-2.7 1,-3.08 1,-3.54 1,-4.08 1,-4.7 1,-5.45 1,-6.31 1,-7.31 1,-8.44 1,-9.67 1,-10.95 1,-12.15 1,-13.05 1,-13.35 1,-12.660002 1,-10.63 1,-7.17 1,-2.54 1,2.54 1,7.17 1,10.63 1,12.660002 1,13.35 1,13.05 1,12.15 1,10.95 1,9.67 1,8.44 1,7.31 1,6.31 1,5.45 1,4.7 1,4.08 1,3.54 1,3.08 1,2.7 1,2.37 1,2.09 1,1.85 1,1.64 1,1.46 1,1.31 1,1.17 1,1.06 1,0.95 1,0.87 1,0.78 1,0.72 1,0.65 1,0.59 1,0.55 1,0.5 1,0.47 1,0.43 1,0.39 1,0.37 1,0.34 1,0.31 1,0.3 1,0.27 1,0.26 1,0.24 1,0.22 1,0.22 1,0.19 1,0.19 1,0.18 1,0.16 1,0.16 1,0.14 1,0.14 1,0.14 1,0.12 1,0.12 1,0.11 1,0.11 1,0.1 1,0.1 1,0.09 1,0.09 1,0.08 1,0.08 1,0.08 1,0.07 1,0.07 1,0.07 1,0.06 1,0.06 1,0.06 1,0.06 1,0.05 1,0.05 1,0.05 1,0.05 1,0.05 1,0.04 1,0.05 1,0.04 1,0.04 1,0.03 1,0.04 1,0.04 1,0.03 1,0.03 1,0.04 1,0.03 1,0.03 1,0.03 1,0.02 1,0.03 1,0.03 1,0.02 1,0.03 1,0.02 1,0.03 1,0.02 1,0.02 1,0.02" stroke="currentColor" stroke-width="5.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="m 0,235.4972 1,-0.15 1,-0.14 1,-0.16 1,-0.15 1,-0.16 1,-0.17 1,-0.17 1,-0.17 1,-0.18 1,-0.19 1,-0.19 1,-0.19 1,-0.21 1,-0.2 1,-0.22 1,-0.22 1,-0.23 1,-0.23 1,-0.25 1,-0.25 1,-0.26 1,-0.27 1,-0.27 1,-0.29 1,-0.29 1,-0.31 1,-0.32 1,-0.32 1,-0.34 1,-0.35 1,-0.37 1,-0.37 1,-0.39 1,-0.41 1,-0.41 1,-0.44 1,-0.45 1,-0.47 1,-0.48 1,-0.51 1,-0.52 1,-0.55 1,-0.56 1,-0.59 1,-0.61 1,-0.64 1,-0.66 1,-0.69 1,-0.72 1,-0.75 1,-0.78 1,-0.81 1,-0.84 1,-0.89 1,-0.92 1,-0.95 1,-1 1,-1.05 1,-1.09 1,-1.13 1,-1.18 1,-1.24 1,-1.29 1,-1.34 1,-1.4 1,-1.45 1,-1.52 1,-1.58 1,-1.64 1,-1.71 1,-1.78 1,-1.84 1,-1.91 1,-1.97 1,-2.04 1,-2.11 1,-2.17 1,-2.22 1,-2.28 1,-2.33 1,-2.37 1,-2.4 1,-2.42 1,-2.43 1,-2.44 1,-2.41 1,-2.38 1,-2.33 1,-2.25 1,-2.16 1,-2.05 1,-1.91 1,-1.74 1,-1.57 1,-1.37 1,-1.14 1,-0.91 1,-0.66 1,-0.4 1,-0.13 1,0.13 1,0.4 1,0.66 1,0.91 1,1.14 1,1.37 1,1.57 1,1.74 1,1.91 1,2.05 1,2.16 1,2.25 1,2.33 1,2.38 1,2.41 1,2.44 1,2.43 1,2.42 1,2.4 1,2.37 1,2.33 1,2.28 1,2.22 1,2.17 1,2.11 1,2.04 1,1.97 1,1.91 1,1.84 1,1.78 1,1.71 1,1.64 1,1.58 1,1.52 1,1.45 1,1.4 1,1.34 1,1.29 1,1.24 1,1.18 1,1.13 1,1.09 1,1.05 1,1 1,0.95 1,0.92 1,0.89 1,0.84 1,0.81 1,0.78 1,0.75 1,0.72 1,0.69 1,0.66 1,0.64 1,0.61 1,0.59 1,0.56 1,0.55 1,0.52 1,0.51 1,0.48 1,0.47 1,0.45 1,0.44 1,0.41 1,0.41 1,0.39 1,0.37 1,0.37 1,0.35 1,0.34 1,0.32 1,0.32 1,0.31 1,0.29 1,0.29 1,0.27 1,0.27 1,0.26 1,0.25 1,0.25 1,0.23 1,0.23 1,0.22 1,0.22 1,0.2 1,0.21 1,0.19 1,0.19 1,0.19 1,0.18 1,0.17 1,0.17 1,0.17 1,0.16 1,0.15 1,0.16 1,0.14 1,0.15" stroke="currentColor" stroke-width="5.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="m 0,235.4972 1,-0.18 1,-0.19 1,-0.19 1,-0.2 1,-0.2 1,-0.21 1,-0.21 1,-0.22 1,-0.22 1,-0.23 1,-0.23 1,-0.24 1,-0.25 1,-0.25 1,-0.26 1,-0.26 1,-0.28 1,-0.27 1,-0.29 1,-0.29 1,-0.3 1,-0.31 1,-0.32 1,-0.32 1,-0.34 1,-0.34 1,-0.35 1,-0.36 1,-0.37 1,-0.38 1,-0.39 1,-0.4 1,-0.41 1,-0.42 1,-0.43 1,-0.44 1,-0.46 1,-0.47 1,-0.48 1,-0.49 1,-0.51 1,-0.51 1,-0.54 1,-0.54 1,-0.56 1,-0.58 1,-0.59 1,-0.6 1,-0.62 1,-0.64 1,-0.65 1,-0.67 1,-0.68 1,-0.7 1,-0.71 1,-0.73 1,-0.75 1,-0.76 1,-0.78 1,-0.79 1,-0.81 1,-0.82 1,-0.84 1,-0.85 1,-0.87 1,-0.88 1,-0.89 1,-0.9 1,-0.91 1,-0.92 1,-0.93 1,-0.93 1,-0.93 1,-0.94 1,-0.94 1,-0.94 1,-0.93 1,-0.92 1,-0.92 1,-0.9 1,-0.89 1,-0.86 1,-0.85 1,-0.82 1,-0.8 1,-0.76 1,-0.73 1,-0.69 1,-0.66 1,-0.6 1,-0.56 1,-0.51 1,-0.45 1,-0.4 1,-0.35 1,-0.28 1,-0.22 1,-0.16 1,-0.1 1,-0.03 1,0.03 1,0.1 1,0.16 1,0.22 1,0.28 1,0.35 1,0.4 1,0.45 1,0.51 1,0.56 1,0.6 1,0.66 1,0.69 1,0.73 1,0.76 1,0.8 1,0.82 1,0.85 1,0.86 1,0.89 1,0.9 1,0.92 1,0.92 1,0.93 1,0.94 1,0.94 1,0.94 1,0.93 1,0.93 1,0.93 1,0.92 1,0.91 1,0.9 1,0.89 1,0.88 1,0.87 1,0.85 1,0.84 1,0.82 1,0.81 1,0.79 1,0.78 1,0.76 1,0.75 1,0.73 1,0.71 1,0.7 1,0.68 1,0.67 1,0.65 1,0.64 1,0.62 1,0.6 1,0.59 1,0.58 1,0.56 1,0.54 1,0.54 1,0.51 1,0.51 1,0.49 1,0.48 1,0.47 1,0.46 1,0.44 1,0.43 1,0.42 1,0.41 1,0.4 1,0.39 1,0.38 1,0.37 1,0.36 1,0.35 1,0.34 1,0.34 1,0.32 1,0.32 1,0.31 1,0.3 1,0.29 1,0.29 1,0.27 1,0.28 1,0.26 1,0.26 1,0.25 1,0.25 1,0.24 1,0.23 1,0.23 1,0.22 1,0.22 1,0.21 1,0.21 1,0.2 1,0.2 1,0.19 1,0.19 1,0.18" stroke="currentColor" stroke-width="5.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        </g></g>
      </svg>
      <div class="about-logo">REASONANCE</div>
      <div class="about-subtitle">{$tr('about.subtitle')}</div>
      <div class="about-version">v{__APP_VERSION__}</div>
      <div class="about-stack">{$tr('about.stack')}</div>
      <div class="about-license">{$tr('about.license')}</div>
      <button class="about-close" onclick={() => showAbout = false}>OK</button>
    </div>
  </div>
{/if}

<ShortcutsDialog visible={showShortcuts} onClose={() => { showShortcuts = false; }} />

<SessionPanel
  {adapter}
  visible={showSessions}
  onClose={() => { showSessions = false; }}
  onRestore={(id) => { /* TODO: wire session restore */ }}
/>

<style>

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
    padding: var(--space-6) var(--space-6);
    text-align: center;
    font-family: var(--font-ui);
    min-width: 280px;
  }

  .about-icon {
    width: 56px;
    height: 56px;
    color: var(--text-primary);
    margin-bottom: var(--space-3);
  }

  .about-logo {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-hero);
    line-height: var(--line-height-lg);
    color: var(--text-primary);
    letter-spacing: -0.02em;
    margin-bottom: var(--space-1);
  }

  .about-subtitle {
    font-size: var(--font-size-base);
    color: var(--text-secondary);
    margin-bottom: var(--space-4);
  }

  .about-version {
    font-size: var(--font-size-small);
    color: var(--accent-text);
    font-weight: 700;
    margin-bottom: var(--space-1);
  }

  .about-stack {
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin-bottom: var(--space-1);
  }

  .about-license {
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin-bottom: var(--space-5);
  }

  .about-close {
    background: var(--accent);
    border: none;
    color: var(--text-on-accent);
    padding: var(--space-2) var(--space-5);
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
    font-size: var(--font-size-sm);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    padding: var(--space-1) var(--space-2);
    cursor: pointer;
    transition: background var(--transition-fast), color var(--transition-fast);
    white-space: nowrap;
  }

  .editor-action:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
  }

  .editor-action.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }

</style>
