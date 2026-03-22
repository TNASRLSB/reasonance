<script lang="ts">
  import { get } from 'svelte/store';
  import Terminal from './Terminal.svelte';
  import ImageDrop from './ImageDrop.svelte';
  import TerminalToolbar from './TerminalToolbar.svelte';
  import SwarmPanel from './swarm/SwarmPanel.svelte';
  import type { Adapter } from '$lib/adapter/index';
  import { llmConfigs } from '$lib/stores/config';
  import { terminalTabs, activeTerminalTab, activeInstanceId } from '$lib/stores/terminals';
  import type { LlmConfig } from '$lib/stores/config';
  import { yoloMode, showSettings } from '$lib/stores/ui';
  import { tr } from '$lib/i18n/index';
  import { defaultSlashCommands } from '$lib/data/slash-commands';
  import { menuKeyHandler } from '$lib/utils/a11y';

  let { adapter, cwd = '.' }: { adapter: Adapter; cwd?: string } = $props();

  let showSwarmTab = $state(false);
  let showLLMDropdown = $state(false);
  let llmMenuEl = $state<HTMLElement | null>(null);

  $effect(() => {
    if (showLLMDropdown && llmMenuEl) {
      const first = llmMenuEl.querySelector<HTMLElement>('[role="menuitem"]');
      first?.focus();
    }
  });
  let selectedLlmName = $state<string>('');

  // Derived CLI configs from store
  let configs = $derived($llmConfigs.filter((c) => c.type === 'cli' && c.command));

  // Auto-select first LLM when configs load
  $effect(() => {
    if (!selectedLlmName && configs.length > 0) {
      selectedLlmName = configs[0].name;
    }
  });

  // Count instances per LLM for label generation
  const instanceCounters: Record<string, number> = {};

  export async function addInstance(llmName: string) {
    const config = get(llmConfigs).find((c) => c.name === llmName);
    if (!config || !config.command) return;

    // Build args, optionally appending yoloFlag when YOLO mode is active
    const args = [...(config.args ?? [])];
    if (get(yoloMode) && config.yoloFlag) {
      args.push(config.yoloFlag);
    }

    let handle;
    try {
      handle = await adapter.spawnProcess(config.command, args, cwd);
    } catch (err) {
      console.error('Failed to spawn process:', err);
      return;
    }

    instanceCounters[llmName] = (instanceCounters[llmName] ?? 0) + 1;
    const label = `${llmName} ${instanceCounters[llmName]}`;

    const instance: import('$lib/stores/terminals').TerminalInstance = {
      id: handle.id,
      llmName,
      label,
    };

    terminalTabs.update((current) => {
      const existing = current.find((t) => t.llmName === llmName);
      if (existing) {
        existing.instances.push(instance);
        return [...current];
      } else {
        return [...current, { llmName, instances: [instance] }];
      }
    });

    activeTerminalTab.set(llmName);
    activeInstanceId.set(handle.id);
  }

  function selectTab(llmName: string) {
    showSwarmTab = false;
    activeTerminalTab.set(llmName);
    // Select first instance of that tab if current instance is not in this tab
    const tab = get(terminalTabs).find((t) => t.llmName === llmName);
    if (tab && tab.instances.length > 0) {
      const current = get(activeInstanceId);
      const inTab = tab.instances.find((i) => i.id === current);
      if (!inTab) {
        activeInstanceId.set(tab.instances[0].id);
      }
    }
  }

  function selectInstance(id: string) {
    activeInstanceId.set(id);
  }

  function closeInstance(llmName: string, id: string) {
    const inst = get(terminalTabs).flatMap((t) => t.instances).find((i) => i.id === id);
    const label = inst ? inst.label : id;
    const ok = confirm(`Terminate session?\n\n${llmName} — ${label}\n\nThis will kill the running process. Any unsaved work in the terminal will be lost.`);
    if (!ok) return;
    adapter.killProcess(id).catch((e) => console.warn('Failed to kill process:', e));
    terminalTabs.update((current) => {
      return current
        .map((t) => {
          if (t.llmName !== llmName) return t;
          return { ...t, instances: t.instances.filter((i) => i.id !== id) };
        })
        .filter((t) => t.instances.length > 0);
    });

    // Update active selections
    const newTabs = get(terminalTabs);
    if (newTabs.length === 0) {
      activeTerminalTab.set(null);
      activeInstanceId.set(null);
    } else {
      const stillActiveTab = newTabs.find((t) => t.llmName === get(activeTerminalTab));
      if (!stillActiveTab) {
        const first = newTabs[0];
        activeTerminalTab.set(first.llmName);
        activeInstanceId.set(first.instances[0]?.id ?? null);
      } else {
        const stillActiveInstance = stillActiveTab.instances.find(
          (i) => i.id === get(activeInstanceId)
        );
        if (!stillActiveInstance) {
          activeInstanceId.set(stillActiveTab.instances[0]?.id ?? null);
        }
      }
    }
  }

  // Active tab instances
  let activeTabInstances = $derived(
    $terminalTabs.find((t) => t.llmName === $activeTerminalTab)?.instances ?? []
  );

  // Slash commands are defined in src/lib/data/slash-commands.ts

  let activeConfig = $derived(configs.find((c) => c.name === $activeTerminalTab));

  // Merge default slash commands with config
  let activeSlashCommands = $derived.by(() => {
    if (activeConfig?.slashCommands?.length) return activeConfig.slashCommands;
    const cmd = activeConfig?.command?.toLowerCase() ?? '';
    return defaultSlashCommands[cmd] ?? [];
  });

  let activeInstanceData = $derived(
    activeTabInstances.find((i) => i.id === $activeInstanceId)
  );

  function generateBar(percent: number): string {
    const filled = Math.round(percent / 12.5);
    const empty = 8 - filled;
    return '\u2588'.repeat(filled) + '\u2591'.repeat(empty);
  }

</script>

<div class="terminal-manager">
  <!-- LLM Tab Bar -->
  <div class="llm-tabs" role="tablist" aria-label="LLM sessions">
    {#each $terminalTabs as tab (tab.llmName)}
      <button
        class="llm-tab"
        class:active={tab.llmName === $activeTerminalTab}
        role="tab"
        aria-selected={tab.llmName === $activeTerminalTab}
        onclick={() => selectTab(tab.llmName)}
      >
        {tab.llmName}
      </button>
    {/each}

    <!-- Add LLM dropdown -->
    <div class="llm-add-wrapper">
      <button class="llm-tab add-btn" onclick={() => showLLMDropdown = !showLLMDropdown} aria-label="Add LLM" aria-haspopup="true" aria-expanded={showLLMDropdown}>+</button>
      {#if showLLMDropdown}
        <div class="llm-dropdown" role="menu" bind:this={llmMenuEl} onkeydown={(e) => menuKeyHandler(e, llmMenuEl!, '[role="menuitem"]')}>
          {#each configs as config (config.name)}
            <button
              class="llm-dropdown-item"
              role="menuitem"
              tabindex="-1"
              onclick={() => { addInstance(config.name); showLLMDropdown = false; }}
            >
              {config.name}
            </button>
          {/each}
          {#if configs.length === 0}
            <span class="llm-dropdown-empty">{$tr('terminal.configHint')}</span>
          {/if}
        </div>
      {/if}
    </div>

    <button
      class="llm-tab"
      class:active={showSwarmTab}
      role="tab"
      aria-selected={showSwarmTab}
      onclick={() => { showSwarmTab = true; activeTerminalTab.set(null); }}
    >
      {$tr('terminal.swarm')}
    </button>
  </div>

  {#if showSwarmTab}
    <div class="terminal-area">
      <div class="empty-state">
        <p>{$tr('terminal.swarm')}</p>
        <p class="hint">{$tr('terminal.swarmComingSoon')}</p>
      </div>
    </div>
  {:else if $terminalTabs.length === 0}
    <div class="empty-state">
      <div class="empty-header">TERMINAL</div>
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
          <div class="llm-card-list">
            {#each configs as config (config.name)}
              <button
                class="llm-card"
                class:selected={config.name === selectedLlmName}
                onclick={() => { selectedLlmName = config.name; }}
              >
                <span class="llm-card-name">{config.name}</span>
                <span class="llm-card-cmd">{config.command}</span>
              </button>
            {/each}
          </div>
          <button class="start-btn" onclick={() => { if (selectedLlmName) addInstance(selectedLlmName); }}>
            &#9654; {$tr('terminal.startLLM')}
          </button>
        </div>
      {/if}
    </div>
  {:else}
    <!-- Instance Tab Bar -->
    <div class="instance-tabs" role="tablist" aria-label="Terminal instances">
      {#each activeTabInstances as inst (inst.id)}
        <button
          class="instance-tab"
          class:active={inst.id === $activeInstanceId}
          role="tab"
          aria-selected={inst.id === $activeInstanceId}
          onclick={() => selectInstance(inst.id)}
        >
          {inst.label}
          <span
            class="close-btn"
            role="button"
            tabindex="0"
            aria-label="Close tab"
            onclick={(e) => { e.stopPropagation(); closeInstance(inst.llmName, inst.id); }}
            onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); closeInstance(inst.llmName, inst.id); } }}
          >×</span>
        </button>
      {/each}

      <!-- Add instance button -->
      {#if $activeTerminalTab}
        <button
          class="instance-tab add-instance"
          onclick={() => addInstance($activeTerminalTab!)}
        >+</button>
      {/if}
    </div>

    <!-- Terminal Toolbar -->
    {#if $activeInstanceId && activeConfig}
      <TerminalToolbar
        {adapter}
        instanceId={$activeInstanceId}
        llmName={$activeTerminalTab ?? ''}
        activeMode={activeInstanceData?.activeMode}
        modes={activeConfig.modes ?? []}
        slashCommands={activeSlashCommands}
      />
    {/if}

    <!-- Terminal instances (hidden with display:none to keep them alive) -->
    <div class="terminal-area">
      {#each $terminalTabs as tab (tab.llmName)}
        {#each tab.instances as inst (inst.id)}
          <div
            class="terminal-wrap"
            style="display: {inst.id === $activeInstanceId ? 'flex' : 'none'};"
          >
            <ImageDrop {adapter} instanceId={inst.id} llmName={inst.llmName}>
              {#snippet children()}
                <Terminal {adapter} ptyId={inst.id} />
              {/snippet}
            </ImageDrop>
          </div>
        {/each}
      {/each}
    </div>

    <!-- Model info bar (per-instance) -->
    {#if activeInstanceData}
      <div class="model-info-bar" aria-live="polite">
        {#if activeInstanceData.contextPercent != null}
          <span class="ctx-info" title="Context window usage">
            ctx <span class="ctx-bar">{generateBar(activeInstanceData.contextPercent)}</span> {activeInstanceData.contextPercent}%
          </span>
        {/if}
        {#if activeInstanceData.tokenCount}
          <span class="token-count" title="Tokens used">{activeInstanceData.tokenCount} tokens</span>
        {/if}
        {#if activeInstanceData.modelName}
          <span class="model-name">{activeInstanceData.modelName}</span>
        {/if}
        {#if activeInstanceData.resetTimer}
          <span class="reset-timer" title="Rate limit reset">&#9201; {activeInstanceData.resetTimer}</span>
        {/if}
        {#if activeInstanceData.messagesLeft != null}
          <span class="msg-left" title="Messages remaining">&#9993; {activeInstanceData.messagesLeft}</span>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .terminal-manager {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-surface);
  }

  .model-info-bar {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 4px 10px;
    background: var(--bg-primary);
    border-top: 2px solid var(--border);
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .ctx-info {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .ctx-bar {
    letter-spacing: 0.05em;
    font-size: 10px;
    color: var(--accent);
  }

  .token-count {
    color: var(--text-body);
    font-weight: 500;
  }

  .model-name {
    font-weight: 700;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .reset-timer {
    color: var(--warning);
  }

  .msg-left {
    color: var(--text-secondary);
  }

  .llm-tabs {
    display: flex;
    flex-wrap: nowrap;
    gap: 0;
    padding: 0;
    background: var(--bg-primary);
    border-bottom: 2px solid var(--border);
    flex-shrink: 0;
    font-family: var(--font-ui);
    height: 38px;
    align-items: stretch;
  }

  .llm-tab {
    padding: 0 16px;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: none;
    border-radius: 0;
    border-bottom: 2px solid transparent;
    background: var(--bg-primary);
    color: var(--text-muted);
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
    display: flex;
    align-items: center;
  }

  .llm-tab:hover {
    background: var(--bg-surface);
    color: var(--text-primary);
  }

  .llm-tab.active {
    color: var(--text-primary);
    border-bottom-color: var(--accent);
    background: var(--bg-secondary);
  }

  .llm-add-wrapper {
    position: relative;
  }

  .add-btn {
    font-size: 16px;
    font-weight: 700;
    padding: 0 14px;
  }

  .llm-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    z-index: 100;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    min-width: 160px;
    padding: 4px 0;
  }

  .llm-dropdown-item {
    display: block;
    width: 100%;
    padding: 6px 14px;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--text-body);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    cursor: pointer;
  }

  .llm-dropdown-item:hover {
    background: var(--bg-hover);
  }

  .llm-dropdown-empty {
    display: block;
    padding: 6px 14px;
    color: var(--text-muted);
    font-size: var(--font-size-small);
  }

  .instance-tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px 8px;
    background: var(--bg-primary);
    border-bottom: 2px solid var(--border);
    flex-shrink: 0;
    overflow-x: auto;
    font-family: var(--font-ui);
  }

  .instance-tab {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.1s, color 0.1s;
  }

  .instance-tab:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .instance-tab.active {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-color: var(--accent);
  }

  .instance-tab.add-instance {
    font-size: 14px;
    font-weight: 700;
    padding: 1px 7px;
  }

  .close-btn {
    font-size: 13px;
    line-height: 1;
    opacity: 0.6;
    cursor: pointer;
    padding: 5px 6px;
    min-width: 24px;
    min-height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .close-btn:hover {
    opacity: 1;
    color: var(--danger);
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
    gap: 16px;
    color: var(--text-primary);
    font-family: var(--font-ui);
    padding: 24px;
    min-height: 200px;
    background: var(--bg-primary);
  }

  .empty-header {
    font-size: 11px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    margin-bottom: 4px;
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

  .hint {
    font-size: var(--font-size-small) !important;
    color: var(--text-muted);
  }

  .llm-selector {
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: center;
    margin-top: 8px;
    width: 100%;
    max-width: 280px;
  }

  .llm-card-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
  }

  .llm-card {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    padding: 10px 14px;
    background: var(--bg-secondary);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-body);
    font-family: var(--font-ui);
    cursor: pointer;
    text-align: left;
    transition: background 0.1s, border-color 0.1s;
  }

  .llm-card:hover {
    background: var(--bg-tertiary);
    border-color: var(--text-muted);
  }

  .llm-card.selected {
    border-color: var(--accent);
    background: var(--bg-tertiary);
  }

  .llm-card-name {
    font-size: var(--font-size-base);
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

  .start-btn {
    width: 100%;
    padding: 12px 20px;
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: 2px solid var(--accent);
    border-radius: 0;
    background: var(--accent);
    color: #fff;
    cursor: pointer;
    transition: background 0.1s, opacity 0.1s;
  }

  .start-btn:hover {
    opacity: 0.85;
  }

  .no-llm-banner {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
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
    padding: 7px 18px;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: 2px solid var(--accent);
    border-radius: 0;
    background: var(--accent);
    color: #fff;
    cursor: pointer;
    transition: opacity 0.1s;
  }

  .no-llm-btn:hover {
    opacity: 0.85;
  }
</style>
