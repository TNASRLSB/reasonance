<script lang="ts">
  import { contrastRatio, wcagLevel, type WcagLevel } from '$lib/engine/contrast-checker';

  let { foreground, background }: { foreground: string; background: string } = $props();

  let ratio = $derived(
    foreground && background && foreground.startsWith('#') && background.startsWith('#')
      ? contrastRatio(foreground, background)
      : null
  );

  let level = $derived<WcagLevel | null>(ratio !== null ? wcagLevel(ratio) : null);

  let badgeClass = $derived(
    level === 'AAA' ? 'badge--aaa' :
    level === 'AA'  ? 'badge--aa' :
    level === 'FAIL' ? 'badge--fail' :
    'badge--unknown'
  );
</script>

{#if ratio !== null && level !== null}
  <span class="contrast-badge {badgeClass}" title="Contrast ratio: {ratio.toFixed(2)}:1">
    {ratio.toFixed(1)}:1 <span class="badge-level">{level}</span>
  </span>
{/if}

<style>
  .contrast-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    padding: 1px 6px;
    border-radius: 3px;
    border: 1px solid currentColor;
    white-space: nowrap;
  }

  .badge-level {
    font-weight: 700;
    font-size: 10px;
    letter-spacing: 0.04em;
  }

  .badge--aaa {
    color: #22c55e;
    border-color: #22c55e44;
    background: #22c55e18;
  }

  .badge--aa {
    color: #f59e0b;
    border-color: #f59e0b44;
    background: #f59e0b18;
  }

  .badge--fail {
    color: #ef4444;
    border-color: #ef444444;
    background: #ef444418;
  }

  .badge--unknown {
    color: var(--text-muted);
    border-color: var(--border);
    background: transparent;
  }
</style>
