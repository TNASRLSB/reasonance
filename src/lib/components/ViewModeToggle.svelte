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
  aria-label={mode === 'chat' ? 'Switch to terminal view' : 'Switch to chat view'}
  aria-pressed={mode === 'terminal'}
  aria-describedby="viewmode-desc"
  title={mode === 'chat' ? 'Switch to terminal' : 'Switch to chat'}
>
  {mode === 'chat' ? '▸ TERM' : '◂ CHAT'}
</button>
<span id="viewmode-desc" class="sr-only">Switch between structured chat interface and raw terminal output</span>

<style>
  .view-mode-toggle {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    background: transparent;
    border: var(--border-width) solid var(--border);
    padding: var(--btn-padding-sm);
    min-height: 2rem;
    display: inline-flex;
    align-items: center;
    cursor: pointer;
    white-space: nowrap;
    transition: color var(--transition-fast), border-color var(--transition-fast);
  }

  .view-mode-toggle:hover {
    color: var(--text-primary);
    border-color: var(--accent);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
