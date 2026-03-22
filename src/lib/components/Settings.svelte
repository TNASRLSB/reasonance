<script lang="ts">
  import { parse, stringify } from 'smol-toml';
  import { parseLlmConfig } from '$lib/utils/config-parser';
  import type { Adapter } from '$lib/adapter/index';
  import type { LlmConfig, AppSettings } from '$lib/stores/config';
  import { llmConfigs, appSettings } from '$lib/stores/config';
  import { fontFamily, fontSize, enhancedReadability } from '$lib/stores/ui';
  import { themeMode, type ThemeMode } from '$lib/stores/theme';
  import { tr, locale, loadLocale, type Locale } from '$lib/i18n/index';
  import { get } from 'svelte/store';
  import { getUpdateSettings, saveUpdateSettings, checkForUpdate, type UpdateMode } from '$lib/updater';
  import { trapFocus } from '$lib/utils/a11y';
  import type { ConnectionTestStep, AnalyticsBudget } from '$lib/types/analytics';
  import { budget, providerConfigVersion } from '$lib/stores/analytics';
  import { getModelsForProvider } from '$lib/data/model-info';
  import { getProviderVisual } from '$lib/utils/provider-patterns';

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
    provider: 'anthropic',
    apiKeyEnv: '',
    model: '',
    endpoint: '',
  });
  let newLlmArgsStr = $state('');

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
      error = 'Could not load configuration. Your settings have been preserved in memory.';
      llms = get(llmConfigs);
      settings = get(appSettings);
    }
    localFontFamily = get(fontFamily);
    localFontSize = get(fontSize);
    localTheme = get(themeMode);
    localEnhancedReadability = get(enhancedReadability);
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
      provider: 'anthropic',
      apiKeyEnv: '',
      model: '',
      endpoint: '',
    };
    newLlmArgsStr = '';
    showAddForm = true;
  }

  function startEdit(index: number) {
    editingIndex = index;
    const l = llms[index];
    newLlm = { ...l };
    newLlmArgsStr = (l.args ?? []).join(', ');
    showAddForm = true;
  }

  function cancelForm() {
    showAddForm = false;
    editingIndex = null;
  }

  function submitLlm() {
    if (!newLlm.name.trim()) {
      error = 'LLM name is required.';
      return;
    }
    error = '';

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

      // Update stores — close modal first to avoid re-render conflicts
      onClose();

      // Apply immediately - no async, no microtask
      llmConfigs.set(effectiveLlms);
      appSettings.set(settings);
      budget.set({ ...localBudget });
      fontFamily.set("'Atkinson Hyperlegible Mono', monospace");
      fontSize.set(localFontSize);
      enhancedReadability.set(localEnhancedReadability);
      themeMode.set(localTheme);
      if (localLocale !== get(locale)) {
        loadLocale(localLocale).then(() => {
          locale.set(localLocale);
        }).catch((e) => console.warn('Failed to load locale:', e));
      }
    } catch (e) {
      console.error('Settings save error:', e);
      error = 'Could not save settings. Check file permissions and try again.';
    } finally {
      saving = false;
    }
  }

  function debounce(fn: (...args: unknown[]) => void, ms: number) {
    let timer: ReturnType<typeof setTimeout>;
    return (...args: unknown[]) => {
      clearTimeout(timer);
      timer = setTimeout(() => fn(...args), ms);
    };
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
                  <input id="llm-command" type="text" bind:value={newLlm.command} placeholder="e.g. claude" />
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
                    <input id="llm-endpoint" type="text" bind:value={newLlm.endpoint} placeholder="https://api.example.com/v1" />
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
            <label>{$tr('settings.font')}</label>
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
            {#each (['light', 'dark', 'system'] as ThemeMode[]) as mode}
              <button
                class="theme-btn"
                class:active={localTheme === mode}
                onclick={() => (localTheme = mode)}
              >
                {mode === 'light' ? $tr('settings.theme.light') : mode === 'dark' ? $tr('settings.theme.dark') : $tr('settings.theme.system')}
              </button>
            {/each}
          </div>
        </section>

        <!-- Updates -->
        <section>
          <h3>{$tr('settings.updates') ?? 'Updates'}</h3>

          <div class="field-row">
            <label for="auto-update">{$tr('settings.autoUpdate') ?? 'Automatic updates'}</label>
            <input
              id="auto-update"
              type="checkbox"
              bind:checked={localAutoUpdate}
            />
          </div>

          <div class="field-row">
            <label for="update-mode">{$tr('settings.updateMode') ?? 'Update mode'}</label>
            <select id="update-mode" bind:value={localUpdateMode} disabled={!localAutoUpdate}>
              <option value="notify">{$tr('settings.updateNotify') ?? 'Notify when available'}</option>
              <option value="silent">{$tr('settings.updateSilent') ?? 'Download and install silently'}</option>
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
                ? ($tr('settings.checking') ?? 'Checking...')
                : ($tr('settings.checkNow') ?? 'Check now')}
            </button>
          </div>

          <div class="field-row version-label">
            v{__APP_VERSION__}
          </div>
        </section>

        <!-- Provider Settings -->
        <section>
          <div class="provider-section-header">
            <h3>{$tr('settings.provider.title') ?? 'PROVIDER'}</h3>
            <button
              class="scan-cli-btn"
              onclick={async () => { await adapter.discoverLlms(); providerConfigVersion.update(v => v + 1); }}
            >
              {$tr('settings.provider.scanCli') ?? 'Scan CLI'}
            </button>
          </div>

          <div class="provider-list">
            {#each llms as llm, i}
              {@const visual = getProviderVisual(llm.provider ?? llm.name)}
              {@const models = getModelsForProvider(llm.provider ?? llm.name)}
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
                    {llm.model ? llm.model : ($tr('settings.provider.notConfigured') ?? 'Not configured')}
                  </span>
                  <button
                    class="provider-expand-btn"
                    onclick={() => expandedProvider = isExpanded ? null : llm.name}
                    aria-expanded={isExpanded}
                    aria-label={$tr('settings.provider.expand') ?? 'Expand'}
                  >
                    {isExpanded ? '▲' : '▼'}
                  </button>
                </div>

                {#if isExpanded}
                  <div class="provider-detail">
                    {#if models.length > 0}
                      <div class="field-row">
                        <label for="provider-model-{i}">{$tr('settings.provider.defaultModel') ?? 'Default model'}</label>
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
                        <label for="provider-model-text-{i}">{$tr('settings.provider.defaultModel') ?? 'Default model'}</label>
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
                        <label for="provider-binary-{i}">{$tr('settings.provider.binary') ?? 'Binary'}</label>
                        <input
                          id="provider-binary-{i}"
                          type="text"
                          bind:value={llm.command}
                          oninput={() => { llms = [...llms]; }}
                        />
                      </div>
                    {/if}

                    <div class="field-row">
                      <label for="provider-max-tokens-{i}">{$tr('settings.provider.maxTokens') ?? 'Max tokens'}</label>
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
                      <label for="provider-api-key-{i}">{$tr('settings.provider.apiKeyEnv') ?? 'API Key env'}</label>
                      <input
                        id="provider-api-key-{i}"
                        type="text"
                        bind:value={llm.apiKeyEnv}
                        oninput={() => { llms = [...llms]; }}
                        placeholder="e.g. ANTHROPIC_API_KEY"
                      />
                    </div>

                    <div class="field-row">
                      <label>{$tr('settings.provider.shortcut') ?? 'Shortcut'}</label>
                      <button
                        class="shortcut-capture-btn"
                        class:capturing={capturingShortcut === llm.name}
                        aria-label={`Shortcut for ${llm.name}`}
                        onclick={() => capturingShortcut = capturingShortcut === llm.name ? null : llm.name}
                        onkeydown={handleShortcutCapture}
                      >
                        {capturingShortcut === llm.name ? 'Press keys…' : llm.shortcut || '—'}
                      </button>
                    </div>

                    <div class="provider-actions">
                      <button
                        class="test-btn"
                        onclick={() => testConnection(llm.name)}
                        disabled={isTesting}
                      >
                        {isTesting
                          ? ($tr('settings.provider.test.testing') ?? 'Testing…')
                          : ($tr('settings.provider.test.button') ?? 'Test connection')}
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
                                  ? ($tr('settings.provider.test.binaryFound') ?? 'Binary found')
                                  : ($tr('settings.provider.test.binaryNotFound') ?? 'Binary not found')}
                              {:else if step.step === 'api_key'}
                                {isOk
                                  ? ($tr('settings.provider.test.apiKeyOk') ?? 'API key configured')
                                  : ($tr('settings.provider.test.apiKeyMissing') ?? 'API key not set')}
                              {:else if step.step === 'connection'}
                                {isOk
                                  ? ($tr('settings.provider.test.connectionOk') ?? 'Connection OK')
                                  : ($tr('settings.provider.test.connectionFailed') ?? 'Connection failed')}
                              {/if}
                            </span>
                            {#if step.detail}
                              <span class="step-detail">{step.detail}</span>
                            {/if}
                          </div>
                        {/each}
                        {#if steps.length === 3 && steps.every(s => s.status === 'ok')}
                          <div class="connection-ready">
                            {($tr('settings.provider.test.ready') ?? '{provider} ready').replace('{provider}', llm.name)}
                          </div>
                        {/if}
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            {/each}
          </div>

          <!-- Budget sub-section -->
          <div class="budget-section">
            <h4 class="budget-title">{$tr('settings.provider.budget') ?? 'Budget'}</h4>
            <div class="field-row">
              <label for="budget-daily">{$tr('analytics.budget.daily') ?? 'Daily limit (USD)'}</label>
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
              <label for="budget-weekly">{$tr('analytics.budget.weekly') ?? 'Weekly limit (USD)'}</label>
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
              <label for="budget-notify">{$tr('analytics.budget.notifyAt') ?? 'Notify at'}</label>
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
    background: rgba(0, 0, 0, 0.55);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .settings-modal {
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    border-radius: 0;
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
    padding: 16px 20px 12px;
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 16px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 0;
  }

  .close-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .error-banner {
    background: var(--danger, #e74c3c);
    color: #fff;
    padding: 8px 20px;
    font-size: 12px;
    flex-shrink: 0;
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  section h3 {
    margin: 0 0 12px;
    font-size: 13px;
    font-weight: 600;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .field-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
  }

  .field-row label {
    width: 160px;
    flex-shrink: 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .field-row input,
  .field-row select {
    flex: 1;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    border-radius: 0;
    color: var(--text-primary);
    padding: 5px 8px;
    font-size: 12px;
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
    font-size: 13px;
    color: var(--text-secondary);
  }

  /* LLM list */
  .llm-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 10px;
  }

  .llm-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    border-radius: 0;
    padding: 8px 10px;
    transition: background 0.15s, border-color 0.15s;
  }

  .llm-item.pending-delete {
    background: color-mix(in srgb, var(--danger, #e74c3c) 8%, var(--bg-secondary));
    border-color: var(--danger, #e74c3c);
    opacity: 0.75;
  }

  .llm-item.pending-delete .llm-name {
    text-decoration: line-through;
    color: var(--text-muted);
  }

  .pending-delete-label {
    font-size: 10px;
    color: var(--danger, #e74c3c);
    font-style: italic;
  }

  .llm-info {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .llm-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .llm-type-badge {
    font-size: 10px;
    background: var(--accent);
    color: #fff;
    padding: 1px 5px;
    border-radius: 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .llm-detail {
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .llm-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .empty-hint {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0 0 10px;
  }

  /* LLM form */
  .llm-form {
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    border-radius: 0;
    padding: 14px;
    margin-bottom: 10px;
  }

  .llm-form h4 {
    margin: 0 0 12px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .form-actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }

  /* Theme toggle */
  .theme-toggle {
    display: flex;
    gap: 8px;
  }

  .theme-btn {
    padding: 6px 16px;
    font-size: 12px;
    border-radius: 0;
    border: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 700;
  }

  .theme-btn:hover {
    border-color: var(--accent);
    background: var(--accent);
    color: #fff;
  }

  .theme-btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .theme-btn.active:hover {
    opacity: 0.85;
    color: #fff;
  }

  /* Buttons */
  button {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: var(--border-width) solid var(--border);
    border-radius: 0;
    padding: 5px 12px;
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition: background 0.15s;
  }

  button:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
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
    border-color: var(--danger, #e74c3c);
    color: var(--danger, #e74c3c);
    background: transparent;
  }

  button.danger:hover {
    background: var(--danger, #e74c3c);
    border-color: var(--danger, #e74c3c);
    color: #fff;
  }

  .add-btn {
    font-size: 12px;
    padding: 5px 12px;
  }

  /* Modal footer */
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 20px;
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
    border-radius: 0;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    cursor: pointer;
    min-width: 50px;
    transition: background 0.1s, color 0.1s;
  }

  .toggle-btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .field-hint {
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin: 4px 0 0;
  }

  /* Provider section */
  .provider-section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .provider-section-header h3 {
    margin: 0;
  }

  .scan-cli-btn {
    font-size: 11px;
    padding: 3px 10px;
  }

  .provider-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 16px;
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
    gap: 8px;
    padding: 8px 10px;
  }

  .provider-dot {
    width: 8px;
    height: 8px;
    flex-shrink: 0;
    border-radius: 0;
    display: inline-block;
  }

  .provider-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    min-width: 80px;
  }

  .provider-model-hint {
    flex: 1;
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .provider-expand-btn {
    font-size: 10px;
    padding: 2px 6px;
    flex-shrink: 0;
  }

  .provider-detail {
    border-top: var(--border-width) solid var(--border);
    padding: 12px 10px;
    background: var(--bg-primary);
  }

  .provider-actions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
    align-items: center;
  }

  .test-btn {
    font-size: 11px;
    padding: 4px 10px;
  }

  .shortcut-capture-btn {
    font-size: 11px;
    padding: 4px 10px;
    flex: 1;
  }

  .shortcut-capture-btn.capturing {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  /* Connection steps */
  .connection-steps {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .connection-step {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    padding: 4px 6px;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
  }

  .connection-step.ok {
    border-color: var(--success);
    color: var(--success);
  }

  .connection-step.failed {
    border-color: var(--danger, #e74c3c);
    color: var(--danger, #e74c3c);
  }

  .connection-step.checking {
    color: var(--text-secondary);
    opacity: 0.75;
  }

  .step-icon {
    font-size: 12px;
    font-weight: 700;
    flex-shrink: 0;
    width: 14px;
    text-align: center;
  }

  .step-label {
    flex: 1;
  }

  .step-detail {
    font-size: 10px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 200px;
  }

  .connection-ready {
    font-size: 11px;
    font-weight: 700;
    color: var(--success);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 4px 6px;
    border: var(--border-width) solid var(--success);
    background: color-mix(in srgb, var(--success) 10%, var(--bg-secondary));
  }

  /* Budget section */
  .budget-section {
    margin-top: 12px;
    padding: 12px;
    border: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
  }

  .budget-title {
    margin: 0 0 10px;
    font-size: 11px;
    font-weight: 700;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .version-label {
    font-size: 11px;
    color: var(--text-muted);
    justify-content: flex-end;
  }

  .range-value {
    font-size: var(--font-size-small, 11px);
    font-family: var(--font-mono);
    color: var(--text-secondary);
    min-width: 32px;
    text-align: right;
    flex-shrink: 0;
  }
</style>
