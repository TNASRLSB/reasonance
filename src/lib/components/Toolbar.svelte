<script lang="ts">
  import { yoloMode, showSettings, analyticsDashboard } from '$lib/stores/ui';
  import type { Adapter } from '$lib/adapter/index';
  import MenuBar from './MenuBar.svelte';
  import { activeInstanceId } from '$lib/stores/terminals';
  import { get } from 'svelte/store';
  import { menuKeyHandler } from '$lib/utils/a11y';
  import { tr } from '$lib/i18n/index';

  let { adapter }: { adapter: Adapter } = $props();

  function toggleYolo() {
    const current = get(yoloMode);
    if (!current) {
      const ok = confirm($tr('toolbar.yoloConfirm'));
      if (!ok) return;
    }
    yoloMode.update((v) => !v);
  }

  function openSettings() {
    showSettings.set(true);
  }

  let showGitMenu = $state(false);
  let gitMenuEl = $state<HTMLElement | null>(null);

  $effect(() => {
    if (showGitMenu && gitMenuEl) {
      const first = gitMenuEl.querySelector<HTMLElement>('[role="menuitem"]');
      first?.focus();
    }
  });

  function gitCmd(cmd: string) {
    if (cmd === 'git push\n') {
      const ok = confirm($tr('toolbar.pushConfirm'));
      if (!ok) { showGitMenu = false; return; }
    }
    const id = get(activeInstanceId);
    if (id) adapter.writePty(id, cmd);
    showGitMenu = false;
  }

  function handleDrag(e: MouseEvent) {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest('button, a, input, select, [role="menuitem"], [role="menubar"]')) return;
    e.preventDefault();
    adapter.startDragging();
  }

  const gitCommands = $derived([
    { label: $tr('toolbar.git.status'), cmd: 'git status\n', icon: '?' },
    { label: $tr('toolbar.git.diff'), cmd: 'git diff\n', icon: '~' },
    { label: $tr('toolbar.git.addAll'), cmd: 'git add -A\n', icon: '+' },
    { label: $tr('toolbar.git.commit'), cmd: 'git commit -m ""', icon: 'C' },
    { label: $tr('toolbar.git.push'), cmd: 'git push\n', icon: '\u2191' },
    { label: $tr('toolbar.git.pull'), cmd: 'git pull\n', icon: '\u2193' },
    { label: $tr('toolbar.git.log'), cmd: 'git log --oneline -20\n', icon: 'L' },
    { label: $tr('toolbar.git.branch'), cmd: 'git branch\n', icon: 'B' },
    { label: $tr('toolbar.git.stash'), cmd: 'git stash\n', icon: 'S' },
    { label: $tr('toolbar.git.stashPop'), cmd: 'git stash pop\n', icon: 'P' },
  ]);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<svelte:window onclick={() => { showGitMenu = false; }} />
<div class="toolbar" data-tauri-drag-region onmousedown={handleDrag}>
  <div class="toolbar-left">
    <svg class="logo-icon" viewBox="0 0 250.44 250.44" aria-hidden="true">
      <g transform="translate(25.222,12.197)"><g transform="translate(0,-40.497)">
        <path d="m 0,235.4972 1,-0.02 1,-0.02 1,-0.02 1,-0.03 1,-0.02 1,-0.03 1,-0.02 1,-0.03 1,-0.03 1,-0.02 1,-0.03 1,-0.03 1,-0.03 1,-0.04 1,-0.03 1,-0.03 1,-0.04 1,-0.04 1,-0.03 1,-0.04 1,-0.04 1,-0.05 1,-0.04 1,-0.05 1,-0.05 1,-0.05 1,-0.05 1,-0.05 1,-0.06 1,-0.06 1,-0.06 1,-0.06 1,-0.07 1,-0.07 1,-0.07 1,-0.08 1,-0.08 1,-0.08 1,-0.09 1,-0.09 1,-0.1 1,-0.1 1,-0.11 1,-0.11 1,-0.12 1,-0.12 1,-0.14 1,-0.14 1,-0.14 1,-0.16 1,-0.16 1,-0.18 1,-0.19 1,-0.19 1,-0.22 1,-0.22 1,-0.24 1,-0.26 1,-0.27 1,-0.3 1,-0.31 1,-0.34 1,-0.37 1,-0.39 1,-0.43 1,-0.47 1,-0.5 1,-0.55 1,-0.59 1,-0.65 1,-0.72 1,-0.78 1,-0.87 1,-0.95 1,-1.06 1,-1.17 1,-1.31 1,-1.46 1,-1.64 1,-1.85 1,-2.09 1,-2.37 1,-2.7 1,-3.08 1,-3.54 1,-4.08 1,-4.7 1,-5.45 1,-6.31 1,-7.31 1,-8.44 1,-9.67 1,-10.95 1,-12.15 1,-13.05 1,-13.35 1,-12.660002 1,-10.63 1,-7.17 1,-2.54 1,2.54 1,7.17 1,10.63 1,12.660002 1,13.35 1,13.05 1,12.15 1,10.95 1,9.67 1,8.44 1,7.31 1,6.31 1,5.45 1,4.7 1,4.08 1,3.54 1,3.08 1,2.7 1,2.37 1,2.09 1,1.85 1,1.64 1,1.46 1,1.31 1,1.17 1,1.06 1,0.95 1,0.87 1,0.78 1,0.72 1,0.65 1,0.59 1,0.55 1,0.5 1,0.47 1,0.43 1,0.39 1,0.37 1,0.34 1,0.31 1,0.3 1,0.27 1,0.26 1,0.24 1,0.22 1,0.22 1,0.19 1,0.19 1,0.18 1,0.16 1,0.16 1,0.14 1,0.14 1,0.14 1,0.12 1,0.12 1,0.11 1,0.11 1,0.1 1,0.1 1,0.09 1,0.09 1,0.08 1,0.08 1,0.08 1,0.07 1,0.07 1,0.07 1,0.06 1,0.06 1,0.06 1,0.06 1,0.05 1,0.05 1,0.05 1,0.05 1,0.05 1,0.04 1,0.05 1,0.04 1,0.04 1,0.03 1,0.04 1,0.04 1,0.03 1,0.03 1,0.04 1,0.03 1,0.03 1,0.03 1,0.02 1,0.03 1,0.03 1,0.02 1,0.03 1,0.02 1,0.03 1,0.02 1,0.02 1,0.02" stroke="currentColor" stroke-width="5.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="m 0,235.4972 1,-0.15 1,-0.14 1,-0.16 1,-0.15 1,-0.16 1,-0.17 1,-0.17 1,-0.17 1,-0.18 1,-0.19 1,-0.19 1,-0.19 1,-0.21 1,-0.2 1,-0.22 1,-0.22 1,-0.23 1,-0.23 1,-0.25 1,-0.25 1,-0.26 1,-0.27 1,-0.27 1,-0.29 1,-0.29 1,-0.31 1,-0.32 1,-0.32 1,-0.34 1,-0.35 1,-0.37 1,-0.37 1,-0.39 1,-0.41 1,-0.41 1,-0.44 1,-0.45 1,-0.47 1,-0.48 1,-0.51 1,-0.52 1,-0.55 1,-0.56 1,-0.59 1,-0.61 1,-0.64 1,-0.66 1,-0.69 1,-0.72 1,-0.75 1,-0.78 1,-0.81 1,-0.84 1,-0.89 1,-0.92 1,-0.95 1,-1 1,-1.05 1,-1.09 1,-1.13 1,-1.18 1,-1.24 1,-1.29 1,-1.34 1,-1.4 1,-1.45 1,-1.52 1,-1.58 1,-1.64 1,-1.71 1,-1.78 1,-1.84 1,-1.91 1,-1.97 1,-2.04 1,-2.11 1,-2.17 1,-2.22 1,-2.28 1,-2.33 1,-2.37 1,-2.4 1,-2.42 1,-2.43 1,-2.44 1,-2.41 1,-2.38 1,-2.33 1,-2.25 1,-2.16 1,-2.05 1,-1.91 1,-1.74 1,-1.57 1,-1.37 1,-1.14 1,-0.91 1,-0.66 1,-0.4 1,-0.13 1,0.13 1,0.4 1,0.66 1,0.91 1,1.14 1,1.37 1,1.57 1,1.74 1,1.91 1,2.05 1,2.16 1,2.25 1,2.33 1,2.38 1,2.41 1,2.44 1,2.43 1,2.42 1,2.4 1,2.37 1,2.33 1,2.28 1,2.22 1,2.17 1,2.11 1,2.04 1,1.97 1,1.91 1,1.84 1,1.78 1,1.71 1,1.64 1,1.58 1,1.52 1,1.45 1,1.4 1,1.34 1,1.29 1,1.24 1,1.18 1,1.13 1,1.09 1,1.05 1,1 1,0.95 1,0.92 1,0.89 1,0.84 1,0.81 1,0.78 1,0.75 1,0.72 1,0.69 1,0.66 1,0.64 1,0.61 1,0.59 1,0.56 1,0.55 1,0.52 1,0.51 1,0.48 1,0.47 1,0.45 1,0.44 1,0.41 1,0.41 1,0.39 1,0.37 1,0.37 1,0.35 1,0.34 1,0.32 1,0.32 1,0.31 1,0.29 1,0.29 1,0.27 1,0.27 1,0.26 1,0.25 1,0.25 1,0.23 1,0.23 1,0.22 1,0.22 1,0.2 1,0.21 1,0.19 1,0.19 1,0.19 1,0.18 1,0.17 1,0.17 1,0.17 1,0.16 1,0.15 1,0.16 1,0.14 1,0.15" stroke="currentColor" stroke-width="5.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="m 0,235.4972 1,-0.18 1,-0.19 1,-0.19 1,-0.2 1,-0.2 1,-0.21 1,-0.21 1,-0.22 1,-0.22 1,-0.23 1,-0.23 1,-0.24 1,-0.25 1,-0.25 1,-0.26 1,-0.26 1,-0.28 1,-0.27 1,-0.29 1,-0.29 1,-0.3 1,-0.31 1,-0.32 1,-0.32 1,-0.34 1,-0.34 1,-0.35 1,-0.36 1,-0.37 1,-0.38 1,-0.39 1,-0.4 1,-0.41 1,-0.42 1,-0.43 1,-0.44 1,-0.46 1,-0.47 1,-0.48 1,-0.49 1,-0.51 1,-0.51 1,-0.54 1,-0.54 1,-0.56 1,-0.58 1,-0.59 1,-0.6 1,-0.62 1,-0.64 1,-0.65 1,-0.67 1,-0.68 1,-0.7 1,-0.71 1,-0.73 1,-0.75 1,-0.76 1,-0.78 1,-0.79 1,-0.81 1,-0.82 1,-0.84 1,-0.85 1,-0.87 1,-0.88 1,-0.89 1,-0.9 1,-0.91 1,-0.92 1,-0.93 1,-0.93 1,-0.93 1,-0.94 1,-0.94 1,-0.94 1,-0.93 1,-0.92 1,-0.92 1,-0.9 1,-0.89 1,-0.86 1,-0.85 1,-0.82 1,-0.8 1,-0.76 1,-0.73 1,-0.69 1,-0.66 1,-0.6 1,-0.56 1,-0.51 1,-0.45 1,-0.4 1,-0.35 1,-0.28 1,-0.22 1,-0.16 1,-0.1 1,-0.03 1,0.03 1,0.1 1,0.16 1,0.22 1,0.28 1,0.35 1,0.4 1,0.45 1,0.51 1,0.56 1,0.6 1,0.66 1,0.69 1,0.73 1,0.76 1,0.8 1,0.82 1,0.85 1,0.86 1,0.89 1,0.9 1,0.92 1,0.92 1,0.93 1,0.94 1,0.94 1,0.94 1,0.93 1,0.93 1,0.93 1,0.92 1,0.91 1,0.9 1,0.89 1,0.88 1,0.87 1,0.85 1,0.84 1,0.82 1,0.81 1,0.79 1,0.78 1,0.76 1,0.75 1,0.73 1,0.71 1,0.7 1,0.68 1,0.67 1,0.65 1,0.64 1,0.62 1,0.6 1,0.59 1,0.58 1,0.56 1,0.54 1,0.54 1,0.51 1,0.51 1,0.49 1,0.48 1,0.47 1,0.46 1,0.44 1,0.43 1,0.42 1,0.41 1,0.4 1,0.39 1,0.38 1,0.37 1,0.36 1,0.35 1,0.34 1,0.34 1,0.32 1,0.32 1,0.31 1,0.3 1,0.29 1,0.29 1,0.27 1,0.28 1,0.26 1,0.26 1,0.25 1,0.25 1,0.24 1,0.23 1,0.23 1,0.22 1,0.22 1,0.21 1,0.21 1,0.2 1,0.2 1,0.19 1,0.19 1,0.18" stroke="currentColor" stroke-width="5.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
      </g></g>
    </svg>
    <span class="logo">REASONANCE</span>
    <MenuBar {adapter} />
  </div>

  <div class="drag-spacer" data-tauri-drag-region></div>

  <div class="toolbar-right">
    <div class="git-dropdown-wrapper">
      <button class="git-trigger" onclick={(e) => { e.stopPropagation(); showGitMenu = !showGitMenu; }} title={$tr('toolbar.gitCommands')} aria-haspopup="true" aria-expanded={showGitMenu}>
        GIT &#9662;
      </button>
      {#if showGitMenu}
        <div class="git-dropdown" role="menu" bind:this={gitMenuEl} onkeydown={(e) => menuKeyHandler(e, gitMenuEl!, '[role="menuitem"]')}>
          {#each gitCommands as g (g.label)}
            <button class="git-dropdown-item" role="menuitem" tabindex="-1" onclick={(e) => { e.stopPropagation(); gitCmd(g.cmd); }}>
              <span class="git-icon">{g.icon}</span>
              <span class="git-label">{g.label}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
    <button
      class="yolo-btn"
      class:active={$yoloMode}
      onclick={toggleYolo}
      title={$tr('toolbar.yoloTitle')}
    >
      {$yoloMode ? '\u26A1 ' + $tr('toolbar.yoloOn') : $tr('toolbar.yoloOff')}
    </button>
    <button
      class="analytics-btn"
      class:active={$analyticsDashboard.open}
      aria-label={$tr('toolbar.analytics')}
      aria-pressed={$analyticsDashboard.open}
      onclick={() => analyticsDashboard.update(v => ({ ...v, open: !v.open, focus: null }))}
      title={$tr('toolbar.analytics')}
    >📊</button>
    <button class="settings-btn" onclick={openSettings} title={$tr('toolbar.settings')} aria-label={$tr('toolbar.settings')}>&#9881;</button>
    <div class="window-controls">
      <button class="win-btn" onclick={() => adapter.minimizeWindow()} title={$tr('toolbar.minimize')} aria-label={$tr('toolbar.minimize')}>&#8722;</button>
      <button class="win-btn" onclick={() => adapter.maximizeWindow()} title={$tr('toolbar.maximize')} aria-label={$tr('toolbar.maximize')}>&#9723;</button>
      <button class="win-btn close" onclick={() => adapter.closeWindow()} title={$tr('toolbar.close')} aria-label={$tr('toolbar.close')}>&#10005;</button>
    </div>
  </div>
</div>

<style>
  .toolbar {
    height: var(--toolbar-height);
    background: var(--bg-surface);
    border-bottom: var(--border-width) solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0 0 12px;
    flex-shrink: 0;
    user-select: none;
    -webkit-app-region: drag;
  }

  .toolbar-left,
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 8px;
    -webkit-app-region: no-drag;
    flex-shrink: 0;
    white-space: nowrap;
    overflow: visible;
    min-width: 0;
  }

  .drag-spacer {
    flex: 1;
    align-self: stretch;
    -webkit-app-region: drag;
    min-width: 0;
  }

  .toolbar-right {
    gap: 4px;
  }

  .logo-icon {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }

  .logo {
    font-weight: 800;
    font-size: 14px;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: -0.02em;
    margin-inline-end: 8px;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 1;
  }

  button {
    background: var(--bg-surface);
    color: var(--text-body);
    border: var(--border-width) solid var(--border);
    border-radius: 0;
    padding: 3px 10px;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
    min-height: 26px;
  }

  button:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
    border-color: var(--text-primary);
  }

  .git-dropdown-wrapper {
    position: relative;
    margin-inline-end: 4px;
  }

  .git-trigger {
    padding: 3px 10px;
    font-size: var(--font-size-tiny);
    min-height: 26px;
    border: 2px solid var(--border);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: pointer;
  }

  .git-trigger:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
    border-color: var(--text-primary);
  }

  .git-dropdown {
    position: absolute;
    top: 100%;
    right: 0;
    z-index: 200;
    min-width: 160px;
    background: var(--bg-secondary);
    border: 2px solid var(--border);
    padding: 4px 0;
  }

  .git-dropdown-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 12px;
    background: transparent;
    border: none;
    border-radius: 0;
    color: var(--text-body);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 600;
    text-transform: none;
    letter-spacing: normal;
    cursor: pointer;
    min-height: unset;
  }

  .git-dropdown-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    border: none;
  }

  .git-icon {
    width: 16px;
    text-align: center;
    font-family: var(--font-mono);
    font-weight: 700;
    color: var(--text-muted);
  }

  .git-label {
    flex: 1;
    text-align: start;
  }

  .yolo-btn {
    letter-spacing: 0.05em;
  }

  .yolo-btn.active {
    background: var(--danger-dark);
    border-color: var(--danger);
    color: var(--text-primary);
  }

  .yolo-btn.active:hover {
    background: var(--danger);
  }

  .analytics-btn {
    font-size: 14px;
    padding: 3px 8px;
  }

  .analytics-btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .settings-btn {
    font-size: 14px;
    padding: 3px 8px;
  }

  .window-controls {
    display: flex;
    align-items: stretch;
    margin-inline-start: 4px;
    height: var(--toolbar-height);
  }

  .win-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 12px;
    padding: 0 14px;
    min-height: unset;
    height: 100%;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
    display: flex;
    align-items: center;
  }

  .win-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: none;
  }

  .win-btn.close:hover {
    background: var(--danger);
    color: #fff;
  }
</style>
