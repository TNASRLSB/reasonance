<script lang="ts">
  import { tr } from '$lib/i18n/index';
  import { yoloMode } from '$lib/stores/ui';
  import { activeInstanceId, terminalTabs } from '$lib/stores/terminals';
  import { activeFilePath } from '$lib/stores/files';
  import { llmConfigs } from '$lib/stores/config';
  import type { TerminalInstance, TerminalTab } from '$lib/stores/terminals';
  import { onDestroy } from 'svelte';

  let tabs = $state<TerminalTab[]>([]);
  let activeId = $state<string | null>(null);
  let activePath = $state<string | null>(null);
  let configCount = $state(0);

  const unsubTabs = terminalTabs.subscribe((v) => { tabs = v; });
  const unsubActive = activeInstanceId.subscribe((v) => { activeId = v; });
  const unsubPath = activeFilePath.subscribe((v) => { activePath = v; });
  const unsubConfigs = llmConfigs.subscribe((v) => { configCount = v.length; });

  onDestroy(() => { unsubTabs(); unsubActive(); unsubPath(); unsubConfigs(); });

  let activeInst = $derived.by(() => {
    for (const tab of tabs) {
      const inst = tab.instances.find((i: TerminalInstance) => i.id === activeId);
      if (inst) return inst;
    }
    return null;
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

<div class="status-bar" class:yolo={$yoloMode}>
  {#if $yoloMode}
    <span class="yolo-label">{$tr('status.yolo')}</span>
  {:else}
    <div class="status-left">
      <span class="app-name">REASONANCE</span>
      <span class="separator">|</span>
      <span class="llm-count">{$tr('status.llmDetected', { count: String(configCount) })}</span>
    </div>

    <div class="status-center">
      {#if activeInst}
        {#if activeInst.contextPercent != null}
          <span class="ctx-info" title="Context window usage">
            {$tr('status.session', { percent: String(activeInst.contextPercent) })}
            <span class="ctx-bar">{generateBar(activeInst.contextPercent)}</span>
          </span>
        {/if}
        {#if activeInst.modelName}
          <span class="model-name">{activeInst.modelName}</span>
        {/if}
        {#if activeInst.resetTimer}
          <span class="reset-timer">{$tr('status.resetIn', { time: activeInst.resetTimer })}</span>
        {/if}
        {#if activeInst.messagesLeft != null}
          <span class="msg-count">{$tr('status.messagesLeft', { count: String(activeInst.messagesLeft) })}</span>
        {/if}
      {/if}
    </div>

    <div class="status-right">
      {#if activePath}
        <span class="file-name">{activePath.split('/').pop()}</span>
        <span class="file-lang">{getLang(activePath)}</span>
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
    border-top: none;
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

  .ctx-info {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .ctx-bar {
    font-family: var(--font-mono);
    letter-spacing: 0.05em;
  }

  .model-name {
    opacity: 0.85;
    font-weight: 500;
  }

  .reset-timer {
    opacity: 0.85;
  }

  .msg-count {
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
</style>
