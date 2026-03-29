<script lang="ts">
  import { onMount } from 'svelte';
  import Toolbar from './Toolbar.svelte';
  import StatusBar from './StatusBar.svelte';
  import AnalyticsDashboard from './AnalyticsDashboard.svelte';
  import ProjectSidebar from './project/ProjectSidebar.svelte';
  import ProjectQuickSwitcher from './project/ProjectQuickSwitcher.svelte';
  import ErrorBoundary from './ErrorBoundary.svelte';
  import { get } from 'svelte/store';
  import { fileTreeWidth, terminalWidth, analyticsDashboard } from '$lib/stores/ui';
  import { startUpdateChecker } from '$lib/updater';
  import { startLiveTracking } from '$lib/stores/analytics';
  import { llmConfigs, appSettings } from '$lib/stores/config';
  import { projectSummaries, switchProject, projectRoot } from '$lib/stores/projects';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { Snippet } from 'svelte';
  import type { Adapter } from '$lib/adapter/index';

  let { adapter, fileTree, editor, terminal }: {
    adapter: Adapter;
    fileTree?: Snippet;
    editor?: Snippet;
    terminal?: Snippet;
  } = $props();

  let draggingLeft = $state(false);
  let draggingRight = $state(false);
  let showQuickSwitcher = $state(false);
  let sidebarCollapsed = $state(false);

  // Update window title when active project changes
  $effect(() => {
    const root = $projectRoot;
    if (root) {
      const label = root.split('/').pop() || root;
      getCurrentWindow().setTitle(`${label} — Reasonance`).catch(() => {});
    }
  });

  onMount(() => {
    startUpdateChecker();
    const stopTracking = startLiveTracking(adapter);
    return () => stopTracking();
  });

  function onMouseMove(e: MouseEvent) {
    if (draggingLeft) {
      const maxTree = window.innerWidth * 0.3;
      fileTreeWidth.set(Math.max(120, Math.min(maxTree, e.clientX)));
    }
    if (draggingRight) {
      const w = window.innerWidth - e.clientX;
      const maxTerm = window.innerWidth * 0.5;
      terminalWidth.set(Math.max(250, Math.min(maxTerm, w)));
    }
  }

  function onMouseUp() {
    draggingLeft = false;
    draggingRight = false;
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    // Ctrl+Shift+A → toggle analytics dashboard
    if (e.ctrlKey && e.shiftKey && e.key === 'A') {
      e.preventDefault();
      analyticsDashboard.update(v => ({ ...v, open: !v.open, focus: null }));
      return;
    }

    // Ctrl+1..9 → switch active provider
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key >= '1' && e.key <= '9') {
      const index = parseInt(e.key) - 1;
      const configs = get(llmConfigs);
      if (configs && index < configs.length) {
        e.preventDefault();
        appSettings.set({ default: configs[index].name });
      }
    }

    // Alt+1..9 → switch to project N
    if (e.altKey && !e.ctrlKey && !e.shiftKey && e.key >= '1' && e.key <= '9') {
      const index = parseInt(e.key) - 1;
      const summaries = get(projectSummaries);
      if (index < summaries.length) {
        e.preventDefault();
        switchProject(summaries[index].id);
      }
    }

    // Ctrl+Shift+E → toggle quick switcher
    if (e.ctrlKey && e.shiftKey && e.key === 'E') {
      e.preventDefault();
      showQuickSwitcher = !showQuickSwitcher;
      return;
    }

    // Ctrl+B → toggle sidebar collapsed
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key === 'b') {
      e.preventDefault();
      sidebarCollapsed = !sidebarCollapsed;
      return;
    }
  }

  function onDividerKeydown(e: KeyboardEvent, which: 'left' | 'right') {
    const step = e.shiftKey ? 50 : 10;
    if (which === 'left') {
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        fileTreeWidth.update(w => Math.max(120, w - step));
      } else if (e.key === 'ArrowRight') {
        e.preventDefault();
        const max = window.innerWidth * 0.3;
        fileTreeWidth.update(w => Math.min(max, w + step));
      }
    } else {
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        const max = window.innerWidth * 0.5;
        terminalWidth.update(w => Math.min(max, w + step));
      } else if (e.key === 'ArrowRight') {
        e.preventDefault();
        terminalWidth.update(w => Math.max(250, w - step));
      }
    }
  }
</script>

<svelte:window onmousemove={onMouseMove} onmouseup={onMouseUp} onkeydown={handleGlobalKeydown} />

<div class="app-root" class:resizing={draggingLeft || draggingRight}>
  <h1 class="sr-only">Reasonance</h1>
  {#if draggingLeft || draggingRight}
    <div class="resize-overlay"></div>
  {/if}
  <!-- Skip links (visually hidden, visible on focus) -->
  <div class="skip-links">
    <a class="skip-link" href="#project-sidebar">Skip to projects</a>
    <a class="skip-link" href="#file-tree">Skip to file tree</a>
    <a class="skip-link" href="#editor">Skip to editor</a>
    <a class="skip-link" href="#terminal">Skip to terminal</a>
  </div>

  <header>
    <Toolbar {adapter} />
  </header>

  <div class="main-content" data-main-content>
    {#if !sidebarCollapsed}
      <div id="project-sidebar">
        <ErrorBoundary name="ProjectSidebar">
          <ProjectSidebar />
        </ErrorBoundary>
      </div>
    {/if}

    <nav id="file-tree" aria-label="File explorer" class="panel file-tree" style="width: {$fileTreeWidth}px">
      <ErrorBoundary name="FileTree">
        {#if fileTree}
          {@render fileTree()}
        {:else}
          <p class="placeholder">File Tree</p>
        {/if}
      </ErrorBoundary>
    </nav>

    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="divider" onmousedown={() => (draggingLeft = true)} role="separator" aria-label="Resize file tree" tabindex="0" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round($fileTreeWidth / window.innerWidth * 100)} onkeydown={(e) => onDividerKeydown(e, 'left')}>
      <span class="divider-handle" aria-hidden="true">···</span>
    </div>

    <main id="editor" class="panel editor">
      <ErrorBoundary name="Editor">
        {#if $analyticsDashboard.open}
          <AnalyticsDashboard {adapter} />
        {:else if editor}
          {@render editor()}
        {:else}
          <p class="placeholder">Editor</p>
        {/if}
      </ErrorBoundary>
    </main>

    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="divider" onmousedown={() => (draggingRight = true)} role="separator" aria-label="Resize terminal" tabindex="0" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round($terminalWidth / window.innerWidth * 100)} onkeydown={(e) => onDividerKeydown(e, 'right')}>
      <span class="divider-handle" aria-hidden="true">···</span>
    </div>

    <aside id="terminal" aria-label="Terminal" class="panel terminal" style="width: {$terminalWidth}px">
      <ErrorBoundary name="Terminal">
        {#if terminal}
          {@render terminal()}
        {:else}
          <p class="placeholder">Terminal</p>
        {/if}
      </ErrorBoundary>
    </aside>
  </div>

  <footer>
    <StatusBar />
  </footer>

  {#if showQuickSwitcher}
    <ErrorBoundary name="ProjectQuickSwitcher">
      <ProjectQuickSwitcher open={showQuickSwitcher} onClose={() => { showQuickSwitcher = false; }} />
    </ErrorBoundary>
  {/if}
</div>

<style>
  .skip-links {
    position: fixed;
    top: 0;
    left: 0;
    z-index: var(--layer-top);
    pointer-events: none;
  }

  .skip-link {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
    background: var(--bg-primary);
    color: var(--accent-text);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-decoration: none;
  }

  .skip-link:focus {
    position: fixed;
    top: 4px;
    left: 4px;
    width: auto;
    height: auto;
    padding: var(--space-2) var(--space-4);
    margin: 0;
    overflow: visible;
    clip: auto;
    border: 2px solid var(--accent);
    outline: var(--focus-ring);
    outline-offset: var(--focus-offset);
    z-index: calc(var(--layer-top) + 1);
    pointer-events: auto;
    text-decoration: underline;
  }

  header,
  footer {
    flex-shrink: 0;
  }

  .app-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
    color: var(--text-primary);
    overflow: hidden;
  }

  .app-root.resizing {
    cursor: col-resize;
  }

  .resize-overlay {
    position: fixed;
    inset: 0;
    z-index: calc(var(--layer-top) - 1);
    cursor: col-resize;
  }

  .main-content {
    display: flex;
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }

  #project-sidebar {
    height: 100%;
    flex-shrink: 0;
  }

  .panel {
    overflow: hidden;
    background: var(--bg-primary);
    display: flex;
    flex-direction: column;
  }

  .panel.file-tree {
    flex-shrink: 0;
    background: var(--bg-surface);
    border-right: none;
    contain: layout style;
  }

  .panel.editor {
    flex: 1;
    min-width: 0;
    contain: layout style;
  }

  .panel.terminal {
    flex-shrink: 0;
    background: var(--bg-surface);
    border-left: none;
    contain: style;
  }

  .divider {
    width: 6px;
    background: var(--bg-tertiary);
    cursor: col-resize;
    flex-shrink: 0;
    transition: background var(--transition-fast);
    position: relative;
    z-index: var(--layer-raised);
    display: flex;
    align-items: center;
    justify-content: center;
    border-inline-start: 1px solid var(--border);
    border-inline-end: 1px solid var(--border);
  }

  .divider::before {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: -4px;
    right: -4px;
    z-index: calc(var(--layer-raised) + 1);
    cursor: col-resize;
  }

  .divider:hover {
    background: var(--accent);
    border-color: var(--accent);
  }

  .divider-handle {
    writing-mode: vertical-lr;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    pointer-events: none;
    user-select: none;
    letter-spacing: 3px;
  }

  .divider:hover .divider-handle {
    color: var(--text-primary);
  }

  .placeholder {
    color: var(--text-secondary);
    font-size: var(--font-size-small);
    padding: var(--panel-padding);
    text-align: center;
    margin-top: auto;
    margin-bottom: auto;
    align-self: center;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }


</style>
