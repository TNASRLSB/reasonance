<script lang="ts">
  import { tr } from '$lib/i18n/index';
  import { activeFilePath, cursorLine, cursorCol } from '$lib/stores/files';
  import { llmConfigs } from '$lib/stores/config';
  import { activeInstance } from '$lib/stores/terminals';
  import { updateState } from '$lib/stores/update';
  import { installUpdate, postponeUpdate } from '$lib/updater';
  import { toasts, dismissToast, pauseToastTimer, resumeToastTimer } from '$lib/stores/toast';

  let activeInstanceData = $derived($activeInstance);
  let installing = $state(false);

  // Notification area: show the most recent toast, with a count badge when queued
  let currentToast = $derived($toasts.length > 0 ? $toasts[$toasts.length - 1] : null);
  let queueCount = $derived($toasts.length);

  // Expand/collapse for the full notification list
  let expanded = $state(false);

  const typeIcons: Record<string, string> = {
    error: '✕', warning: '⚠', success: '✓', info: 'ℹ', update: '↑',
  };
  const typeColors: Record<string, string> = {
    error: 'var(--danger)', warning: 'var(--warning)',
    success: 'var(--success)', info: 'var(--accent)', update: 'var(--accent)',
  };

  async function handleInstall() {
    installing = true;
    await installUpdate();
  }

  function generateBar(percent: number): string {
    const filled = Math.round(percent / 12.5);
    const empty = 8 - filled;
    return '\u2588'.repeat(filled) + '\u2591'.repeat(empty);
  }

  function getLang(path: string): string {
    const ext = path.split('.').pop()?.toLowerCase() ?? '';
    const map: Record<string, string> = {
      rs: 'Rust', ts: 'TypeScript', js: 'JavaScript', py: 'Python',
      svelte: 'Svelte', html: 'HTML', css: 'CSS', json: 'JSON',
      md: 'Markdown', toml: 'TOML', yaml: 'YAML', yml: 'YAML',
    };
    return map[ext] ?? ext.toUpperCase();
  }

  function toggleExpanded() {
    expanded = !expanded;
  }

  function handleDismiss(id: number, e: MouseEvent) {
    e.stopPropagation();
    dismissToast(id);
    if ($toasts.length <= 1) expanded = false;
  }

  function handleDismissAll(e: MouseEvent) {
    e.stopPropagation();
    const ids = $toasts.map(t => t.id);
    for (const id of ids) dismissToast(id);
    expanded = false;
  }

  function handleWindowClick() {
    if (expanded) expanded = false;
  }
</script>

<svelte:window onclick={handleWindowClick} />

<div class="status-bar" role="status">
    <div class="status-left">
      <span class="app-name">REASONANCE</span>
      {#if $updateState.newVersion}
        <span class="separator">|</span>
        <span class="update-indicator">
          {#if $updateState.downloadProgress != null}
            Updating... {$updateState.downloadProgress}%
          {:else}
            v{__APP_VERSION__} → v{$updateState.newVersion}
            <button class="status-action" onclick={handleInstall} disabled={installing}>Install</button>
            <button class="status-action dim" onclick={postponeUpdate}>Later</button>
          {/if}
        </span>
      {/if}

      {#if currentToast}
        <span class="separator">|</span>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span
          class="notif-inline"
          onclick={(e) => e.stopPropagation()}
          onmouseenter={() => pauseToastTimer(currentToast!.id)}
          onmouseleave={() => resumeToastTimer(currentToast!.id)}
        >
          <span class="notif-icon" aria-hidden="true" style="color: {typeColors[currentToast.type] ?? typeColors.info}">
            {typeIcons[currentToast.type] ?? typeIcons.info}
          </span>
          <span class="notif-text" title={currentToast.body || currentToast.title}>
            {currentToast.title}
          </span>
          {#if currentToast.progress !== undefined}
            <span class="notif-progress-track">
              <span class="notif-progress-fill" style="width: {currentToast.progress}%"></span>
            </span>
          {/if}
          {#if currentToast.actions?.length}
            {#each currentToast.actions as action}
              <button class="status-action" onclick={action.onClick}>{action.label}</button>
            {/each}
          {/if}
          <button
            class="notif-dismiss"
            onclick={(e) => handleDismiss(currentToast!.id, e)}
            aria-label="Dismiss notification"
          >×</button>
          {#if queueCount > 1}
            <button
              class="notif-badge"
              onclick={toggleExpanded}
              aria-label="Show all {queueCount} notifications"
              aria-expanded={expanded}
            >{queueCount}</button>
          {/if}
        </span>
      {/if}
    </div>

    <div class="status-center">
      {#if activeInstanceData}
        {#if activeInstanceData.contextPercent != null}
          <span class="ctx-status">ctx {generateBar(activeInstanceData.contextPercent)} {activeInstanceData.contextPercent}%</span>
        {/if}
        {#if activeInstanceData.tokenCount}
          <span class="token-status">{activeInstanceData.tokenCount} tokens</span>
        {/if}
        {#if activeInstanceData.resetTimer}
          <span class="reset-status">reset: {activeInstanceData.resetTimer}</span>
        {/if}
        {#if activeInstanceData.messagesLeft != null}
          <span class="msg-status">msg: {activeInstanceData.messagesLeft}</span>
        {/if}
        {#if activeInstanceData.progressState === 1}
          <span class="progress-status"><span aria-hidden="true">&#9654;</span> {activeInstanceData.progressValue}%</span>
        {:else if activeInstanceData.progressState === 2}
          <span class="progress-status error"><span aria-hidden="true">&#10007;</span> error</span>
        {:else if activeInstanceData.progressState === 3}
          <span class="progress-status"><span aria-hidden="true">&#8943;</span> working</span>
        {:else if activeInstanceData.progressState === 4}
          <span class="progress-status paused"><span aria-hidden="true">&#10074;&#10074;</span> paused</span>
        {/if}
      {:else}
        <span class="idle-hint">{$tr('status.noSession')}</span>
      {/if}
    </div>

    <div class="status-right">
      {#if $activeFilePath}
        <span class="cursor-pos">Ln {$cursorLine}, Col {$cursorCol}</span>
        <span class="file-name">{$activeFilePath.split('/').pop()}</span>
        <span class="file-lang">{getLang($activeFilePath)}</span>
        <span class="file-encoding">UTF-8</span>
      {/if}
    </div>
</div>

<!-- Expanded notification list (flyout above status bar) -->
{#if expanded && $toasts.length > 0}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="notif-flyout" role="log" aria-label="Notifications" aria-live="polite" onclick={(e) => e.stopPropagation()}>
    <div class="notif-flyout-header">
      <span class="notif-flyout-title">Notifications ({$toasts.length})</span>
      <button class="notif-flyout-clear" onclick={handleDismissAll}>Clear all</button>
    </div>
    {#each [...$toasts].reverse() as toast (toast.id)}
      <div
        class="notif-flyout-item"
        role="alert"
        onmouseenter={() => pauseToastTimer(toast.id)}
        onmouseleave={() => resumeToastTimer(toast.id)}
      >
        <span class="notif-icon" aria-hidden="true" style="color: {typeColors[toast.type] ?? typeColors.info}">
          {typeIcons[toast.type] ?? typeIcons.info}
        </span>
        <div class="notif-flyout-content">
          <span class="notif-flyout-item-title">{toast.title}</span>
          {#if toast.body}
            <span class="notif-flyout-item-body">{toast.body}</span>
          {/if}
          {#if toast.progress !== undefined}
            <span class="notif-progress-track wide">
              <span class="notif-progress-fill" style="width: {toast.progress}%"></span>
            </span>
          {/if}
          {#if toast.actions?.length}
            <div class="notif-flyout-actions">
              {#each toast.actions as action}
                <button class="status-action" onclick={action.onClick}>{action.label}</button>
              {/each}
            </div>
          {/if}
        </div>
        <button
          class="notif-dismiss"
          onclick={(e) => handleDismiss(toast.id, e)}
          aria-label="Dismiss notification"
        >×</button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .status-bar {
    height: var(--statusbar-height);
    background: var(--accent-statusbar);
    color: var(--text-on-accent);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--space-3);
    flex-shrink: 0;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 600;
    user-select: none;
    border-top: 2px solid var(--border);
    position: relative;
    z-index: var(--layer-sticky);
  }

  .status-left,
  .status-center,
  .status-right {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .status-left {
    flex-shrink: 1;
    min-width: 0;
  }

  .status-center {
    flex: 1;
    justify-content: center;
    gap: var(--space-4);
  }

  .status-right {
    flex-shrink: 0;
  }

  .app-name {
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .separator {
    opacity: 0.75;
  }

  .cursor-pos {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    opacity: 0.85;
  }

  .file-name {
    font-weight: 500;
  }

  .file-lang {
    opacity: 0.7;
  }

  .file-encoding {
    opacity: 0.75;
  }

  .ctx-status {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    letter-spacing: 0.02em;
  }

  .token-status,
  .reset-status,
  .msg-status {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    opacity: 0.85;
  }

  .progress-status {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    opacity: 0.85;
  }

  .progress-status.error {
    color: var(--danger-text);
  }

  .progress-status.paused {
    color: var(--warning-text);
  }

  .idle-hint {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    opacity: 0.75;
    font-style: italic;
  }

  .update-indicator {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: 700;
  }

  .status-action {
    background: none;
    border: 1px solid currentColor;
    color: inherit;
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    padding: 0 var(--space-1);
    cursor: pointer;
    line-height: 1.4;
    opacity: 0.9;
    transition: opacity var(--transition-fast);
    min-width: 24px;
    min-height: 20px;
  }

  .status-action:hover {
    opacity: 1;
    background: var(--highlight-hover);
  }

  .status-action:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .status-action.dim {
    border: none;
    opacity: 0.6;
  }

  .status-action.dim:hover {
    opacity: 0.9;
  }

  /* Inline notification in status bar */
  .notif-inline {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    max-width: 400px;
    animation: notif-flash 0.3s ease-out;
  }

  .notif-icon {
    font-size: var(--font-size-sm);
    font-weight: 700;
    flex-shrink: 0;
    line-height: 1;
  }

  .notif-text {
    overflow: auto;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    font-size: var(--font-size-small);
  }

  .notif-dismiss {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: var(--font-size-base);
    line-height: 1;
    padding: 0 2px;
    opacity: 0.6;
    transition: opacity var(--transition-fast);
    min-width: 24px;
    min-height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .notif-dismiss:hover {
    opacity: 1;
  }

  .notif-badge {
    background: var(--highlight-hover);
    border: 1px solid rgba(255, 255, 255, 0.3);
    color: inherit;
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 800;
    padding: 0 5px;
    cursor: pointer;
    line-height: 1.5;
    border-radius: 2px;
    min-width: 20px;
    min-height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background var(--transition-fast);
  }

  .notif-badge:hover {
    background: var(--highlight-hover);
  }

  /* Inline progress bar */
  .notif-progress-track {
    display: inline-block;
    width: 48px;
    height: 3px;
    background: rgba(255, 255, 255, 0.15);
    border-radius: 1px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .notif-progress-track.wide {
    width: 100%;
    height: 3px;
    margin-top: 4px;
  }

  .notif-progress-fill {
    display: block;
    height: 100%;
    background: currentColor;
    transition: width var(--transition-normal);
  }

  /* Flyout panel (notification list) */
  .notif-flyout {
    position: absolute;
    bottom: calc(var(--statusbar-height) + 2px);
    left: 0;
    width: 380px;
    max-height: 320px;
    overflow-y: auto;
    background: var(--bg-secondary);
    border: var(--border-width) solid var(--border);
    border-bottom: none;
    z-index: calc(var(--layer-sticky) * 2);
    font-family: var(--font-ui);
    animation: flyout-up 0.15s ease-out;
    scroll-padding-top: calc(var(--font-size-small, 0.875rem) + var(--space-2, 0.5rem) * 2 + 4px);
  }

  .notif-flyout-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    position: sticky;
    top: 0;
    background: var(--bg-secondary);
    z-index: 1;
  }

  .notif-flyout-title {
    font-size: var(--font-size-small);
    font-weight: 700;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .notif-flyout-clear {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    cursor: pointer;
    padding: var(--space-1) var(--space-2);
    min-height: 24px;
    transition: color var(--transition-fast);
  }

  .notif-flyout-clear:hover {
    color: var(--text-primary);
  }

  .notif-flyout-item {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--border);
    transition: background var(--transition-fast);
  }

  .notif-flyout-item:last-child {
    border-bottom: none;
  }

  .notif-flyout-item:hover {
    background: var(--bg-tertiary);
  }

  .notif-flyout-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .notif-flyout-item-title {
    font-size: var(--font-size-small);
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.3;
  }

  .notif-flyout-item-body {
    font-size: var(--font-size-tiny);
    color: var(--text-secondary);
    line-height: 1.4;
    word-break: break-word;
  }

  .notif-flyout-actions {
    display: flex;
    gap: var(--space-2);
    margin-top: 4px;
  }

  .notif-flyout-actions .status-action {
    color: var(--accent-text);
    border-color: var(--accent);
  }

  @keyframes notif-flash {
    0% { opacity: 0; transform: translateX(-8px); }
    100% { opacity: 1; transform: translateX(0); }
  }

  @keyframes flyout-up {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  @media (prefers-reduced-motion: reduce) {
    .notif-inline { animation: none; }
    .notif-flyout { animation: none; }
    .notif-progress-fill { transition: none; }
  }
</style>
