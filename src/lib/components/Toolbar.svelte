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
<div class="toolbar" data-tauri-drag-region>
  <div class="toolbar-left">
    <svg class="logo-icon" viewBox="0 0 200 200" aria-hidden="true">
      <path d="M0,179 L30,178 L50,176 L65,172 L75,165 L82,153 L87,135 L91,107 L94,74 L97,35 L100,15 L103,35 L106,74 L109,107 L113,135 L118,153 L125,165 L135,172 L150,176 L170,178 L200,179" stroke="currentColor" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M0,177 L30,173 L50,167 L65,156 L75,139 L84,110 L91,74 L95,52 L100,40 L105,52 L109,74 L116,110 L125,139 L135,156 L150,167 L170,173 L200,177" stroke="currentColor" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M0,172 L30,166 L50,155 L60,145 L70,131 L80,115 L88,91 L94,80 L100,75 L106,80 L112,91 L120,115 L130,131 L140,145 L150,155 L170,166 L200,172" stroke="currentColor" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M0,169 L30,161 L50,151 L60,144 L70,135 L80,126 L90,119 L96,116 L100,115 L104,116 L110,119 L120,126 L130,135 L140,144 L150,151 L170,161 L200,169" stroke="currentColor" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
    <span class="logo">REASONANCE</span>
    <MenuBar {adapter} />
  </div>

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
    overflow: hidden;
    min-width: 0;
  }

  .toolbar-right {
    gap: 4px;
  }

  .logo-icon {
    width: 14px;
    height: 14px;
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
