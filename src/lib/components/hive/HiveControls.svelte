<script lang="ts">
  import type { Adapter } from '$lib/adapter/index';
  import { currentRun, currentRunId, runStatus } from '$lib/stores/engine';
  import { currentWorkflowPath } from '$lib/stores/workflow';
  import { get } from 'svelte/store';

  let { adapter, cwd = '.' }: { adapter: Adapter; cwd?: string } = $props();

  let status = $state<string>('idle');
  const unsubStatus = runStatus.subscribe((val) => { status = val; });

  async function play() {
    const path = get(currentWorkflowPath);
    if (!path) return;
    try {
      const runId = await adapter.playWorkflow(path, cwd);
      currentRunId.set(runId);
      const run = await adapter.getRunStatus(runId);
      if (run) currentRun.set(run);
    } catch (e) {
      console.error('Failed to start workflow:', e);
    }
  }

  async function pause() {
    const runId = get(currentRunId);
    if (!runId) return;
    await adapter.pauseWorkflow(runId);
    const run = await adapter.getRunStatus(runId);
    if (run) currentRun.set(run);
  }

  async function resume() {
    const runId = get(currentRunId);
    const path = get(currentWorkflowPath);
    if (!runId || !path) return;
    await adapter.resumeWorkflow(runId, path, cwd);
    const run = await adapter.getRunStatus(runId);
    if (run) currentRun.set(run);
  }

  async function stop() {
    const runId = get(currentRunId);
    if (!runId) return;
    await adapter.stopWorkflow(runId);
    const run = await adapter.getRunStatus(runId);
    if (run) currentRun.set(run);
  }

  async function step() {
    const runId = get(currentRunId);
    const path = get(currentWorkflowPath);
    if (!runId || !path) return;
    await adapter.stepWorkflow(runId, path, cwd);
    const run = await adapter.getRunStatus(runId);
    if (run) currentRun.set(run);
  }

  import { onDestroy } from 'svelte';
  onDestroy(() => unsubStatus());
</script>

<div class="hive-controls">
  {#if status === 'idle' || status === 'stopped' || status === 'completed' || status === 'failed'}
    <button class="ctrl-btn play" onclick={play} title="Play">&#9654;</button>
  {:else if status === 'running'}
    <button class="ctrl-btn pause" onclick={pause} title="Pause">&#9646;&#9646;</button>
  {:else if status === 'paused'}
    <button class="ctrl-btn resume" onclick={resume} title="Resume">&#9654;</button>
  {/if}
  <button class="ctrl-btn stop" onclick={stop} title="Stop" disabled={status === 'idle' || status === 'stopped'}>&#9632;</button>
  <button class="ctrl-btn step" onclick={step} title="Step" disabled={status !== 'paused' && status !== 'running'}>&#9654;|</button>
</div>

<style>
  .hive-controls {
    display: flex;
    gap: var(--interactive-gap);
    align-items: center;
  }
  .ctrl-btn {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: var(--border-width) solid var(--border);
    padding: var(--space-1) var(--space-2);
    font-size: var(--font-size-base);
    cursor: pointer;
    font-family: var(--font-ui);
  }
  .ctrl-btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }
  .ctrl-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  .ctrl-btn.play, .ctrl-btn.resume {
    color: var(--success-text);
  }
  .ctrl-btn.stop {
    color: var(--danger-text);
  }
</style>
