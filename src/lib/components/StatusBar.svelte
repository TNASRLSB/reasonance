<script lang="ts">
  import { yoloMode } from '$lib/stores/ui';
</script>

<div class="status-bar">
  <div class="status-left">
    <span class="app-name">Forge IDE</span>
  </div>
  <div class="status-right">
    {#if $yoloMode}
      <span class="yolo-indicator">⚡ YOLO MODE — confirmations disabled</span>
    {/if}
  </div>
</div>

<style>
  .status-bar {
    height: 24px;
    background: var(--accent);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    flex-shrink: 0;
    font-size: 12px;
    user-select: none;
  }

  .status-left,
  .status-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .app-name {
    font-weight: 600;
    letter-spacing: 0.04em;
  }

  .yolo-indicator {
    background: var(--danger);
    padding: 1px 8px;
    border-radius: 3px;
    font-weight: 600;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }
</style>
