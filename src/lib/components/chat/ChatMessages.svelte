<script lang="ts">
  import type { AgentEvent } from '$lib/types/agent-event';
  import type { Adapter } from '$lib/adapter/index';
  import ContentRenderer from './ContentRenderer.svelte';
  import ThinkingBlock from './ThinkingBlock.svelte';
  import ToolUseBlock from './ToolUseBlock.svelte';
  import ErrorBlock from './ErrorBlock.svelte';
  import StreamingIndicator from './StreamingIndicator.svelte';
  import ActionableMessage from './ActionableMessage.svelte';

  let { events = [], streaming = false, adapter, onFork }: {
    events: AgentEvent[];
    streaming: boolean;
    adapter?: Adapter;
    onFork?: (eventIndex: number) => void;
  } = $props();

  let messagesEl: HTMLElement | undefined = $state();

  // Auto-scroll to bottom when new events arrive
  $effect(() => {
    if (events.length && messagesEl) {
      const { scrollTop, scrollHeight, clientHeight } = messagesEl;
      const nearBottom = scrollHeight - scrollTop - clientHeight < 100;
      if (nearBottom) {
        requestAnimationFrame(() => {
          messagesEl?.scrollTo({ top: messagesEl.scrollHeight, behavior: 'smooth' });
        });
      }
    }
  });

  // Build a map of tool_use ID → tool_result for linking
  let toolResults = $derived.by(() => {
    const map = new Map<string, AgentEvent>();
    for (const event of events) {
      if (event.event_type === 'tool_result' && event.parent_id) {
        map.set(event.parent_id, event);
      }
    }
    return map;
  });

  // Group consecutive events from same role into message groups
  interface MessageGroup {
    role: 'user' | 'agent';
    events: AgentEvent[];
    lastEventIndex: number;
  }

  let messageGroups = $derived.by(() => {
    const groups: MessageGroup[] = [];

    for (let i = 0; i < events.length; i++) {
      const event = events[i];
      // Skip non-renderable events
      if (event.event_type === 'usage' || event.event_type === 'metrics'
          || event.event_type === 'status' || event.event_type === 'done'
          || event.event_type === 'tool_result') {
        continue;
      }

      const role: 'user' | 'agent' = event.metadata.provider === 'user' ? 'user' : 'agent';

      const lastGroup = groups[groups.length - 1];
      if (lastGroup && lastGroup.role === role) {
        lastGroup.events.push(event);
        lastGroup.lastEventIndex = i;
      } else {
        groups.push({ role, events: [event], lastEventIndex: i });
      }
    }

    return groups;
  });
</script>

<div class="chat-messages" role="log" aria-live="polite" bind:this={messagesEl}>
  {#each messageGroups as group, gi (gi)}
    <ActionableMessage events={group.events} role={group.role} forkIndex={group.lastEventIndex} {onFork}>
      {#snippet children()}
        <div class="message-group {group.role}" role="article" tabindex="0">
          <div class="message-role">{group.role === 'agent' ? 'AGENT' : 'YOU'}</div>
          <div class="message-content">
            {#each group.events as event (event.id)}
              {#if event.event_type === 'thinking'}
                <ThinkingBlock text={event.content.type === 'text' ? event.content.value : ''} />
              {:else if event.event_type === 'tool_use'}
                <ToolUseBlock {event} result={toolResults.get(event.id)} />
              {:else if event.event_type === 'error'}
                <ErrorBlock
                  message={event.content.type === 'text' ? event.content.value : 'Unknown error'}
                  severity={event.metadata.error_severity ?? 'fatal'}
                  code={event.metadata.error_code ?? ''}
                />
              {:else}
                <ContentRenderer {event} {adapter} />
              {/if}
            {/each}
          </div>
        </div>
      {/snippet}
    </ActionableMessage>
  {/each}

  {#if streaming}
    <StreamingIndicator />
  {/if}
</div>

<style>
  .chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .message-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .message-role {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
  }

  .message-content {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .message-group.agent .message-content {
    padding-left: 0;
  }
</style>
