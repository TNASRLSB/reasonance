<script lang="ts">
  import type { Adapter } from '$lib/adapter/index';
  import { menuKeyHandler } from '$lib/utils/a11y';

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
  let slashMenuEl = $state<HTMLElement | null>(null);
  let modeMenuEl = $state<HTMLElement | null>(null);

  $effect(() => {
    if (showSlashMenu && slashMenuEl) {
      const first = slashMenuEl.querySelector<HTMLElement>('.dropdown-item');
      first?.focus();
    }
  });

  $effect(() => {
    if (showModeMenu && modeMenuEl) {
      const first = modeMenuEl.querySelector<HTMLElement>('.dropdown-item');
      first?.focus();
    }
  });

  async function addFileToContext() {
    const filePath = await adapter.openFileDialog();
    if (filePath) {
      await adapter.writePty(instanceId, `/file ${filePath}`);
    }
  }

  function selectSlashCommand(command: string) {
    adapter.writePty(instanceId, command + '\n');
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
      class="term-tbtn term-tbtn--labeled"
      title="Add file to context"
      onclick={(e) => { e.stopPropagation(); addFileToContext(); }}
    ><span class="tbtn-icon" aria-hidden="true">+</span><span class="tbtn-label">Add file</span></button>

    <div class="slash-wrapper">
      <button
        class="term-tbtn term-tbtn--labeled"
        title="Slash commands"
        onclick={(e) => { e.stopPropagation(); showSlashMenu = !showSlashMenu; showModeMenu = false; }}
        aria-haspopup="true"
        aria-expanded={showSlashMenu}
      ><span class="tbtn-icon" aria-hidden="true">/</span><span class="tbtn-label">Commands</span></button>

      {#if showSlashMenu && slashCommands.length > 0}
        <div class="dropdown" role="menu" bind:this={slashMenuEl} onclick={(e) => e.stopPropagation()} onkeydown={(e) => { e.stopPropagation(); menuKeyHandler(e, slashMenuEl!, '.dropdown-item'); }}>
          {#each slashCommands as cmd (cmd.command)}
            <button class="dropdown-item" tabindex="-1" onclick={() => selectSlashCommand(cmd.command)}>
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
        aria-haspopup="true"
        aria-expanded={showModeMenu}
      >
        <span class="mode-dot" aria-hidden="true"></span>
        {activeMode ?? 'Default'}
      </button>

      {#if showModeMenu && modes.length > 0}
        <div class="dropdown mode-dropdown" role="menu" bind:this={modeMenuEl} onclick={(e) => e.stopPropagation()} onkeydown={(e) => { e.stopPropagation(); menuKeyHandler(e, modeMenuEl!, '.dropdown-item'); }}>
          {#each modes as mode (mode.name)}
            <button
              class="dropdown-item"
              tabindex="-1"
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
    padding: 4px 8px;
    background: var(--bg-primary);
    border-bottom: 2px solid var(--border);
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
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-tertiary);
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-primary);
    font-family: var(--font-ui);
    font-size: 14px;
    font-weight: 800;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }

  .term-tbtn--labeled {
    width: auto;
    padding: 0 8px;
    gap: 5px;
  }

  .tbtn-icon {
    font-size: 14px;
    font-weight: 800;
    line-height: 1;
  }

  .tbtn-label {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .term-tbtn:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
    border-color: var(--text-primary);
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
    border: 2px solid var(--border);
    border-radius: 0;
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
    white-space: nowrap;
  }

  .term-mode:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
    border-color: var(--text-primary);
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
