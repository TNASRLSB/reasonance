<script lang="ts">
  import type { DiffHunk } from '$lib/types/agent-event';
  import type { Adapter } from '$lib/adapter/index';
  import { addOpenFile } from '$lib/stores/files';
  import { showToast } from '$lib/stores/toast';

  let { filePath, hunks, adapter }: {
    filePath: string;
    hunks: DiffHunk[];
    adapter?: Adapter;
  } = $props();

  let expanded = $state(true);
  let applyState = $state<'idle' | 'applying' | 'applied' | 'rejected'>('idle');

  function applyHunks(original: string, hunksToApply: DiffHunk[]): string {
    const lines = original.split('\n');
    // Sort hunks by old_start descending so indices stay valid
    const sorted = [...hunksToApply].sort((a, b) => b.old_start - a.old_start);
    for (const hunk of sorted) {
      const start = hunk.old_start - 1; // 1-indexed → 0-indexed
      const deleteCount = hunk.old_lines.length;
      lines.splice(start, deleteCount, ...hunk.new_lines);
    }
    return lines.join('\n');
  }

  async function handleApply() {
    if (!adapter || applyState !== 'idle') return;
    applyState = 'applying';
    try {
      const content = await adapter.readFile(filePath);
      const patched = applyHunks(content, hunks);
      await adapter.writeFile(filePath, patched);
      applyState = 'applied';
      // Refresh the open file tab
      const name = filePath.split('/').pop() ?? filePath;
      addOpenFile({ path: filePath, name, content: patched, isDirty: false, isDeleted: false });
      showToast('success', 'Diff applied', filePath);
    } catch (e) {
      applyState = 'idle';
      showToast('error', 'Failed to apply diff', String(e));
    }
  }

  function handleReject() {
    applyState = 'rejected';
  }
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
              <div class="line removed"><span class="diff-prefix" aria-hidden="true">-</span> {line}</div>
            {/each}
            {#each hunk.new_lines as line}
              <div class="line added"><span class="diff-prefix" aria-hidden="true">+</span> {line}</div>
            {/each}
          </div>
        </div>
      {/each}
    </div>

    {#if adapter}
      <div class="diff-actions">
        {#if applyState === 'idle'}
          <button class="diff-action apply" onclick={handleApply} aria-label="Apply changes to {filePath}">Apply</button>
          <button class="diff-action reject" onclick={handleReject} aria-label="Reject changes to {filePath}">Reject</button>
        {:else if applyState === 'applying'}
          <span class="diff-status">Applying...</span>
        {:else if applyState === 'applied'}
          <span class="diff-status applied">Applied</span>
        {:else if applyState === 'rejected'}
          <span class="diff-status rejected">Rejected</span>
          <button class="diff-action undo" onclick={() => applyState = 'idle'} aria-label="Undo rejection">Undo</button>
        {/if}
      </div>
    {/if}
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
    text-align: start;
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

  .diff-prefix {
    font-weight: 700;
    font-size: 1.1em;
    margin-inline-end: 2px;
  }

  .line.removed {
    background: rgba(220, 38, 38, 0.1);
    color: var(--danger-text);
    border-inline-start: 3px solid var(--danger);
  }

  .line.added {
    background: rgba(22, 163, 74, 0.1);
    color: var(--success-text);
    border-inline-start: 3px solid var(--success);
  }

  .diff-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-top: var(--border-width) solid var(--border);
    background: var(--bg-tertiary);
  }

  .diff-action {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    padding: 4px 14px;
    border: var(--border-width) solid var(--border);
    cursor: pointer;
    min-height: 28px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .diff-action.apply {
    background: var(--success);
    color: #fff;
    border-color: var(--success);
  }

  .diff-action.apply:hover {
    opacity: 0.85;
  }

  .diff-action.reject {
    background: transparent;
    color: var(--text-secondary);
  }

  .diff-action.reject:hover {
    color: var(--danger-text);
    border-color: var(--danger);
  }

  .diff-action.undo {
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--font-size-tiny);
    padding: 2px 8px;
    min-height: 24px;
  }

  .diff-status {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }

  .diff-status.applied {
    color: var(--success-text);
  }

  .diff-status.rejected {
    color: var(--text-muted);
    text-decoration: line-through;
  }
</style>
