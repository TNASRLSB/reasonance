<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import AgentNode from './AgentNode.svelte';
  import { selectedNodeId } from '$lib/stores/ui';

  let { data }: { data: any } = $props();
</script>

<Handle type="target" position={Position.Top} />
<AgentNode
  id={data.id ?? ''}
  label={data.label ?? 'Agent'}
  llm={data.config?.llm ?? ''}
  nodeState={data.state ?? 'idle'}
  selected={data.selected ?? false}
  onselect={(id) => selectedNodeId.set(id)}
/>
<Handle type="source" position={Position.Bottom} />
