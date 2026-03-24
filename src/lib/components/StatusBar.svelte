<script lang="ts">
  import { tr } from '$lib/i18n/index';
  import { activeFilePath, cursorLine, cursorCol } from '$lib/stores/files';
  import { llmConfigs } from '$lib/stores/config';
  import { activeInstance } from '$lib/stores/terminals';
  import { updateState } from '$lib/stores/update';
  import { installUpdate, postponeUpdate } from '$lib/updater';

  let activeInstanceData = $derived($activeInstance);
  let installing = $state(false);

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
</script>

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
          <span class="progress-status">&#9654; {activeInstanceData.progressValue}%</span>
        {:else if activeInstanceData.progressState === 2}
          <span class="progress-status error">&#10007; error</span>
        {:else if activeInstanceData.progressState === 3}
          <span class="progress-status">&#8943; working</span>
        {:else if activeInstanceData.progressState === 4}
          <span class="progress-status paused">&#10074;&#10074; paused</span>
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
  }

  .status-left,
  .status-center,
  .status-right {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .status-left {
    flex-shrink: 0;
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

  .llm-count {
    opacity: 0.85;
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
  }

  .status-action:hover {
    opacity: 1;
    background: rgba(255, 255, 255, 0.15);
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
</style>
