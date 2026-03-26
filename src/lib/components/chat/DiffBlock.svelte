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
  let conflictState = $state<'none' | 'detected'>('none');
  let conflictDetails = $state<string[]>([]);

  function validateHunks(currentContent: string, hunksToValidate: DiffHunk[]): { valid: boolean; mismatches: string[] } {
    const lines = currentContent.split('\n');
    const mismatches: string[] = [];

    for (const hunk of hunksToValidate) {
      const start = hunk.old_start - 1; // 0-indexed
      for (let i = 0; i < hunk.old_lines.length; i++) {
        const lineIdx = start + i;
        if (lineIdx >= lines.length || lines[lineIdx] !== hunk.old_lines[i]) {
          mismatches.push(`Line ${lineIdx + 1}: expected "${hunk.old_lines[i]?.slice(0, 50)}" but found "${lines[lineIdx]?.slice(0, 50) ?? '<EOF>'}"`);
          break; // One mismatch per hunk is enough
        }
      }
    }

    return { valid: mismatches.length === 0, mismatches };
  }

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

  async function applyDiff() {
    if (!adapter) return;
    applyState = 'applying';
    try {
      const content = await adapter.readFile(filePath);
      const patched = applyHunks(content, hunks);
      await adapter.writeFile(filePath, patched);
      applyState = 'applied';
      // Refresh the open file tab
      addOpenFile(filePath, patched);
      showToast('success', 'Diff applied', filePath);
    } catch (e) {
      applyState = 'idle';
      showToast('error', 'Failed to apply diff', String(e));
    }
  }

  async function handleApply() {
    if (!adapter || applyState !== 'idle') return;

    const content = await adapter.readFile(filePath);
    const validation = validateHunks(content, hunks);

    if (!validation.valid) {
      conflictState = 'detected';
      conflictDetails = validation.mismatches;
      return; // Show conflict UI, don't apply
    }

    await applyDiff();
  }

  async function handleForceApply() {
    conflictState = 'none';
    conflictDetails = [];
    await applyDiff();
  }

  function handleDismissConflict() {
    conflictState = 'none';
    conflictDetails = [];
  }

  function handleReject() {
    applyState = 'rejected';
  }
</script>

<div class="diff-block">
  <button class="diff-header" onclick={() => expanded = !expanded} aria-expanded={expanded} aria-label="{expanded ? 'Collapse' : 'Expand'} diff for {filePath}">
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
              <div class="line removed"><span class="diff-prefix">-</span> {line}</div>
            {/each}
            {#each hunk.new_lines as line}
              <div class="line added"><span class="diff-prefix">+</span> {line}</div>
            {/each}
          </div>
        </div>
      {/each}
    </div>

    {#if conflictState === 'detected'}
      <div class="diff-conflict" role="alert">
        <p class="conflict-warning">File has been modified since this diff was generated</p>
        <ul class="conflict-details">
          {#each conflictDetails as detail}
            <li>{detail}</li>
          {/each}
        </ul>
        <div class="conflict-actions">
          <button class="diff-action force-apply" onclick={handleForceApply} aria-label="Apply diff anyway despite conflicts">Apply anyway</button>
          <button class="diff-action dismiss" onclick={handleDismissConflict} aria-label="Dismiss conflict warning">Dismiss</button>
        </div>
      </div>
    {/if}

    {#if adapter}
      <div class="diff-actions">
        {#if applyState === 'idle'}
          <button class="diff-action apply" onclick={handleApply} aria-label="Apply changes to {filePath}" disabled={conflictState === 'detected'}>Apply</button>
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
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-1) var(--space-3);
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
    padding: var(--stack-tight) var(--space-3);
    border-bottom: 1px solid var(--border);
  }

  .hunk-lines {
    font-family: var(--font-mono);
    font-size: var(--font-size-code);
    line-height: 1.5;
  }

  .line {
    padding: 0 var(--space-3);
    white-space: pre-wrap;
    word-break: break-all;
  }

  .diff-prefix {
    font-weight: 700;
    font-size: var(--font-size-base);
    margin-inline-end: var(--space-1);
    min-width: 1em;
    display: inline-block;
    text-align: center;
  }

  .line.removed {
    background: var(--diff-remove-bg);
    color: var(--danger-text);
    border-inline-start: 3px solid var(--danger);
  }

  .line.added {
    background: var(--diff-add-bg);
    color: var(--success-text);
    border-inline-start: 3px solid var(--success);
  }

  .diff-actions {
    display: flex;
    align-items: center;
    gap: var(--interactive-gap);
    padding: var(--space-1) var(--space-3);
    border-top: var(--border-width) solid var(--border);
    background: var(--bg-tertiary);
  }

  .diff-action {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    padding: var(--space-1) var(--space-3);
    border: var(--border-width) solid var(--border);
    cursor: pointer;
    min-height: 28px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .diff-action.apply {
    background: var(--success);
    color: var(--text-on-accent);
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
    padding: var(--stack-tight) var(--space-2);
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

  .diff-conflict {
    padding: var(--space-2) var(--space-3);
    background: var(--warning-bg, color-mix(in srgb, var(--warning, #f59e0b) 12%, transparent));
    border-top: var(--border-width) solid var(--warning, #f59e0b);
    border-bottom: var(--border-width) solid var(--warning, #f59e0b);
  }

  .conflict-warning {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    color: var(--warning-text, #92400e);
    margin: 0 0 var(--space-1) 0;
  }

  .conflict-details {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--warning-text, #92400e);
    margin: 0 0 var(--space-2) 0;
    padding-inline-start: var(--space-4);
    opacity: 0.85;
  }

  .conflict-details li {
    margin-bottom: var(--stack-tight);
    word-break: break-all;
  }

  .conflict-actions {
    display: flex;
    gap: var(--interactive-gap);
  }

  .diff-action.force-apply {
    background: var(--warning, #f59e0b);
    color: var(--text-on-accent, #fff);
    border-color: var(--warning, #f59e0b);
  }

  .diff-action.force-apply:hover {
    opacity: 0.85;
  }

  .diff-action.dismiss {
    background: transparent;
    color: var(--text-secondary);
  }

  .diff-action.dismiss:hover {
    color: var(--text-primary);
    border-color: var(--text-secondary);
  }
</style>
