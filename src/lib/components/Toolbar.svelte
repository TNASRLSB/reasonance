<script lang="ts">
  import { get } from 'svelte/store';
  import { yoloMode, showSettings } from '$lib/stores/ui';
  import { activeInstanceId } from '$lib/stores/terminals';
  import type { Adapter } from '$lib/adapter/index';

  let { adapter }: { adapter: Adapter } = $props();

  async function sendGitCommand(command: string) {
    const id = get(activeInstanceId);
    if (!id) return;
    await adapter.writePty(id, command);
  }

  function gitStatus() {
    sendGitCommand('git status\n');
  }

  function gitCommit() {
    sendGitCommand('git commit -m ""');
  }

  function gitPush() {
    sendGitCommand('git push\n');
  }

  function gitPull() {
    sendGitCommand('git pull\n');
  }

  function gitLog() {
    sendGitCommand('git log --oneline -20\n');
  }

  function toggleYolo() {
    yoloMode.update((v) => !v);
  }

  function openSettings() {
    showSettings.set(true);
  }
</script>

<div class="toolbar">
  <div class="toolbar-left">
    <span class="logo">FORGE</span>
    <div class="git-actions">
      <button onclick={gitStatus} title="Git Status">Status</button>
      <button onclick={gitCommit} title="Git Commit">Commit</button>
      <button onclick={gitPush} title="Git Push">Push</button>
      <button onclick={gitPull} title="Git Pull">Pull</button>
      <button onclick={gitLog} title="Git Log">Log</button>
    </div>
  </div>

  <div class="toolbar-right">
    <button
      class="yolo-btn"
      class:active={$yoloMode}
      onclick={toggleYolo}
      title="Toggle YOLO mode — skip confirmations"
    >
      {$yoloMode ? '⚡ YOLO ON' : 'YOLO'}
    </button>
    <button class="settings-btn" onclick={openSettings} title="Settings">⚙</button>
  </div>
</div>

<style>
  .toolbar {
    height: var(--toolbar-height);
    background: var(--bg-secondary);
    border-bottom: var(--border-width) solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    flex-shrink: 0;
    user-select: none;
  }

  .toolbar-left,
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .logo {
    font-weight: 800;
    font-size: 16px;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: -0.02em;
    margin-right: 12px;
  }

  .git-actions {
    display: flex;
    gap: 4px;
  }

  button {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    padding: var(--btn-padding);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
    min-height: var(--toolbar-height);
  }

  button:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
  }

  .yolo-btn {
    letter-spacing: 0.05em;
  }

  .yolo-btn.active {
    background: var(--danger-dark);
    border-color: var(--danger);
    color: #fff;
    animation: yolo-pulse 2s ease-in-out infinite;
  }

  .yolo-btn.active:hover {
    background: var(--danger);
  }

  @keyframes yolo-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.8; }
  }

  .settings-btn {
    font-size: 16px;
    padding: 4px 10px;
  }
</style>
