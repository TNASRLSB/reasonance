<script lang="ts">
  import { tr } from '$lib/i18n/index';
  import SlashMenu from './SlashMenu.svelte';
  import type { SlashCommand } from '$lib/stores/config';
  import { defaultSlashCommands } from '$lib/data/slash-commands';
  import type { ImageAttachment } from '$lib/adapter/index';
  import { showToast } from '$lib/stores/toast';
  import { appAnnouncer } from '$lib/utils/a11y-announcer';

  const MAX_IMAGES = 5;
  const MAX_IMAGE_BYTES = 5 * 1024 * 1024; // 5MB
  const SUPPORTED_TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp'];

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
    permissionLevel = 'ask',
    onPermissionChange = (_level: 'yolo' | 'ask' | 'locked') => {},
  }: {
    onSend: (text: string, images?: ImageAttachment[]) => void;
    disabled?: boolean;
    contextPercent?: number | null;
    resetTimer?: string | null;
    messagesLeft?: number | null;
    turnCount?: number;
    currentSpeed?: number;
    elapsed?: number;
    streaming?: boolean;
    provider?: string;
    permissionLevel?: 'yolo' | 'ask' | 'locked';
    onPermissionChange?: (level: 'yolo' | 'ask' | 'locked') => void;
  } = $props();

  let text = $state('');
  let sending = $state(false);
  let sendTimer: ReturnType<typeof setTimeout> | null = null;

  type AttachedImage = ImageAttachment & { id: string; size: number };
  const emptyImages: AttachedImage[] = [];
  let attachedImages = $state(emptyImages);
  let dragOver = $state(false);
  let fileInput: HTMLInputElement;

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

  function readFileAsAttachment(file: File): Promise<ImageAttachment & { id: string; size: number }> {
    return new Promise((resolve, reject) => {
      if (!SUPPORTED_TYPES.includes(file.type)) {
        reject(new Error(`Unsupported format: ${file.type}`));
        return;
      }
      if (file.size > MAX_IMAGE_BYTES) {
        reject(new Error('Image exceeds 5MB limit'));
        return;
      }
      const reader = new FileReader();
      reader.onload = () => {
        const dataUrl = reader.result as string;
        const base64 = dataUrl.split(',')[1] ?? '';
        resolve({
          id: crypto.randomUUID(),
          data: base64,
          mimeType: file.type,
          name: file.name || 'image',
          size: file.size,
        });
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
  }

  async function addImageFiles(files: File[]) {
    for (const file of files) {
      if (attachedImages.length >= MAX_IMAGES) {
        showToast('warning', `Maximum ${MAX_IMAGES} images per message`);
        break;
      }
      try {
        const attachment = await readFileAsAttachment(file);
        attachedImages = [...attachedImages, attachment];
        appAnnouncer.announce(`Image attached: ${attachment.name}`);
      } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : 'Failed to read image';
        showToast('error', msg);
      }
    }
  }

  function removeImage(id: string) {
    const removed = attachedImages.find(img => img.id === id);
    attachedImages = attachedImages.filter(img => img.id !== id);
    if (removed) {
      appAnnouncer.announce(`Image removed: ${removed.name}`);
    }
  }

  async function handlePaste(e: ClipboardEvent) {
    // Try ClipboardEvent.clipboardData first (works in Chromium-based webviews)
    const items = Array.from(e.clipboardData?.items ?? []);
    const imageFiles = items
      .filter(item => item.kind === 'file' && item.type.startsWith('image/'))
      .map(item => item.getAsFile())
      .filter((f): f is File => f !== null);
    if (imageFiles.length > 0) {
      e.preventDefault();
      addImageFiles(imageFiles);
      return;
    }

    // Fallback: navigator.clipboard.read() — returns original PNG/JPEG blob
    try {
      const clipItems = await navigator.clipboard.read();
      for (const item of clipItems) {
        const imageType = item.types.find(t => t.startsWith('image/'));
        if (imageType) {
          e.preventDefault();
          const blob = await item.getType(imageType);
          const file = new File([blob], 'clipboard-image', { type: imageType });
          await addImageFiles([file]);
          return;
        }
      }
    } catch {
      // navigator.clipboard.read() not available or denied — try Tauri plugin
    }

    // Last resort: Tauri clipboard plugin (RGBA pixels → Canvas → PNG)
    try {
      const { readImage } = await import('@tauri-apps/plugin-clipboard-manager');
      const img = await readImage();
      if (img) {
        const rgba = await img.rgba();
        const imgSize = await img.size();
        if (rgba && rgba.length > 0 && imgSize.width > 0 && imgSize.height > 0) {
          e.preventDefault();
          const canvas = document.createElement('canvas');
          canvas.width = imgSize.width;
          canvas.height = imgSize.height;
          const ctx = canvas.getContext('2d');
          if (ctx) {
            const imageData = new ImageData(
              new Uint8ClampedArray(rgba),
              imgSize.width,
              imgSize.height,
            );
            ctx.putImageData(imageData, 0, 0);
            const dataUrl = canvas.toDataURL('image/png');
            const base64 = dataUrl.split(',')[1] ?? '';
            if (base64) {
              const byteSize = Math.round((base64.length * 3) / 4);
              if (byteSize > MAX_IMAGE_BYTES) {
                showToast('error', 'Image exceeds 5MB limit');
                return;
              }
              const attachment: AttachedImage = {
                id: crypto.randomUUID(),
                data: base64,
                mimeType: 'image/png',
                name: 'clipboard-image',
                size: byteSize,
              };
              attachedImages = [...attachedImages, attachment];
              appAnnouncer.announce('Image attached: clipboard-image');
            }
          }
        }
      }
    } catch {
      // No image in clipboard — let normal paste happen
    }
  }

  function handleDragOver(e: DragEvent) {
    if (!e.dataTransfer) return;
    const hasImage = Array.from(e.dataTransfer.items).some(
      item => item.kind === 'file' && item.type.startsWith('image/')
    );
    if (!hasImage) return;
    e.preventDefault();
    e.stopPropagation();
    dragOver = true;
  }

  function handleDragLeave() { dragOver = false; }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    dragOver = false;
    const files = Array.from(e.dataTransfer?.files ?? []).filter(f => f.type.startsWith('image/'));
    if (files.length > 0) addImageFiles(files);
  }

  function handleFilePick(e: Event) {
    const input = e.target as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    if (files.length > 0) addImageFiles(files);
    input.value = '';
  }

  function handleSubmit() {
    const trimmed = text.trim();
    if ((!trimmed && attachedImages.length === 0) || disabled || sending) return;
    sending = true;
    if (sendTimer) clearTimeout(sendTimer);
    sendTimer = setTimeout(() => { sending = false; }, 300);
    const images = attachedImages.length > 0
      ? attachedImages.map(({ data, mimeType, name }) => ({ data, mimeType, name }))
      : undefined;
    onSend(trimmed || '(image)', images);
    text = '';
    attachedImages = [];
  }

  function cyclePermission() {
    const order: Array<'yolo' | 'ask' | 'locked'> = ['yolo', 'ask', 'locked'];
    const idx = order.indexOf(permissionLevel);
    const next = order[(idx + 1) % order.length];
    onPermissionChange(next);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.altKey && e.key === 'p') {
      e.preventDefault();
      cyclePermission();
      return;
    }
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

<div
  class="chat-input-wrapper"
  class:drag-over={dragOver}
  role="region"
  aria-label="Chat input area"
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
>
  {#if dragOver}
    <div class="drop-overlay" aria-hidden="true">Drop images here</div>
  {/if}
  {#if showSlashMenu}
    <SlashMenu
      commands={slashFiltered}
      activeIndex={slashActiveIndex}
      onSelect={handleSlashSelect}
      onClose={() => { text = ''; }}
    />
  {/if}
  {#if attachedImages.length > 0}
    <div class="image-strip" role="list" aria-label="Attached images">
      {#each attachedImages as img (img.id)}
        <div class="image-thumb" role="listitem" aria-label="{img.name}, {(img.size / 1024).toFixed(0)}KB">
          <img src="data:{img.mimeType};base64,{img.data}" alt={img.name} width="48" height="48" />
          <button
            class="remove-btn"
            onclick={() => removeImage(img.id)}
            aria-label="Remove {img.name}"
          >&times;</button>
        </div>
      {/each}
    </div>
  {/if}
  <div class="input-row">
    <button
      class="attach-btn"
      onclick={() => fileInput?.click()}
      disabled={disabled || attachedImages.length >= MAX_IMAGES}
      aria-label="Attach images"
      title="Attach images (Ctrl+V to paste)"
    >&#x1F4CE;</button>
    <input
      bind:this={fileInput}
      type="file"
      accept="image/png,image/jpeg,image/gif,image/webp"
      multiple
      onchange={handleFilePick}
      class="sr-only"
      tabindex={-1}
      aria-hidden="true"
    />
    <textarea
      bind:value={text}
      onkeydown={handleKeydown}
      onpaste={handlePaste}
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
      disabled={disabled || sending || (!text.trim() && attachedImages.length === 0)}
      aria-label="Send message"
      aria-busy={sending}
    >
      SEND
    </button>
  </div>

  <div class="input-footer">
    <div class="footer-left">
      <button
        class="permission-badge"
        class:auto={permissionLevel === 'yolo'}
        class:locked={permissionLevel === 'locked'}
        title={permissionLevel === 'yolo'
          ? $tr('permission.autoTooltip')
          : permissionLevel === 'locked'
            ? $tr('permission.lockedTooltip')
            : $tr('permission.confirmTooltip')}
        onclick={cyclePermission}
        aria-label="Permission mode: {permissionLevel === 'yolo' ? 'AUTO' : permissionLevel === 'locked' ? 'LOCKED' : 'CONFIRM'}. Click to change."
      >
        {#if permissionLevel === 'yolo'}
          &#x26A1; {$tr('permission.auto')}
        {:else if permissionLevel === 'locked'}
          &#x1F512; {$tr('permission.locked')}
        {:else}
          ? {$tr('permission.confirm')}
        {/if}
      </button>
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
    position: relative;
    display: flex;
    flex-direction: column;
    padding: var(--space-2) var(--space-4) var(--space-1);
    border-top: 2px solid var(--border);
    background: var(--bg-surface);
    flex-shrink: 0;
    gap: var(--interactive-gap);
  }

  .chat-input-wrapper.drag-over {
    outline: 2px dashed var(--accent);
    outline-offset: -2px;
  }

  .drop-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(124, 106, 247, 0.12);
    font-weight: 600;
    color: var(--accent);
    pointer-events: none;
    z-index: 10;
  }

  .image-strip {
    display: flex;
    gap: var(--space-1);
    padding: 0 var(--space-1);
    overflow-x: auto;
    max-height: 60px;
  }

  .image-thumb {
    position: relative;
    flex-shrink: 0;
    width: 48px;
    height: 48px;
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .image-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .remove-btn {
    position: absolute;
    top: -2px;
    right: -2px;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--text-error, #e55);
    color: white;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .remove-btn:focus-visible {
    outline: 2px solid var(--focus-ring, var(--accent));
    outline-offset: 1px;
  }

  .attach-btn {
    background: transparent;
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-1);
    cursor: pointer;
    font-size: var(--font-size-base);
    color: var(--text-muted);
    align-self: flex-end;
    min-height: 2.75rem;
    min-width: 2.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color var(--transition-fast), border-color var(--transition-fast);
  }

  .attach-btn:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
  }

  .attach-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
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

  .input-row {
    display: flex;
    align-items: flex-end;
    gap: var(--space-2);
  }

  textarea {
    flex: 1;
    resize: none;
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
    color: var(--text-body);
    background: var(--bg-primary);
    border: var(--border-width) solid var(--border);
    padding: var(--space-2) var(--space-3);
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
    background: var(--accent-btn);
    border: var(--border-width) solid var(--accent);
    padding: var(--btn-padding);
    min-height: 2.75rem;
    cursor: pointer;
    align-self: flex-end;
    transition: opacity var(--transition-fast);
  }

  .send-btn[aria-busy="true"] {
    cursor: wait;
    opacity: 0.8;
    pointer-events: none;
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
    padding: 0 var(--stack-tight);
    min-height: 20px;
  }

  .footer-left {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .footer-right {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .metrics {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    white-space: nowrap;
  }

  .permission-badge {
    font-family: var(--font-ui);
    font-size: var(--font-size-sm);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: var(--stack-tight) var(--space-1);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .permission-badge:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .permission-badge.auto {
    background: var(--accent-btn);
    border-color: var(--accent);
    color: var(--text-on-accent);
  }

  .permission-badge.locked {
    border-color: var(--text-muted);
    color: var(--text-secondary);
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

</style>
