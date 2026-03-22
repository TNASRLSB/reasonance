<script lang="ts">
  import { get } from 'svelte/store';
  import { llmConfigs, appSettings } from '$lib/stores/config';
  import { activeInstanceId } from '$lib/stores/terminals';
  import { callLlm } from '$lib/utils/llm-api';
  import type { Adapter } from '$lib/adapter';
  import { tr } from '$lib/i18n/index';
  import { menuKeyHandler } from '$lib/utils/a11y';

  interface Props {
    adapter: Adapter;
    x: number;
    y: number;
    visible: boolean;
    selectedText: string;
    onResponse: (content: string) => void;
    onClose: () => void;
  }

  const { adapter, x, y, visible, selectedText, onResponse, onClose }: Props = $props();

  let loading = $state(false);
  let contextMenuEl = $state<HTMLElement | null>(null);

  $effect(() => {
    if (visible && contextMenuEl) {
      const first = contextMenuEl.querySelector<HTMLElement>('[role="menuitem"]');
      first?.focus();
    }
  });

  const actions = [
    { key: 'contextMenu.explain', promptPrefix: 'Explain this code clearly and concisely:\n\n```\n' },
    { key: 'contextMenu.rewrite', promptPrefix: 'Rewrite the following code improving readability and best practices:\n\n```\n' },
    { key: 'contextMenu.findBugs', promptPrefix: 'Analyze the following code and find any bugs, issues, or vulnerabilities:\n\n```\n' },
    { key: 'contextMenu.document', promptPrefix: 'Add documentation (JSDoc/docstring comments) to the following code:\n\n```\n' },
  ];

  function getApiLlm() {
    const configs = get(llmConfigs);
    const settings = get(appSettings);

    // Prefer contextMenuLlm if set
    if (settings.contextMenuLlm) {
      const preferred = configs.find((c) => c.name === settings.contextMenuLlm && c.type === 'api');
      if (preferred) return preferred;
    }

    // Fall back to first API-type LLM
    return configs.find((c) => c.type === 'api') ?? null;
  }

  function getCliLlm() {
    const configs = get(llmConfigs);
    return configs.find((c) => c.type === 'cli') ?? null;
  }

  function hasAnyLlm() {
    const configs = get(llmConfigs);
    return configs.length > 0;
  }

  async function handleAction(promptPrefix: string) {
    if (!selectedText.trim() || loading) return;

    onClose();

    const apiLlm = getApiLlm();

    if (apiLlm) {
      loading = true;
      const prompt = `${promptPrefix}${selectedText}\n\`\`\``;
      const result = await callLlm(apiLlm, prompt);
      loading = false;

      if (result.error) {
        onResponse(`${get(tr)('contextMenu.error')} ${result.error}`);
      } else {
        onResponse(result.content);
      }
      return;
    }

    // Fall back to CLI LLM — paste prompt into active terminal
    const cliLlm = getCliLlm();
    if (cliLlm) {
      const instanceId = get(activeInstanceId);
      if (instanceId) {
        const prompt = `${promptPrefix}${selectedText}\n\`\`\`\n`;
        await adapter.writePty(instanceId, prompt);
      }
      return;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

{#if visible}
  <div
    class="context-menu-backdrop"
    role="presentation"
    onclick={onClose}
    onkeydown={handleKeydown}
  ></div>
  <div
    class="context-menu"
    style="left: {x}px; top: {y}px;"
    role="menu"
    bind:this={contextMenuEl}
    onkeydown={(e) => { menuKeyHandler(e, contextMenuEl!, '[role="menuitem"]'); if (e.key === 'Escape') onClose(); }}
  >
    {#each actions as action}
      {@const disabled = !hasAnyLlm() || !selectedText.trim() || loading}
      <button
        class="context-menu-item"
        class:disabled
        {disabled}
        role="menuitem"
        tabindex="-1"
        title={!hasAnyLlm() ? 'Configure an LLM in Settings' : ''}
        onclick={() => handleAction(action.promptPrefix)}
      >
        {$tr(action.key)}
        {#if loading}
          <span class="spinner"></span>
        {/if}
      </button>
    {/each}

    {#if !hasAnyLlm()}
      <div class="context-menu-hint">Configure an LLM in Settings</div>
    {/if}
  </div>
{/if}

<style>
  .context-menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 999;
  }

  .context-menu {
    position: fixed;
    z-index: 1000;
    background: var(--bg-secondary, #1e293b);
    border: var(--border-width) solid var(--border, #334155);
    border-radius: var(--radius);
    padding: 4px 0;
    min-width: 200px;
    max-width: 280px;
    font-size: 13px;
    font-family: var(--font-ui);
  }

  .context-menu-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 7px 14px;
    background: none;
    border: none;
    color: var(--text-primary, #e2e8f0);
    cursor: pointer;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    gap: 8px;
    transition: background 0.1s;
  }

  .context-menu-item:hover:not(.disabled) {
    background: var(--bg-hover);
    color: var(--accent);
  }

  .context-menu-item.disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .context-menu-hint {
    padding: 6px 14px;
    font-size: 11px;
    color: var(--text-secondary, #94a3b8);
    border-top: 1px solid var(--border, #334155);
    margin-top: 4px;
    padding-top: 8px;
  }

  .spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
