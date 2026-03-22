<script lang="ts">
  import type { AgentEvent } from '$lib/types/agent-event';
  import TextBlock from './TextBlock.svelte';
  import CodeBlock from './CodeBlock.svelte';

  let { event }: { event: AgentEvent } = $props();
</script>

{#if event.content.type === 'text'}
  <TextBlock text={event.content.value} />
{:else if event.content.type === 'code'}
  <CodeBlock language={event.content.language} source={event.content.source} />
{:else}
  <!-- Fallback for unhandled content types (Phase 5: diff, file_ref, json) -->
  <TextBlock text={JSON.stringify(event.content, null, 2)} />
{/if}
