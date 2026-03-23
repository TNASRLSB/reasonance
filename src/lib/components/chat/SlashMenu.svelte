<script lang="ts">
  import type { SlashCommand } from '$lib/stores/config';

  let {
    commands,
    activeIndex = 0,
    onSelect,
    onClose,
  }: {
    commands: SlashCommand[];
    activeIndex?: number;
    onSelect: (command: SlashCommand) => void;
    onClose: () => void;
  } = $props();
</script>

{#if commands.length > 0}
  <div
    class="slash-menu"
    id="slash-menu"
    role="listbox"
    aria-label="Commands"
  >
    {#each commands as cmd, i (cmd.command)}
      <div
        id="slash-cmd-{i}"
        class="slash-item"
        class:active={i === activeIndex}
        role="option"
        aria-selected={i === activeIndex}
        onclick={() => onSelect(cmd)}
      >
        <span class="cmd-name">{cmd.command}</span>
        <span class="cmd-desc">{cmd.description}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .slash-menu {
    background: var(--bg-surface);
    border: 2px solid var(--border);
    padding: var(--space-1) 0;
    max-height: 200px;
    overflow-y: auto;
  }

  .slash-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-1) var(--space-3);
    cursor: pointer;
    gap: var(--space-3);
  }

  .slash-item:hover,
  .slash-item.active {
    background: var(--bg-hover);
  }

  .slash-item.active {
    border-left: 2px solid var(--accent);
  }

  .cmd-name {
    font-family: var(--font-mono);
    font-size: var(--font-size-small);
    font-weight: 600;
    color: var(--text-primary);
  }

  .cmd-desc {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    text-align: end;
  }
</style>
