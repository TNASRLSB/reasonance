<script lang="ts">
  import { parse, stringify } from 'smol-toml';
  import { parseLlmConfig } from '$lib/utils/config-parser';
  import type { Adapter } from '$lib/adapter/index';
  import type { LlmConfig, AppSettings } from '$lib/stores/config';
  import { llmConfigs, appSettings } from '$lib/stores/config';
  import { fontFamily, fontSize, enhancedReadability } from '$lib/stores/ui';
  import { themeMode, type ThemeMode, activeThemeName, loadBuiltinTheme, toggleModifier, activeModifierNames, suppressFileWatcher } from '$lib/stores/theme';
  import { validateTheme } from '$lib/engine/theme-validator';
  import { invoke } from '@tauri-apps/api/core';
  import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
  import { readTextFile } from '@tauri-apps/plugin-fs';
  import type { ThemeFile } from '$lib/engine/theme-types';
  import { tr, t, locale, loadLocale, type Locale } from '$lib/i18n/index';
  import { get } from 'svelte/store';
  import { getUpdateSettings, saveUpdateSettings, checkForUpdate, type UpdateMode } from '$lib/updater';
  import { trapFocus } from '$lib/utils/a11y';
  import type { ConnectionTestStep, AnalyticsBudget } from '$lib/types/analytics';
  import { budget, providerConfigVersion } from '$lib/stores/analytics';
  import { getModelsForProvider } from '$lib/data/model-info';
  import { getProviderVisual } from '$lib/utils/provider-patterns';
  import type { TrustEntry } from '$lib/stores/workspace-trust';
  import { workspaceTrustLevel } from '$lib/stores/workspace-trust';

  let {
    adapter,
    visible,
    onClose,
  }: {
    adapter: Adapter;
    visible: boolean;
    onClose: () => void;
  } = $props();

  // Local state
  let llms = $state<LlmConfig[]>([]);
  let settings = $state<AppSettings>({});
  let localFontFamily = $state("'Atkinson Hyperlegible Mono', monospace");
  let localFontSize = $state(14);
  let localTheme = $state<ThemeMode>('system');
  let localActiveTheme = $state('reasonance-dark');
  let localModifiers = $state<string[]>([]);
  let userThemes = $state<string[]>([]);
  let importError = $state('');
  let localEnhancedReadability = $state(false);
  let localLocale = $state<Locale>('en');
  let dialogEl = $state<HTMLElement | null>(null);

  $effect(() => {
    if (visible && dialogEl) {
      const destroy = trapFocus(dialogEl);
      return destroy;
    }
  });

  const localeLabels: Record<Locale, string> = {
    en: 'English',
    it: 'Italiano',
    de: 'Deutsch',
    es: 'Español',
    fr: 'Français',
    pt: 'Português',
    zh: '中文',
    hi: 'हिन्दी',
    ar: 'العربية',
  };

  // Add LLM form state
  let showAddForm = $state(false);
  let editingIndex = $state<number | null>(null);
  // Tracks indices of LLMs marked for deletion (pending until Save)
  let pendingDeleteIndices = $state<Set<number>>(new Set());

  let newLlm = $state<LlmConfig>({
    name: '',
    type: 'cli',
    command: '',
    args: [],
    yoloFlag: '',
    imageMode: 'path',
    permissionLevel: 'ask',
    allowedTools: [],
    provider: 'anthropic',
    apiKeyEnv: '',
    model: '',
    endpoint: '',
  });
  let newLlmArgsStr = $state('');
  let newLlmAllowedToolsStr = $state('');

  let saving = $state(false);
  let error = $state('');
  let localAutoUpdate = $state(true);
  let localUpdateMode = $state<UpdateMode>('notify');
  let checkingUpdate = $state(false);

  // Provider section state
  let expandedProvider = $state<string | null>(null);
  let connectionSteps = $state<Map<string, ConnectionTestStep[]>>(new Map());
  let testingProvider = $state<string | null>(null);
  let localBudget = $state<AnalyticsBudget>({ daily_limit_usd: null, weekly_limit_usd: null, notify_at_percent: 80 });
  let capturingShortcut = $state<string | null>(null);
  let trustEntries = $state<TrustEntry[]>([]);

  $effect(() => {
    adapter.listWorkspaceTrust().then((entries) => {
      trustEntries = entries;
    }).catch((e) => console.warn('Failed to load trust entries:', e));
  });

  async function handleRevokeTrust(hash: string) {
    await adapter.revokeWorkspaceTrust(hash);
    trustEntries = trustEntries.filter((e) => e.hash !== hash);
    // Trigger reactive suspension for active sessions in this workspace
    workspaceTrustLevel.set(null);
  }

  function handleDefaultPermissionChange(e: Event) {
    const value = (e.target as HTMLSelectElement).value as 'yolo' | 'ask' | 'locked';
    appSettings.update((s) => ({ ...s, defaultPermissionLevel: value }));
  }

  // Load config when visible
  $effect(() => {
    if (visible) {
      loadConfig();
    }
  });

  async function loadConfig() {
    error = '';
    try {
      const raw = await adapter.readConfig();
      if (raw && raw.trim()) {
        const parsed = parse(raw) as {
          settings?: { default?: string; context_menu_llm?: string };
          llm?: Array<Record<string, unknown>>;
        };

        const rawLlms = parsed.llm ?? [];
        llms = parseLlmConfig(rawLlms);

        // If TOML has no LLMs but store has auto-discovered ones, use store
        if (llms.length === 0) {
          const storeConfigs = get(llmConfigs);
          if (storeConfigs.length > 0) {
            llms = [...storeConfigs];
          }
        }

        const s = parsed.settings ?? {};
        settings = {
          default: s.default !== undefined ? String(s.default) : undefined,
          contextMenuLlm:
            s.context_menu_llm !== undefined ? String(s.context_menu_llm) : undefined,
        };
      } else {
        // No config file — use in-memory store (may have auto-discovered LLMs)
        llms = get(llmConfigs);
        settings = get(appSettings);
      }
    } catch (e) {
      console.error('Settings loadConfig error:', e);
      error = t('settings.configError');
      llms = get(llmConfigs);
      settings = get(appSettings);
    }
    localFontFamily = get(fontFamily);
    localFontSize = get(fontSize);
    localTheme = get(themeMode);
    localActiveTheme = get(activeThemeName);
    localModifiers = [...get(activeModifierNames)];
    localEnhancedReadability = get(enhancedReadability);
    try {
      userThemes = await invoke<string[]>('list_user_themes');
    } catch {
      userThemes = [];
    }
    importError = '';
    localLocale = get(locale);
    pendingDeleteIndices = new Set();
    const updateSettings = await getUpdateSettings();
    localAutoUpdate = updateSettings.autoUpdate;
    localUpdateMode = updateSettings.updateMode;
    localBudget = { ...get(budget) };
  }

  function startAdd() {
    editingIndex = null;
    newLlm = {
      name: '',
      type: 'cli',
      command: '',
      args: [],
      yoloFlag: '',
      imageMode: 'path',
      permissionLevel: 'ask',
      allowedTools: [],
      provider: 'anthropic',
      apiKeyEnv: '',
      model: '',
      endpoint: '',
    };
    newLlmArgsStr = '';
    newLlmAllowedToolsStr = '';
    showAddForm = true;
  }

  function startEdit(index: number) {
    editingIndex = index;
    const l = llms[index];
    newLlm = { ...l };
    newLlmArgsStr = (l.args ?? []).join(', ');
    newLlmAllowedToolsStr = (l.allowedTools ?? []).join(', ');
    showAddForm = true;
  }

  function cancelForm() {
    showAddForm = false;
    editingIndex = null;
  }

  // Validation helpers
  let commandError = $state('');
  let endpointError = $state('');

  function validateCommand(cmd: string): string {
    if (!cmd.trim()) return '';
    // Must look like a valid command name or path (alphanumeric, hyphens, underscores, slashes, dots)
    if (!/^[a-zA-Z0-9_.\/\\~-][a-zA-Z0-9_.\/\\ -]*$/.test(cmd.trim())) {
      return 'Invalid command path. Use letters, numbers, hyphens, dots, or slashes.';
    }
    return '';
  }

  function validateEndpoint(url: string): string {
    if (!url.trim()) return '';
    try {
      const u = new URL(url.trim());
      if (!['http:', 'https:'].includes(u.protocol)) {
        return 'URL must start with http:// or https://';
      }
    } catch {
      return 'Invalid URL format (e.g. https://api.example.com/v1)';
    }
    return '';
  }

  function submitLlm() {
    if (!newLlm.name.trim()) {
      error = t('settings.llm.nameRequired');
      return;
    }

    // Validate command/endpoint fields
    if (newLlm.type === 'cli') {
      const cmdErr = validateCommand(newLlm.command ?? '');
      if (cmdErr) { commandError = cmdErr; return; }
    } else if (newLlm.provider === 'openai-compatible') {
      const urlErr = validateEndpoint(newLlm.endpoint ?? '');
      if (urlErr) { endpointError = urlErr; return; }
    }

    error = '';
    commandError = '';
    endpointError = '';

    const entry: LlmConfig = {
      name: newLlm.name.trim(),
      type: newLlm.type,
    };

    if (newLlm.type === 'cli') {
      entry.command = newLlm.command ?? '';
      entry.args = newLlmArgsStr
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean);
      entry.yoloFlag = newLlm.yoloFlag ?? '';
      entry.imageMode = newLlm.imageMode ?? 'path';
      entry.permissionLevel = newLlm.permissionLevel ?? 'ask';
      entry.allowedTools = newLlmAllowedToolsStr
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean);
    } else {
      entry.provider = newLlm.provider ?? 'anthropic';
      entry.apiKeyEnv = newLlm.apiKeyEnv ?? '';
      entry.model = newLlm.model ?? '';
      if (newLlm.provider === 'openai-compatible') {
        entry.endpoint = newLlm.endpoint ?? '';
      }
    }

    if (editingIndex !== null) {
      llms = llms.map((l, i) => (i === editingIndex ? entry : l));
    } else {
      llms = [...llms, entry];
    }

    showAddForm = false;
    editingIndex = null;
  }

  function deleteLlm(index: number) {
    if (pendingDeleteIndices.has(index)) {
      // Undo pending delete
      pendingDeleteIndices = new Set([...pendingDeleteIndices].filter((i) => i !== index));
    } else {
      pendingDeleteIndices = new Set([...pendingDeleteIndices, index]);
    }
  }

  async function save() {
    saving = true;
    error = '';

    // Apply pending deletions before saving
    const effectiveLlms = llms.filter((_, i) => !pendingDeleteIndices.has(i));

    try {
      // Build TOML object
      const tomlObj: Record<string, unknown> = {
        settings: {
          default: settings.default ?? '',
          context_menu_llm: settings.contextMenuLlm ?? '',
        },
        llm: effectiveLlms.map((l) => {
          const entry: Record<string, unknown> = {
            name: l.name,
            type: l.type,
          };
          if (l.type === 'cli') {
            entry.command = l.command ?? '';
            entry.args = l.args ?? [];
            entry.yolo_flag = l.yoloFlag ?? '';
            entry.image_mode = l.imageMode ?? 'path';
            if (l.permissionLevel && l.permissionLevel !== 'ask') entry.permission_level = l.permissionLevel;
            if (l.allowedTools?.length) entry.allowed_tools = l.allowedTools;
          } else {
            entry.provider = l.provider ?? 'anthropic';
            entry.api_key_env = l.apiKeyEnv ?? '';
            entry.model = l.model ?? '';
            if (l.provider === 'openai-compatible') {
              entry.endpoint = l.endpoint ?? '';
            }
          }
          return entry;
        }),
      };

      const tomlStr = stringify(tomlObj);
      await adapter.writeConfig(tomlStr);

      await saveUpdateSettings({
        autoUpdate: localAutoUpdate,
        updateMode: localUpdateMode,
      });

      // Apply stores before closing modal
      llmConfigs.set(effectiveLlms);
      appSettings.set(settings);
      budget.set({ ...localBudget });
      fontFamily.set(localFontFamily);
      fontSize.set(localFontSize);
      enhancedReadability.set(localEnhancedReadability);
      themeMode.set(localTheme);
      console.info(`[Settings] Theme save: local="${localActiveTheme}", active="${get(activeThemeName)}"`);
      if (localActiveTheme !== get(activeThemeName)) {
        console.info(`[Settings] Loading theme: ${localActiveTheme}`);
        await loadBuiltinTheme(localActiveTheme);
        console.info(`[Settings] Theme loaded, active now: ${get(activeThemeName)}`);
      }
      if (localLocale !== get(locale)) {
        loadLocale(localLocale).then(() => {
          locale.set(localLocale);
        }).catch((e) => console.warn('Failed to load locale:', e));
      }

      onClose();
    } catch (e) {
      console.error('Settings save error:', e);
      error = t('settings.saveError');
    } finally {
      saving = false;
    }
  }

  async function testConnection(providerName: string) {
    testingProvider = providerName;
    connectionSteps.set(providerName, []);
    connectionSteps = new Map(connectionSteps);

    const unlisten = await adapter.onConnectionTest((step: ConnectionTestStep) => {
      const current = connectionSteps.get(providerName) ?? [];
      connectionSteps.set(providerName, [...current, step]);
      connectionSteps = new Map(connectionSteps);
    });

    try {
      await adapter.testProviderConnection(providerName);
    } catch {
      // Error handling via connection steps
    } finally {
      testingProvider = null;
      unlisten();
    }
  }

  function handleShortcutCapture(e: KeyboardEvent) {
    if (!capturingShortcut) return;
    e.preventDefault();
    e.stopPropagation();
    const parts: string[] = [];
    if (e.ctrlKey) parts.push('Ctrl');
    if (e.shiftKey) parts.push('Shift');
    if (e.altKey) parts.push('Alt');
    if (e.metaKey) parts.push('Meta');
    if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return;
    parts.push(e.key.length === 1 ? e.key.toUpperCase() : e.key);
    const combo = parts.join('+');
    // Store shortcut in the matching LLM entry
    const idx = llms.findIndex(l => l.name === capturingShortcut);
    if (idx >= 0) {
      llms[idx] = { ...llms[idx], shortcut: combo };
      llms = [...llms];
    }
    capturingShortcut = null;
  }

  function handleOverlayClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('settings-overlay')) {
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if visible}
  <div class="settings-overlay" role="presentation" onclick={handleOverlayClick} onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}>
    <div class="settings-modal" role="dialog" aria-modal="true" aria-label={$tr('settings.title')} bind:this={dialogEl}>
      <div class="modal-header">
        <h2>{$tr('settings.title')}</h2>
        <button class="close-btn" onclick={onClose} aria-label={$tr('settings.close')}>✕</button>
      </div>

      {#if error}
        <div class="error-banner" role="alert">{error}</div>
      {/if}

      <div class="modal-body">
        <!-- LLM Configuration -->
        <section>
          <h3>{$tr('settings.llm.title')}</h3>

          <!-- Default LLM selector -->
          <div class="field-row">
            <label for="default-llm">{$tr('settings.llm.default')}</label>
            <select id="default-llm" bind:value={settings.default}>
              <option value="">{$tr('settings.llm.none')}</option>
              {#each llms as l}
                <option value={l.name}>{l.name}</option>
              {/each}
            </select>
          </div>

          <div class="field-row">
            <label for="context-menu-llm">{$tr('settings.llm.contextMenu')}</label>
            <select id="context-menu-llm" bind:value={settings.contextMenuLlm}>
              <option value="">{$tr('settings.llm.none')}</option>
              {#each llms as l}
                <option value={l.name}>{l.name}</option>
              {/each}
            </select>
          </div>

          <!-- LLM list -->
          {#if llms.length > 0}
            <div class="llm-list">
              {#each llms as l, i}
                {@const isPendingDelete = pendingDeleteIndices.has(i)}
                <div class="llm-item" class:pending-delete={isPendingDelete}>
                  <div class="llm-info">
                    <span class="llm-name">{l.name}</span>
                    <span class="llm-type-badge">{l.type}</span>
                    {#if l.type === 'cli'}
                      <span class="llm-detail">{l.command}</span>
                    {:else}
                      <span class="llm-detail">{l.provider} / {l.model}</span>
                    {/if}
                    {#if isPendingDelete}
                      <span class="pending-delete-label">{$tr('settings.llm.pendingDelete')}</span>
                    {/if}
                  </div>
                  <div class="llm-actions">
                    {#if !isPendingDelete}
                      <button onclick={() => startEdit(i)}>{$tr('settings.llm.edit')}</button>
                    {/if}
                    <button class="danger" onclick={() => deleteLlm(i)}>
                      {isPendingDelete ? $tr('settings.cancel') : $tr('settings.llm.delete')}
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <p class="empty-hint">{$tr('settings.llm.empty')}</p>
          {/if}

          {#if !showAddForm}
            <button class="add-btn" onclick={startAdd}>{$tr('settings.llm.add')}</button>
          {:else}
            <div class="llm-form">
              <h4>{editingIndex !== null ? $tr('settings.llm.formEdit') : $tr('settings.llm.formAdd')}</h4>

              <div class="field-row">
                <label for="llm-name">{$tr('settings.llm.name')}</label>
                <input id="llm-name" type="text" bind:value={newLlm.name} placeholder="e.g. Claude" />
              </div>

              <div class="field-row">
                <label for="llm-type">{$tr('settings.llm.type')}</label>
                <select id="llm-type" bind:value={newLlm.type}>
                  <option value="cli">CLI</option>
                  <option value="api">API</option>
                </select>
              </div>

              {#if newLlm.type === 'cli'}
                <div class="field-row">
                  <label for="llm-command">{$tr('settings.llm.command')}</label>
                  <input id="llm-command" type="text" bind:value={newLlm.command} placeholder="e.g. claude"
                    oninput={() => { commandError = validateCommand(newLlm.command ?? ''); }} />
                  {#if commandError}<span class="field-error">{commandError}</span>{/if}
                </div>
                <div class="field-row">
                  <label for="llm-args">{$tr('settings.llm.args')}</label>
                  <input id="llm-args" type="text" bind:value={newLlmArgsStr} placeholder="e.g. --no-update-notification" />
                </div>
                <div class="field-row">
                  <label for="llm-yolo">{$tr('settings.llm.yoloFlag')}</label>
                  <input id="llm-yolo" type="text" bind:value={newLlm.yoloFlag} placeholder="e.g. --dangerously-skip-permissions" />
                </div>
                <div class="field-row">
                  <label for="llm-permission">Permission Level</label>
                  <select id="llm-permission" bind:value={newLlm.permissionLevel}>
                    <option value="ask">ASK (default)</option>
                    <option value="yolo">YOLO</option>
                    <option value="locked">LOCKED</option>
                  </select>
                </div>
                <div class="field-row">
                  <label for="llm-allowed-tools">Allowed Tools</label>
                  <input id="llm-allowed-tools" type="text" bind:value={newLlmAllowedToolsStr} placeholder="e.g. Read, Edit, Bash" />
                  <span class="field-hint">Comma-separated tool names pre-approved for this model</span>
                </div>
                <div class="field-row">
                  <label for="llm-image-mode">{$tr('settings.llm.imageMode')}</label>
                  <select id="llm-image-mode" bind:value={newLlm.imageMode}>
                    <option value="path">{$tr('settings.llm.imagePath')}</option>
                    <option value="base64">Base64</option>
                    <option value="none">{$tr('settings.llm.imageNone')}</option>
                  </select>
                </div>
              {:else}
                <div class="field-row">
                  <label for="llm-provider">{$tr('settings.llm.provider')}</label>
                  <select id="llm-provider" bind:value={newLlm.provider}>
                    <option value="anthropic">Anthropic</option>
                    <option value="openai">OpenAI</option>
                    <option value="openai-compatible">OpenAI-Compatible</option>
                  </select>
                </div>
                <div class="field-row">
                  <label for="llm-api-key-env">{$tr('settings.llm.apiKeyEnv')}</label>
                  <input id="llm-api-key-env" type="text" bind:value={newLlm.apiKeyEnv} placeholder="e.g. ANTHROPIC_API_KEY" />
                </div>
                <div class="field-row">
                  <label for="llm-model">{$tr('settings.llm.model')}</label>
                  <input id="llm-model" type="text" bind:value={newLlm.model} placeholder="e.g. claude-opus-4-5" />
                </div>
                {#if newLlm.provider === 'openai-compatible'}
                  <div class="field-row">
                    <label for="llm-endpoint">{$tr('settings.llm.endpoint')}</label>
                    <input id="llm-endpoint" type="text" bind:value={newLlm.endpoint} placeholder="https://api.example.com/v1"
                      oninput={() => { endpointError = validateEndpoint(newLlm.endpoint ?? ''); }} />
                    {#if endpointError}<span class="field-error">{endpointError}</span>{/if}
                  </div>
                {/if}
              {/if}

              <div class="form-actions">
                <button onclick={submitLlm}>{editingIndex !== null ? $tr('settings.llm.update') : $tr('settings.llm.addBtn')}</button>
                <button class="secondary" onclick={cancelForm}>{$tr('settings.cancel')}</button>
              </div>
            </div>
          {/if}
        </section>

        <!-- Language -->
        <section>
          <h3>{$tr('settings.language')}</h3>
          <div class="field-row">
            <label for="language-select">{$tr('settings.language')}</label>
            <select id="language-select" bind:value={localLocale}>
              {#each (['en', 'it', 'de', 'es', 'fr', 'pt', 'zh', 'hi', 'ar'] as Locale[]) as loc}
                <option value={loc}>{localeLabels[loc]}</option>
              {/each}
            </select>
          </div>
        </section>

        <!-- Terminal Settings -->
        <section>
          <h3>{$tr('settings.terminal.title')}</h3>
          <div class="field-row">
            <span class="field-label">{$tr('settings.font')}</span>
            <span class="font-preview">Atkinson Hyperlegible Mono</span>
          </div>
          <div class="field-row">
            <label for="font-size">{$tr('settings.fontSize')}</label>
            <input id="font-size" type="number" min="8" max="32" bind:value={localFontSize} />
          </div>
        </section>

        <!-- Accessibility -->
        <section>
          <h3>{$tr('settings.accessibility.title')}</h3>
          <div class="field-row">
            <label for="enhanced-readability">{$tr('settings.readability')}</label>
            <button
              id="enhanced-readability"
              class="toggle-btn"
              class:active={localEnhancedReadability}
              onclick={() => (localEnhancedReadability = !localEnhancedReadability)}
              role="switch"
              aria-checked={localEnhancedReadability}
            >
              {localEnhancedReadability ? $tr('settings.on') : $tr('settings.off')}
            </button>
          </div>
          <p class="field-hint">{$tr('settings.readability.hint')}</p>
        </section>

        <!-- Theme -->
        <section>
          <h3>{$tr('settings.theme')}</h3>
          <div class="theme-toggle">
            <button
              class="theme-btn"
              class:active={localActiveTheme === 'reasonance-dark'}
              onclick={() => (localActiveTheme = 'reasonance-dark')}
            >
              Dark
            </button>
            <button
              class="theme-btn"
              class:active={localActiveTheme === 'reasonance-light'}
              onclick={() => (localActiveTheme = 'reasonance-light')}
            >
              Light
            </button>
            {#each userThemes as ut}
              <button
                class="theme-btn"
                class:active={localActiveTheme === ut}
                onclick={() => (localActiveTheme = ut)}
              >
                {ut}
              </button>
            {/each}
          </div>

          <div class="field-row" style="margin-top: var(--space-3);">
            <span class="field-label">{$tr('settings.theme.import')}</span>
            <button
              class="import-theme-btn"
              onclick={async () => {
                try {
                  const selected = await openFileDialog({
                    title: 'Import Theme',
                    filters: [{ name: 'JSON', extensions: ['json'] }],
                    multiple: false,
                  });
                  if (!selected) return;
                  const filePath = typeof selected === 'string' ? selected : selected.path;
                  if (!filePath) return;
                  const text = await readTextFile(filePath);
                  const parsed = JSON.parse(text);
                  const validation = validateTheme(parsed);
                  if (!validation.valid) {
                    importError = validation.errors.join(', ');
                    return;
                  }
                  const theme = parsed as ThemeFile;
                  const name = theme.meta.name.toLowerCase().replace(/\s+/g, '-');
                  suppressFileWatcher(true);
                  try {
                    await invoke('save_user_theme', { name, content: text });
                    userThemes = await invoke<string[]>('list_user_themes');
                    localActiveTheme = name;
                    await loadBuiltinTheme(name);
                    importError = '';
                  } finally {
                    suppressFileWatcher(false);
                  }
                } catch (e) {
                  importError = e instanceof Error ? e.message : String(e);
                }
              }}
            >
              {$tr('settings.theme.importBtn')}
            </button>
          </div>
          {#if importError}
            <p class="field-error">{importError}</p>
          {/if}
        </section>

        <!-- Updates -->
        <section>
          <h3>{$tr('settings.updates')}</h3>

          <div class="field-row">
            <label for="auto-update">{$tr('settings.autoUpdate')}</label>
            <input
              id="auto-update"
              type="checkbox"
              bind:checked={localAutoUpdate}
            />
          </div>

          <div class="field-row">
            <label for="update-mode">{$tr('settings.updateMode')}</label>
            <select id="update-mode" bind:value={localUpdateMode} disabled={!localAutoUpdate}>
              <option value="notify">{$tr('settings.updateNotify')}</option>
              <option value="silent">{$tr('settings.updateSilent')}</option>
            </select>
          </div>

          <div class="field-row">
            <button
              class="check-update-btn"
              onclick={async () => {
                checkingUpdate = true;
                await checkForUpdate(true);
                checkingUpdate = false;
              }}
              disabled={checkingUpdate}
            >
              {checkingUpdate
                ? $tr('settings.checking')
                : $tr('settings.checkNow')}
            </button>
          </div>

          <div class="field-row version-label">
            v{__APP_VERSION__}
          </div>
        </section>

        <!-- Provider Settings -->
        <section>
          <div class="provider-section-header">
            <h3>{$tr('settings.provider.title')}</h3>
            <button
              class="scan-cli-btn"
              onclick={async () => { await adapter.discoverLlms(); providerConfigVersion.update(v => v + 1); }}
            >
              {$tr('settings.provider.scanCli')}
            </button>
          </div>

          <div class="provider-list">
            {#each llms as llm, i}
              {@const visual = getProviderVisual(llm.provider ?? llm.name)}
              {@const models = getModelsForProvider((llm.provider ?? llm.name).toLowerCase())}
              {@const isExpanded = expandedProvider === llm.name}
              {@const steps = connectionSteps.get(llm.name) ?? []}
              {@const isTesting = testingProvider === llm.name}

              <div class="provider-item" class:expanded={isExpanded}>
                <div class="provider-row">
                  <span
                    class="provider-dot"
                    style="background-color: {visual.color};"
                    aria-label={llm.provider ?? llm.name}
                  ></span>
                  <span class="provider-name">{llm.name}</span>
                  <span class="provider-model-hint">
                    {llm.model ? llm.model : $tr('settings.provider.notConfigured')}
                  </span>
                  <button
                    class="provider-expand-btn"
                    onclick={() => expandedProvider = isExpanded ? null : llm.name}
                    aria-expanded={isExpanded}
                    aria-label={$tr('settings.provider.expand')}
                  >
                    {isExpanded ? '▲' : '▼'}
                  </button>
                </div>

                {#if isExpanded}
                  <div class="provider-detail">
                    {#if models.length > 0}
                      <div class="field-row">
                        <label for="provider-model-{i}">{$tr('settings.provider.defaultModel')}</label>
                        <select
                          id="provider-model-{i}"
                          bind:value={llm.model}
                          onchange={() => { llms = [...llms]; }}
                        >
                          {#each models as m}
                            <option value={m.id}>
                              {m.name} — ${m.cost_per_1m_input}/1M in, ${m.cost_per_1m_output}/1M out
                            </option>
                          {/each}
                        </select>
                      </div>
                    {:else}
                      <div class="field-row">
                        <label for="provider-model-text-{i}">{$tr('settings.provider.defaultModel')}</label>
                        <input
                          id="provider-model-text-{i}"
                          type="text"
                          bind:value={llm.model}
                          oninput={() => { llms = [...llms]; }}
                        />
                      </div>
                    {/if}

                    {#if llm.type === 'cli'}
                      <div class="field-row">
                        <label for="provider-binary-{i}">{$tr('settings.provider.binary')}</label>
                        <input
                          id="provider-binary-{i}"
                          type="text"
                          bind:value={llm.command}
                          oninput={() => { llms = [...llms]; }}
                        />
                      </div>
                    {/if}

                    <div class="field-row">
                      <label for="provider-max-tokens-{i}">{$tr('settings.provider.maxTokens')}</label>
                      <input
                        id="provider-max-tokens-{i}"
                        type="number"
                        min="1"
                        max="1000000"
                        placeholder="8192"
                        value={llm.maxTokens ?? ''}
                        oninput={(e) => {
                          const v = (e.currentTarget as HTMLInputElement).value;
                          llms[i] = { ...llms[i], maxTokens: v ? parseInt(v, 10) : undefined };
                          llms = [...llms];
                        }}
                      />
                    </div>

                    <div class="field-row">
                      <label for="provider-api-key-{i}">{$tr('settings.provider.apiKeyEnv')}</label>
                      <input
                        id="provider-api-key-{i}"
                        type="text"
                        bind:value={llm.apiKeyEnv}
                        oninput={() => { llms = [...llms]; }}
                        placeholder="e.g. ANTHROPIC_API_KEY"
                      />
                    </div>

                    <div class="field-row">
                      <span class="field-label">{$tr('settings.provider.shortcut')}</span>
                      <button
                        class="shortcut-capture-btn"
                        class:capturing={capturingShortcut === llm.name}
                        aria-label={`Shortcut for ${llm.name}`}
                        onclick={() => capturingShortcut = capturingShortcut === llm.name ? null : llm.name}
                        onkeydown={handleShortcutCapture}
                      >
                        {capturingShortcut === llm.name ? $tr('settings.provider.pressKeys') : llm.shortcut || '—'}
                      </button>
                    </div>

                    <div class="provider-actions">
                      <button
                        class="test-btn"
                        onclick={() => testConnection(llm.name)}
                        disabled={isTesting}
                      >
                        {isTesting
                          ? $tr('settings.provider.test.testing')
                          : $tr('settings.provider.test.button')}
                      </button>
                    </div>

                    {#if steps.length > 0}
                      <div class="connection-steps" role="list" aria-live="polite">
                        {#each steps as step}
                          {@const isOk = step.status === 'ok'}
                          {@const isFailed = step.status === 'failed'}
                          {@const isChecking = step.status === 'checking'}
                          <div class="connection-step" role="listitem" class:ok={isOk} class:failed={isFailed} class:checking={isChecking}>
                            <span class="step-icon">
                              {isOk ? '✓' : isFailed ? '✗' : '…'}
                            </span>
                            <span class="step-label">
                              {#if step.step === 'binary'}
                                {isOk
                                  ? $tr('settings.provider.test.binaryFound')
                                  : $tr('settings.provider.test.binaryNotFound')}
                              {:else if step.step === 'api_key'}
                                {isOk
                                  ? $tr('settings.provider.test.apiKeyOk')
                                  : $tr('settings.provider.test.apiKeyMissing')}
                              {:else if step.step === 'connection'}
                                {isOk
                                  ? $tr('settings.provider.test.connectionOk')
                                  : $tr('settings.provider.test.connectionFailed')}
                              {/if}
                            </span>
                            {#if step.detail}
                              <span class="step-detail">{step.detail}</span>
                            {/if}
                          </div>
                        {/each}
                        {#if steps.length === 3 && steps.every(s => s.status === 'ok')}
                          <div class="connection-ready">
                            {$tr('settings.provider.test.ready', { provider: llm.name })}
                          </div>
                        {/if}
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            {/each}
          </div>

          <!-- Trusted Workspaces sub-section -->
          <div class="trust-section">
            <h4 class="trust-title">{$tr('settings.trustedWorkspaces')}</h4>

            {#if trustEntries.length === 0}
              <p class="hint">{$tr('settings.noTrustedWorkspaces')}</p>
            {:else}
              <div class="trust-table">
                {#each trustEntries as entry}
                  <div class="trust-row">
                    <span class="trust-path" title={entry.path}>{entry.path}</span>
                    <span class="trust-level">{entry.level}</span>
                    <span class="trust-date">{new Date(entry.trusted_at).toLocaleDateString()}</span>
                    <button class="btn-small" onclick={() => handleRevokeTrust(entry.hash)}>
                      {$tr('settings.revoke')}
                    </button>
                  </div>
                {/each}
              </div>
            {/if}

            <div class="field-row setting-row">
              <label for="default-permission">{$tr('settings.defaultPermission')}</label>
              <select id="default-permission" value={$appSettings.defaultPermissionLevel ?? 'yolo'} onchange={handleDefaultPermissionChange}>
                <option value="yolo">AUTO</option>
                <option value="ask">CONFIRM</option>
                <option value="locked">LOCKED</option>
              </select>
            </div>
            <p class="hint">{$tr('settings.defaultPermissionHint')}</p>
          </div>

          <!-- Budget sub-section -->
          <div class="budget-section">
            <h4 class="budget-title">{$tr('settings.provider.budget')}</h4>
            <div class="field-row">
              <label for="budget-daily">{$tr('analytics.budget.daily')}</label>
              <input
                id="budget-daily"
                type="number"
                min="0"
                step="0.01"
                placeholder="No limit"
                value={localBudget.daily_limit_usd ?? ''}
                oninput={(e) => {
                  const v = (e.currentTarget as HTMLInputElement).value;
                  localBudget = { ...localBudget, daily_limit_usd: v ? parseFloat(v) : null };
                }}
              />
            </div>
            <div class="field-row">
              <label for="budget-weekly">{$tr('analytics.budget.weekly')}</label>
              <input
                id="budget-weekly"
                type="number"
                min="0"
                step="0.01"
                placeholder="No limit"
                value={localBudget.weekly_limit_usd ?? ''}
                oninput={(e) => {
                  const v = (e.currentTarget as HTMLInputElement).value;
                  localBudget = { ...localBudget, weekly_limit_usd: v ? parseFloat(v) : null };
                }}
              />
            </div>
            <div class="field-row">
              <label for="budget-notify">{$tr('analytics.budget.notifyAt')}</label>
              <input
                id="budget-notify"
                type="range"
                min="50"
                max="95"
                step="5"
                bind:value={localBudget.notify_at_percent}
                aria-valuetext="{localBudget.notify_at_percent}%"
              />
              <span class="range-value">{localBudget.notify_at_percent}%</span>
            </div>
          </div>
        </section>
      </div>

      <div class="modal-footer">
        <button class="secondary" onclick={onClose}>{$tr('settings.cancel')}</button>
        <button class="primary" onclick={save} disabled={saving}>
          {saving ? $tr('settings.saving') : $tr('settings.save')}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .settings-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    z-index: var(--layer-modal);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .settings-modal {
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    width: 580px;
    max-width: 95vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--font-ui);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-5) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .modal-header h2 {
    margin: 0;
    font-size: var(--font-size-base);
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: var(--font-size-base);
    cursor: pointer;
    padding: var(--space-1) var(--space-1);
    border-radius: var(--radius);
    min-width: 32px;
    min-height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .close-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .error-banner {
    background: var(--danger);
    color: var(--text-on-accent);
    padding: var(--space-2) var(--space-5);
    font-size: var(--font-size-sm);
    flex-shrink: 0;
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4) var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  section h3 {
    margin: 0 0 var(--space-3);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-md);
    color: var(--accent-text);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    line-height: var(--line-height-sm);
  }

  .field-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .field-row label,
  .field-row .field-label {
    width: 160px;
    flex-shrink: 0;
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .field-row input,
  .field-row select {
    flex: 1;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    padding: var(--space-1) var(--space-2);
    font-size: var(--font-size-sm);
    font-family: inherit;
  }

  .field-row input:focus-visible,
  .field-row select:focus-visible {
    border-color: var(--accent);
    outline: var(--focus-ring);
    outline-offset: var(--focus-offset);
  }

  .font-preview {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  /* LLM list */
  .llm-list {
    display: flex;
    flex-direction: column;
    gap: var(--stack-tight);
    margin-bottom: var(--space-2);
  }

  .llm-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-2) var(--space-2);
    transition: background var(--transition-fast), border-color var(--transition-fast);
  }

  .llm-item.pending-delete {
    background: color-mix(in srgb, var(--danger) 8%, var(--bg-secondary));
    border-color: var(--danger);
    opacity: 0.75;
  }

  .llm-item.pending-delete .llm-name {
    text-decoration: line-through;
    color: var(--text-muted);
  }

  .pending-delete-label {
    font-size: var(--font-size-sm);
    color: var(--danger);
    font-style: italic;
  }

  .llm-info {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .llm-name {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
  }

  .llm-type-badge {
    font-size: var(--font-size-sm);
    background: var(--accent-btn);
    color: var(--text-on-accent);
    padding: var(--stack-tight) var(--space-1);
    border-radius: var(--radius);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .llm-detail {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .llm-actions {
    display: flex;
    gap: var(--interactive-gap);
    flex-shrink: 0;
  }

  .import-theme-btn {
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-size-sm);
    border: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    transition: border-color var(--transition-fast);
  }

  .import-theme-btn:hover {
    border-color: var(--accent);
  }

  .field-error {
    font-size: var(--font-size-sm);
    color: var(--danger-text);
    margin: var(--space-1) 0 0;
  }

  .empty-hint {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin: 0 0 var(--space-2);
  }

  /* LLM form */
  .llm-form {
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-3);
    margin-bottom: var(--space-2);
  }

  .llm-form h4 {
    margin: 0 0 var(--space-3);
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
  }

  .form-actions {
    display: flex;
    gap: var(--interactive-gap);
    margin-top: var(--space-3);
  }

  /* Theme toggle */
  .theme-toggle {
    display: flex;
    gap: var(--interactive-gap);
  }

  .theme-btn {
    padding: var(--space-1) var(--space-4);
    font-size: var(--font-size-sm);
    border-radius: var(--radius);
    border: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    transition: background var(--transition-fast), border-color var(--transition-fast);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 700;
  }

  .theme-btn:hover {
    border-color: var(--accent);
    background: var(--accent-btn);
    color: var(--text-on-accent);
  }

  .theme-btn.active {
    background: var(--accent-btn);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }

  .theme-btn.active:hover {
    opacity: 0.85;
    color: var(--text-on-accent);
  }

  /* Buttons */
  button {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-size-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  button:hover {
    background: var(--accent-btn);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }

  button.primary {
    background: var(--accent-btn);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }

  button.primary:hover {
    opacity: 0.85;
  }

  button.primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  button.secondary {
    background: var(--bg-secondary);
  }

  button.danger {
    border-color: var(--danger);
    color: var(--danger);
    background: transparent;
  }

  button.danger:hover {
    background: var(--danger);
    border-color: var(--danger);
    color: var(--text-on-accent);
  }

  .add-btn {
    font-size: var(--font-size-sm);
    padding: var(--space-1) var(--space-3);
  }

  /* Modal footer */
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-5);
    border-top: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .toggle-btn {
    padding: var(--btn-padding);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    cursor: pointer;
    min-width: 50px;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .toggle-btn.active {
    background: var(--accent-btn);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }

  .field-hint {
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin: var(--space-1) 0 0;
  }

  .field-error {
    display: block;
    font-size: var(--font-size-tiny);
    color: var(--danger);
    margin: var(--stack-tight) 0 0;
  }

  /* Provider section */
  .provider-section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-3);
  }

  .provider-section-header h3 {
    margin: 0;
  }

  .scan-cli-btn {
    font-size: var(--font-size-sm);
    padding: var(--space-1) var(--space-2);
  }

  .provider-list {
    display: flex;
    flex-direction: column;
    gap: var(--stack-tight);
    margin-bottom: var(--space-4);
  }

  .provider-item {
    border: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
  }

  .provider-item.expanded {
    border-color: var(--accent);
  }

  .provider-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-2);
  }

  .provider-dot {
    width: 8px;
    height: 8px;
    flex-shrink: 0;
    border-radius: var(--radius);
    display: inline-block;
  }

  .provider-name {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
    min-width: 80px;
  }

  .provider-model-hint {
    flex: 1;
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .provider-expand-btn {
    font-size: var(--font-size-sm);
    padding: var(--stack-tight) var(--space-1);
    flex-shrink: 0;
  }

  .provider-detail {
    border-top: var(--border-width) solid var(--border);
    padding: var(--space-3) var(--space-2);
    background: var(--bg-primary);
  }

  .provider-actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-2);
    align-items: center;
  }

  .test-btn {
    font-size: var(--font-size-sm);
    padding: var(--space-1) var(--space-2);
  }

  .shortcut-capture-btn {
    font-size: var(--font-size-sm);
    padding: var(--space-1) var(--space-2);
    flex: 1;
  }

  .shortcut-capture-btn.capturing {
    background: var(--accent-btn);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }

  /* Connection steps */
  .connection-steps {
    margin-top: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--stack-tight);
  }

  .connection-step {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
    font-size: var(--font-size-sm);
    padding: var(--space-1) var(--space-1);
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
  }

  .connection-step.ok {
    border-color: var(--success);
    color: var(--success-text);
  }

  .connection-step.failed {
    border-color: var(--danger);
    color: var(--danger-text);
  }

  .connection-step.checking {
    color: var(--text-secondary);
    opacity: 0.75;
  }

  .step-icon {
    font-size: var(--font-size-sm);
    font-weight: 700;
    flex-shrink: 0;
    width: 14px;
    text-align: center;
  }

  .step-label {
    flex: 1;
  }

  .step-detail {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 200px;
  }

  .connection-ready {
    font-size: var(--font-size-sm);
    font-weight: 700;
    color: var(--success-text);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: var(--space-1) var(--space-1);
    border: var(--border-width) solid var(--success);
    background: color-mix(in srgb, var(--success) 10%, var(--bg-secondary));
  }

  /* Budget section */
  .budget-section {
    margin-top: var(--space-3);
    padding: var(--space-3);
    border: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
  }

  .budget-title {
    margin: 0 0 var(--space-2);
    font-size: var(--font-size-sm);
    font-weight: 700;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .version-label {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    justify-content: flex-end;
  }

  .range-value {
    font-size: var(--font-size-sm);
    font-family: var(--font-mono);
    color: var(--text-secondary);
    min-width: 32px;
    text-align: end;
    flex-shrink: 0;
  }

  /* Trusted Workspaces section */
  .trust-section {
    margin-top: var(--space-3);
    padding: var(--space-3);
    border: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
  }

  .trust-title {
    margin: 0 0 var(--space-2);
    font-size: var(--font-size-sm);
    font-weight: 700;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .trust-table { display: flex; flex-direction: column; gap: var(--stack-tight); margin-bottom: var(--space-2); }
  .trust-row { display: flex; align-items: center; gap: var(--space-2); font-size: var(--font-size-small); }
  .trust-path { flex: 1; font-family: var(--font-mono); overflow: hidden; text-overflow: ellipsis; }
  .trust-level { text-transform: uppercase; font-weight: 700; font-size: var(--font-size-tiny); }
  .trust-date { color: var(--text-muted); font-size: var(--font-size-tiny); }

  .hint {
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin: var(--space-1) 0;
  }
</style>
