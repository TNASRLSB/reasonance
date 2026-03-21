<script lang="ts">
  import { yoloMode } from '$lib/stores/ui';
</script>

<div class="status-bar" class:yolo={$yoloMode}>
  <div class="status-left">
    {#if $yoloMode}
      <span class="yolo-label">✕ YOLO MODE — CONFIRMATIONS DISABLED</span>
    {:else}
      <span class="app-name">FORGE IDE</span>
    {/if}
  </div>
  <div class="status-right">
    <!-- File info will be added later when we wire up active file tracking -->
  </div>
</div>

<style>
  .status-bar {
    height: var(--statusbar-height);
    background: var(--accent);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    flex-shrink: 0;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    user-select: none;
    border-top: var(--border-width) solid var(--border);
  }

  .status-bar.yolo {
    background: var(--danger);
  }

  .status-left,
  .status-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .app-name {
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .yolo-label {
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
</style>
