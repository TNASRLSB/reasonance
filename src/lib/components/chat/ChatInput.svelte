<script lang="ts">
  import { get } from 'svelte/store';
  import { yoloMode } from '$lib/stores/ui';
  import { tr } from '$lib/i18n/index';
  import SlashMenu from './SlashMenu.svelte';
  import type { SlashCommand } from '$lib/stores/config';
  import { defaultSlashCommands } from '$lib/data/slash-commands';

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
    provider = '',
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
    provider?: string;
  } = $props();

  let text = $state('');
  let sending = $state(false);
  let sendTimer: ReturnType<typeof setTimeout> | null = null;

  let showSlashMenu = $derived(text.startsWith('/') && text.indexOf(' ') === -1);
  let slashFilter = $derived(text.startsWith('/') ? text.slice(1) : '');

  let slashCommands = $derived.by(() => {
    const cmd = provider.toLowerCase();
    return defaultSlashCommands[cmd] ?? defaultSlashCommands['claude'] ?? [];
  });

  let slashFiltered = $derived.by(() => {
    if (!slashFilter) return slashCommands;
    const lower = slashFilter.toLowerCase();
    return slashCommands.filter(c =>
      c.command.toLowerCase().includes(lower) ||
      c.description.toLowerCase().includes(lower)
    );
  });

  let slashActiveIndex = $state(0);

  $effect(() => {
    slashFilter;
    slashActiveIndex = 0;
  });

  async function handleSlashSelect(cmd: SlashCommand) {
    if (cmd.command === '/clear') {
      // WCAG 2.2 AAA 3.3.6 — confirm destructive actions
      const { ask } = await import('@tauri-apps/plugin-dialog');
      const ok = await ask('Clear the entire conversation? This cannot be undone.', {
        title: 'Reasonance',
        kind: 'warning',
      });
      if (!ok) return;
      onSend(cmd.command);
      text = '';
    } else if (['/fork', '/compact', '/export'].includes(cmd.command)) {
      onSend(cmd.command);
      text = '';
    } else {
      text = cmd.command + ' ';
    }
  }

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
    if (showSlashMenu && slashFiltered.length > 0) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        slashActiveIndex = Math.min(slashActiveIndex + 1, slashFiltered.length - 1);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        slashActiveIndex = Math.max(slashActiveIndex - 1, 0);
        return;
      }
      if (e.key === 'Enter') {
        e.preventDefault();
        handleSlashSelect(slashFiltered[slashActiveIndex]);
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        text = '';
        return;
      }
    }
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
  {#if showSlashMenu}
    <SlashMenu
      commands={slashFiltered}
      activeIndex={slashActiveIndex}
      onSelect={handleSlashSelect}
      onClose={() => { text = ''; }}
    />
  {/if}
  <div class="input-row">
    <textarea
      bind:value={text}
      onkeydown={handleKeydown}
      placeholder={$tr('chat.placeholder')}
      rows="1"
      {disabled}
      role={showSlashMenu ? 'combobox' : undefined}
      aria-autocomplete={showSlashMenu ? 'list' : undefined}
      aria-expanded={showSlashMenu}
      aria-controls={showSlashMenu ? 'slash-menu' : undefined}
      aria-activedescendant={showSlashMenu && slashFiltered.length > 0 ? `slash-cmd-${slashActiveIndex}` : undefined}
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
      <button
        class="yolo-toggle"
        class:active={$yoloMode}
        onclick={toggleYolo}
        title={$tr('toolbar.yoloTitle')}
        aria-pressed={$yoloMode}
        aria-describedby="yolo-desc"
      >
        {$yoloMode ? '\u26A1 YOLO' : 'YOLO'}
      </button>
      <span id="yolo-desc" class="sr-only">Auto-approve all tool executions without confirmation prompts</span>
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
    </div>
    <div class="footer-right">
      {#if contextPercent != null}
        <span class="stat" role="meter" aria-label="Context window usage: {contextPercent}%, {100 - contextPercent}% remaining" aria-valuenow={contextPercent} aria-valuemin={0} aria-valuemax={100}>
          Session: {contextPercent}%
          <span class="progress-bar" aria-hidden="true">{generateBar(contextPercent)}</span>
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
    color: var(--text-on-accent);
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
    font-size: var(--font-size-sm);
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
    font-size: var(--font-size-sm);
    letter-spacing: -0.5px;
    opacity: 0.7;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
