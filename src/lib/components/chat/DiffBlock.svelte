<script lang="ts">
  import type { DiffHunk } from '$lib/types/agent-event';
  import type { Adapter } from '$lib/adapter/index';

  let { filePath, hunks, adapter }: {
    filePath: string;
    hunks: DiffHunk[];
    adapter?: Adapter;
  } = $props();

  let expanded = $state(true);
</script>

<div class="diff-block">
  <button class="diff-header" onclick={() => expanded = !expanded}>
    <span class="diff-icon">±</span>
    <span class="diff-file">{filePath}</span>
    <span class="diff-stats">
      {hunks.length} hunk{hunks.length !== 1 ? 's' : ''}
    </span>
    <span class="diff-toggle">{expanded ? '▾' : '▸'}</span>
  </button>

  {#if expanded}
    <div class="diff-content">
      {#each hunks as hunk, i}
        <div class="hunk">
          <div class="hunk-header">@@ -{hunk.old_start} +{hunk.new_start} @@</div>
          <div class="hunk-lines">
            {#each hunk.old_lines as line}
              <div class="line removed">- {line}</div>
            {/each}
            {#each hunk.new_lines as line}
              <div class="line added">+ {line}</div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .diff-block {
    border: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
    overflow: hidden;
  }

  .diff-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 12px;
    background: var(--bg-tertiary);
    border: none;
    border-bottom: var(--border-width) solid var(--border);
    cursor: pointer;
    color: var(--text-secondary);
  }

  .diff-header:hover {
    color: var(--text-primary);
  }

  .diff-icon {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: var(--font-size-base);
  }

  .diff-file {
    font-family: var(--font-mono);
    font-size: var(--font-size-small);
    font-weight: 700;
    flex: 1;
    text-align: left;
  }

  .diff-stats {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
  }

  .diff-toggle {
    font-size: var(--font-size-small);
  }

  .diff-content {
    overflow-x: auto;
  }

  .hunk {
    border-bottom: var(--border-width) solid var(--border);
  }

  .hunk:last-child {
    border-bottom: none;
  }

  .hunk-header {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    background: var(--bg-tertiary);
    padding: 2px 12px;
    border-bottom: 1px solid var(--border);
  }

  .hunk-lines {
    font-family: var(--font-mono);
    font-size: var(--font-size-code);
    line-height: 1.5;
  }

  .line {
    padding: 0 12px;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .line.removed {
    background: rgba(220, 38, 38, 0.1);
    color: var(--danger);
  }

  .line.added {
    background: rgba(22, 163, 74, 0.1);
    color: var(--success);
  }
</style>
