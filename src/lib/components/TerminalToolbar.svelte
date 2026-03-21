<script lang="ts">
  import type { Adapter } from '$lib/adapter/index';

  let {
    adapter,
    instanceId,
    llmName,
    activeMode = undefined,
    modes = [],
    slashCommands = [],
  }: {
    adapter: Adapter;
    instanceId: string;
    llmName: string;
    activeMode?: string;
    modes?: { name: string; label: string; description: string }[];
    slashCommands?: { command: string; description: string }[];
  } = $props();

  let showSlashMenu = $state(false);
  let showModeMenu = $state(false);

  async function addFileToContext() {
    // TODO: Use @tauri-apps/plugin-dialog when available
    // For now, type a placeholder into the terminal
    await adapter.writePty(instanceId, '/file ');
  }

  function selectSlashCommand(command: string) {
    adapter.writePty(instanceId, command + ' ');
    showSlashMenu = false;
  }

  function selectMode(modeName: string) {
    // TODO: Wire mode switching via adapter
    showModeMenu = false;
  }

  function handleClickOutside(e: MouseEvent) {
    showSlashMenu = false;
    showModeMenu = false;
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="term-toolbar">
  <div class="term-toolbar-left">
    <button
      class="term-tbtn"
      title="Add file to context"
      onclick={(e) => { e.stopPropagation(); addFileToContext(); }}
    >+</button>

    <div class="slash-wrapper">
      <button
        class="term-tbtn"
        title="Slash commands"
        onclick={(e) => { e.stopPropagation(); showSlashMenu = !showSlashMenu; showModeMenu = false; }}
      >/</button>

      {#if showSlashMenu && slashCommands.length > 0}
        <div class="dropdown" role="menu" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
          {#each slashCommands as cmd (cmd.command)}
            <button class="dropdown-item" onclick={() => selectSlashCommand(cmd.command)}>
              <span class="cmd-name">{cmd.command}</span>
              <span class="cmd-desc">{cmd.description}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <div class="term-toolbar-right">
    <div class="mode-wrapper">
      <button
        class="term-mode"
        onclick={(e) => { e.stopPropagation(); showModeMenu = !showModeMenu; showSlashMenu = false; }}
      >
        <span class="mode-dot"></span>
        {activeMode ?? 'Default'}
      </button>

      {#if showModeMenu && modes.length > 0}
        <div class="dropdown mode-dropdown" role="menu" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
          {#each modes as mode (mode.name)}
            <button
              class="dropdown-item"
              class:active={mode.name === activeMode}
              onclick={() => selectMode(mode.name)}
            >
              <span class="mode-label">{mode.label}</span>
              <span class="mode-desc">{mode.description}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .term-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 2px 6px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    font-family: var(--font-ui);
    gap: 4px;
  }

  .term-toolbar-left,
  .term-toolbar-right {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .term-tbtn {
    width: 28px;
    height: 28px;
    /* 44px effective target via padding */
    padding: 8px;
    box-sizing: content-box;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-tertiary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-family: var(--font-ui);
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }

  .term-tbtn:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
  }

  .slash-wrapper,
  .mode-wrapper {
    position: relative;
  }

  .term-mode {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    background: var(--bg-tertiary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
    white-space: nowrap;
  }

  .term-mode:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
  }

  .mode-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--success);
    flex-shrink: 0;
  }

  .dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    z-index: 100;
    min-width: 200px;
    max-height: 300px;
    overflow-y: auto;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    display: flex;
    flex-direction: column;
  }

  .mode-dropdown {
    left: auto;
    right: 0;
    min-width: 240px;
  }

  .dropdown-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 10px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--text-body);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    text-align: left;
    cursor: pointer;
    transition: background 0.1s;
  }

  .dropdown-item:last-child {
    border-bottom: none;
  }

  .dropdown-item:hover {
    background: var(--bg-hover);
  }

  .dropdown-item.active {
    background: rgba(29, 78, 216, 0.15);
  }

  .cmd-name {
    font-weight: 600;
    color: var(--text-primary);
  }

  .cmd-desc,
  .mode-desc {
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
  }

  .mode-label {
    font-weight: 600;
    color: var(--text-primary);
  }
</style>
