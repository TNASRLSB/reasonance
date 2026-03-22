<script lang="ts">
  import type { ViewMode } from '$lib/types/agent-event';

  let { mode = 'terminal' as ViewMode, onToggle }: {
    mode: ViewMode;
    onToggle: (mode: ViewMode) => void;
  } = $props();
</script>

<button
  class="view-mode-toggle"
  onclick={() => onToggle(mode === 'chat' ? 'terminal' : 'chat')}
  aria-label="Toggle between chat and terminal view"
  title={mode === 'chat' ? 'Switch to terminal' : 'Switch to chat'}
>
  {mode === 'chat' ? '▸ TERM' : '◂ CHAT'}
</button>

<style>
  .view-mode-toggle {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    background: transparent;
    border: var(--border-width) solid var(--border);
    padding: 2px 8px;
    cursor: pointer;
    white-space: nowrap;
  }

  .view-mode-toggle:hover {
    color: var(--text-primary);
    border-color: var(--accent);
  }
</style>
