<script lang="ts">
  import { get } from 'svelte/store';
  import { projectSummaries, activeProjectId, switchProject, removeProject, addProject } from '$lib/stores/projects';
  import type { ProjectSummary } from '$lib/stores/projects';

  let summaries = $derived($projectSummaries);
  let visible = $derived(summaries.length > 1);

  // Context menu state
  let contextMenu = $state<{ x: number; y: number; project: ProjectSummary } | null>(null);

  // Tooltip state
  let tooltip = $state<{ x: number; y: number; text: string } | null>(null);
  let tooltipTimer: ReturnType<typeof setTimeout> | undefined;

  // Drag-drop highlight
  let dragOver = $state(false);

  // Focus tracking for keyboard navigation
  let tablistEl = $state<HTMLElement | null>(null);

  function handleClick(id: string) {
    switchProject(id);
  }

  function handleMiddleClick(e: MouseEvent, project: ProjectSummary) {
    if (e.button !== 1) return;
    e.preventDefault();
    closeProject(project);
  }

  function closeProject(project: ProjectSummary) {
    if (project.hasUnsavedChanges) {
      const ok = confirm(`"${project.label}" has unsaved changes. Close anyway?`);
      if (!ok) return;
    }
    removeProject(project.id);
  }

  function handleContextMenu(e: MouseEvent, project: ProjectSummary) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, project };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function handleRename() {
    if (!contextMenu) return;
    const project = contextMenu.project;
    const newName = prompt('Rename project:', project.label);
    if (newName && newName.trim()) {
      // updateProjectContext is available but we import what we need
      import('$lib/stores/projects').then(mod => {
        mod.updateProjectContext(project.id, { label: newName.trim() });
      });
    }
    closeContextMenu();
  }

  function handleChangeColor() {
    if (!contextMenu) return;
    const project = contextMenu.project;
    const newColor = prompt('Enter a CSS color (e.g. #ff6b6b):', project.color);
    if (newColor && newColor.trim()) {
      import('$lib/stores/projects').then(mod => {
        mod.updateProjectContext(project.id, { color: newColor.trim() });
      });
    }
    closeContextMenu();
  }

  function handleTogglePin() {
    if (!contextMenu) return;
    const project = contextMenu.project;
    import('$lib/stores/projects').then(mod => {
      mod.updateProjectContext(project.id, { pinned: !project.pinned });
    });
    closeContextMenu();
  }

  function handleCloseFromMenu() {
    if (!contextMenu) return;
    closeProject(contextMenu.project);
    closeContextMenu();
  }

  function handleAddProject() {
    window.dispatchEvent(new CustomEvent('reasonance:openFolder'));
  }

  function showTooltip(e: MouseEvent, path: string) {
    clearTimeout(tooltipTimer);
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    tooltipTimer = setTimeout(() => {
      tooltip = { x: rect.right + 8, y: rect.top + rect.height / 2, text: path };
    }, 500);
  }

  function hideTooltip() {
    clearTimeout(tooltipTimer);
    tooltip = null;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!tablistEl) return;
    const tabs = Array.from(tablistEl.querySelectorAll<HTMLElement>('[role="tab"]'));
    const current = tabs.findIndex(t => t === document.activeElement);
    if (current === -1) return;

    let next = current;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      next = (current + 1) % tabs.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      next = (current - 1 + tabs.length) % tabs.length;
    } else if (e.key === 'Home') {
      e.preventDefault();
      next = 0;
    } else if (e.key === 'End') {
      e.preventDefault();
      next = tabs.length - 1;
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      const id = tabs[current].dataset.projectId;
      if (id) switchProject(id);
      return;
    } else {
      return;
    }

    tabs[next].focus();
  }

  // Drag-and-drop from file manager
  $effect(() => {
    let unlisten: (() => void) | undefined;

    (async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const unlistenFn = await getCurrentWindow().onDragDropEvent((event) => {
          if (event.payload.type === 'over') {
            dragOver = true;
          } else if (event.payload.type === 'cancel' || event.payload.type === 'leave') {
            dragOver = false;
          } else if (event.payload.type === 'drop') {
            dragOver = false;
            for (const path of event.payload.paths) {
              addProject(path);
            }
          }
        });
        unlisten = unlistenFn;
      } catch {
        // Not in Tauri environment
      }
    })();

    return () => {
      unlisten?.();
    };
  });

  function getInitial(label: string): string {
    return label.charAt(0).toUpperCase();
  }
</script>

{#if visible}
  <nav class="project-sidebar" class:drag-over={dragOver}>
    <div
      class="tab-list"
      role="tablist"
      aria-orientation="vertical"
      aria-label="Project tabs"
      bind:this={tablistEl}
      onkeydown={handleKeydown}
    >
      {#each summaries as project (project.id)}
        {@const active = project.isActive}
        <button
          class="tab"
          class:active
          role="tab"
          aria-selected={active}
          tabindex={active ? 0 : -1}
          data-project-id={project.id}
          title={project.rootPath}
          onclick={() => handleClick(project.id)}
          onauxclick={(e) => handleMiddleClick(e, project)}
          oncontextmenu={(e) => handleContextMenu(e, project)}
          onmouseenter={(e) => showTooltip(e, project.rootPath)}
          onmouseleave={hideTooltip}
        >
          <span
            class="tab-circle"
            style="background-color: {project.color};"
            aria-hidden="true"
          >
            {getInitial(project.label)}
          </span>

          {#if project.hasRunningAgent}
            <span class="indicator running" aria-label="Agent running"></span>
          {:else if project.activeTerminals > 0}
            <span class="indicator terminals" aria-label="{project.activeTerminals} active terminal{project.activeTerminals > 1 ? 's' : ''}"></span>
          {/if}

          {#if project.hasUnsavedChanges}
            <span class="indicator unsaved" aria-label="Unsaved changes"></span>
          {/if}

          {#if project.pinned}
            <span class="pin-badge" aria-label="Pinned">
              <svg width="8" height="8" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M4.146.146A.5.5 0 0 1 4.5 0h7a.5.5 0 0 1 .5.5c0 .68-.342 1.174-.646 1.479-.126.125-.25.224-.354.298v4.431l.078.048c.203.127.476.314.751.555C12.36 7.775 13 8.527 13 9.5a.5.5 0 0 1-.5.5h-4v4.5c0 .276-.224 1.5-.5 1.5s-.5-1.224-.5-1.5V10h-4a.5.5 0 0 1-.5-.5c0-.973.64-1.725 1.17-2.189A6 6 0 0 1 5 6.708V2.277a3 3 0 0 1-.354-.298C4.342 1.674 4 1.179 4 .5a.5.5 0 0 1 .146-.354z"/>
              </svg>
            </span>
          {/if}
        </button>
      {/each}
    </div>

    <button
      class="add-button"
      aria-label="Add project"
      title="Open folder"
      onclick={handleAddProject}
    >
      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
        <path d="M8 2a.5.5 0 0 1 .5.5v5h5a.5.5 0 0 1 0 1h-5v5a.5.5 0 0 1-1 0v-5h-5a.5.5 0 0 1 0-1h5v-5A.5.5 0 0 1 8 2z"/>
      </svg>
    </button>
  </nav>
{/if}

<!-- Context menu -->
{#if contextMenu}
  <div
    class="ctx-backdrop"
    role="presentation"
    onclick={closeContextMenu}
    onkeydown={(e) => { if (e.key === 'Escape') closeContextMenu(); }}
  ></div>
  <div
    class="ctx-menu"
    role="menu"
    style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
  >
    <button role="menuitem" class="ctx-item" onclick={handleRename}>Rename</button>
    <button role="menuitem" class="ctx-item" onclick={handleChangeColor}>Change color</button>
    <button role="menuitem" class="ctx-item" onclick={handleTogglePin}>
      {contextMenu.project.pinned ? 'Unpin' : 'Pin'}
    </button>
    <div class="ctx-separator"></div>
    <button role="menuitem" class="ctx-item danger" onclick={handleCloseFromMenu}>Close</button>
  </div>
{/if}

<!-- Tooltip -->
{#if tooltip}
  <div
    class="tooltip"
    role="tooltip"
    style="left: {tooltip.x}px; top: {tooltip.y}px;"
  >
    {tooltip.text}
  </div>
{/if}

<style>
  .project-sidebar {
    display: flex;
    flex-direction: column;
    width: var(--sidebar-width, 48px);
    min-width: var(--sidebar-width, 48px);
    max-width: var(--sidebar-width, 48px);
    height: 100%;
    background: var(--sidebar-bg);
    border-right: 1px solid var(--sidebar-border);
    align-items: center;
    padding-top: 8px;
    padding-bottom: 8px;
    overflow-y: auto;
    overflow-x: hidden;
    user-select: none;
    transition: background var(--sidebar-transition-speed, 150ms);
  }

  .project-sidebar.drag-over {
    background: var(--sidebar-dropzone-bg);
    border-color: var(--sidebar-dropzone-border);
  }

  .tab-list {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    flex: 1;
    width: 100%;
  }

  .tab {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 48px;
    min-height: 48px;
    padding: 0;
    background: none;
    border: none;
    border-left: 3px solid transparent;
    cursor: pointer;
    transition: background var(--sidebar-transition-speed, 150ms),
                border-color var(--sidebar-transition-speed, 150ms);
  }

  .tab:hover {
    background: var(--sidebar-bg-hover);
  }

  .tab:focus-visible {
    outline: 2px solid var(--sidebar-tab-active-accent);
    outline-offset: -2px;
  }

  .tab.active {
    border-left-color: var(--sidebar-tab-active-accent);
    background: var(--sidebar-tab-active-bg);
  }

  .tab-circle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    font-size: 14px;
    font-weight: 600;
    color: var(--sidebar-tab-text-active, #fff);
    line-height: 1;
    flex-shrink: 0;
    transition: border-radius var(--sidebar-transition-speed, 150ms);
  }

  .tab.active .tab-circle {
    border-radius: 8px;
  }

  /* Status indicators */
  .indicator {
    position: absolute;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .indicator.running {
    bottom: 6px;
    right: 6px;
    background: var(--sidebar-indicator-running);
    animation: pulse 1.5s ease-in-out infinite;
  }

  .indicator.terminals {
    bottom: 6px;
    right: 6px;
    background: var(--sidebar-indicator-idle);
  }

  .indicator.unsaved {
    top: 6px;
    right: 6px;
    background: var(--sidebar-indicator-unsaved);
  }

  .pin-badge {
    position: absolute;
    top: 4px;
    left: 4px;
    color: var(--sidebar-tab-text);
    opacity: 0.6;
    font-size: 8px;
    line-height: 1;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  @media (prefers-reduced-motion: reduce) {
    .indicator.running {
      animation: none;
      outline: 2px solid var(--sidebar-indicator-running);
      outline-offset: 1px;
    }
  }

  /* Add button */
  .add-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    margin-top: 4px;
    padding: 0;
    background: none;
    border: 1px dashed var(--sidebar-separator);
    border-radius: 50%;
    color: var(--sidebar-tab-text);
    cursor: pointer;
    opacity: 0.6;
    transition: opacity var(--sidebar-transition-speed, 150ms),
                background var(--sidebar-transition-speed, 150ms);
  }

  .add-button:hover {
    opacity: 1;
    background: var(--sidebar-bg-hover);
  }

  .add-button:focus-visible {
    outline: 2px solid var(--sidebar-tab-active-accent);
    outline-offset: 2px;
  }

  /* Context menu */
  .ctx-backdrop {
    position: fixed;
    inset: 0;
    z-index: 999;
  }

  .ctx-menu {
    position: fixed;
    z-index: 1000;
    background: var(--sidebar-bg);
    border: 1px solid var(--sidebar-border);
    border-radius: 6px;
    padding: 4px 0;
    min-width: 160px;
    font-size: 13px;
  }

  .ctx-item {
    display: block;
    width: 100%;
    padding: 6px 12px;
    background: none;
    border: none;
    color: var(--sidebar-tab-text);
    cursor: pointer;
    text-align: start;
  }

  .ctx-item:hover {
    background: var(--sidebar-bg-hover);
  }

  .ctx-item.danger {
    color: var(--sidebar-indicator-error);
  }

  .ctx-separator {
    height: 1px;
    margin: 4px 0;
    background: var(--sidebar-separator);
  }

  /* Tooltip */
  .tooltip {
    position: fixed;
    z-index: 1001;
    padding: 4px 8px;
    background: var(--sidebar-bg);
    border: 1px solid var(--sidebar-border);
    border-radius: 4px;
    font-size: 12px;
    color: var(--sidebar-tab-text);
    white-space: nowrap;
    pointer-events: none;
    transform: translateY(-50%);
  }
</style>
