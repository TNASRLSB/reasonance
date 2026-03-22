<script lang="ts">
  import type { FileAction } from '$lib/types/agent-event';

  let { path, action }: {
    path: string;
    action: FileAction;
  } = $props();

  const actionIcons: Record<FileAction, string> = {
    read: '📖',
    write: '✏',
    create: '+',
    delete: '✕',
  };

  const actionColors: Record<FileAction, string> = {
    read: 'var(--text-secondary)',
    write: 'var(--accent-text)',
    create: 'var(--success)',
    delete: 'var(--danger)',
  };

  let icon = $derived(actionIcons[action] ?? '📄');
  let color = $derived(actionColors[action] ?? 'var(--text-secondary)');

  // Extract filename from path
  let filename = $derived(path.split('/').pop() ?? path);
</script>

<span class="file-ref-badge" style="color: {color};" title="{action}: {path}">
  <span class="file-icon">{icon}</span>
  <span class="file-name">{filename}</span>
</span>

<style>
  .file-ref-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: var(--font-mono);
    font-size: var(--font-size-small);
    background: var(--bg-tertiary);
    border: var(--border-width) solid var(--border);
    padding: 2px 8px;
    cursor: default;
  }

  .file-icon {
    font-size: var(--font-size-tiny);
  }

  .file-name {
    font-weight: 700;
  }
</style>
