<script lang="ts">
  import { parse, stringify } from 'smol-toml';
  import type { Adapter } from '$lib/adapter/index';
  import type { LlmConfig, AppSettings } from '$lib/stores/config';
  import { llmConfigs, appSettings } from '$lib/stores/config';
  import { fontFamily, fontSize, enhancedReadability } from '$lib/stores/ui';
  import { themeMode, type ThemeMode } from '$lib/stores/theme';
  import { tr, locale, loadLocale, type Locale } from '$lib/i18n/index';
  import { get } from 'svelte/store';

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
        llms = rawLlms.map((l) => ({
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
        }));

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
      error = `Failed to load config: ${e}`;
      llms = get(llmConfigs);
      settings = get(appSettings);
    }
    localFontFamily = get(fontFamily);
    localFontSize = get(fontSize);
    localTheme = get(themeMode);
    localEnhancedReadability = get(enhancedReadability);
    localLocale = get(locale);
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
    llms = llms.filter((_, i) => i !== index);
  }

  async function save() {
    saving = true;
    error = '';

    try {
      // Build TOML object
      const tomlObj: Record<string, unknown> = {
        settings: {
          default: settings.default ?? '',
          context_menu_llm: settings.contextMenuLlm ?? '',
        },
        llm: llms.map((l) => {
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

      // Update stores — close modal first to avoid re-render conflicts
      onClose();

      // Apply immediately - no async, no microtask
      llmConfigs.set(llms);
      appSettings.set(settings);
      fontFamily.set("'Atkinson Hyperlegible Mono', monospace");
      fontSize.set(localFontSize);
      enhancedReadability.set(localEnhancedReadability);
      themeMode.set(localTheme);
      if (localLocale !== get(locale)) {
        loadLocale(localLocale).then(() => {
          locale.set(localLocale);
        }).catch(() => {});
      }
    } catch (e) {
      error = `Failed to save config: ${e}`;
    } finally {
      saving = false;
    }
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
  <div class="settings-overlay" role="button" tabindex="-1" onclick={handleOverlayClick} onkeydown={(e) => { if (e.key === 'Escape') handleOverlayClick(); }}>
    <div class="settings-modal" role="dialog" aria-modal="true" aria-label={$tr('settings.title')}>
      <div class="modal-header">
        <h2>{$tr('settings.title')}</h2>
        <button class="close-btn" onclick={onClose} aria-label={$tr('settings.close')}>✕</button>
      </div>

      {#if error}
        <div class="error-banner">{error}</div>
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
                <div class="llm-item">
                  <div class="llm-info">
                    <span class="llm-name">{l.name}</span>
                    <span class="llm-type-badge">{l.type}</span>
                    {#if l.type === 'cli'}
                      <span class="llm-detail">{l.command}</span>
                    {:else}
                      <span class="llm-detail">{l.provider} / {l.model}</span>
                    {/if}
                  </div>
                  <div class="llm-actions">
                    <button onclick={() => startEdit(i)}>{$tr('settings.llm.edit')}</button>
                    <button class="danger" onclick={() => deleteLlm(i)}>{$tr('settings.llm.delete')}</button>
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
    border-radius: var(--radius);
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
    border-radius: var(--radius);
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
    border-radius: var(--radius);
    padding: 8px 10px;
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
    border-radius: var(--radius);
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
    border-radius: var(--radius);
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
    border-radius: var(--radius);
    border: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
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
    border-radius: var(--radius);
    padding: 5px 12px;
    font-size: 12px;
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
    border-radius: var(--radius);
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
</style>
