<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import LogicNode from './LogicNode.svelte';
  import { selectedNodeId } from '$lib/stores/ui';

  let { data }: { data: any } = $props();
</script>

<Handle type="target" position={Position.Top} />
<LogicNode
  id={data.id ?? ''}
  label={data.label ?? 'Logic'}
  kind={data.config?.kind ?? 'condition'}
  rule={data.config?.rule ?? ''}
  nodeState={data.state ?? 'idle'}
  selected={data.selected ?? false}
  onselect={(id) => selectedNodeId.set(id)}
/>
<Handle type="source" position={Position.Bottom} />
