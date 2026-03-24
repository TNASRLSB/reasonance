<script lang="ts">
  import { selectedNodeId } from '$lib/stores/ui';
  import { currentWorkflow, workflowDirty } from '$lib/stores/workflow';
  import type { WorkflowNode } from '$lib/adapter/index';
  import { onDestroy } from 'svelte';
  import { get } from 'svelte/store';

  let node = $state<WorkflowNode | null>(null);
  let showJson = $state(false);
  let selId = $state<string | null>(null);

  const unsubSel = selectedNodeId.subscribe((id) => {
    selId = id;
    const wf = get(currentWorkflow);
    if (id && wf) {
      node = wf.nodes.find((n) => n.id === id) ?? null;
    } else {
      node = null;
    }
  });

  const unsubWf = currentWorkflow.subscribe((wf) => {
    if (selId && wf) {
      node = wf.nodes.find((n) => n.id === selId) ?? null;
    } else {
      node = null;
    }
  });

  onDestroy(() => { unsubSel(); unsubWf(); });

  function updateConfig(field: string, value: unknown) {
    const wf = get(currentWorkflow);
    if (!wf || !selId) return;
    currentWorkflow.set({
      ...wf,
      nodes: wf.nodes.map((n) =>
        n.id === selId ? { ...n, config: { ...n.config, [field]: value } } : n
      ),
    });
    workflowDirty.set(true);
  }

  function updateNestedConfig(parent: string, field: string, value: unknown) {
    const wf = get(currentWorkflow);
    if (!wf || !selId) return;
    currentWorkflow.set({
      ...wf,
      nodes: wf.nodes.map((n) => {
        if (n.id !== selId) return n;
        const parentObj = (n.config[parent] as Record<string, unknown>) ?? {};
        return { ...n, config: { ...n.config, [parent]: { ...parentObj, [field]: value } } };
      }),
    });
    workflowDirty.set(true);
  }

  // Derived config helpers
  let isAgent = $derived(node?.type === 'agent');
  let memoryEnabled = $derived((node?.config?.memory as any)?.enabled ?? false);
  let memoryMaxEntries = $derived((node?.config?.memory as any)?.maxEntries ?? 50);
  let memoryPersist = $derived((node?.config?.memory as any)?.persist ?? 'none');
  let retryCount = $derived((node?.config?.retry as number) ?? 0);
  let fallbackAgent = $derived((node?.config?.fallback as string) ?? '');
  let capabilities = $derived((node?.config?.capabilities as string[]) ?? []);

  const availableCapabilities = [
    'read_file', 'write_file', 'execute', 'network', 'browser', 'shell',
  ];

  function toggleCapability(cap: string) {
    const current = [...capabilities];
    const idx = current.indexOf(cap);
    if (idx >= 0) current.splice(idx, 1);
    else current.push(cap);
    updateConfig('capabilities', current);
  }

  function deleteNode() {
    const wf = get(currentWorkflow);
    if (!wf || !selId) return;
    currentWorkflow.set({
      ...wf,
      nodes: wf.nodes.filter((n) => n.id !== selId),
      edges: wf.edges.filter((e) => e.from !== selId && e.to !== selId),
    });
    workflowDirty.set(true);
    selectedNodeId.set(null);
  }
</script>

{#if node}
  <div class="inspector">
    <div class="inspector-header">
      <h3>{node.label}</h3>
      <span class="node-type">{node.type}</span>
    </div>

    <div class="inspector-section">
      <div class="section-label">ID</div>
      <div class="section-value mono">{node.id}</div>
    </div>

    <div class="inspector-section">
      <div class="section-label">Position</div>
      <div class="section-value">x: {node.position.x.toFixed(0)}, y: {node.position.y.toFixed(0)}</div>
    </div>

    {#if isAgent}
      <div class="inspector-section">
        <div class="section-label">Capabilities</div>
        <div class="caps-grid">
          {#each availableCapabilities as cap}
            <label class="cap-toggle">
              <input
                type="checkbox"
                checked={capabilities.includes(cap)}
                onchange={() => toggleCapability(cap)}
              />
              {cap}
            </label>
          {/each}
        </div>
      </div>

      <div class="inspector-section">
        <div class="section-label">Memory</div>
        <label class="toggle-row">
          <input
            type="checkbox"
            checked={memoryEnabled}
            onchange={(e) => updateNestedConfig('memory', 'enabled', (e.target as HTMLInputElement).checked)}
          />
          Enabled
        </label>
        {#if memoryEnabled}
          <div class="field-row">
            <label class="field-label">Max entries</label>
            <input
              type="number"
              class="field-input small"
              value={memoryMaxEntries}
              min="1"
              max="500"
              onchange={(e) => updateNestedConfig('memory', 'maxEntries', parseInt((e.target as HTMLInputElement).value) || 50)}
            />
          </div>
          <div class="field-row">
            <label class="field-label">Persist</label>
            <select
              class="field-input"
              value={memoryPersist}
              onchange={(e) => updateNestedConfig('memory', 'persist', (e.target as HTMLSelectElement).value)}
            >
              <option value="none">None</option>
              <option value="workflow">Workflow</option>
              <option value="global">Global</option>
            </select>
          </div>
        {/if}
      </div>

      <div class="inspector-section">
        <div class="section-label">Error handling</div>
        <div class="field-row">
          <label class="field-label">Retries</label>
          <input
            type="number"
            class="field-input small"
            value={retryCount}
            min="0"
            max="10"
            onchange={(e) => updateConfig('retry', parseInt((e.target as HTMLInputElement).value) || 0)}
          />
        </div>
        <div class="field-row">
          <label class="field-label">Fallback</label>
          <input
            type="text"
            class="field-input"
            value={fallbackAgent}
            placeholder="node-id..."
            oninput={(e) => updateConfig('fallback', (e.target as HTMLInputElement).value)}
          />
        </div>
      </div>
    {/if}

    <div class="inspector-section">
      <button class="toggle-json" onclick={() => showJson = !showJson}>
        {showJson ? 'Hide' : 'Show'} JSON
      </button>
      {#if showJson}
        <pre class="json-raw">{JSON.stringify(node.config, null, 2)}</pre>
      {/if}
    </div>

    <div class="inspector-section danger-zone">
      <button class="delete-btn" onclick={deleteNode}>Delete node</button>
    </div>
  </div>
{:else}
  <div class="inspector empty">
    <p>Select a node to inspect</p>
  </div>
{/if}

<style>
  .inspector {
    padding: var(--inset-component);
    font-family: var(--font-ui, sans-serif);
    height: 100%;
    overflow-y: auto;
  }
  .inspector.empty {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .inspector-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-3);
    padding-bottom: var(--space-2);
    border-bottom: var(--border-width) solid var(--border);
  }
  .inspector-header h3 {
    margin: 0;
    font-size: var(--font-size-base);
    color: var(--text-primary);
  }
  .node-type {
    font-size: var(--font-size-sm);
    text-transform: uppercase;
    color: var(--text-secondary);
    padding: var(--stack-tight) var(--space-1);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
  }
  .inspector-section {
    margin-bottom: var(--space-3);
  }
  .section-label {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    text-transform: uppercase;
    margin-bottom: var(--stack-tight);
    font-weight: 600;
  }
  .section-value {
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }
  .section-value.mono {
    font-family: var(--font-mono, monospace);
    font-size: var(--font-size-sm);
  }

  .caps-grid {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .cap-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--font-size-sm);
    color: var(--text-body);
    cursor: pointer;
  }
  .cap-toggle input {
    cursor: pointer;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--font-size-sm);
    color: var(--text-body);
    cursor: pointer;
    margin-bottom: var(--space-1);
  }
  .toggle-row input {
    cursor: pointer;
  }

  .field-row {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    margin-bottom: var(--space-1);
  }
  .field-label {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    min-width: 70px;
  }
  .field-input {
    flex: 1;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    padding: 2px var(--space-1);
    font-size: var(--font-size-sm);
    font-family: var(--font-ui, sans-serif);
    outline: none;
  }
  .field-input.small {
    max-width: 60px;
  }
  .field-input:focus {
    border-color: var(--accent);
  }

  .toggle-json {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    padding: var(--space-1) var(--space-2);
    font-size: var(--font-size-sm);
    cursor: pointer;
    font-family: var(--font-ui);
    width: 100%;
  }
  .toggle-json:hover {
    background: var(--bg-hover);
  }
  .json-raw {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    padding: var(--space-2);
    font-size: var(--font-size-sm);
    font-family: var(--font-mono, monospace);
    color: var(--text-body);
    margin-top: var(--interactive-gap);
    overflow-x: auto;
    white-space: pre-wrap;
  }

  .danger-zone {
    border-top: 1px solid var(--border);
    padding-top: var(--space-2);
    margin-top: var(--space-3);
  }
  .delete-btn {
    background: var(--bg-tertiary);
    color: var(--danger-text, #ef4444);
    border: 1px solid var(--danger, #ef4444);
    padding: var(--space-1) var(--space-2);
    font-size: var(--font-size-sm);
    cursor: pointer;
    font-family: var(--font-ui);
    width: 100%;
  }
  .delete-btn:hover {
    background: var(--danger, #ef4444);
    color: white;
  }
</style>
