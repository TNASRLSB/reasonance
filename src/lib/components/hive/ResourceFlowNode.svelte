<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import ResourceNode from './ResourceNode.svelte';
  import { selectedNodeId } from '$lib/stores/ui';

  let { data }: { data: any } = $props();
</script>

<Handle type="target" position={Position.Top} />
<ResourceNode
  id={data.id ?? ''}
  label={data.label ?? 'Resource'}
  kind={data.config?.kind ?? 'folder'}
  path={data.config?.path ?? ''}
  access={data.config?.access ?? 'read'}
  selected={data.selected ?? false}
  onselect={(id) => selectedNodeId.set(id)}
  onchange={(field, value) => data.onchange?.(data.id, field, value)}
/>
<Handle type="source" position={Position.Bottom} />
