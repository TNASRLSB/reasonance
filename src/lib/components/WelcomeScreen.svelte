<script lang="ts">
  import { tr } from '$lib/i18n/index';
  import { recentProjects } from '$lib/stores/files';
  import { themeMode, isDark } from '$lib/stores/theme';
  import type { Adapter } from '$lib/adapter/index';

  let { adapter, onOpenFolder, onSelectProject }: {
    adapter: Adapter;
    onOpenFolder: () => void;
    onSelectProject: (path: string) => void;
  } = $props();

  function cycleTheme() {
    themeMode.update((m) => m === 'dark' ? 'light' : 'dark');
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="welcome" data-tauri-drag-region>
  <div class="top-bar">
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
    <h1 class="welcome-logo">REASONANCE</h1>
    <p class="welcome-subtitle">IDE for Vibecoders</p>

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
    text-align: left;
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
</style>
