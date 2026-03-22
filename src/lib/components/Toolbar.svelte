<script lang="ts">
  import { yoloMode, showSettings } from '$lib/stores/ui';
  import type { Adapter } from '$lib/adapter/index';
  import MenuBar from './MenuBar.svelte';
  import { activeInstanceId } from '$lib/stores/terminals';
  import { get } from 'svelte/store';

  let { adapter }: { adapter: Adapter } = $props();

  function toggleYolo() {
    const current = get(yoloMode);
    if (!current) {
      const ok = confirm(
        'Enable YOLO mode?\n\nThis disables all permission prompts for new terminal instances. The LLM will be able to run commands without asking for approval.\n\nProceed?'
      );
      if (!ok) return;
    }
    yoloMode.update((v) => !v);
  }

  function openSettings() {
    showSettings.set(true);
  }

  let showGitMenu = $state(false);

  function gitCmd(cmd: string) {
    if (cmd === 'git push\n') {
      const ok = confirm('Push to remote?\n\nThis will run: git push\n\nProceed?');
      if (!ok) { showGitMenu = false; return; }
    }
    const id = get(activeInstanceId);
    if (id) adapter.writePty(id, cmd);
    showGitMenu = false;
  }

  const gitCommands = [
    { label: 'Status', cmd: 'git status\n', icon: '?' },
    { label: 'Diff', cmd: 'git diff\n', icon: '~' },
    { label: 'Add All', cmd: 'git add -A\n', icon: '+' },
    { label: 'Commit', cmd: 'git commit -m ""', icon: 'C' },
    { label: 'Push', cmd: 'git push\n', icon: '\u2191' },
    { label: 'Pull', cmd: 'git pull\n', icon: '\u2193' },
    { label: 'Log', cmd: 'git log --oneline -20\n', icon: 'L' },
    { label: 'Branch', cmd: 'git branch\n', icon: 'B' },
    { label: 'Stash', cmd: 'git stash\n', icon: 'S' },
    { label: 'Stash Pop', cmd: 'git stash pop\n', icon: 'P' },
  ];
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<svelte:window onclick={() => { showGitMenu = false; }} />
<div class="toolbar" data-tauri-drag-region>
  <div class="toolbar-left">
    <span class="logo">REASONANCE</span>
    <MenuBar {adapter} />
  </div>

  <div class="toolbar-right">
    <div class="git-dropdown-wrapper">
      <button class="git-trigger" onclick={(e) => { e.stopPropagation(); showGitMenu = !showGitMenu; }} title="Git commands">
        GIT &#9662;
      </button>
      {#if showGitMenu}
        <div class="git-dropdown" role="menu">
          {#each gitCommands as g (g.label)}
            <button class="git-dropdown-item" role="menuitem" onclick={(e) => { e.stopPropagation(); gitCmd(g.cmd); }}>
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
      title="YOLO mode — auto-approve permissions (applies to new terminal instances)"
    >
      {$yoloMode ? '\u26A1 YOLO ON' : 'YOLO'}
    </button>
    <button class="settings-btn" onclick={openSettings} title="Settings" aria-label="Settings">&#9881;</button>
    <div class="window-controls">
      <button class="win-btn" onclick={() => adapter.minimizeWindow()} title="Minimize">&#8722;</button>
      <button class="win-btn" onclick={() => adapter.maximizeWindow()} title="Maximize">&#9723;</button>
      <button class="win-btn close" onclick={() => adapter.closeWindow()} title="Close">&#10005;</button>
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

  .logo {
    font-weight: 800;
    font-size: 14px;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: -0.02em;
    margin-right: 8px;
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
    margin-right: 4px;
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
    text-align: left;
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

  .settings-btn {
    font-size: 14px;
    padding: 3px 8px;
  }

  .window-controls {
    display: flex;
    align-items: stretch;
    margin-left: 4px;
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
