<script lang="ts">
  // @ts-ignore
  import { Svelvet, Node, Anchor, Edge } from 'svelvet';
  import type { Adapter } from '$lib/adapter/index';
  import type { WorkflowNode, WorkflowEdge } from '$lib/adapter/index';
  import { currentWorkflow, currentWorkflowPath, workflowDirty } from '$lib/stores/workflow';
  import { nodeStates } from '$lib/stores/engine';
  import { selectedNodeId, showSwarmCanvas, swarmViewMode } from '$lib/stores/ui';
  import SwarmControls from './SwarmControls.svelte';
  import SwarmInspector from './SwarmInspector.svelte';
  import NodeCatalog from './NodeCatalog.svelte';
  import AgentNode from './AgentNode.svelte';
  import ResourceNode from './ResourceNode.svelte';
  import LogicNode from './LogicNode.svelte';
  import { get } from 'svelte/store';
  import { onDestroy } from 'svelte';
  import type { NodeRunState } from '$lib/adapter/index';

  let { adapter, cwd = '.' }: { adapter: Adapter; cwd?: string } = $props();

  let wfNodes = $state<WorkflowNode[]>([]);
  let wfEdges = $state<WorkflowEdge[]>([]);
  let runNodeStates = $state<NodeRunState[]>([]);
  let selId = $state<string | null>(null);
  let viewMode = $state<'visual' | 'code' | 'split'>('visual');

  const unsubWf = currentWorkflow.subscribe((wf) => {
    wfNodes = wf?.nodes ?? [];
    wfEdges = wf?.edges ?? [];
  });
  const unsubNs = nodeStates.subscribe((val) => { runNodeStates = val; });
  const unsubSel = selectedNodeId.subscribe((val) => { selId = val; });
  const unsubMode = swarmViewMode.subscribe((val) => { viewMode = val; });

  function getNodeState(nodeId: string): string {
    const ns = runNodeStates.find((s) => s.node_id === nodeId);
    return ns?.state ?? 'idle';
  }

  function selectNode(id: string) {
    selectedNodeId.set(id);
  }

  function closeCanvas() {
    showSwarmCanvas.set(false);
  }

  function addNode(type: 'agent' | 'resource' | 'logic') {
    const wf = get(currentWorkflow);
    if (!wf) return;

    const id = `${type}-${Date.now()}`;
    const newNode: WorkflowNode = {
      id,
      type,
      label: `New ${type.charAt(0).toUpperCase() + type.slice(1)}`,
      config: type === 'agent' ? { llm: 'claude' } : type === 'resource' ? { kind: 'folder', path: '' } : { kind: 'condition', rule: '' },
      position: { x: 200 + Math.random() * 200, y: 100 + Math.random() * 200 },
    };

    const updated = { ...wf, nodes: [...wf.nodes, newNode] };
    currentWorkflow.set(updated);
    workflowDirty.set(true);
    selectedNodeId.set(id);
  }

  // NOTE: Code view is read-only in Phase 3. Bidirectional JSON↔Canvas sync deferred to Phase 4.
  let jsonText = $state('');
  const unsubJson = currentWorkflow.subscribe((wf) => {
    if (wf) jsonText = JSON.stringify(wf, null, 2);
  });

  onDestroy(() => { unsubWf(); unsubNs(); unsubSel(); unsubMode(); unsubJson(); });
</script>

<div class="swarm-canvas-root">
  <!-- Toolbar -->
  <div class="canvas-toolbar">
    <NodeCatalog onadd={addNode} />
    <div class="toolbar-spacer"></div>
    <SwarmControls {adapter} {cwd} />
    <div class="toolbar-spacer"></div>
    <div class="view-modes">
      <button class:active={viewMode === 'visual'} onclick={() => swarmViewMode.set('visual')}>Visual</button>
      <button class:active={viewMode === 'code'} onclick={() => swarmViewMode.set('code')}>Code</button>
      <button class:active={viewMode === 'split'} onclick={() => swarmViewMode.set('split')}>Split</button>
    </div>
    <button class="close-btn" onclick={closeCanvas} title="Close canvas">&#x2715;</button>
  </div>

  <!-- Content area -->
  <div class="canvas-content">
    {#if viewMode === 'visual' || viewMode === 'split'}
      <div class="canvas-area" class:half={viewMode === 'split'}>
        <Svelvet fitView minimap>
          {#each wfNodes as node (node.id)}
            <Node id={node.id} position={{ x: node.position.x, y: node.position.y }}>
              {#if node.type === 'agent'}
                <AgentNode
                  id={node.id}
                  label={node.label}
                  llm={node.config?.llm as string ?? ''}
                  state={getNodeState(node.id)}
                  selected={selId === node.id}
                  onselect={selectNode}
                />
              {:else if node.type === 'resource'}
                <ResourceNode
                  id={node.id}
                  label={node.label}
                  kind={node.config?.kind as string ?? 'folder'}
                  path={node.config?.path as string ?? ''}
                  selected={selId === node.id}
                  onselect={selectNode}
                />
              {:else if node.type === 'logic'}
                <LogicNode
                  id={node.id}
                  label={node.label}
                  kind={node.config?.kind as string ?? 'condition'}
                  rule={node.config?.rule as string ?? ''}
                  state={getNodeState(node.id)}
                  selected={selId === node.id}
                  onselect={selectNode}
                />
              {/if}
              <Anchor />
            </Node>
          {/each}
          {#each wfEdges as edge (edge.id)}
            <Edge source={edge.from} target={edge.to} label={edge.label ?? ''} />
          {/each}
        </Svelvet>
      </div>
    {/if}

    {#if viewMode === 'code' || viewMode === 'split'}
      <div class="code-area" class:half={viewMode === 'split'}>
        <textarea class="json-editor" bind:value={jsonText} spellcheck="false" readonly></textarea>
      </div>
    {/if}

    <!-- Inspector (right side) -->
    <div class="inspector-area">
      <SwarmInspector />
    </div>
  </div>
</div>

<style>
  .swarm-canvas-root {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }
  .canvas-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: var(--bg-secondary);
    border-bottom: var(--border-width) solid var(--border);
  }
  .toolbar-spacer {
    flex: 1;
  }
  .view-modes {
    display: flex;
    gap: 0;
  }
  .view-modes button {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    padding: 3px 10px;
    font-size: 11px;
    cursor: pointer;
    font-family: var(--font-ui);
  }
  .view-modes button.active {
    background: var(--accent);
    color: var(--text-primary);
  }
  .close-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    padding: 3px 8px;
    cursor: pointer;
    font-size: 14px;
  }
  .close-btn:hover {
    color: var(--danger);
  }
  .canvas-content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }
  .canvas-area {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
  .canvas-area.half {
    flex: 0.5;
  }
  .code-area {
    flex: 1;
    overflow: hidden;
  }
  .code-area.half {
    flex: 0.5;
    border-left: var(--border-width) solid var(--border);
  }
  .json-editor {
    width: 100%;
    height: 100%;
    background: var(--bg-primary);
    color: var(--text-body);
    border: none;
    padding: 12px;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    resize: none;
  }
  .inspector-area {
    width: 240px;
    min-width: 200px;
    border-left: var(--border-width) solid var(--border);
    background: var(--bg-secondary);
    overflow-y: auto;
  }
</style>
