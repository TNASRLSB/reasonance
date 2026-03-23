<script lang="ts">
  let { onSend, disabled = false }: { onSend: (text: string) => void; disabled?: boolean } = $props();

  let text = $state('');
  let sending = $state(false);
  let sendTimer: ReturnType<typeof setTimeout> | null = null;

  function handleSubmit() {
    const trimmed = text.trim();
    if (!trimmed || disabled || sending) return;
    sending = true;
    // Debounce: prevent rapid double-sends within 300ms
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
</script>

<div class="chat-input">
  <textarea
    bind:value={text}
    onkeydown={handleKeydown}
    placeholder="Send a message..."
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

<style>
  .chat-input {
    display: flex;
    gap: 8px;
    padding: 12px 16px;
    border-top: var(--border-width) solid var(--border);
    background: var(--bg-surface);
    flex-shrink: 0;
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
  }

  .send-btn:hover:not(:disabled) {
    opacity: 0.85;
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
