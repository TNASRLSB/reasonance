<script lang="ts">
  import { recentProjectsList, addProject } from '$lib/stores/projects';

  let {
    open,
    anchorX,
    anchorY,
    onClose,
  }: {
    open: boolean;
    anchorX: number;
    anchorY: number;
    onClose: () => void;
  } = $props();

  function handleOpenFolder() {
    onClose();
    document.dispatchEvent(new CustomEvent('reasonance:openFolder'));
  }

  function handleRecentProject(path: string) {
    addProject(path);
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="add-menu-backdrop"
    onclick={onClose}
    onkeydown={handleKeydown}
    role="presentation"
  ></div>
  <div
    class="add-menu"
    style="left: {anchorX}px; top: {anchorY}px;"
    role="menu"
    aria-label="Add project"
  >
    <button
      class="add-menu-item primary"
      role="menuitem"
      onclick={handleOpenFolder}
    >
      Open Folder...
    </button>

    {#if $recentProjectsList.length > 0}
      <div class="add-menu-divider" role="separator"></div>
      <div class="add-menu-section-label">Recent</div>
      {#each $recentProjectsList as project (project.path)}
        <button
          class="add-menu-item"
          role="menuitem"
          title={project.path}
          onclick={() => handleRecentProject(project.path)}
        >
          <span
            class="project-dot"
            style="background: {project.color}"
            aria-hidden="true"
          ></span>
          <span class="project-label">{project.label}</span>
          <span class="project-path">{project.path}</span>
        </button>
      {/each}
    {/if}
  </div>
{/if}

<style>
  .add-menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: calc(var(--layer-dropdown) - 1);
  }

  .add-menu {
    position: fixed;
    z-index: var(--layer-dropdown);
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    padding: var(--space-1) 0;
    min-width: 220px;
    max-width: 320px;
    font-family: var(--font-ui);
  }

  .add-menu-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: none;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
    text-align: start;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    transition: background var(--transition-fast);
  }

  .add-menu-item:hover {
    background: var(--bg-hover);
    color: var(--accent-text);
  }

  .add-menu-item.primary {
    font-weight: 600;
  }

  .add-menu-divider {
    border: none;
    border-top: 1px solid var(--border);
    margin: var(--space-1) 0;
  }

  .add-menu-section-label {
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .project-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .project-label {
    font-weight: 600;
    flex-shrink: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 120px;
  }

  .project-path {
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }
</style>
