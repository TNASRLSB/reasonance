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

  // Track previous event count to detect new messages
  let prevEventCount = $state(0);

  // Auto-scroll to bottom when new events arrive
  $effect(() => {
    const count = events.length;
    if (count && messagesEl) {
      const isNewMessage = count > prevEventCount;
      const { scrollTop, scrollHeight, clientHeight } = messagesEl;
      const nearBottom = scrollHeight - scrollTop - clientHeight < 100;
      // Scroll if user is near bottom OR a new message just arrived
      if (nearBottom || isNewMessage) {
        requestAnimationFrame(() => {
          messagesEl?.scrollTo({ top: messagesEl.scrollHeight, behavior: 'smooth' });
        });
      }
      prevEventCount = count;
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

  const VISIBLE_GROUPS = 50;
  let showAll = $state(false);

  let allMessageGroups = $derived.by(() => {
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

  // Show only the last N groups unless user requests all
  let messageGroups = $derived(
    showAll || allMessageGroups.length <= VISIBLE_GROUPS
      ? allMessageGroups
      : allMessageGroups.slice(allMessageGroups.length - VISIBLE_GROUPS)
  );

  let hiddenCount = $derived(allMessageGroups.length - messageGroups.length);
</script>

<div class="chat-messages" role="log" aria-live="polite" bind:this={messagesEl}>
  {#if messageGroups.length === 0 && !streaming}
    <div class="empty-state">
      <p class="empty-title">Start a conversation</p>
      <p class="empty-hint">Send a message below to begin working with the AI agent.</p>
    </div>
  {/if}
  {#if hiddenCount > 0}
    <button class="load-more" onclick={() => showAll = true}>
      Show {hiddenCount} earlier message{hiddenCount !== 1 ? 's' : ''}
    </button>
  {/if}
  {#each messageGroups as group, gi (gi)}
    <ActionableMessage events={group.events} role={group.role} forkIndex={group.lastEventIndex} {onFork}>
      {#snippet children()}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
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
    min-height: 0;
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
    padding-inline-start: 0;
  }

  .load-more {
    align-self: center;
    padding: 6px 16px;
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 700;
    color: var(--accent-text);
    background: transparent;
    border: var(--border-width) solid var(--border);
    cursor: pointer;
  }

  .load-more:hover {
    background: var(--bg-hover);
    border-color: var(--accent);
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-muted);
    text-align: center;
  }

  .empty-title {
    font-family: var(--font-ui);
    font-size: var(--font-size-base);
    font-weight: 700;
    color: var(--text-secondary);
    margin: 0;
  }

  .empty-hint {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    color: var(--text-muted);
    margin: 0;
  }
</style>
