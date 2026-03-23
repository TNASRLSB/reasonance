<script lang="ts">
  import type { Adapter } from '$lib/adapter/index';
  import type { SessionSummary } from '$lib/types/agent-event';
  import { showToast } from '$lib/stores/toast';
  import { tr } from '$lib/i18n/index';

  let { adapter, visible, onClose, onRestore }: {
    adapter: Adapter;
    visible: boolean;
    onClose: () => void;
    onRestore: (sessionId: string) => void;
  } = $props();

  function autoFocus(node: HTMLElement) { node.focus(); }

  let sessions = $state<SessionSummary[]>([]);
  let loading = $state(false);
  let filter = $state('');
  let renamingId = $state<string | null>(null);
  let renameValue = $state('');

  $effect(() => {
    if (visible) loadSessions();
  });

  async function loadSessions() {
    loading = true;
    try {
      sessions = await adapter.sessionList();
      sessions.sort((a, b) => b.created_at - a.created_at);
    } catch (e) {
      showToast('error', 'Failed to load sessions', String(e));
    }
    loading = false;
  }

  let filtered = $derived(
    filter.trim()
      ? sessions.filter((s) =>
          s.title.toLowerCase().includes(filter.toLowerCase()) ||
          s.provider.toLowerCase().includes(filter.toLowerCase())
        )
      : sessions
  );

  function formatDate(ts: number): string {
    return new Intl.DateTimeFormat(undefined, {
      month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
    }).format(new Date(ts * 1000));
  }

  async function deleteSession(id: string) {
    try {
      await adapter.sessionDelete(id);
      sessions = sessions.filter((s) => s.id !== id);
      showToast('success', 'Session deleted', '');
    } catch (e) {
      showToast('error', 'Delete failed', String(e));
    }
  }

  function startRename(s: SessionSummary) {
    renamingId = s.id;
    renameValue = s.title;
  }

  async function commitRename() {
    if (!renamingId || !renameValue.trim()) {
      renamingId = null;
      return;
    }
    try {
      await adapter.sessionRename(renamingId, renameValue.trim());
      sessions = sessions.map((s) =>
        s.id === renamingId ? { ...s, title: renameValue.trim() } : s
      );
    } catch (e) {
      showToast('error', 'Rename failed', String(e));
    }
    renamingId = null;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (renamingId) { renamingId = null; }
      else { onClose(); }
    }
  }
</script>

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="session-overlay" onclick={(e) => { if ((e.target as HTMLElement).classList.contains('session-overlay')) onClose(); }} onkeydown={handleKeydown}>
    <div class="session-panel" role="dialog" aria-label="Session history" aria-modal="true">
      <div class="panel-header">
        <h2 class="panel-title">{$tr('sessions.title')}</h2>
        <button class="panel-close" onclick={onClose} aria-label="Close">&#10005;</button>
      </div>

      <div class="panel-search">
        <input
          type="text"
          bind:value={filter}
          placeholder={$tr('sessions.search')}
          class="search-input"
          aria-label="Filter sessions"
        />
      </div>

      {#if loading}
        <div class="panel-loading">Loading...</div>
      {:else if filtered.length === 0}
        <div class="panel-empty">{filter ? 'No matching sessions' : 'No sessions yet'}</div>
      {:else}
        <ul class="session-list" role="list">
          {#each filtered as session (session.id)}
            <li class="session-item">
              <div class="session-info">
                {#if renamingId === session.id}
                  <input
                    class="rename-input"
                    bind:value={renameValue}
                    onblur={commitRename}
                    onkeydown={(e) => { if (e.key === 'Enter') commitRename(); if (e.key === 'Escape') { renamingId = null; } }}
                    use:autoFocus
                  />
                {:else}
                  <button class="session-title" onclick={() => { onRestore(session.id); onClose(); }}>
                    {session.title || 'Untitled'}
                  </button>
                {/if}
                <div class="session-meta">
                  <span class="session-provider">{session.provider}</span>
                  <span class="session-date">{formatDate(session.created_at)}</span>
                  <span class="session-status status-{session.status}">{session.status}</span>
                </div>
              </div>
              <div class="session-actions">
                <button class="action-btn" onclick={() => startRename(session)} aria-label="Rename session" title="Rename">&#9998;</button>
                <button class="action-btn danger" onclick={() => deleteSession(session.id)} aria-label="Delete session" title="Delete">&#10005;</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
{/if}

<style>
  .session-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 2000;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 60px;
  }

  .session-panel {
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    width: 520px;
    max-width: 95vw;
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--font-ui);
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .panel-title {
    font-size: var(--font-size-tiny);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-primary);
    margin: 0;
  }

  .panel-close {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    cursor: pointer;
    min-width: 32px;
    min-height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .panel-close:hover {
    color: var(--text-primary);
  }

  .panel-search {
    padding: var(--space-2) var(--space-4);
    border-bottom: var(--border-width) solid var(--border);
    flex-shrink: 0;
  }

  .search-input {
    width: 100%;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    color: var(--text-primary);
    padding: var(--space-1) var(--space-2);
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .panel-loading,
  .panel-empty {
    padding: var(--space-5) var(--space-4);
    text-align: center;
    color: var(--text-muted);
    font-size: var(--font-size-small);
  }

  .session-list {
    list-style: none;
    margin: 0;
    padding: var(--space-1) 0;
    overflow-y: auto;
    flex: 1;
  }

  .session-item {
    display: flex;
    align-items: center;
    padding: var(--space-2) var(--space-4);
    gap: var(--space-2);
    border-bottom: 1px solid var(--bg-tertiary);
  }

  .session-item:hover {
    background: var(--bg-hover);
  }

  .session-info {
    flex: 1;
    min-width: 0;
  }

  .session-title {
    display: block;
    width: 100%;
    background: none;
    border: none;
    text-align: start;
    font-family: var(--font-ui);
    font-size: var(--font-size-sm);
    font-weight: 700;
    color: var(--text-primary);
    cursor: pointer;
    padding: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .session-title:hover {
    color: var(--accent-text);
  }

  .rename-input {
    width: 100%;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--accent);
    color: var(--text-primary);
    padding: var(--stack-tight) var(--space-1);
    font-family: var(--font-ui);
    font-size: var(--font-size-sm);
  }

  .session-meta {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--stack-tight);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
  }

  .session-provider {
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .session-status {
    text-transform: uppercase;
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .status-idle { color: var(--text-muted); }
  .status-running { color: var(--success-text); }
  .status-error { color: var(--danger-text); }

  .session-actions {
    display: flex;
    gap: var(--stack-tight);
    flex-shrink: 0;
    opacity: 0;
  }

  .session-item:hover .session-actions {
    opacity: 1;
  }

  .action-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: var(--font-size-sm);
    cursor: pointer;
    min-width: 28px;
    min-height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .action-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .action-btn.danger:hover {
    color: var(--danger-text);
  }
</style>
