<script lang="ts">
  import type { Adapter } from '$lib/adapter/index';
  import { currentWorkflow, currentWorkflowPath, workflowDirty } from '$lib/stores/workflow';
  import { showToast } from '$lib/stores/toast';
  import { get } from 'svelte/store';

  let { adapter, cwd = '.' }: { adapter: Adapter; cwd?: string } = $props();

  let open = $state(false);
  let templates = $state<string[]>([]);
  let showTemplates = $state(false);

  function toggle() {
    open = !open;
    if (open) loadTemplates();
  }

  function close() {
    open = false;
    showTemplates = false;
  }

  async function loadTemplates() {
    try {
      templates = await adapter.listGlobalWorkflows();
    } catch {
      templates = [];
    }
  }

  async function save() {
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
    close();
  }

  async function saveToLibrary() {
    const path = get(currentWorkflowPath);
    if (!path) return;
    try {
      const dest = await adapter.saveToGlobal(path);
      console.log('Saved to library:', dest);
      showToast('success', 'Saved to library', dest.split('/').pop() ?? dest);
    } catch (e) {
      console.error('Save to library failed:', e);
      showToast('error', 'Save to library failed', String(e));
    }
    close();
  }

  async function importWorkflow() {
    close();
    try {
      const filePath = await adapter.openFileDialog([{ name: 'Workflow', extensions: ['json'] }]);
      if (!filePath) return;
      const wf = await adapter.loadWorkflow(filePath);
      currentWorkflow.set(wf);
      currentWorkflowPath.set(filePath);
      workflowDirty.set(false);
      showToast('success', 'Workflow imported', filePath.split('/').pop() ?? filePath);
    } catch (e) {
      console.error('Import failed:', e);
      showToast('error', 'Import failed', String(e));
    }
  }

  async function exportWorkflow() {
    close();
    const wf = get(currentWorkflow);
    if (!wf) return;
    try {
      const selected = await adapter.saveFileDialog(
        `${wf.name.replace(/\s+/g, '-').toLowerCase()}.json`,
        [{ name: 'Workflow', extensions: ['json'] }]
      );
      if (!selected) return;
      await adapter.saveWorkflow(selected, wf);
      showToast('success', 'Workflow exported', selected.split('/').pop() ?? selected);
    } catch (e) {
      console.error('Export failed:', e);
      showToast('error', 'Export failed', String(e));
    }
  }

  async function loadTemplate(path: string) {
    try {
      const wf = await adapter.loadWorkflow(path);
      currentWorkflow.set(wf);
      currentWorkflowPath.set(null);
      workflowDirty.set(true);
      showToast('success', 'Template loaded', path.split('/').pop()?.replace('.json', '') ?? path);
    } catch (e) {
      console.error('Load template failed:', e);
    }
    close();
  }

  function handleWindowClick(e: MouseEvent) {
    if (open) {
      const target = e.target as HTMLElement;
      if (!target.closest('.workflow-menu')) {
        close();
      }
    }
  }
</script>

<svelte:window onclick={handleWindowClick} />

<div class="workflow-menu">
  <button class="menu-trigger" onclick={toggle} title="Workflow actions">
    &#9776; Workflow
  </button>

  {#if open}
    <div class="menu-dropdown">
      <button class="menu-item" onclick={save}>
        Save <span class="shortcut">Ctrl+S</span>
      </button>
      <button class="menu-item" onclick={saveToLibrary}>
        Save to Library
      </button>
      <div class="menu-divider"></div>
      <button class="menu-item" onclick={importWorkflow}>
        Import...
      </button>
      <button class="menu-item" onclick={exportWorkflow}>
        Export...
      </button>
      <div class="menu-divider"></div>
      <button class="menu-item" onclick={() => { showTemplates = !showTemplates; }}>
        Templates {showTemplates ? '▴' : '▾'}
      </button>
      {#if showTemplates}
        <div class="template-list">
          {#if templates.length === 0}
            <span class="no-templates">No templates saved</span>
          {:else}
            {#each templates as tmpl}
              <button class="menu-item template-item" onclick={() => loadTemplate(tmpl)}>
                {tmpl.split('/').pop()?.replace('.json', '') ?? tmpl}
              </button>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .workflow-menu {
    position: relative;
  }
  .menu-trigger {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: var(--border-width) solid var(--border);
    padding: var(--space-1) var(--space-2);
    font-size: var(--font-size-sm);
    font-family: var(--font-ui);
    cursor: pointer;
  }
  .menu-trigger:hover {
    background: var(--bg-hover);
  }
  .menu-dropdown {
    position: absolute;
    top: 100%;
    inset-inline-start: 0;
    z-index: var(--layer-sticky);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    min-width: 200px;
    padding: var(--space-1) 0;
    box-shadow: var(--shadow-md);
  }
  .menu-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    background: none;
    border: none;
    color: var(--text-primary);
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-size-sm);
    font-family: var(--font-ui);
    cursor: pointer;
    text-align: start;
  }
  .menu-item:hover {
    background: var(--bg-hover);
  }
  .shortcut {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }
  .menu-divider {
    height: 1px;
    background: var(--border);
    margin: var(--stack-tight) 0;
  }
  .template-list {
    padding-inline-start: var(--space-3);
  }
  .template-item {
    font-size: var(--font-size-sm);
  }
  .no-templates {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    padding: var(--space-1) var(--space-3);
    display: block;
  }
</style>
