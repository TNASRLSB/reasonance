<script lang="ts">
  import { yoloMode, showSettings } from '$lib/stores/ui';
  import type { Adapter } from '$lib/adapter/index';
  import MenuBar from './MenuBar.svelte';

  let { adapter }: { adapter: Adapter } = $props();

  function toggleYolo() {
    yoloMode.update((v) => !v);
  }

  function openSettings() {
    showSettings.set(true);
  }
</script>

<div class="toolbar">
  <div class="toolbar-left">
    <span class="logo">REASONANCE</span>
    <MenuBar {adapter} />
  </div>

  <div class="toolbar-right">
    <button
      class="yolo-btn"
      class:active={$yoloMode}
      onclick={toggleYolo}
      title="Toggle YOLO mode"
    >
      {$yoloMode ? '⚡ YOLO ON' : 'YOLO'}
    </button>
    <button class="settings-btn" onclick={openSettings} title="Settings">&#9881;</button>
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

  button {
    background: var(--bg-surface);
    color: var(--text-body);
    border: var(--border-width) solid var(--border);
    border-radius: 0;
    padding: var(--btn-padding);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
    min-height: 32px;
  }

  button:hover {
    background: var(--text-primary);
    color: var(--bg-primary);
    border-color: var(--text-primary);
  }

  .yolo-btn {
    letter-spacing: 0.05em;
  }

  .yolo-btn.active {
    background: var(--danger-dark);
    border-color: var(--danger);
    color: var(--text-primary);
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
