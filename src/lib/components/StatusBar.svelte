<script lang="ts">
  import { yoloMode } from '$lib/stores/ui';
  import { tr } from '$lib/i18n/index';
</script>

<div class="status-bar" class:yolo={$yoloMode}>
  <div class="status-left">
    {#if $yoloMode}
      <span class="yolo-label">✕ {$tr('status.yolo')}</span>
    {:else}
      <span class="app-name">REASONANCE</span>
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
    padding: 0 14px;
    flex-shrink: 0;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 600;
    user-select: none;
    border-top: none;
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
