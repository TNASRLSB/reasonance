<script lang="ts">
  import type { AgentEvent } from '$lib/types/agent-event';
  import type { Adapter } from '$lib/adapter/index';
  import TextBlock from './TextBlock.svelte';
  import CodeBlock from './CodeBlock.svelte';
  import DiffBlock from './DiffBlock.svelte';
  import FileRefBadge from './FileRefBadge.svelte';

  let { event, adapter }: { event: AgentEvent; adapter?: Adapter } = $props();
</script>

{#if event.content.type === 'text'}
  <TextBlock text={event.content.value} />
{:else if event.content.type === 'code'}
  <CodeBlock language={event.content.language} source={event.content.source} />
{:else if event.content.type === 'diff'}
  <DiffBlock filePath={event.content.file_path} hunks={event.content.hunks} {adapter} />
{:else if event.content.type === 'file_ref'}
  <FileRefBadge path={event.content.path} action={event.content.action} />
{:else if event.content.type === 'json'}
  <CodeBlock language="json" source={JSON.stringify(event.content.value, null, 2)} />
{/if}
