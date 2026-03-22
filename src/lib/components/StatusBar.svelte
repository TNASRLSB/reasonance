<script lang="ts">
  import { tr } from '$lib/i18n/index';
  import { yoloMode } from '$lib/stores/ui';
  import { activeFilePath } from '$lib/stores/files';
  import { llmConfigs } from '$lib/stores/config';
  import { terminalTabs, activeTerminalTab, activeInstanceId } from '$lib/stores/terminals';

  let activeInstanceData = $derived.by(() => {
    if (!$activeTerminalTab || !$activeInstanceId) return null;
    const tab = $terminalTabs.find(t => t.llmName === $activeTerminalTab);
    return tab?.instances.find(i => i.id === $activeInstanceId) ?? null;
  });

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

<div class="status-bar" class:yolo={$yoloMode} role="status">
  {#if $yoloMode}
    <span class="yolo-label">&#10005; YOLO MODE — CONFIRMATIONS DISABLED</span>
  {:else}
    <div class="status-left">
      <span class="app-name">REASONANCE</span>
      <span class="separator">|</span>
      <span class="llm-count">{$tr('status.llmDetected', { count: String($llmConfigs.length) })}</span>
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
        <span class="file-name">{$activeFilePath.split('/').pop()}</span>
        <span class="file-lang">{getLang($activeFilePath)}</span>
        <span class="file-encoding">UTF-8</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .status-bar {
    height: var(--statusbar-height);
    background: var(--accent);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 14px;
    flex-shrink: 0;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 600;
    user-select: none;
    border-top: 2px solid var(--border);
  }

  .status-bar.yolo {
    background: var(--danger);
  }

  .yolo-label {
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    width: 100%;
    text-align: center;
  }

  .status-left,
  .status-center,
  .status-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .status-left {
    flex-shrink: 0;
  }

  .status-center {
    flex: 1;
    justify-content: center;
    gap: 16px;
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
    opacity: 0.5;
  }

  .llm-count {
    opacity: 0.85;
  }

  .file-name {
    font-weight: 500;
  }

  .file-lang {
    opacity: 0.7;
  }

  .file-encoding {
    opacity: 0.5;
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
    color: #fca5a5;
  }

  .progress-status.paused {
    color: #fef08a;
  }

  .idle-hint {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    opacity: 0.5;
    font-style: italic;
  }
</style>
