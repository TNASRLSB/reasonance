<script lang="ts">
  import App from '$lib/components/App.svelte';
  import FileTree from '$lib/components/FileTree.svelte';
  import EditorTabs from '$lib/components/EditorTabs.svelte';
  import Editor from '$lib/components/Editor.svelte';
  import DiffView from '$lib/components/DiffView.svelte';
  import TerminalManager from '$lib/components/TerminalManager.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import SearchPalette from '$lib/components/SearchPalette.svelte';
  import FindInFiles from '$lib/components/FindInFiles.svelte';
  import { TauriAdapter } from '$lib/adapter/tauri';
  import { initTheme, themeMode } from '$lib/stores/theme';
  import { openFiles, activeFilePath, addOpenFile } from '$lib/stores/files';
  import { llmConfigs, appSettings } from '$lib/stores/config';
  import { showSettings, fontFamily, fontSize } from '$lib/stores/ui';
  import { terminalTabs } from '$lib/stores/terminals';
  import { registerKeybinding, initKeybindings } from '$lib/utils/keybindings';
  import Toast from '$lib/components/Toast.svelte';
  import { showToast } from '$lib/stores/toast';
  import { onMount, onDestroy } from 'svelte';
  import { parse } from 'smol-toml';
  import { load } from '@tauri-apps/plugin-store';
  import { getCurrentWindow } from '@tauri-apps/api/window';
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

  // ── Session persistence ──────────────────────────────────────────────────
  // Saved as: openFiles[], activeFile, theme, fontFamily, fontSize, terminalTabs[]

  async function saveSession() {
    try {
      const { get } = await import('svelte/store');
      const store = await load('session.json', { autoSave: false });

      const currentFiles = get(openFiles);
      await store.set('openFiles', currentFiles.map((f) => f.path));
      await store.set('activeFile', get(activeFilePath));
      await store.set('theme', get(themeMode));
      await store.set('fontFamily', get(fontFamily));
      await store.set('fontSize', get(fontSize));

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
      llmConfigs.set(
        rawLlms.map((l) => ({
          name: String(l.name ?? ''),
          type: (l.type === 'api' ? 'api' : 'cli') as 'cli' | 'api',
          command: l.command !== undefined ? String(l.command) : undefined,
          args: Array.isArray(l.args) ? l.args.map(String) : [],
          yoloFlag: l.yolo_flag !== undefined ? String(l.yolo_flag) : undefined,
          imageMode: (['path', 'base64', 'none'].includes(String(l.image_mode))
            ? l.image_mode
            : 'path') as 'path' | 'base64' | 'none',
          provider: l.provider !== undefined ? String(l.provider) : undefined,
          apiKeyEnv: l.api_key_env !== undefined ? String(l.api_key_env) : undefined,
          model: l.model !== undefined ? String(l.model) : undefined,
          endpoint: l.endpoint !== undefined ? String(l.endpoint) : undefined,
        }))
      );

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

    // Restore persisted session state before anything else
    await restoreSession();

    await loadInitialConfig();

    // Listen for window close to save session state
    const appWindow = getCurrentWindow();
    appWindow.onCloseRequested(async () => {
      await saveSession();
    });

    // Register global keyboard shortcuts
    registerKeybinding('ctrl+p', () => { showSearchPalette = true; });
    registerKeybinding('ctrl+shift+f', () => { showFindInFiles = true; });
    registerKeybinding('ctrl+,', () => showSettings.set(true));
    initKeybindings();

    // Start watching the project directory for external changes
    const { get } = await import('svelte/store');
    unwatchFiles = await adapter.watchFiles('.', async (event) => {
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

<App {adapter}>
  {#snippet fileTree()}
    <FileTree {adapter} />
  {/snippet}

  {#snippet editor()}
    <EditorTabs />
    {#if diffState}
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
    <TerminalManager {adapter} />
  {/snippet}
</App>

<Settings
  {adapter}
  visible={$showSettings}
  onClose={() => showSettings.set(false)}
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

<Toast />
