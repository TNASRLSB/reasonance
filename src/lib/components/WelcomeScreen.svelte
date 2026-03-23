<script lang="ts">
  import { tr } from '$lib/i18n/index';
  import { recentProjects } from '$lib/stores/files';
  import { themeMode, isDark } from '$lib/stores/theme';
  import { llmConfigs } from '$lib/stores/config';
  import { showSettings } from '$lib/stores/ui';
  import type { Adapter } from '$lib/adapter/index';

  let { adapter, onOpenFolder, onSelectProject }: {
    adapter: Adapter;
    onOpenFolder: () => void;
    onSelectProject: (path: string) => void;
  } = $props();

  function cycleTheme() {
    themeMode.update((m) => m === 'dark' ? 'light' : 'dark');
  }

  function openLlmSettings() {
    showSettings.set(true);
  }

  let dragOver = $state(false);

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    dragOver = true;
  }

  function handleDragLeave() {
    dragOver = false;
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    if (!e.dataTransfer?.files.length) return;
    const item = e.dataTransfer.files[0];
    // In Tauri, dropped files/folders expose a path property
    const path = (item as unknown as { path?: string }).path ?? item.name;
    if (path) {
      onSelectProject(path);
    }
  }

  function handleWindowDrag(e: MouseEvent) {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest('button, a, input, select, .welcome-content')) return;
    e.preventDefault();
    adapter.startDragging();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="welcome" class:drag-over={dragOver}
  onmousedown={handleWindowDrag}
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}>
  <div class="top-bar" onmousedown={handleWindowDrag}>
    <button class="theme-toggle" onclick={cycleTheme} title="Toggle theme">
      {$isDark ? '☀' : '☾'}
    </button>
    <div class="window-controls">
      <button class="win-btn" onclick={() => adapter.minimizeWindow()} title="Minimize">&#8722;</button>
      <button class="win-btn" onclick={() => adapter.maximizeWindow()} title="Maximize">&#9723;</button>
      <button class="win-btn close" onclick={() => adapter.closeWindow()} title="Close">&#10005;</button>
    </div>
  </div>
  <div class="welcome-content">
    <svg class="welcome-icon" viewBox="0 0 250.44 250.44" aria-hidden="true">
      <g transform="translate(25.222,12.197)"><g transform="translate(0,-40.497)">
        <path d="m 0,235.4972 1,-0.02 1,-0.02 1,-0.02 1,-0.03 1,-0.02 1,-0.03 1,-0.02 1,-0.03 1,-0.03 1,-0.02 1,-0.03 1,-0.03 1,-0.03 1,-0.04 1,-0.03 1,-0.03 1,-0.04 1,-0.04 1,-0.03 1,-0.04 1,-0.04 1,-0.05 1,-0.04 1,-0.05 1,-0.05 1,-0.05 1,-0.05 1,-0.05 1,-0.06 1,-0.06 1,-0.06 1,-0.06 1,-0.07 1,-0.07 1,-0.07 1,-0.08 1,-0.08 1,-0.08 1,-0.09 1,-0.09 1,-0.1 1,-0.1 1,-0.11 1,-0.11 1,-0.12 1,-0.12 1,-0.14 1,-0.14 1,-0.14 1,-0.16 1,-0.16 1,-0.18 1,-0.19 1,-0.19 1,-0.22 1,-0.22 1,-0.24 1,-0.26 1,-0.27 1,-0.3 1,-0.31 1,-0.34 1,-0.37 1,-0.39 1,-0.43 1,-0.47 1,-0.5 1,-0.55 1,-0.59 1,-0.65 1,-0.72 1,-0.78 1,-0.87 1,-0.95 1,-1.06 1,-1.17 1,-1.31 1,-1.46 1,-1.64 1,-1.85 1,-2.09 1,-2.37 1,-2.7 1,-3.08 1,-3.54 1,-4.08 1,-4.7 1,-5.45 1,-6.31 1,-7.31 1,-8.44 1,-9.67 1,-10.95 1,-12.15 1,-13.05 1,-13.35 1,-12.660002 1,-10.63 1,-7.17 1,-2.54 1,2.54 1,7.17 1,10.63 1,12.660002 1,13.35 1,13.05 1,12.15 1,10.95 1,9.67 1,8.44 1,7.31 1,6.31 1,5.45 1,4.7 1,4.08 1,3.54 1,3.08 1,2.7 1,2.37 1,2.09 1,1.85 1,1.64 1,1.46 1,1.31 1,1.17 1,1.06 1,0.95 1,0.87 1,0.78 1,0.72 1,0.65 1,0.59 1,0.55 1,0.5 1,0.47 1,0.43 1,0.39 1,0.37 1,0.34 1,0.31 1,0.3 1,0.27 1,0.26 1,0.24 1,0.22 1,0.22 1,0.19 1,0.19 1,0.18 1,0.16 1,0.16 1,0.14 1,0.14 1,0.14 1,0.12 1,0.12 1,0.11 1,0.11 1,0.1 1,0.1 1,0.09 1,0.09 1,0.08 1,0.08 1,0.08 1,0.07 1,0.07 1,0.07 1,0.06 1,0.06 1,0.06 1,0.06 1,0.05 1,0.05 1,0.05 1,0.05 1,0.05 1,0.04 1,0.05 1,0.04 1,0.04 1,0.03 1,0.04 1,0.04 1,0.03 1,0.03 1,0.04 1,0.03 1,0.03 1,0.03 1,0.02 1,0.03 1,0.03 1,0.02 1,0.03 1,0.02 1,0.03 1,0.02 1,0.02 1,0.02" stroke="currentColor" stroke-width="5.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="m 0,235.4972 1,-0.15 1,-0.14 1,-0.16 1,-0.15 1,-0.16 1,-0.17 1,-0.17 1,-0.17 1,-0.18 1,-0.19 1,-0.19 1,-0.19 1,-0.21 1,-0.2 1,-0.22 1,-0.22 1,-0.23 1,-0.23 1,-0.25 1,-0.25 1,-0.26 1,-0.27 1,-0.27 1,-0.29 1,-0.29 1,-0.31 1,-0.32 1,-0.32 1,-0.34 1,-0.35 1,-0.37 1,-0.37 1,-0.39 1,-0.41 1,-0.41 1,-0.44 1,-0.45 1,-0.47 1,-0.48 1,-0.51 1,-0.52 1,-0.55 1,-0.56 1,-0.59 1,-0.61 1,-0.64 1,-0.66 1,-0.69 1,-0.72 1,-0.75 1,-0.78 1,-0.81 1,-0.84 1,-0.89 1,-0.92 1,-0.95 1,-1 1,-1.05 1,-1.09 1,-1.13 1,-1.18 1,-1.24 1,-1.29 1,-1.34 1,-1.4 1,-1.45 1,-1.52 1,-1.58 1,-1.64 1,-1.71 1,-1.78 1,-1.84 1,-1.91 1,-1.97 1,-2.04 1,-2.11 1,-2.17 1,-2.22 1,-2.28 1,-2.33 1,-2.37 1,-2.4 1,-2.42 1,-2.43 1,-2.44 1,-2.41 1,-2.38 1,-2.33 1,-2.25 1,-2.16 1,-2.05 1,-1.91 1,-1.74 1,-1.57 1,-1.37 1,-1.14 1,-0.91 1,-0.66 1,-0.4 1,-0.13 1,0.13 1,0.4 1,0.66 1,0.91 1,1.14 1,1.37 1,1.57 1,1.74 1,1.91 1,2.05 1,2.16 1,2.25 1,2.33 1,2.38 1,2.41 1,2.44 1,2.43 1,2.42 1,2.4 1,2.37 1,2.33 1,2.28 1,2.22 1,2.17 1,2.11 1,2.04 1,1.97 1,1.91 1,1.84 1,1.78 1,1.71 1,1.64 1,1.58 1,1.52 1,1.45 1,1.4 1,1.34 1,1.29 1,1.24 1,1.18 1,1.13 1,1.09 1,1.05 1,1 1,0.95 1,0.92 1,0.89 1,0.84 1,0.81 1,0.78 1,0.75 1,0.72 1,0.69 1,0.66 1,0.64 1,0.61 1,0.59 1,0.56 1,0.55 1,0.52 1,0.51 1,0.48 1,0.47 1,0.45 1,0.44 1,0.41 1,0.41 1,0.39 1,0.37 1,0.37 1,0.35 1,0.34 1,0.32 1,0.32 1,0.31 1,0.29 1,0.29 1,0.27 1,0.27 1,0.26 1,0.25 1,0.25 1,0.23 1,0.23 1,0.22 1,0.22 1,0.2 1,0.21 1,0.19 1,0.19 1,0.19 1,0.18 1,0.17 1,0.17 1,0.17 1,0.16 1,0.15 1,0.16 1,0.14 1,0.15" stroke="currentColor" stroke-width="5.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="m 0,235.4972 1,-0.18 1,-0.19 1,-0.19 1,-0.2 1,-0.2 1,-0.21 1,-0.21 1,-0.22 1,-0.22 1,-0.23 1,-0.23 1,-0.24 1,-0.25 1,-0.25 1,-0.26 1,-0.26 1,-0.28 1,-0.27 1,-0.29 1,-0.29 1,-0.3 1,-0.31 1,-0.32 1,-0.32 1,-0.34 1,-0.34 1,-0.35 1,-0.36 1,-0.37 1,-0.38 1,-0.39 1,-0.4 1,-0.41 1,-0.42 1,-0.43 1,-0.44 1,-0.46 1,-0.47 1,-0.48 1,-0.49 1,-0.51 1,-0.51 1,-0.54 1,-0.54 1,-0.56 1,-0.58 1,-0.59 1,-0.6 1,-0.62 1,-0.64 1,-0.65 1,-0.67 1,-0.68 1,-0.7 1,-0.71 1,-0.73 1,-0.75 1,-0.76 1,-0.78 1,-0.79 1,-0.81 1,-0.82 1,-0.84 1,-0.85 1,-0.87 1,-0.88 1,-0.89 1,-0.9 1,-0.91 1,-0.92 1,-0.93 1,-0.93 1,-0.93 1,-0.94 1,-0.94 1,-0.94 1,-0.93 1,-0.92 1,-0.92 1,-0.9 1,-0.89 1,-0.86 1,-0.85 1,-0.82 1,-0.8 1,-0.76 1,-0.73 1,-0.69 1,-0.66 1,-0.6 1,-0.56 1,-0.51 1,-0.45 1,-0.4 1,-0.35 1,-0.28 1,-0.22 1,-0.16 1,-0.1 1,-0.03 1,0.03 1,0.1 1,0.16 1,0.22 1,0.28 1,0.35 1,0.4 1,0.45 1,0.51 1,0.56 1,0.6 1,0.66 1,0.69 1,0.73 1,0.76 1,0.8 1,0.82 1,0.85 1,0.86 1,0.89 1,0.9 1,0.92 1,0.92 1,0.93 1,0.94 1,0.94 1,0.94 1,0.93 1,0.93 1,0.93 1,0.92 1,0.91 1,0.9 1,0.89 1,0.88 1,0.87 1,0.85 1,0.84 1,0.82 1,0.81 1,0.79 1,0.78 1,0.76 1,0.75 1,0.73 1,0.71 1,0.7 1,0.68 1,0.67 1,0.65 1,0.64 1,0.62 1,0.6 1,0.59 1,0.58 1,0.56 1,0.54 1,0.54 1,0.51 1,0.51 1,0.49 1,0.48 1,0.47 1,0.46 1,0.44 1,0.43 1,0.42 1,0.41 1,0.4 1,0.39 1,0.38 1,0.37 1,0.36 1,0.35 1,0.34 1,0.34 1,0.32 1,0.32 1,0.31 1,0.3 1,0.29 1,0.29 1,0.27 1,0.28 1,0.26 1,0.26 1,0.25 1,0.25 1,0.24 1,0.23 1,0.23 1,0.22 1,0.22 1,0.21 1,0.21 1,0.2 1,0.2 1,0.19 1,0.19 1,0.18" stroke="currentColor" stroke-width="5.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
      </g></g>
    </svg>
    <h1 class="welcome-logo">REASONANCE</h1>
    <p class="welcome-subtitle">IDE for Vibecoders</p>

    {#if $llmConfigs.length === 0}
      <div class="onboarding-banner" role="alert">
        <div class="onboarding-icon">&#9881;</div>
        <div class="onboarding-text">
          <strong>{$tr('welcome.noLlmTitle')}</strong>
          <p>{$tr('welcome.noLlmBody')}</p>
        </div>
        <button class="welcome-btn secondary" onclick={openLlmSettings}>
          {$tr('welcome.configureLlm')}
        </button>
      </div>
    {/if}

    <button class="welcome-btn primary" onclick={onOpenFolder}>
      {$tr('welcome.openFolder')}
    </button>

    <div class="recent-section">
      <h2 class="recent-title">{$tr('welcome.recentProjects')}</h2>
      {#if $recentProjects.length === 0}
        <p class="no-recent">{$tr('welcome.noRecent')}</p>
      {:else}
        <ul class="recent-list">
          {#each $recentProjects as project}
            <li>
              <button class="recent-item" onclick={() => onSelectProject(project)}>
                {project}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
</div>

<style>
  .welcome {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    background: var(--bg-primary);
    font-family: var(--font-ui);
    position: relative;
    -webkit-app-region: drag;
    transition: outline 0.15s;
  }

  .welcome.drag-over {
    outline: 3px dashed var(--accent);
    outline-offset: -6px;
  }

  .top-bar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    -webkit-app-region: no-drag;
    z-index: 10;
  }

  .theme-toggle {
    width: 36px;
    height: 32px;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 16px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.1s, color 0.1s;
  }

  .theme-toggle:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .window-controls {
    display: flex;
  }

  .win-btn {
    width: 46px;
    height: 32px;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.1s, color 0.1s;
  }

  .win-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .win-btn.close:hover {
    background: var(--danger);
    color: #fff;
  }

  .welcome-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    max-width: 480px;
    width: 100%;
    padding: 32px;
    -webkit-app-region: no-drag;
  }

  .welcome-icon {
    width: 80px;
    height: 80px;
    color: var(--text-primary);
    margin-bottom: 8px;
  }

  .welcome-logo {
    font-weight: 800;
    font-size: 32px;
    color: var(--text-primary);
    letter-spacing: -0.02em;
    margin: 0;
    font-family: var(--font-ui);
  }

  .welcome-subtitle {
    color: var(--text-muted);
    font-size: 14px;
    margin: 0 0 24px;
    font-family: var(--font-ui);
  }

  .welcome-btn.primary {
    padding: 12px 32px;
    font-family: var(--font-ui);
    font-size: 14px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: 2px solid var(--accent);
    border-radius: 0;
    background: transparent;
    color: var(--accent);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .welcome-btn.primary:hover {
    background: var(--accent);
    color: var(--bg-primary);
  }

  .recent-section {
    width: 100%;
    margin-top: 32px;
  }

  .recent-title {
    font-size: 12px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--text-muted);
    margin: 0 0 12px;
    font-family: var(--font-ui);
  }

  .no-recent {
    color: var(--text-muted);
    font-size: 13px;
    margin: 0;
    font-family: var(--font-ui);
  }

  .recent-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .recent-item {
    display: block;
    width: 100%;
    padding: 8px 12px;
    font-family: var(--font-ui);
    font-size: 13px;
    text-align: start;
    color: var(--text-body);
    background: none;
    border: none;
    border-radius: 0;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .recent-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .onboarding-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 18px;
    border: 2px solid var(--warning);
    background: rgba(202, 138, 4, 0.08);
    width: 100%;
    margin-bottom: 8px;
  }

  .onboarding-icon {
    font-size: 22px;
    flex-shrink: 0;
  }

  .onboarding-text {
    flex: 1;
  }

  .onboarding-text strong {
    display: block;
    font-size: 13px;
    font-weight: 800;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .onboarding-text p {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.4;
  }

  .welcome-btn.secondary {
    padding: 8px 16px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border: 2px solid var(--warning);
    background: transparent;
    color: var(--warning);
    cursor: pointer;
    flex-shrink: 0;
    white-space: nowrap;
  }

  .welcome-btn.secondary:hover {
    background: var(--warning);
    color: var(--bg-primary);
  }
</style>
