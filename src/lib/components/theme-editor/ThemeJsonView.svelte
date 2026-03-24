<script lang="ts">
  import { validateTheme } from '$lib/engine/theme-validator';
  import type { ThemeFile } from '$lib/engine/theme-types';

  let {
    theme,
    onUpdate,
  }: {
    theme: ThemeFile;
    onUpdate: (theme: ThemeFile) => void;
  } = $props();

  let jsonText = $state(JSON.stringify(theme, null, 2));
  let parseError = $state<string | null>(null);
  let validationErrors = $state<string[]>([]);

  // Keep jsonText in sync when theme prop changes externally, but only if not currently editing
  let lastKnownJson = $state(JSON.stringify(theme, null, 2));
  $effect(() => {
    const incoming = JSON.stringify(theme, null, 2);
    if (incoming !== lastKnownJson) {
      lastKnownJson = incoming;
      jsonText = incoming;
      parseError = null;
      validationErrors = [];
    }
  });

  function handleInput(e: Event) {
    const text = (e.target as HTMLTextAreaElement).value;
    jsonText = text;
    parseError = null;
    validationErrors = [];

    let parsed: unknown;
    try {
      parsed = JSON.parse(text);
    } catch (err) {
      parseError = (err as Error).message;
      return;
    }

    const result = validateTheme(parsed);
    if (!result.valid) {
      validationErrors = result.errors;
      return;
    }

    lastKnownJson = text;
    onUpdate(parsed as ThemeFile);
  }

  let hasErrors = $derived(parseError !== null || validationErrors.length > 0);
</script>

<div class="json-view">
  <textarea
    class="json-textarea"
    class:has-errors={hasErrors}
    value={jsonText}
    oninput={handleInput}
    aria-label="Raw JSON editor"
    spellcheck="false"
    autocomplete="off"
    autocorrect="off"
    autocapitalize="off"
  ></textarea>

  {#if parseError}
    <div class="error-panel" role="alert">
      <span class="error-label">JSON Error:</span>
      <span class="error-text">{parseError}</span>
    </div>
  {/if}

  {#if validationErrors.length > 0}
    <div class="error-panel" role="alert">
      <span class="error-label">Validation Errors:</span>
      <ul class="error-list">
        {#each validationErrors as err}
          <li>{err}</li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if !hasErrors}
    <div class="status-ok" aria-live="polite">Valid theme JSON</div>
  {/if}
</div>

<style>
  .json-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .json-textarea {
    flex: 1;
    width: 100%;
    background: var(--bg-primary, #0d0d0d);
    color: var(--text-primary, #eee);
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    line-height: 1.6;
    border: none;
    border-bottom: 1px solid var(--border, #2a2a2a);
    padding: var(--space-4, 16px);
    resize: none;
    box-sizing: border-box;
    outline: none;
    tab-size: 2;
  }

  .json-textarea.has-errors {
    border-bottom-color: var(--danger, #ef4444);
  }

  .json-textarea:focus {
    outline: none;
    box-shadow: inset 0 0 0 1px var(--accent, #4a9eff);
  }

  .error-panel {
    flex-shrink: 0;
    background: var(--danger, #ef4444)18;
    border-top: 2px solid var(--danger, #ef4444);
    padding: var(--space-2, 8px) var(--space-4, 16px);
    max-height: 150px;
    overflow-y: auto;
  }

  .error-label {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--danger-text, #fca5a5);
    margin-right: var(--space-2, 8px);
  }

  .error-text {
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    color: var(--danger-text, #fca5a5);
  }

  .error-list {
    list-style: none;
    margin: var(--space-1, 4px) 0 0 0;
    padding: 0;
  }

  .error-list li {
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    color: var(--danger-text, #fca5a5);
    padding: 2px 0;
  }

  .error-list li::before {
    content: '• ';
  }

  .status-ok {
    flex-shrink: 0;
    font-size: 11px;
    color: #22c55e;
    padding: 4px var(--space-4, 16px);
    background: #22c55e0c;
    border-top: 1px solid #22c55e22;
  }
</style>
