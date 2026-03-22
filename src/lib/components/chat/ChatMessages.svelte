<script lang="ts">
  import type { AgentEvent } from '$lib/types/agent-event';
  import ContentRenderer from './ContentRenderer.svelte';
  import StreamingIndicator from './StreamingIndicator.svelte';

  let { events = [], streaming = false }: { events: AgentEvent[]; streaming: boolean } = $props();

  let messagesEl: HTMLElement | undefined = $state();

  // Auto-scroll to bottom when new events arrive
  $effect(() => {
    if (events.length && messagesEl) {
      // Only scroll if user is near bottom (within 100px)
      const { scrollTop, scrollHeight, clientHeight } = messagesEl;
      const nearBottom = scrollHeight - scrollTop - clientHeight < 100;
      if (nearBottom) {
        requestAnimationFrame(() => {
          messagesEl?.scrollTo({ top: messagesEl.scrollHeight, behavior: 'smooth' });
        });
      }
    }
  });

  // Group consecutive text events from same source into messages
  interface MessageGroup {
    role: 'user' | 'agent';
    events: AgentEvent[];
  }

  let messageGroups = $derived.by(() => {
    const groups: MessageGroup[] = [];

    for (const event of events) {
      // Skip non-renderable events
      if (event.event_type === 'usage' || event.event_type === 'metrics' || event.event_type === 'status' || event.event_type === 'done') {
        continue;
      }

      // User messages have provider === 'user' (synthetic events from ChatView)
      const role: 'user' | 'agent' = event.metadata.provider === 'user' ? 'user' : 'agent';

      const lastGroup = groups[groups.length - 1];
      if (lastGroup && lastGroup.role === role) {
        lastGroup.events.push(event);
      } else {
        groups.push({ role, events: [event] });
      }
    }

    return groups;
  });
</script>

<div class="chat-messages" role="log" aria-live="polite" bind:this={messagesEl}>
  {#each messageGroups as group, gi (gi)}
    <div class="message-group {group.role}" role="article" tabindex="0">
      <div class="message-role">{group.role === 'agent' ? 'AGENT' : 'YOU'}</div>
      <div class="message-content">
        {#each group.events as event (event.id)}
          <ContentRenderer {event} />
        {/each}
      </div>
    </div>
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
