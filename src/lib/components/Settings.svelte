<script lang="ts">
  import { parse, stringify } from 'smol-toml';
  import type { Adapter } from '$lib/adapter/index';
  import type { LlmConfig, AppSettings } from '$lib/stores/config';
  import { llmConfigs, appSettings } from '$lib/stores/config';
  import { fontFamily, fontSize } from '$lib/stores/ui';
  import { themeMode, type ThemeMode } from '$lib/stores/theme';
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
  let localFontFamily = $state("'JetBrains Mono', 'Fira Code', monospace");
  let localFontSize = $state(14);
  let localTheme = $state<ThemeMode>('system');

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

        const s = parsed.settings ?? {};
        settings = {
          default: s.default !== undefined ? String(s.default) : undefined,
          contextMenuLlm:
            s.context_menu_llm !== undefined ? String(s.context_menu_llm) : undefined,
        };
      } else {
        llms = [];
        settings = {};
      }
    } catch (e) {
      error = `Failed to load config: ${e}`;
      llms = get(llmConfigs);
      settings = get(appSettings);
    }

    localFontFamily = get(fontFamily);
    localFontSize = get(fontSize);
    localTheme = get(themeMode);
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

      // Update stores
      llmConfigs.set(llms);
      appSettings.set(settings);
      fontFamily.set(localFontFamily);
      fontSize.set(localFontSize);
      themeMode.set(localTheme);

      onClose();
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
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="settings-overlay" onclick={handleOverlayClick}>
    <div class="settings-modal" role="dialog" aria-modal="true" aria-label="Settings">
      <div class="modal-header">
        <h2>Settings</h2>
        <button class="close-btn" onclick={onClose} aria-label="Close">✕</button>
      </div>

      {#if error}
        <div class="error-banner">{error}</div>
      {/if}

      <div class="modal-body">
        <!-- LLM Configuration -->
        <section>
          <h3>LLM Configuration</h3>

          <!-- Default LLM selector -->
          <div class="field-row">
            <label for="default-llm">Default LLM</label>
            <select id="default-llm" bind:value={settings.default}>
              <option value="">— none —</option>
              {#each llms as l}
                <option value={l.name}>{l.name}</option>
              {/each}
            </select>
          </div>

          <div class="field-row">
            <label for="context-menu-llm">Context Menu LLM</label>
            <select id="context-menu-llm" bind:value={settings.contextMenuLlm}>
              <option value="">— none —</option>
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
                    <button onclick={() => startEdit(i)}>Edit</button>
                    <button class="danger" onclick={() => deleteLlm(i)}>Delete</button>
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <p class="empty-hint">No LLMs configured yet.</p>
          {/if}

          {#if !showAddForm}
            <button class="add-btn" onclick={startAdd}>+ Add LLM</button>
          {:else}
            <div class="llm-form">
              <h4>{editingIndex !== null ? 'Edit LLM' : 'Add LLM'}</h4>

              <div class="field-row">
                <label for="llm-name">Name</label>
                <input id="llm-name" type="text" bind:value={newLlm.name} placeholder="e.g. Claude" />
              </div>

              <div class="field-row">
                <label for="llm-type">Type</label>
                <select id="llm-type" bind:value={newLlm.type}>
                  <option value="cli">CLI</option>
                  <option value="api">API</option>
                </select>
              </div>

              {#if newLlm.type === 'cli'}
                <div class="field-row">
                  <label for="llm-command">Command</label>
                  <input id="llm-command" type="text" bind:value={newLlm.command} placeholder="e.g. claude" />
                </div>
                <div class="field-row">
                  <label for="llm-args">Args <small>(comma-separated)</small></label>
                  <input id="llm-args" type="text" bind:value={newLlmArgsStr} placeholder="e.g. --no-update-notification" />
                </div>
                <div class="field-row">
                  <label for="llm-yolo">YOLO Flag</label>
                  <input id="llm-yolo" type="text" bind:value={newLlm.yoloFlag} placeholder="e.g. --dangerously-skip-permissions" />
                </div>
                <div class="field-row">
                  <label for="llm-image-mode">Image Mode</label>
                  <select id="llm-image-mode" bind:value={newLlm.imageMode}>
                    <option value="path">Path</option>
                    <option value="base64">Base64</option>
                    <option value="none">None</option>
                  </select>
                </div>
              {:else}
                <div class="field-row">
                  <label for="llm-provider">Provider</label>
                  <select id="llm-provider" bind:value={newLlm.provider}>
                    <option value="anthropic">Anthropic</option>
                    <option value="openai">OpenAI</option>
                    <option value="openai-compatible">OpenAI-Compatible</option>
                  </select>
                </div>
                <div class="field-row">
                  <label for="llm-api-key-env">API Key Env Var</label>
                  <input id="llm-api-key-env" type="text" bind:value={newLlm.apiKeyEnv} placeholder="e.g. ANTHROPIC_API_KEY" />
                </div>
                <div class="field-row">
                  <label for="llm-model">Model</label>
                  <input id="llm-model" type="text" bind:value={newLlm.model} placeholder="e.g. claude-opus-4-5" />
                </div>
                {#if newLlm.provider === 'openai-compatible'}
                  <div class="field-row">
                    <label for="llm-endpoint">Endpoint</label>
                    <input id="llm-endpoint" type="text" bind:value={newLlm.endpoint} placeholder="https://api.example.com/v1" />
                  </div>
                {/if}
              {/if}

              <div class="form-actions">
                <button onclick={submitLlm}>{editingIndex !== null ? 'Update' : 'Add'}</button>
                <button class="secondary" onclick={cancelForm}>Cancel</button>
              </div>
            </div>
          {/if}
        </section>

        <!-- Terminal Settings -->
        <section>
          <h3>Terminal Settings</h3>
          <div class="field-row">
            <label for="font-family">Font Family</label>
            <input id="font-family" type="text" bind:value={localFontFamily} />
          </div>
          <div class="field-row">
            <label for="font-size">Font Size</label>
            <input id="font-size" type="number" min="8" max="32" bind:value={localFontSize} />
          </div>
        </section>

        <!-- Theme -->
        <section>
          <h3>Theme</h3>
          <div class="theme-toggle">
            {#each (['light', 'dark', 'system'] as ThemeMode[]) as mode}
              <button
                class="theme-btn"
                class:active={localTheme === mode}
                onclick={() => (localTheme = mode)}
              >
                {mode === 'light' ? 'Light' : mode === 'dark' ? 'Dark' : 'System'}
              </button>
            {/each}
          </div>
        </section>
      </div>

      <div class="modal-footer">
        <button class="secondary" onclick={onClose}>Cancel</button>
        <button class="primary" onclick={save} disabled={saving}>
          {saving ? 'Saving…' : 'Save'}
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
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 580px;
    max-width: 95vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--border);
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
    border-radius: 4px;
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
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    padding: 5px 8px;
    font-size: 12px;
    font-family: inherit;
  }

  .field-row input:focus,
  .field-row select:focus {
    outline: none;
    border-color: var(--accent);
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
    border: 1px solid var(--border);
    border-radius: 4px;
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
    border-radius: 3px;
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
    border: 1px solid var(--border);
    border-radius: 6px;
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
    border-radius: 4px;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }

  .theme-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .theme-btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  /* Buttons */
  button {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
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
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }
</style>
