<script lang="ts">
  import type { MenuItemDef } from '$lib/types/menu';

  let {
    label,
    items,
    open,
    onOpen,
    onClose,
    onHover,
  }: {
    label: string;
    items: MenuItemDef[];
    open: boolean;
    onOpen: () => void;
    onClose: () => void;
    onHover: () => void;
  } = $props();

  let openSubmenuIndex = $state<number | null>(null);

  function handleItemClick(item: MenuItemDef) {
    if (item.submenu) return;
    item.action?.();
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

<div class="menu-item" role="menubar" onkeydown={handleKeydown}>
  <button
    class="menu-trigger"
    class:active={open}
    onclick={onOpen}
    onmouseenter={onHover}
  >
    {label}
  </button>

  {#if open}
    <div class="menu-dropdown" role="menu">
      {#each items as item, i}
        {#if item.divider}
          <div class="menu-divider"></div>
        {:else if item.submenu}
          <div
            class="menu-row has-submenu"
            role="menuitem"
            tabindex="-1"
            onmouseenter={() => openSubmenuIndex = i}
            onmouseleave={() => openSubmenuIndex = null}
          >
            <span class="menu-label">{item.label}</span>
            <span class="menu-arrow">&#9654;</span>
            {#if openSubmenuIndex === i}
              <div class="submenu-dropdown" role="menu">
                {#each item.submenu as sub}
                  {#if sub.divider}
                    <div class="menu-divider"></div>
                  {:else}
                    <button
                      class="menu-row"
                      role="menuitem"
                      onclick={() => handleItemClick(sub)}
                    >
                      <span class="menu-label">{sub.label}</span>
                      {#if sub.shortcut}
                        <span class="menu-shortcut">{sub.shortcut}</span>
                      {/if}
                    </button>
                  {/if}
                {/each}
              </div>
            {/if}
          </div>
        {:else}
          <button
            class="menu-row"
            role="menuitem"
            onclick={() => handleItemClick(item)}
          >
            <span class="menu-label">{item.label}</span>
            {#if item.shortcut}
              <span class="menu-shortcut">{item.shortcut}</span>
            {/if}
          </button>
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .menu-item {
    position: relative;
  }

  .menu-trigger {
    background: transparent;
    border: none;
    color: var(--text-body);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 400;
    padding: 4px 8px;
    cursor: pointer;
    border-radius: 0;
    line-height: 1;
  }

  .menu-trigger:hover,
  .menu-trigger.active {
    background: var(--bg-hover);
  }

  .menu-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    min-width: 220px;
    background: var(--bg-surface);
    border: var(--border-width) solid var(--border);
    z-index: 1000;
    padding: 4px 0;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .menu-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 6px 12px;
    background: transparent;
    border: none;
    color: var(--text-body);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 400;
    cursor: pointer;
    text-align: left;
    position: relative;
  }

  .menu-row:hover {
    background: var(--bg-hover);
  }

  .menu-label {
    flex: 1;
  }

  .menu-shortcut {
    color: var(--text-muted);
    margin-left: 24px;
    font-size: calc(var(--font-size-small) - 1px);
  }

  .menu-arrow {
    color: var(--text-muted);
    font-size: 8px;
    margin-left: 12px;
  }

  .menu-divider {
    height: 1px;
    background: var(--border);
    margin: 4px 0;
  }

  .has-submenu {
    cursor: default;
  }

  .submenu-dropdown {
    position: absolute;
    left: 100%;
    top: 0;
    min-width: 180px;
    background: var(--bg-surface);
    border: var(--border-width) solid var(--border);
    z-index: 1001;
    padding: 4px 0;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }
</style>
