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
    <span class="logo">⚒ Forge</span>
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
    height: 40px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
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
    font-weight: 600;
    font-size: 14px;
    color: var(--accent);
    margin-right: 8px;
  }

  .git-actions {
    display: flex;
    gap: 4px;
  }

  button {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 3px 10px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s;
  }

  button:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .yolo-btn {
    font-weight: 600;
    letter-spacing: 0.03em;
  }

  .yolo-btn.active {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }

  .yolo-btn.active:hover {
    background: #c0392b;
  }

  .settings-btn {
    font-size: 16px;
    padding: 2px 8px;
  }
</style>
