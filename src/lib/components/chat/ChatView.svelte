<script lang="ts">
  import type { Adapter } from '$lib/adapter/index';
  import type { AgentEvent } from '$lib/types/agent-event';
  import ChatMessages from './ChatMessages.svelte';
  import ChatInput from './ChatInput.svelte';
  import { agentEvents, streamingSessionIds, processAgentEvent, setSessionEvents, setStreaming } from '$lib/stores/agent-events';

  let { adapter, sessionId, provider, model }: {
    adapter: Adapter;
    sessionId: string;
    provider: string;
    model: string;
  } = $props();

  let events = $derived(($agentEvents).get(sessionId) ?? []);
  let streaming = $derived(($streamingSessionIds).has(sessionId));

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
      await adapter.agentSend(text, provider, model, sessionId);
    } catch (e) {
      console.error('Failed to send message:', e);
      setStreaming(sessionId, false);
    }
  }
</script>

<div class="chat-view">
  <ChatMessages {events} {streaming} />
  <ChatInput onSend={handleSend} disabled={streaming} />
</div>

<style>
  .chat-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-surface);
  }
</style>
