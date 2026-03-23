<script lang="ts">
  import type { MenuItemDef } from '$lib/types/menu';
  import { menuKeyHandler } from '$lib/utils/a11y';

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
  let menuDropdownEl = $state<HTMLElement | null>(null);

  $effect(() => {
    if (open && menuDropdownEl) {
      const first = menuDropdownEl.querySelector<HTMLElement>('[role="menuitem"]');
      first?.focus();
    }
  });

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

  function handleSubmenuKeydown(e: KeyboardEvent, item: MenuItemDef, index: number) {
    if (e.key === 'ArrowRight' && item.submenu) {
      e.preventDefault();
      e.stopPropagation();
      openSubmenuIndex = index;
      // Focus first submenu item after it renders
      requestAnimationFrame(() => {
        const row = (e.currentTarget as HTMLElement);
        const subItem = row.querySelector<HTMLElement>('.submenu-dropdown [role="menuitem"]');
        subItem?.focus();
      });
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      if (item.submenu) {
        openSubmenuIndex = index;
        requestAnimationFrame(() => {
          const row = (e.currentTarget as HTMLElement);
          const subItem = row.querySelector<HTMLElement>('.submenu-dropdown [role="menuitem"]');
          subItem?.focus();
        });
      } else {
        handleItemClick(item);
      }
    }
  }

  function handleSubItemKeydown(e: KeyboardEvent, sub: MenuItemDef) {
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      e.stopPropagation();
      openSubmenuIndex = null;
      // Return focus to the parent submenu trigger
      if (menuDropdownEl) {
        const items = menuDropdownEl.querySelectorAll<HTMLElement>('[role="menuitem"]');
        const focused = Array.from(items).find(el => el.classList.contains('has-submenu'));
        focused?.focus();
      }
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleItemClick(sub);
    }
  }
</script>

<div class="menu-item" tabindex="-1" onkeydown={handleKeydown}>
  <button
    class="menu-trigger"
    class:active={open}
    onclick={onOpen}
    onmouseenter={onHover}
    aria-haspopup="true"
    aria-expanded={open}
  >
    {label}
  </button>

  {#if open}
    <div class="menu-dropdown" role="menu" bind:this={menuDropdownEl} onkeydown={(e) => menuKeyHandler(e, menuDropdownEl!, '[role="menuitem"]')}>
      {#each items as item, i}
        {#if item.divider}
          <div class="menu-divider"></div>
        {:else if item.submenu}
          <div
            class="menu-row has-submenu"
            role="menuitem"
            tabindex="-1"
            aria-haspopup="true"
            aria-expanded={openSubmenuIndex === i}
            onmouseenter={() => openSubmenuIndex = i}
            onmouseleave={() => openSubmenuIndex = null}
            onkeydown={(e) => handleSubmenuKeydown(e, item, i)}
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
                      tabindex="-1"
                      onclick={() => handleItemClick(sub)}
                      onkeydown={(e) => handleSubItemKeydown(e, sub)}
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
            tabindex="-1"
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
    inset-inline-start: 0;
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
    text-align: start;
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
    margin-inline-start: 24px;
    font-size: calc(var(--font-size-small) - 1px);
  }

  .menu-arrow {
    color: var(--text-muted);
    font-size: 8px;
    margin-inline-start: 12px;
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
    inset-inline-start: 100%;
    top: 0;
    min-width: 180px;
    background: var(--bg-surface);
    border: var(--border-width) solid var(--border);
    z-index: 1001;
    padding: 4px 0;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }
</style>
