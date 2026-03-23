<script lang="ts">
  import { get } from 'svelte/store';
  import { yoloMode } from '$lib/stores/ui';
  import { tr } from '$lib/i18n/index';

  let {
    onSend,
    disabled = false,
    contextPercent = null,
    resetTimer = null,
    messagesLeft = null,
    turnCount = 0,
    currentSpeed = 0,
    elapsed = 0,
    streaming = false,
  }: {
    onSend: (text: string) => void;
    disabled?: boolean;
    contextPercent?: number | null;
    resetTimer?: string | null;
    messagesLeft?: number | null;
    turnCount?: number;
    currentSpeed?: number;
    elapsed?: number;
    streaming?: boolean;
  } = $props();

  let text = $state('');
  let sending = $state(false);
  let sendTimer: ReturnType<typeof setTimeout> | null = null;

  function handleSubmit() {
    const trimmed = text.trim();
    if (!trimmed || disabled || sending) return;
    sending = true;
    if (sendTimer) clearTimeout(sendTimer);
    sendTimer = setTimeout(() => { sending = false; }, 300);
    onSend(trimmed);
    text = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  async function toggleYolo() {
    const current = get(yoloMode);
    if (!current) {
      const { ask } = await import('@tauri-apps/plugin-dialog');
      const ok = await ask($tr('toolbar.yoloConfirm'), { title: 'Reasonance', kind: 'warning' });
      if (!ok) return;
    }
    yoloMode.update((v) => !v);
  }

  function generateBar(percent: number): string {
    const total = 20;
    const filled = Math.round(percent / (100 / total));
    const empty = total - filled;
    return '\u2588'.repeat(filled) + '\u2591'.repeat(empty);
  }

  let elapsedDisplay = $derived.by(() => {
    if (elapsed < 1000) return `${elapsed}ms`;
    return `${(elapsed / 1000).toFixed(1)}s`;
  });
</script>

<div class="chat-input-wrapper">
  <div class="input-row">
    <textarea
      bind:value={text}
      onkeydown={handleKeydown}
      placeholder={$tr('chat.placeholder')}
      rows="1"
      {disabled}
      aria-label="Message input"
    ></textarea>
    <button
      class="send-btn"
      onclick={handleSubmit}
      disabled={disabled || sending || !text.trim()}
      aria-label="Send message"
    >
      SEND
    </button>
  </div>

  <div class="input-footer">
    <div class="footer-left">
      {#if turnCount > 0 || streaming}
        <span class="metrics">
          Turn {turnCount}
          {#if streaming || currentSpeed > 0}
            &middot; {currentSpeed.toFixed(0)} tok/s
          {/if}
          {#if streaming && elapsed > 0}
            &middot; {elapsedDisplay}
          {/if}
        </span>
      {/if}
      <button
        class="yolo-toggle"
        class:active={$yoloMode}
        onclick={toggleYolo}
        title={$tr('toolbar.yoloTitle')}
        aria-pressed={$yoloMode}
      >
        {$yoloMode ? '\u26A1 YOLO' : 'YOLO'}
      </button>
    </div>
    <div class="footer-right">
      {#if contextPercent != null}
        <span class="stat">
          Session: {contextPercent}%
          <span class="progress-bar">{generateBar(contextPercent)}</span>
        </span>
      {/if}
      {#if resetTimer}
        <span class="stat">Reset in: {resetTimer}</span>
      {/if}
      {#if messagesLeft != null}
        <span class="stat">Messages left: {messagesLeft}</span>
      {/if}
    </div>
  </div>
</div>

<style>
  .chat-input-wrapper {
    display: flex;
    flex-direction: column;
    padding: 8px 16px 6px;
    border-top: 2px solid var(--border);
    background: var(--bg-surface);
    flex-shrink: 0;
    gap: 6px;
  }

  .input-row {
    display: flex;
    align-items: flex-end;
    gap: 8px;
  }

  textarea {
    flex: 1;
    resize: none;
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
    color: var(--text-body);
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    padding: 8px 12px;
    outline: none;
    min-height: 40px;
    max-height: 120px;
    line-height: 1.4;
  }

  textarea:focus {
    border-color: var(--accent);
  }

  textarea::placeholder {
    color: var(--text-muted);
  }

  .send-btn {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #fff;
    background: var(--accent);
    border: var(--border-width) solid var(--accent);
    padding: 8px 16px;
    cursor: pointer;
    align-self: flex-end;
    transition: opacity 0.1s;
  }

  .send-btn:hover:not(:disabled) {
    opacity: 0.85;
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .input-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 2px;
    min-height: 20px;
  }

  .footer-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .footer-right {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .metrics {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    white-space: nowrap;
  }

  .yolo-toggle {
    font-family: var(--font-ui);
    font-size: 10px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border: var(--border-width) solid var(--border);
    border-radius: 0;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
  }

  .yolo-toggle:hover {
    color: var(--text-primary);
    border-color: var(--text-secondary);
  }

  .yolo-toggle.active {
    background: var(--danger-dark);
    border-color: var(--danger);
    color: var(--text-primary);
  }

  .yolo-toggle.active:hover {
    background: var(--danger);
  }

  .stat {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    white-space: nowrap;
  }

  .progress-bar {
    font-size: 9px;
    letter-spacing: -0.5px;
    opacity: 0.7;
  }
</style>
