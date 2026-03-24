<script lang="ts">
  import type { Adapter } from '$lib/adapter/index';
  import type { AgentEvent } from '$lib/types/agent-event';
  import ChatMessages from './ChatMessages.svelte';
  import ChatInput from './ChatInput.svelte';
  import { agentEvents, streamingSessionIds, setSessionEvents, setStreaming } from '$lib/stores/agent-events';
  import { agentSessions, upsertSession, incrementTurnCount } from '$lib/stores/agent-session';
  import { projectRoot } from '$lib/stores/files';
  import { llmConfigs } from '$lib/stores/config';
  import { get } from 'svelte/store';
  import { MODEL_INFO } from '$lib/data/model-info';
  import { showToast } from '$lib/stores/toast';
  import { t, tr } from '$lib/i18n/index';
  import { workspaceTrustLevel } from '$lib/stores/workspace-trust';

  let { adapter, sessionId, provider, model, configName }: {
    adapter: Adapter;
    sessionId: string;
    provider: string;
    model: string;
    configName: string;
  } = $props();

  // Per-session approved tools (not persisted — only for this session)
  let sessionApprovedTools = $state<Set<string>>(new Set());

  // Session-local permission override (not persisted)
  let sessionPermissionOverride = $state<'yolo' | 'ask' | 'locked' | null>(null);

  // Resolve per-model permission level from config (session override takes precedence)
  let modelConfig = $derived($llmConfigs.find((c) => c.name === configName));
  let permissionLevel = $derived(sessionPermissionOverride ?? modelConfig?.permissionLevel ?? 'yolo');
  let configAllowedTools = $derived(modelConfig?.allowedTools ?? []);

  let trustSuspended = $derived($workspaceTrustLevel === 'blocked' || $workspaceTrustLevel === null);

  let events = $derived(($agentEvents).get(sessionId) ?? []);
  let streaming = $derived(($streamingSessionIds).has(sessionId));
  let session = $derived(($agentSessions).get(sessionId));

  let tokenCount = $derived(
    (session?.totalInputTokens ?? 0) + (session?.totalOutputTokens ?? 0)
  );
  let currentSpeed = $derived(session?.currentSpeed ?? 0);
  let elapsed = $derived(session?.elapsed ?? 0);
  let status = $derived(session?.status ?? 'active');
  let turnCount = $derived(session?.turnCount ?? 0);

  // Context window usage: total tokens / model's context window
  let contextPercent = $derived(() => {
    if (tokenCount === 0) return null;
    // Match by provider since the model prop may not be a precise model ID
    const info = MODEL_INFO.find((m) => m.provider === provider.toLowerCase());
    if (!info) return null;
    return Math.min(100, Math.round((tokenCount / info.context_window) * 100));
  });

  // Ensure session exists in the store so token tracking works
  $effect(() => {
    const existing = get(agentSessions).get(sessionId);
    if (!existing) {
      upsertSession({
        id: sessionId,
        provider,
        model,
        status: 'active',
        viewMode: 'chat',
        cliMode: 'structured',
        title: '',
        totalInputTokens: 0,
        totalOutputTokens: 0,
        currentSpeed: 0,
        elapsed: 0,
        turnCount: 0,
      });
    }
  });

  // Load existing events on mount
  $effect(() => {
    adapter.sessionGetEvents(sessionId).then((loaded) => {
      if (loaded.length > 0) {
        setSessionEvents(sessionId, loaded);
      }
    }).catch((e) => console.warn('Failed to load events:', e));
  });

  function handlePermissionChange(level: 'yolo' | 'ask' | 'locked') {
    sessionPermissionOverride = level;
    const msgKey = level === 'yolo' ? 'permission.switchedAuto'
      : level === 'locked' ? 'permission.switchedLocked'
      : 'permission.switchedConfirm';
    showToast('info', t(msgKey));
  }

  async function handleSend(text: string) {
    try {
      // Append a synthetic user message so it appears in the chat
      const userEvent: AgentEvent = {
        id: crypto.randomUUID(),
        parent_id: null,
        event_type: 'text',
        content: { type: 'text', value: text },
        timestamp: Date.now(),
        metadata: {
          session_id: sessionId,
          input_tokens: null,
          output_tokens: null,
          tool_name: null,
          model: null,
          provider: 'user',
          error_severity: null,
          error_code: null,
          stream_metrics: null,
        },
      };
      agentEvents.update((map) => {
        const next = new Map(map);
        const existing = next.get(sessionId) ?? [];
        next.set(sessionId, [...existing, userEvent]);
        return next;
      });

      setStreaming(sessionId, true);
      incrementTurnCount(sessionId);
      const cwd = get(projectRoot) || undefined;
      const isYolo = permissionLevel === 'yolo';
      const mergedTools = [...configAllowedTools, ...sessionApprovedTools];
      const tools = mergedTools.length > 0 ? mergedTools : undefined;
      await adapter.agentSend(text, provider, model, sessionId, cwd, isYolo, tools);
    } catch (e) {
      console.error('Failed to send message:', e);
      setStreaming(sessionId, false);
    }
  }

  async function handleFork(eventIndex: number) {
    try {
      const forkedId = await adapter.sessionFork(sessionId, eventIndex);
      console.log('Session forked:', forkedId);
      // Phase 6/7: navigate to forked session tab
    } catch (e) {
      console.error('Failed to fork session:', e);
    }
  }

  async function handleApproveTools(tools: string[], remember: boolean) {
    for (const t of tools) sessionApprovedTools.add(t);
    sessionApprovedTools = new Set(sessionApprovedTools);

    if (remember && modelConfig) {
      // Persist to config
      const existing = modelConfig.allowedTools ?? [];
      const merged = [...new Set([...existing, ...tools])];
      llmConfigs.update((list) =>
        list.map((c) => c.name === configName ? { ...c, allowedTools: merged } : c)
      );
    }

    // Replay: re-send last user message with updated allowed tools + new session ID
    const lastUserEvent = [...events].reverse().find(
      (e) => e.metadata.provider === 'user' && e.event_type === 'text'
    );
    if (!lastUserEvent || lastUserEvent.content.type !== 'text') return;

    const newSessionId = crypto.randomUUID();
    const cwd = get(projectRoot) || undefined;
    const mergedTools = [...(modelConfig?.allowedTools ?? []), ...sessionApprovedTools];

    try {
      setStreaming(sessionId, true);
      const isYolo = permissionLevel === 'yolo';
      await adapter.agentSend(lastUserEvent.content.value, provider, model, newSessionId, cwd, isYolo, mergedTools);
    } catch (e) {
      console.error('Replay failed:', e);
      setStreaming(sessionId, false);
    }
  }
</script>

<div class="chat-view">
  <ChatMessages {events} {streaming} {adapter} onFork={handleFork} {permissionLevel} onApproveTools={handleApproveTools} />
  {#if trustSuspended}
    <div class="trust-suspended-banner">
      {$tr('trust.revokedBanner')}
    </div>
  {/if}
  <ChatInput
    onSend={handleSend}
    disabled={streaming || trustSuspended}
    contextPercent={contextPercent()}
    {turnCount}
    {currentSpeed}
    {elapsed}
    {streaming}
    {provider}
    {permissionLevel}
    onPermissionChange={handlePermissionChange}
  />
</div>

<style>
  .chat-view {
    display: flex;
    flex-direction: column;
    flex: 1;
    height: 100%;
    min-height: 0;
    background: var(--bg-surface);
    overflow: hidden;
  }

  .trust-suspended-banner {
    padding: var(--space-2) var(--space-3);
    background: var(--bg-secondary);
    border-top: 2px solid var(--border);
    font-size: var(--font-size-small);
    font-weight: 700;
    color: var(--text-muted);
    text-align: center;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
</style>
