<script lang="ts">
  import {
    SvelteFlow,
    Controls,
    Background,
    MiniMap,
    type Node as FlowNode,
    type Edge as FlowEdge,
    type NodeTypes,
    useSvelteFlow,
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import { onMount } from 'svelte';
  import type { Connection } from '@xyflow/system';
  import type { Adapter } from '$lib/adapter/index';
  import type { WorkflowNode, WorkflowEdge } from '$lib/adapter/index';
  import { currentWorkflow, currentWorkflowPath, workflowDirty } from '$lib/stores/workflow';
  import { showToast } from '$lib/stores/toast';
  import { nodeStates, setupHiveEventListeners } from '$lib/stores/engine';
  import { selectedNodeId, showHiveCanvas, hiveViewMode } from '$lib/stores/ui';
  import HiveControls from './HiveControls.svelte';
  import HiveInspector from './HiveInspector.svelte';
  import NodeCatalog from './NodeCatalog.svelte';
  import WorkflowMenu from './WorkflowMenu.svelte';
  import AgentFlowNode from './AgentFlowNode.svelte';
  import ResourceFlowNode from './ResourceFlowNode.svelte';
  import LogicFlowNode from './LogicFlowNode.svelte';
  import { get } from 'svelte/store';
  import type { NodeRunState } from '$lib/adapter/index';
  import { tr } from '$lib/i18n/index';

  let { adapter, cwd = '.' }: { adapter: Adapter; cwd?: string } = $props();

  onMount(() => {
    setupHiveEventListeners();
    // Initialize empty workflow if none loaded
    if (!get(currentWorkflow)) {
      currentWorkflow.set({
        name: 'Untitled',
        version: '1.0.0',
        schemaVersion: 1,
        nodes: [],
        edges: [],
        settings: {
          max_concurrent_agents: 3,
          default_retry: 1,
          timeout: 300,
          permissionLevel: 'supervised'
        }
      });
    }
  });

  let wfNodes = $derived<WorkflowNode[]>($currentWorkflow?.nodes ?? []);
  let wfEdges = $derived<WorkflowEdge[]>($currentWorkflow?.edges ?? []);

  function getNodeState(nodeId: string): string {
    const ns = $nodeStates.find((s) => s.node_id === nodeId);
    return ns?.state ?? 'idle';
  }

  // Convert WorkflowNode[] → SvelteFlow Node[]
  let flowNodes = $derived<FlowNode[]>(wfNodes.map((n) => ({
    id: n.id,
    type: n.type,
    position: { x: n.position.x, y: n.position.y },
    data: {
      id: n.id,
      label: n.label,
      config: n.config,
      state: getNodeState(n.id),
      selected: $selectedNodeId === n.id,
      onchange: onNodeChange,
    },
  })));

  // Convert WorkflowEdge[] → SvelteFlow Edge[]
  let flowEdges = $derived<FlowEdge[]>(wfEdges.map((e) => ({
    id: e.id ?? `edge-${e.from}-${e.to}`,
    source: e.from,
    target: e.to,
    label: e.label ?? '',
    style: 'stroke: var(--text-muted); stroke-width: 2px;',
  })));

  // Custom node types mapping
  const nodeTypes: NodeTypes = {
    agent: AgentFlowNode,
    resource: ResourceFlowNode,
    logic: LogicFlowNode,
  } as unknown as NodeTypes;

  function onNodeClick({ event, node }: { event: MouseEvent | TouchEvent; node: FlowNode }) {
    selectedNodeId.set(node.id);
  }

  function onNodeDragStop({ event, targetNode }: { event: MouseEvent | TouchEvent; targetNode: FlowNode | null }) {
    const node = targetNode;
    if (!node) return;
    // Sync position back to workflow
    const wf = get(currentWorkflow);
    if (!wf) return;
    const updated = {
      ...wf,
      nodes: wf.nodes.map((n) =>
        n.id === node.id ? { ...n, position: { x: node.position.x, y: node.position.y } } : n
      ),
    };
    currentWorkflow.set(updated);
    workflowDirty.set(true);
  }

  function selectNode(id: string) {
    selectedNodeId.set(id);
  }

  function onNodeChange(nodeId: string, field: string, value: string) {
    const wf = get(currentWorkflow);
    if (!wf) return;
    const updated = {
      ...wf,
      nodes: wf.nodes.map((n) => {
        if (n.id !== nodeId) return n;
        if (field === 'label') return { ...n, label: value };
        return { ...n, config: { ...n.config, [field]: value } };
      }),
    };
    currentWorkflow.set(updated);
    workflowDirty.set(true);
  }

  function closeCanvas() {
    showHiveCanvas.set(false);
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

  // Bidirectional JSON ↔ Canvas sync
  let jsonText = $state('');
  let jsonError = $state<string | null>(null);
  let jsonSyncTimer: ReturnType<typeof setTimeout> | null = null;
  let suppressJsonUpdate = false;

  $effect(() => {
    const wf = $currentWorkflow;
    if (suppressJsonUpdate) return;
    if (wf) jsonText = JSON.stringify(wf, null, 2);
  });

  function onJsonInput() {
    if (jsonSyncTimer) clearTimeout(jsonSyncTimer);
    jsonSyncTimer = setTimeout(() => {
      try {
        const parsed = JSON.parse(jsonText);
        if (parsed && parsed.nodes && parsed.edges) {
          suppressJsonUpdate = true;
          currentWorkflow.set(parsed);
          workflowDirty.set(true);
          jsonError = null;
          suppressJsonUpdate = false;
        } else {
          jsonError = 'Missing required fields (nodes, edges)';
        }
      } catch (e) {
        jsonError = (e as Error).message;
      }
    }, 500);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      closeCanvas();
    }
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      saveWorkflow();
    }
  }

  function validateConnection(connection: FlowEdge | Connection): boolean {
    const wf = get(currentWorkflow);
    if (!wf) return false;

    const sourceNode = wf.nodes.find((n) => n.id === connection.source);
    const targetNode = wf.nodes.find((n) => n.id === connection.target);
    if (!sourceNode || !targetNode) return false;

    // Agent → Resource: check write capability
    if (sourceNode.type === 'agent' && targetNode.type === 'resource') {
      const resConfig = targetNode.config as Record<string, unknown>;
      const access = (resConfig.access as string) || 'read';
      if (access === 'write' || access === 'read_write') {
        const agentConfig = sourceNode.config as Record<string, unknown>;
        const caps = (agentConfig.capabilities as string[]) || [];
        if (!caps.includes('write_file')) {
          showToast(
            'error',
            'Invalid connection',
            `Agent "${sourceNode.label}" lacks write_file capability required for write access to "${targetNode.label}"`
          );
          return false;
        }
      }
    }

    return true;
  }

  function onConnect(connection: Connection) {
    if (!validateConnection(connection)) return;

    const wf = get(currentWorkflow);
    if (!wf) return;

    const edgeId = `e-${connection.source}-${connection.target}-${Date.now()}`;
    const newEdge: WorkflowEdge = {
      id: edgeId,
      from: connection.source,
      to: connection.target,
    };

    const updated = { ...wf, edges: [...wf.edges, newEdge] };
    currentWorkflow.set(updated);
    workflowDirty.set(true);
  }

  async function saveWorkflow() {
    const path = get(currentWorkflowPath);
    const wf = get(currentWorkflow);
    if (!path || !wf) return;
    try {
      await adapter.saveWorkflow(path, wf);
      workflowDirty.set(false);
      showToast('success', 'Workflow saved', path.split('/').pop() ?? path);
    } catch (e) {
      console.error('Save failed:', e);
      showToast('error', 'Save failed', String(e));
    }
  }

</script>

<svelte:window onkeydown={handleKeydown} />

<div class="hive-canvas-root">
  <!-- Toolbar -->
  <div class="canvas-toolbar">
    <WorkflowMenu {adapter} {cwd} />
    <NodeCatalog onadd={addNode} />
    <div class="toolbar-spacer"></div>
    <HiveControls {adapter} {cwd} />
    {#if $currentWorkflow?.settings?.permissionLevel}
      <span class="permission-badge"
            class:supervised={$currentWorkflow.settings.permissionLevel === 'supervised'}
            class:trusted={$currentWorkflow.settings.permissionLevel === 'trusted'}
            class:dryrun={$currentWorkflow.settings.permissionLevel === 'dry-run'}
            role="status"
            aria-label={"Permission: " + $currentWorkflow.settings.permissionLevel}
            title={$currentWorkflow.settings.permissionLevel}>
        {#if $currentWorkflow.settings.permissionLevel === 'supervised'}
          <span aria-hidden="true">&#x1F512;</span>
        {:else if $currentWorkflow.settings.permissionLevel === 'trusted'}
          <span aria-hidden="true">&#x1F6E1;</span>
        {:else}
          <span aria-hidden="true">&#x3030;</span>
        {/if}
      </span>
    {/if}
    <div class="toolbar-spacer"></div>
    <div class="view-modes">
      <button class:active={$hiveViewMode === 'visual'} onclick={() => hiveViewMode.set('visual')}>Visual</button>
      <button class:active={$hiveViewMode === 'code'} onclick={() => hiveViewMode.set('code')}>Code</button>
      <button class:active={$hiveViewMode === 'split'} onclick={() => hiveViewMode.set('split')}>Split</button>
    </div>
    <button class="close-btn" onclick={closeCanvas} title={$tr('a11y.closeCanvas')} aria-label={$tr('a11y.closeCanvas')}>&#x2715;</button>
  </div>

  <!-- Content area -->
  <div class="canvas-content">
    {#if $hiveViewMode === 'visual' || $hiveViewMode === 'split'}
      <div class="canvas-area" class:half={$hiveViewMode === 'split'}>
        <SvelteFlow
          nodes={flowNodes}
          edges={flowEdges}
          {nodeTypes}
          fitView
          isValidConnection={validateConnection}
          onconnect={onConnect}
          onnodeclick={onNodeClick}
          onnodedragstop={onNodeDragStop}
          colorMode="dark"
        >
          <Controls />
          <Background />
          <MiniMap />
        </SvelteFlow>
      </div>
    {/if}

    {#if $hiveViewMode === 'code' || $hiveViewMode === 'split'}
      <div class="code-area" class:half={$hiveViewMode === 'split'}>
        <textarea class="json-editor" bind:value={jsonText} oninput={onJsonInput} spellcheck="false"></textarea>
        {#if jsonError}
          <div class="json-error">{jsonError}</div>
        {/if}
      </div>
    {/if}

    <!-- Inspector (right side) -->
    <div class="inspector-area">
      <HiveInspector />
    </div>
  </div>
</div>

<style>
  .hive-canvas-root {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }
  .canvas-toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-2);
    background: var(--bg-secondary);
    border-bottom: var(--border-width) solid var(--border);
    height: 38px;
    flex-shrink: 0;
  }
  .toolbar-spacer {
    flex: 1;
  }
  .permission-badge {
    font-size: 1.2em;
    padding: 0 0.25rem;
  }
  .permission-badge.supervised {
    color: var(--warning);
  }
  .permission-badge.trusted {
    color: var(--success);
  }
  .permission-badge.dryrun {
    opacity: 0.6;
  }
  .view-modes {
    display: flex;
    gap: 0;
  }
  .view-modes button {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 2px solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-1) var(--space-2);
    font-size: var(--font-size-sm);
    cursor: pointer;
    font-family: var(--font-ui);
    font-weight: 600;
    text-transform: uppercase;
  }
  .view-modes button.active {
    background: var(--accent-btn);
    color: var(--text-on-accent);
    border-color: var(--accent);
  }
  .close-btn {
    background: none;
    border: 2px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    padding: var(--space-1) var(--space-2);
    cursor: pointer;
    font-size: var(--font-size-base);
    min-width: 32px;
    min-height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .close-btn:hover {
    color: var(--danger-text);
    border-color: var(--danger);
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
    display: flex;
    flex-direction: column;
  }
  .code-area.half {
    flex: 0.5;
    border-inline-start: 2px solid var(--border);
  }
  .json-editor {
    width: 100%;
    height: 100%;
    background: var(--bg-primary);
    color: var(--text-body);
    border: none;
    padding: var(--inset-component);
    font-family: var(--font-mono, monospace);
    font-size: var(--font-size-sm);
    resize: none;
  }
  .json-error {
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-size-sm);
    color: var(--danger-text);
    background: var(--bg-secondary);
    border-top: 2px solid var(--danger);
    font-family: var(--font-mono, monospace);
  }
  .inspector-area {
    width: 240px;
    min-width: 200px;
    border-inline-start: 2px solid var(--border);
    background: var(--bg-secondary);
    overflow-y: auto;
  }
</style>
