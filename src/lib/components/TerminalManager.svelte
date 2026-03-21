<script lang="ts">
  import { get } from 'svelte/store';
  import { onDestroy } from 'svelte';
  import Terminal from './Terminal.svelte';
  import ImageDrop from './ImageDrop.svelte';
  import TerminalToolbar from './TerminalToolbar.svelte';
  import SwarmPanel from './swarm/SwarmPanel.svelte';
  import type { Adapter } from '$lib/adapter/index';
  import { llmConfigs } from '$lib/stores/config';
  import { terminalTabs, activeTerminalTab, activeInstanceId } from '$lib/stores/terminals';
  import type { LlmConfig } from '$lib/stores/config';
  import { yoloMode } from '$lib/stores/ui';

  let { adapter, cwd = '.' }: { adapter: Adapter; cwd?: string } = $props();

  let configs = $state<LlmConfig[]>([]);
  let tabs = $state<import('$lib/stores/terminals').TerminalTab[]>([]);
  let activeTab = $state<string | null>(null);
  let activeInstance = $state<string | null>(null);
  let showSwarmTab = $state(false);

  const unsubConfigs = llmConfigs.subscribe((val) => {
    configs = val.filter((c) => c.type === 'cli' && c.command);
  });
  const unsubTabs = terminalTabs.subscribe((val) => { tabs = val; });
  const unsubActiveTab = activeTerminalTab.subscribe((val) => { activeTab = val; });
  const unsubActiveInstance = activeInstanceId.subscribe((val) => { activeInstance = val; });

  onDestroy(() => {
    unsubConfigs();
    unsubTabs();
    unsubActiveTab();
    unsubActiveInstance();
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
    const label = `inst. ${instanceCounters[llmName]}`;

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
    adapter.killProcess(id).catch(() => {});
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
    tabs.find((t) => t.llmName === activeTab)?.instances ?? []
  );

  let activeConfig = $derived(configs.find((c) => c.name === activeTab));

  let activeInstanceData = $derived(
    activeTabInstances.find((i) => i.id === activeInstance)
  );

  function generateBar(percent: number): string {
    const filled = Math.round(percent / 12.5);
    const empty = 8 - filled;
    return '\u2588'.repeat(filled) + '\u2591'.repeat(empty);
  }
</script>

<div class="terminal-manager">
  <!-- LLM Tab Bar -->
  <div class="llm-tabs">
    {#each tabs as tab (tab.llmName)}
      <button
        class="llm-tab"
        class:active={tab.llmName === activeTab}
        onclick={() => selectTab(tab.llmName)}
      >
        {tab.llmName}
      </button>
    {/each}

    <!-- Add LLM buttons for unconfigured CLIs -->
    {#each configs as config (config.name)}
      {#if !tabs.find((t) => t.llmName === config.name)}
        <button class="llm-tab add-llm" onclick={() => addInstance(config.name)}>
          + {config.name}
        </button>
      {/if}
    {/each}

    <button
      class="llm-tab"
      class:active={showSwarmTab}
      onclick={() => { showSwarmTab = true; activeTerminalTab.set(null); }}
    >
      Swarm
    </button>
  </div>

  {#if showSwarmTab}
    <div class="terminal-area">
      <SwarmPanel {adapter} {cwd} />
    </div>
  {:else if tabs.length === 0}
    <div class="empty-state">
      <p>Avvia un LLM per iniziare</p>
      {#if configs.length === 0}
        <p class="hint">Configura un LLM nel file di configurazione</p>
      {:else}
        <div class="start-buttons">
          {#each configs as config (config.name)}
            <button class="start-btn" onclick={() => addInstance(config.name)}>
              + {config.name}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Instance Tab Bar -->
    <div class="instance-tabs">
      {#each activeTabInstances as inst (inst.id)}
        <button
          class="instance-tab"
          class:active={inst.id === activeInstance}
          onclick={() => selectInstance(inst.id)}
        >
          {inst.label}
          <span
            class="close-btn"
            role="button"
            tabindex="0"
            onclick={(e) => { e.stopPropagation(); closeInstance(inst.llmName, inst.id); }}
            onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); closeInstance(inst.llmName, inst.id); } }}
          >×</span>
        </button>
      {/each}

      <!-- Add instance button -->
      {#if activeTab}
        <button
          class="instance-tab add-instance"
          onclick={() => addInstance(activeTab!)}
        >+</button>
      {/if}
    </div>

    <!-- Terminal Toolbar -->
    {#if activeInstance && activeConfig}
      <TerminalToolbar
        {adapter}
        instanceId={activeInstance}
        llmName={activeTab ?? ''}
        activeMode={activeInstanceData?.activeMode}
        modes={activeConfig.modes ?? []}
        slashCommands={activeConfig.slashCommands ?? []}
      />
    {/if}

    <!-- Terminal instances (hidden with display:none to keep them alive) -->
    <div class="terminal-area">
      {#each tabs as tab (tab.llmName)}
        {#each tab.instances as inst (inst.id)}
          <div
            class="terminal-wrap"
            style="display: {inst.id === activeInstance ? 'flex' : 'none'};"
          >
            <ImageDrop {adapter} instanceId={inst.id} llmName={inst.llmName}>
              {#snippet children()}
                <Terminal {adapter} ptyId={inst.id} />
              {/snippet}
            </ImageDrop>
            {#if inst.contextPercent != null || inst.tokenCount}
              <div class="ctx-footer">
                {#if inst.contextPercent != null}
                  <span class="ctx-bar" title="Context window usage">
                    ctx {generateBar(inst.contextPercent)} {inst.contextPercent}%
                  </span>
                {/if}
                {#if inst.tokenCount}
                  <span class="token-count">{inst.tokenCount} tokens</span>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      {/each}
    </div>
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

  .llm-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 0;
    padding: 0;
    background: var(--bg-primary);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
    font-family: var(--font-ui);
  }

  .llm-tab {
    padding: 8px 16px;
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

  .llm-tab.add-llm {
    color: var(--text-muted);
    border: var(--border-width) dashed var(--border);
    border-bottom: 2px solid transparent;
    background: transparent;
    margin: 2px 4px;
  }

  .llm-tab.add-llm:hover {
    color: var(--accent);
    border-color: var(--accent);
  }

  .instance-tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 3px 6px;
    background: var(--bg-primary);
    border-bottom: 1px solid var(--border);
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
    padding: 0 1px;
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

  .ctx-footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 2px 8px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .ctx-bar {
    letter-spacing: 0.05em;
  }

  .token-count {
    margin-left: auto;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    color: var(--text-muted);
    font-family: var(--font-ui);
  }

  .empty-state p {
    margin: 0;
    font-size: var(--font-size-base);
    font-weight: 500;
  }

  .hint {
    font-size: var(--font-size-small) !important;
    color: var(--text-muted);
  }

  .start-buttons {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    justify-content: center;
    margin-top: 4px;
  }

  .start-btn {
    padding: 8px 20px;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: 2px solid var(--accent);
    border-radius: 0;
    background: transparent;
    color: var(--accent);
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }

  .start-btn:hover {
    background: var(--accent);
    color: var(--text-primary);
  }
</style>
