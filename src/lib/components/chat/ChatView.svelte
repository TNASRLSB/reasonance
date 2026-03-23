<script lang="ts">
  import type { Adapter } from '$lib/adapter/index';
  import type { AgentEvent } from '$lib/types/agent-event';
  import ChatHeader from './ChatHeader.svelte';
  import ChatMessages from './ChatMessages.svelte';
  import ChatInput from './ChatInput.svelte';
  import { agentEvents, streamingSessionIds, setSessionEvents, setStreaming } from '$lib/stores/agent-events';
  import { agentSessions, upsertSession } from '$lib/stores/agent-session';
  import { projectRoot } from '$lib/stores/files';
  import { yoloMode } from '$lib/stores/ui';
  import { get } from 'svelte/store';
  import { MODEL_INFO } from '$lib/data/model-info';

  let { adapter, sessionId, provider, model }: {
    adapter: Adapter;
    sessionId: string;
    provider: string;
    model: string;
  } = $props();

  let events = $derived(($agentEvents).get(sessionId) ?? []);
  let streaming = $derived(($streamingSessionIds).has(sessionId));
  let session = $derived(($agentSessions).get(sessionId));

  let tokenCount = $derived(
    (session?.totalInputTokens ?? 0) + (session?.totalOutputTokens ?? 0)
  );
  let currentSpeed = $derived(session?.currentSpeed ?? 0);
  let elapsed = $derived(session?.elapsed ?? 0);
  let status = $derived(session?.status ?? 'active');

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
      const cwd = get(projectRoot) || undefined;
      const isYolo = get(yoloMode);
      await adapter.agentSend(text, provider, model, sessionId, cwd, isYolo);
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
</script>

<div class="chat-view">
  <ChatHeader {provider} {model} {status} {streaming} {tokenCount} {currentSpeed} {elapsed} />
  <ChatMessages {events} {streaming} {adapter} onFork={handleFork} />
  <ChatInput
    onSend={handleSend}
    disabled={streaming}
    {model}
    contextPercent={contextPercent()}
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
</style>
