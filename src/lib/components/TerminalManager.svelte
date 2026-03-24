<script lang="ts">
  import { get } from 'svelte/store';
  import Terminal from './Terminal.svelte';
  import ImageDrop from './ImageDrop.svelte';
  import TerminalToolbar from './TerminalToolbar.svelte';
  import type { Adapter } from '$lib/adapter/index';
  import { llmConfigs } from '$lib/stores/config';
  import {
    terminalInstances,
    activeInstanceId,
    activeInstance,
    computedLabels,
    addInstance as addInstanceToStore,
    removeInstance,
  } from '$lib/stores/terminals';
  import type { TerminalInstance } from '$lib/stores/terminals';
  import { showSettings } from '$lib/stores/ui';
  import { tr } from '$lib/i18n/index';
  import { defaultSlashCommands } from '$lib/data/slash-commands';
  import { menuKeyHandler } from '$lib/utils/a11y';
  import ChatView from './chat/ChatView.svelte';
  import type { ViewMode } from '$lib/types/agent-event';
  import { processAgentEvent, streamingSessionIds } from '$lib/stores/agent-events';
  import WorkspaceTrustDialog from './WorkspaceTrustDialog.svelte';
  import { workspaceTrustLevel } from '$lib/stores/workspace-trust';
  import type { TrustLevel, FolderInfo } from '$lib/stores/workspace-trust';

  let { adapter, cwd = '.' }: { adapter: Adapter; cwd?: string } = $props();

  let showLLMDropdown = $state(false);
  let llmMenuEl = $state<HTMLElement | null>(null);
  let addBtnEl = $state<HTMLElement | null>(null);
  let addWrapperEl = $state<HTMLElement | null>(null);
  let showViewModeDropdown = $state(false);
  let viewModeDropdownEl = $state<HTMLElement | null>(null);
  let viewModeBtnEl = $state<HTMLElement | null>(null);
  let viewModeWrapperEl = $state<HTMLElement | null>(null);

  let showTrustDialog = $state(false);
  let trustFolderInfo = $state<FolderInfo | null>(null);
  let pendingProviderName = $state<string | null>(null);

  // Compute fixed dropdown position from anchor button rect
  let llmDropdownStyle = $derived.by(() => {
    if (!showLLMDropdown || !addBtnEl) return '';
    const r = addBtnEl.getBoundingClientRect();
    return `position:fixed;top:${r.bottom}px;left:${r.left}px;`;
  });

  let viewModeDropdownStyle = $derived.by(() => {
    if (!showViewModeDropdown || !viewModeBtnEl) return '';
    const r = viewModeBtnEl.getBoundingClientRect();
    return `position:fixed;top:${r.bottom}px;left:${r.left}px;`;
  });

  $effect(() => {
    if (showLLMDropdown && llmMenuEl) {
      const first = llmMenuEl.querySelector<HTMLElement>('[role="menuitem"]');
      first?.focus();
    }
  });

  // Click-outside handler for the dropdown — uses mousedown on document
  // instead of svelte:window onclick to avoid Svelte 5 event delegation issues
  $effect(() => {
    if (!showLLMDropdown) return;
    function handleOutsideClick(e: MouseEvent) {
      if (addWrapperEl && !addWrapperEl.contains(e.target as Node)) {
        showLLMDropdown = false;
      }
    }
    document.addEventListener('mousedown', handleOutsideClick);
    return () => document.removeEventListener('mousedown', handleOutsideClick);
  });

  // Click-outside handler for view mode dropdown
  $effect(() => {
    if (!showViewModeDropdown) return;
    function handleOutsideClick(e: MouseEvent) {
      if (viewModeWrapperEl && !viewModeWrapperEl.contains(e.target as Node)) {
        showViewModeDropdown = false;
      }
    }
    document.addEventListener('mousedown', handleOutsideClick);
    return () => document.removeEventListener('mousedown', handleOutsideClick);
  });

  // Derived configs from store (CLI with command + API with provider)
  let configs = $derived($llmConfigs.filter((c) => (c.type === 'cli' && c.command) || c.type === 'api'));

  let activeConfig = $derived(configs.find((c) => c.name === $activeInstance?.provider));

  let streaming = $derived($streamingSessionIds.has($activeInstanceId ?? ''));

  // Per-instance view mode: tracks whether each instance shows chat or terminal
  let instanceViewModes: Record<string, ViewMode> = $state({});

  function isApiOnlyInstance(instanceId: string): boolean {
    return get(terminalInstances).find((i) => i.id === instanceId)?.apiOnly === true;
  }

  function getViewMode(instanceId: string): ViewMode {
    if (isApiOnlyInstance(instanceId)) return 'chat';
    return instanceViewModes[instanceId] ?? 'chat';
  }

  function toggleViewMode(instanceId: string) {
    if (isApiOnlyInstance(instanceId)) return;
    instanceViewModes[instanceId] = getViewMode(instanceId) === 'chat' ? 'terminal' : 'chat';
  }

  export async function addInstance(providerName: string) {
    const cwdPath = cwd || '.';

    // Check workspace trust
    const trustResult = await adapter.checkWorkspaceTrust(cwdPath);

    if (trustResult.needs_prompt) {
      // Show trust dialog and wait for decision
      trustFolderInfo = trustResult.folder_info;
      pendingProviderName = providerName;
      showTrustDialog = true;
      return;
    }

    if (trustResult.level === 'blocked') {
      // Already decided to block — don't spawn
      workspaceTrustLevel.set('blocked');
      return;
    }

    workspaceTrustLevel.set(trustResult.level);
    await doAddInstance(providerName);
  }

  async function doAddInstance(providerName: string) {
    const cwdPath = cwd || '.';
    const config = get(llmConfigs).find((c) => c.name === providerName);
    if (!config) return;

    const isApiOnly = config.type === 'api' || !config.command;
    let instanceId: string;

    if (isApiOnly) {
      instanceId = `api-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    } else {
      const args = [...(config.args ?? [])];

      // PTY mode: add permission args based on workspace trust level
      const currentTrust = get(workspaceTrustLevel);
      if (currentTrust === 'trusted') {
        const normConfig = await adapter.getNormalizerConfig?.(providerName);
        if (normConfig?.permission_args) {
          for (const arg of normConfig.permission_args) {
            args.push(arg.replace('{project_root}', cwdPath));
          }
        }
      }
      // read_only: do NOT pass permission_args — user controls CLI directly
      // blocked: should never reach here (caught by addInstance trust check)

      let handle;
      try {
        handle = await adapter.spawnProcess(config.command!, args, cwd);
      } catch (err) {
        console.error('Failed to spawn process:', err);
        return;
      }
      instanceId = handle.id;
    }

    const instance: TerminalInstance = {
      id: instanceId,
      provider: providerName,
      label: `${providerName} ...`,
      modelName: config.model ?? undefined,
      apiOnly: isApiOnly || undefined,
    };

    addInstanceToStore(instance);
    activeInstanceId.set(instanceId);
  }

  async function handleTrustDecision(level: TrustLevel) {
    const cwdPath = cwd || '.';
    showTrustDialog = false;

    await adapter.setWorkspaceTrust(cwdPath, level);
    workspaceTrustLevel.set(level);

    if (level !== 'blocked' && pendingProviderName) {
      await doAddInstance(pendingProviderName);
    }
    pendingProviderName = null;
    trustFolderInfo = null;
  }

  async function closeInstance(id: string) {
    const inst = get(terminalInstances).find(i => i.id === id);
    if (!inst) return;

    const label = get(computedLabels).get(id) ?? id;
    const { ask } = await import('@tauri-apps/plugin-dialog');
    const ok = await ask($tr('terminal.terminateConfirm', { llmName: inst.provider, label }), {
      title: 'Reasonance',
      kind: 'warning',
    });
    if (!ok) return;

    if (!id.startsWith('api-')) {
      adapter.killProcess(id).catch((e) => console.warn('Failed to kill process:', e));
    }

    removeInstance(id);

    const remaining = get(terminalInstances);
    if (remaining.length === 0) {
      activeInstanceId.set(null);
    } else if (get(activeInstanceId) === id) {
      activeInstanceId.set(remaining[0].id);
    }
  }

  // Slash commands are defined in src/lib/data/slash-commands.ts
  let activeSlashCommands = $derived.by(() => {
    if (activeConfig?.slashCommands?.length) return activeConfig.slashCommands;
    const cmd = activeConfig?.command?.toLowerCase() ?? '';
    return defaultSlashCommands[cmd] ?? [];
  });

  // Keyboard navigation for flat tab bar
  function handleTabKeydown(e: KeyboardEvent) {
    const instances = get(terminalInstances);
    if (instances.length === 0) return;
    const currentId = get(activeInstanceId);
    const currentIdx = instances.findIndex(i => i.id === currentId);

    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
      e.preventDefault();
      const next = (currentIdx + 1) % instances.length;
      activeInstanceId.set(instances[next].id);
    } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
      e.preventDefault();
      const prev = (currentIdx - 1 + instances.length) % instances.length;
      activeInstanceId.set(instances[prev].id);
    } else if (e.key === 'Home') {
      e.preventDefault();
      activeInstanceId.set(instances[0].id);
    } else if (e.key === 'End') {
      e.preventDefault();
      activeInstanceId.set(instances[instances.length - 1].id);
    }
  }

  // Global Ctrl+W to close active tab
  function handleGlobalKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === 'w') {
      e.preventDefault();
      const id = get(activeInstanceId);
      if (id) closeInstance(id);
    }
  }

  // Note: Terminal-mode instances use the per-model permissionLevel from config.
  // When permissionLevel changes in Settings, the user needs to restart the terminal instance.
  // Chat-mode instances pick up the new level on the next message send.

  // Listen for agent events from the structured transport
  $effect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;
    adapter.onAgentEvent((payload) => {
      processAgentEvent(payload);
    }).then((fn) => {
      if (cancelled) { fn(); return; }
      unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  });
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<div class="terminal-manager">
  <!-- Flat Tab Bar -->
  <div class="flat-tabs" role="tablist" aria-label="Session tabs" onkeydown={handleTabKeydown}>
    {#each $terminalInstances as inst (inst.id)}
      {@const label = $computedLabels.get(inst.id) ?? inst.provider}
      {@const isActive = inst.id === $activeInstanceId}
      {@const hasError = inst.progressState === 2}
      <div class="tab-group" class:active={isActive} class:error={hasError}>
        <button
          class="flat-tab"
          class:active={isActive}
          class:error={hasError}
          role="tab"
          aria-selected={isActive}
          aria-label="{label}, Provider {inst.provider}{isActive ? ', Active' : ''}{hasError ? ', Error' : ''}"
          title="Provider: {inst.provider}"
          onclick={() => activeInstanceId.set(inst.id)}
          onauxclick={(e) => { if (e.button === 1) closeInstance(inst.id); }}
        >
          {label}{#if hasError} <span class="error-indicator">!</span>{/if}
        </button>
        <button
          class="close-btn"
          aria-label="Close {label}"
          onclick={(e) => { e.stopPropagation(); closeInstance(inst.id); }}
        >&times;</button>
        {#if isActive && !inst.apiOnly}
          <div class="viewmode-wrapper" bind:this={viewModeWrapperEl}>
            <button
              class="viewmode-btn"
              bind:this={viewModeBtnEl}
              aria-label="Switch view mode"
              aria-haspopup="menu"
              aria-expanded={showViewModeDropdown}
              onclick={(e) => { e.stopPropagation(); showViewModeDropdown = !showViewModeDropdown; }}
            >▾</button>
          </div>
        {/if}
      </div>
    {/each}

    <div class="tab-add-wrapper" bind:this={addWrapperEl}>
      <button
        class="flat-tab add-tab"
        bind:this={addBtnEl}
        aria-label="New session"
        aria-haspopup="menu"
        aria-expanded={showLLMDropdown}
        onclick={() => { showLLMDropdown = !showLLMDropdown; }}
      >+</button>
    </div>

    {#if $terminalInstances.length > 0}
      <span class="status-badge" role="status" aria-label="Session status: {streaming ? 'streaming response' : 'active and ready'}">
        {#if streaming}STREAMING{:else}ACTIVE{/if}
      </span>
    {/if}
  </div>

  {#if $workspaceTrustLevel === 'blocked'}
    <div class="empty-state">
      <div class="empty-header">{$tr('trust.bannerBlocked')}</div>
      <button class="no-llm-btn" onclick={() => { showTrustDialog = true; }}>
        {$tr('trust.bannerTrustBtn')}
      </button>
    </div>
  {:else if $terminalInstances.length === 0}
    <div class="empty-state">
      <div class="empty-header">{$tr('terminal.header')}</div>
      <p class="empty-subtitle">{$tr('terminal.startLLM')}</p>
      {#if configs.length === 0}
        <div class="no-llm-banner" role="status">
          <span class="no-llm-msg">{$tr('terminal.configHint')}</span>
          <button class="no-llm-btn" onclick={() => showSettings.set(true)}>
            {$tr('terminal.openSettings')}
          </button>
        </div>
      {:else}
        <div class="llm-selector">
          <span class="llm-count">{$tr('status.llmDetected', { count: String(configs.length) })}</span>
          <div class="llm-card-list">
            {#each configs as config (config.name)}
              <button
                class="llm-card"
                onclick={() => { addInstance(config.name); }}
              >
                <span class="llm-card-name">{config.name}</span>
                {#if config.type === 'api'}
                  <span class="llm-card-badge">API</span>
                {:else if config.command}
                  {@const basename = config.command.split('/').pop()?.split('\\').pop() ?? config.command}
                  {#if basename.toLowerCase() !== config.name.toLowerCase()}
                    <span class="llm-card-cmd">{basename}</span>
                  {/if}
                {/if}
              </button>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {:else}
    <!-- Terminal Toolbar (only in terminal mode for active instance) -->
    {#if $activeInstanceId && activeConfig && getViewMode($activeInstanceId) === 'terminal'}
      <TerminalToolbar
        {adapter}
        instanceId={$activeInstanceId}
        llmName={$activeInstance?.provider ?? ''}
        activeMode={$activeInstance?.activeMode}
        modes={activeConfig.modes ?? []}
        slashCommands={activeSlashCommands}
      />
    {/if}

    <div class="terminal-area">
      {#each $terminalInstances as inst (inst.id)}
        <div
          class="terminal-wrap"
          style="display: {inst.id === $activeInstanceId ? 'flex' : 'none'};"
        >
          {#if getViewMode(inst.id) === 'chat'}
            <ChatView
              {adapter}
              sessionId={inst.id}
              provider={inst.provider}
              model={inst.modelName ?? inst.provider}
              configName={inst.provider}
            />
          {:else if getViewMode(inst.id) === 'terminal'}
            {#if $workspaceTrustLevel === null}
              <div class="pty-trust-warning">
                Trust revoked — new commands may be restricted.
              </div>
            {/if}
            <ImageDrop {adapter} instanceId={inst.id} llmName={inst.provider}>
              {#snippet children()}
                <Terminal {adapter} ptyId={inst.id} />
              {/snippet}
            </ImageDrop>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if showTrustDialog && trustFolderInfo}
    <WorkspaceTrustDialog folderInfo={trustFolderInfo} onDecision={handleTrustDecision} />
  {/if}
</div>

<!-- Fixed-position dropdowns (outside overflow containers) -->
{#if showLLMDropdown}
  <div class="fixed-dropdown" style={llmDropdownStyle}
       role="menu" tabindex="-1"
       bind:this={llmMenuEl}
       onkeydown={(e) => menuKeyHandler(e, llmMenuEl!, '[role="menuitem"]')}>
    <span class="dropdown-header">New instance</span>
    {#each configs as config (config.name)}
      <button
        class="dropdown-item"
        role="menuitem"
        tabindex="-1"
        onclick={() => { addInstance(config.name); showLLMDropdown = false; }}
      >{config.name}</button>
    {/each}
    {#if configs.length === 0}
      <span class="dropdown-empty">{$tr('terminal.configHint')}</span>
    {/if}
  </div>
{/if}

{#if showViewModeDropdown && $activeInstanceId}
  <div class="fixed-dropdown" style={viewModeDropdownStyle}
       role="menu" tabindex="-1"
       bind:this={viewModeDropdownEl}>
    <button
      class="dropdown-item"
      class:active={getViewMode($activeInstanceId) === 'chat'}
      role="menuitem"
      tabindex="-1"
      onclick={() => { if (getViewMode($activeInstanceId!) !== 'chat') toggleViewMode($activeInstanceId!); showViewModeDropdown = false; }}
    >CHAT</button>
    <button
      class="dropdown-item"
      class:active={getViewMode($activeInstanceId) === 'terminal'}
      role="menuitem"
      tabindex="-1"
      onclick={() => { if (getViewMode($activeInstanceId!) !== 'terminal') toggleViewMode($activeInstanceId!); showViewModeDropdown = false; }}
    >XTERM</button>
  </div>
{/if}

<style>
  .terminal-manager {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-surface);
  }

  .flat-tabs {
    display: flex;
    align-items: center;
    gap: var(--stack-tight);
    padding: 0 var(--space-2);
    background: var(--bg-primary);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
    overflow-x: auto;
    font-family: var(--font-ui);
    height: 38px;
  }

  .flat-tab {
    display: flex;
    align-items: center;
    gap: var(--stack-tight);
    padding: var(--space-1) var(--space-2);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 600;
    border: var(--border-width) solid var(--border);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
    min-height: 24px;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .flat-tab:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .flat-tab.active {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-color: var(--accent);
    font-weight: 700;
  }

  .flat-tab.add-tab {
    font-size: var(--font-size-small);
    font-weight: 700;
    padding: var(--space-1) var(--space-2);
  }

  .tab-add-wrapper {
    position: relative;
    flex-shrink: 0;
  }

  .tab-group {
    display: flex;
    align-items: center;
    gap: 0;
    position: relative;
    flex-shrink: 0;
  }

  .close-btn {
    font-size: var(--font-size-sm);
    line-height: 1;
    opacity: 0;
    cursor: pointer;
    padding: var(--space-1) var(--space-1);
    min-width: 24px;
    min-height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    color: inherit;
    transition: opacity var(--transition-fast);
  }

  .tab-group:hover .close-btn,
  .tab-group:focus-within .close-btn {
    opacity: 0.6;
  }

  .close-btn:hover {
    opacity: 1;
    color: var(--danger-text);
  }

  .tab-group.error .flat-tab {
    border-color: var(--danger);
  }

  .flat-tab.error {
    color: var(--danger-text);
    border-color: var(--danger);
  }

  .error-indicator {
    color: var(--danger-text);
    font-weight: 800;
    font-size: var(--font-size-sm);
  }

  .status-badge {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: var(--space-1) var(--space-2);
    min-height: 24px;
    display: inline-flex;
    align-items: center;
    color: var(--success-text);
    white-space: nowrap;
    margin-inline-start: auto;
  }

  .viewmode-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .viewmode-btn {
    font-size: var(--font-size-sm);
    line-height: 1;
    cursor: pointer;
    padding: var(--space-1);
    min-width: 20px;
    min-height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    color: var(--text-secondary);
    transition: color var(--transition-fast);
  }

  .viewmode-btn:hover {
    color: var(--text-primary);
  }

  .fixed-dropdown {
    z-index: 200;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    min-width: 120px;
    padding: var(--space-1) 0;
    font-family: var(--font-ui);
  }

  .fixed-dropdown .dropdown-item.active {
    color: var(--accent-text);
    font-weight: 700;
  }

  .dropdown-header {
    display: block;
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  .dropdown-item {
    display: block;
    width: 100%;
    padding: var(--space-1) var(--space-3);
    text-align: start;
    background: transparent;
    border: none;
    color: var(--text-body);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    cursor: pointer;
  }

  .dropdown-item:hover {
    background: var(--bg-hover);
  }

  .dropdown-empty {
    display: block;
    padding: var(--space-1) var(--space-3);
    color: var(--text-muted);
    font-size: var(--font-size-small);
  }

  .terminal-area {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .terminal-wrap {
    position: absolute;
    inset: 0;
    flex-direction: column;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--stack-loose);
    color: var(--text-primary);
    font-family: var(--font-ui);
    padding: var(--space-5);
    min-height: 200px;
    background: var(--bg-primary);
  }

  .empty-header {
    font-size: var(--font-size-sm);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    margin-bottom: var(--stack-tight);
  }

  .empty-subtitle {
    margin: 0;
    font-size: var(--font-size-base);
    font-weight: 700;
    text-align: center;
    color: var(--text-primary);
  }

  .empty-state p {
    margin: 0;
    font-size: var(--font-size-base);
    font-weight: 600;
    text-align: center;
  }

  .llm-selector {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    align-items: center;
    margin-top: var(--space-2);
    width: 100%;
    max-width: 280px;
  }

  .llm-card-list {
    display: flex;
    flex-direction: column;
    gap: var(--stack-tight);
    width: 100%;
  }

  .llm-count {
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .llm-card {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: var(--bg-secondary);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-body);
    font-family: var(--font-ui);
    cursor: pointer;
    text-align: start;
    transition: background var(--transition-fast), border-color var(--transition-fast);
  }

  .llm-card:hover {
    background: var(--bg-tertiary);
    border-color: var(--text-muted);
  }

  .llm-card-name {
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    color: var(--text-primary);
  }

  .llm-card-cmd {
    font-size: var(--font-size-tiny);
    font-family: var(--font-mono);
    color: var(--text-muted);
  }

  .llm-card-badge {
    font-size: var(--font-size-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: var(--stack-tight) var(--space-1);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }

  .no-llm-banner {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--inset-section) var(--space-5);
    background: var(--bg-secondary);
    border: 2px solid var(--accent);
    max-width: 320px;
    font-family: var(--font-ui);
    text-align: center;
  }

  .no-llm-msg {
    font-size: var(--font-size-base);
    font-weight: 600;
    color: var(--text-primary);
  }

  .no-llm-btn {
    padding: var(--space-2) var(--space-4);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: 2px solid var(--accent);
    border-radius: 0;
    background: var(--accent);
    color: var(--text-on-accent);
    cursor: pointer;
    transition: opacity var(--transition-fast);
  }

  .no-llm-btn:hover {
    opacity: 0.85;
  }

  .pty-trust-warning {
    padding: var(--space-1) var(--space-2);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    text-align: center;
  }
</style>
