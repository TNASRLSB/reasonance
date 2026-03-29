<script lang="ts">
  import { projectSummaries, switchProject } from '$lib/stores/projects';
  import type { ProjectSummary } from '$lib/stores/projects';
  import { trapFocus } from '$lib/utils/a11y';

  let {
    open,
    onClose,
  }: {
    open: boolean;
    onClose: () => void;
  } = $props();

  let query = $state('');
  let selectedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);
  let dialogEl = $state<HTMLElement | null>(null);

  // ── Focus trap ────────────────────────────────────────────────────────────

  $effect(() => {
    if (open && dialogEl) {
      const destroy = trapFocus(dialogEl);
      return destroy;
    }
  });

  $effect(() => {
    if (open && inputEl) {
      query = '';
      selectedIndex = 0;
      inputEl.focus();
    }
  });

  // ── Fuzzy filter ──────────────────────────────────────────────────────────

  function fuzzyMatch(text: string, q: string): boolean {
    if (q === '') return true;
    const lower = text.toLowerCase();
    const qLower = q.toLowerCase();
    let idx = 0;
    for (const ch of qLower) {
      const found = lower.indexOf(ch, idx);
      if (found === -1) return false;
      idx = found + 1;
    }
    return true;
  }

  const filtered = $derived(
    $projectSummaries.filter(
      (p) => fuzzyMatch(p.label, query) || fuzzyMatch(p.rootPath, query)
    )
  );

  $effect(() => {
    // Reset selection when filter changes
    filtered;
    selectedIndex = 0;
  });

  // ── Keyboard navigation ───────────────────────────────────────────────────

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, filtered.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const project = filtered[selectedIndex];
      if (project) select(project);
    }
  }

  function select(project: ProjectSummary) {
    switchProject(project.id);
    onClose();
  }

  function handleOverlayClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('qs-overlay')) onClose();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="qs-overlay"
    onclick={handleOverlayClick}
    onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}
    role="presentation"
  >
    <div
      class="qs-dialog"
      bind:this={dialogEl}
      role="dialog"
      aria-modal="true"
      aria-label="Switch project"
    >
      <div class="qs-input-row">
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={handleKeydown}
          type="text"
          placeholder="Search projects..."
          class="qs-input"
          aria-label="Search projects"
          autocomplete="off"
          spellcheck="false"
        />
      </div>

      {#if filtered.length > 0}
        <ul class="qs-list" role="listbox" aria-label="Projects">
          {#each filtered as project, i (project.id)}
            <li
              class="qs-item"
              class:selected={i === selectedIndex}
              role="option"
              aria-selected={i === selectedIndex}
              tabindex="-1"
              onclick={() => select(project)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') select(project); }}
            >
              <span
                class="qs-dot"
                style="background: {project.color}"
                aria-hidden="true"
              ></span>
              <span class="qs-label">{project.label}</span>
              <span class="qs-path">{project.rootPath}</span>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="qs-empty">
          {query.trim() ? `No projects matching "${query}"` : 'No projects added yet'}
        </p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .qs-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    z-index: var(--layer-modal);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 80px;
  }

  .qs-dialog {
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    width: 500px;
    max-width: 95vw;
    max-height: 50vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--font-ui);
  }

  .qs-input-row {
    display: flex;
    align-items: center;
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .qs-input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: var(--font-size-base);
    font-family: var(--font-ui);
  }

  .qs-input:focus-visible {
    outline: var(--focus-ring);
    outline-offset: var(--focus-offset);
  }

  .qs-input::placeholder {
    color: var(--text-secondary);
  }

  .qs-list {
    list-style: none;
    margin: 0;
    padding: var(--space-1) 0;
    overflow-y: auto;
    flex: 1;
  }

  .qs-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    transition: background var(--transition-fast);
    min-height: 32px;
  }

  .qs-item:hover,
  .qs-item.selected {
    background: var(--accent-btn);
    color: var(--text-on-accent);
  }

  .qs-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .qs-label {
    font-size: var(--font-size-small);
    font-weight: 600;
    flex-shrink: 0;
  }

  .qs-path {
    font-size: var(--font-size-tiny);
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .qs-item.selected .qs-path,
  .qs-item:hover .qs-path {
    color: color-mix(in srgb, var(--text-on-accent) 60%, transparent);
  }

  .qs-empty {
    padding: var(--space-4) var(--space-3);
    font-size: var(--font-size-small);
    color: var(--text-secondary);
    margin: 0;
    text-align: center;
  }
</style>
