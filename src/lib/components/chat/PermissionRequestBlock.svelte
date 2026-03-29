<script lang="ts">
  import { untrack } from 'svelte';
  import type { Adapter } from '$lib/adapter/index';

  let { denials, sessionId, adapter, onAllDecided, timeoutSecs = 300 }: {
    denials: Array<{ tool_name?: string; name?: string; args?: unknown }>;
    sessionId: string;
    adapter: Adapter;
    onAllDecided: () => void;
    timeoutSecs?: number;
  } = $props();

  // Track which tools have been decided: tool_name -> chosen action label
  let decisions = $state<Map<string, string>>(new Map());
  // Track which tools have their deny submenu expanded
  let denyExpanded = $state<Set<string>>(new Set());
  // remaining is intentionally initialised once from the prop (snapshot on mount).
  // untrack suppresses the Svelte "state_referenced_locally" warning — correct here.
  let remaining = $state(untrack(() => timeoutSecs));
  let timedOut = $state(false);

  let toolEntries = $derived.by(() => {
    if (Array.isArray(denials)) {
      return denials.map((d) => ({
        name: d.tool_name ?? d.name ?? 'unknown',
        args: d.args,
      }));
    }
    return [];
  });

  let allDecided = $derived(
    toolEntries.length > 0 && toolEntries.every((t) => decisions.has(t.name))
  );

  // Fire callback once when all tools are decided
  $effect(() => {
    if (allDecided) {
      onAllDecided();
    }
  });

  // Countdown interval — stops when all decided or timed out
  $effect(() => {
    if (allDecided || timedOut) return;

    const interval = setInterval(() => {
      remaining -= 1;
      if (remaining <= 0) {
        clearInterval(interval);
        handleTimeout();
      }
    }, 1000);

    return () => clearInterval(interval);
  });

  async function handleTimeout() {
    timedOut = true;
    for (const tool of toolEntries) {
      if (!decisions.has(tool.name)) {
        decisions.set(tool.name, 'Denied (timeout)');
        try {
          await adapter.recordPermissionDecision(sessionId, tool.name, 'deny', 'once');
        } catch (e) {
          console.error(`Timeout deny failed for ${tool.name}:`, e);
        }
      }
    }
    decisions = new Map(decisions);
    // allDecided will trigger onAllDecided via the existing $effect
  }

  function argsPreview(args: unknown): string {
    if (args == null) return '';
    try {
      const str = typeof args === 'string' ? args : JSON.stringify(args);
      return str.length > 80 ? str.slice(0, 77) + '...' : str;
    } catch {
      return '';
    }
  }

  function formatTime(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  async function handleDecision(
    toolName: string,
    action: string,
    scope: string,
    label: string,
  ) {
    // Prevent double-clicks
    if (decisions.has(toolName)) return;
    decisions.set(toolName, label);
    decisions = new Map(decisions);
    try {
      await adapter.recordPermissionDecision(sessionId, toolName, action, scope);
    } catch (e) {
      console.error(`Permission decision failed for ${toolName}:`, e);
    }
  }
</script>

<div class="permission-request" role="alert">
  <div class="header">
    PERMISSION REQUIRED
    {#if !allDecided && !timedOut}
      <span class="countdown" class:urgent={remaining <= 30}>
        {formatTime(remaining)}
      </span>
    {/if}
  </div>
  {#each toolEntries as tool (tool.name)}
    {@const decided = decisions.has(tool.name)}
    <div class="tool-row" class:decided>
      <div class="tool-info">
        <span class="tool-name">{tool.name}</span>
        {#if tool.args}
          <span class="tool-args">{argsPreview(tool.args)}</span>
        {/if}
      </div>
      {#if !decided}
        <div class="actions">
          <button
            class="btn allow-once"
            onclick={() => handleDecision(tool.name, 'allow', 'once', 'Allowed once')}
          >Allow once</button>
          <button
            class="btn allow-session"
            onclick={() => handleDecision(tool.name, 'allow', 'session', 'Allowed (session)')}
          >Allow session</button>
          <button
            class="btn allow-project"
            onclick={() => handleDecision(tool.name, 'allow', 'project', 'Allowed (project)')}
          >Allow project</button>
          {#if !denyExpanded.has(tool.name)}
            <button
              class="btn deny"
              onclick={() => {
                denyExpanded.add(tool.name);
                denyExpanded = new Set(denyExpanded);
              }}
            >Deny...</button>
          {:else}
            <button
              class="btn deny"
              onclick={() => handleDecision(tool.name, 'deny', 'once', 'Denied')}
            >once</button>
            <button
              class="btn deny"
              onclick={() => handleDecision(tool.name, 'deny', 'session', 'Denied (session)')}
            >session</button>
            <button
              class="btn deny"
              onclick={() => handleDecision(tool.name, 'deny', 'project', 'Denied (project)')}
            >project</button>
          {/if}
        </div>
      {:else}
        <span class="decided-label">{decisions.get(tool.name)}</span>
      {/if}
    </div>
  {/each}
</div>

<style>
  .permission-request {
    border: 2px solid var(--warning);
    background: var(--bg-secondary);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .header {
    font-family: var(--font-ui);
    font-size: var(--font-size-small);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--warning);
  }

  .countdown {
    float: right;
    font-family: var(--font-mono);
    font-size: var(--font-size-small);
    font-weight: 700;
    color: var(--text-muted);
  }

  .countdown.urgent {
    color: var(--error, #ef4444);
  }

  .tool-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    transition: opacity var(--transition-fast);
  }

  .tool-row.decided {
    opacity: 0.5;
  }

  .tool-info {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .tool-name {
    font-family: var(--font-mono);
    font-size: var(--font-size-small);
    font-weight: 700;
    color: var(--text-primary);
  }

  .tool-args {
    font-family: var(--font-mono);
    font-size: var(--font-size-tiny);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 40ch;
  }

  .actions {
    display: flex;
    gap: var(--space-1);
    flex-wrap: wrap;
    margin-top: var(--space-1);
  }

  .btn {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: var(--space-1) var(--space-2);
    border: var(--border-width) solid var(--border);
    cursor: pointer;
    min-height: 1.75rem;
    transition: background var(--transition-fast);
  }

  .btn.allow-once {
    background: var(--accent-btn);
    color: var(--text-on-accent);
    border-color: var(--accent);
  }

  .btn.allow-once:hover {
    opacity: 0.85;
  }

  .btn.allow-session {
    background: transparent;
    color: var(--accent-text);
    border-color: var(--accent);
  }

  .btn.allow-session:hover {
    background: var(--bg-hover);
  }

  .btn.allow-project {
    background: transparent;
    color: var(--warning);
    border-color: var(--warning);
  }

  .btn.allow-project:hover {
    background: var(--bg-hover);
  }

  .btn.deny {
    background: transparent;
    color: var(--text-muted);
  }

  .btn.deny:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .decided-label {
    font-family: var(--font-ui);
    font-size: var(--font-size-tiny);
    font-weight: 700;
    text-transform: uppercase;
    color: var(--text-muted);
  }
</style>
