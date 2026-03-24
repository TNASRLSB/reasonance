<script lang="ts">
  import { suggestFamily } from '$lib/engine/color-harmony';
  import ContrastBadge from './ContrastBadge.svelte';

  let {
    familyName,
    variables,
    bgPrimary,
    onUpdate,
  }: {
    familyName: string;
    variables: Record<string, string>;
    bgPrimary: string;
    onUpdate: (key: string, value: string) => void;
  } = $props();

  // Text variables that should show contrast badges vs bgPrimary
  const TEXT_VARS = new Set([
    '--text-primary', '--text-body', '--text-secondary', '--text-muted', '--text-on-accent',
    '--accent-text', '--danger-text', '--success-text', '--warning-text',
  ]);

  let suggestions = $state<Record<string, string>>({});
  let showSuggestions = $state(false);

  function handleColorChange(key: string, value: string) {
    onUpdate(key, value);

    // If it's the head accent color, compute suggestions
    if (key === '--accent' && bgPrimary) {
      suggestions = suggestFamily(value, bgPrimary);
      showSuggestions = true;
    }
  }

  function applySuggestion(key: string, value: string) {
    onUpdate(key, value);
    delete suggestions[key];
    if (Object.keys(suggestions).length === 0) showSuggestions = false;
  }

  function dismissSuggestions() {
    suggestions = {};
    showSuggestions = false;
  }

  let entries = $derived(Object.entries(variables));
</script>

<div class="family-group">
  <h3 class="family-name">{familyName}</h3>

  <div class="variables">
    {#each entries as [key, value]}
      <div class="var-row">
        <label class="var-label" for="color-{key}">{key}</label>
        <div class="var-controls">
          <input
            type="color"
            id="color-{key}"
            value={value}
            oninput={(e) => handleColorChange(key, (e.target as HTMLInputElement).value)}
            class="color-swatch"
            aria-label="Color picker for {key}"
          />
          <input
            type="text"
            value={value}
            oninput={(e) => handleColorChange(key, (e.target as HTMLInputElement).value)}
            class="color-text"
            aria-label="Hex value for {key}"
            spellcheck="false"
          />
          {#if TEXT_VARS.has(key) && bgPrimary && value.startsWith('#')}
            <ContrastBadge foreground={value} background={bgPrimary} />
          {/if}
        </div>
      </div>
    {/each}
  </div>

  {#if showSuggestions && Object.keys(suggestions).length > 0}
    <div class="suggestions">
      <div class="suggestions-header">
        <span class="suggestions-title">Suggested family updates</span>
        <button class="dismiss-btn" onclick={dismissSuggestions} aria-label="Dismiss suggestions">✕</button>
      </div>
      {#each Object.entries(suggestions) as [key, value]}
        <div class="suggestion-row">
          <span class="suggestion-key">{key}</span>
          <span class="suggestion-preview" style="background: {value}"></span>
          <span class="suggestion-value">{value}</span>
          <button class="apply-btn" onclick={() => applySuggestion(key, value)}>Apply</button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .family-group {
    margin-bottom: var(--space-5, 20px);
  }

  .family-name {
    font-size: var(--font-size-small, 12px);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted, #888);
    margin: 0 0 var(--space-2, 8px) 0;
    padding-bottom: var(--space-1, 4px);
    border-bottom: 1px solid var(--border, #333);
  }

  .variables {
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 8px);
  }

  .var-row {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
    min-height: 28px;
  }

  .var-label {
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    color: var(--text-secondary, #aaa);
    width: 180px;
    flex-shrink: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .var-controls {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
    flex: 1;
    min-width: 0;
  }

  .color-swatch {
    width: 28px;
    height: 28px;
    border: 1px solid var(--border, #333);
    border-radius: 3px;
    padding: 2px;
    cursor: pointer;
    background: transparent;
    flex-shrink: 0;
  }

  .color-text {
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    background: var(--bg-tertiary, #1a1a1a);
    border: 1px solid var(--border, #333);
    color: var(--text-primary, #eee);
    padding: 3px 6px;
    width: 90px;
    flex-shrink: 0;
  }

  .color-text:focus {
    outline: var(--focus-ring, 2px solid #4a9eff);
    outline-offset: 1px;
  }

  .suggestions {
    margin-top: var(--space-3, 12px);
    background: var(--bg-secondary, #1e1e1e);
    border: 1px solid var(--accent, #4a9eff);
    border-radius: 4px;
    padding: var(--space-3, 12px);
  }

  .suggestions-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-2, 8px);
  }

  .suggestions-title {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--accent-text, #7ac7ff);
  }

  .dismiss-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #888);
    cursor: pointer;
    padding: 2px 4px;
    font-size: 12px;
    line-height: 1;
  }

  .dismiss-btn:hover {
    color: var(--text-primary, #eee);
  }

  .suggestion-row {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
    padding: 4px 0;
    font-size: 12px;
  }

  .suggestion-key {
    font-family: var(--font-mono, monospace);
    color: var(--text-secondary, #aaa);
    width: 160px;
    flex-shrink: 0;
    font-size: 11px;
  }

  .suggestion-preview {
    width: 16px;
    height: 16px;
    border-radius: 2px;
    border: 1px solid var(--border, #333);
    flex-shrink: 0;
  }

  .suggestion-value {
    font-family: var(--font-mono, monospace);
    color: var(--text-muted, #888);
    font-size: 11px;
    flex: 1;
  }

  .apply-btn {
    background: var(--bg-tertiary, #1a1a1a);
    border: 1px solid var(--border, #333);
    color: var(--text-primary, #eee);
    padding: 2px 8px;
    font-size: 11px;
    font-family: var(--font-ui, sans-serif);
    cursor: pointer;
    flex-shrink: 0;
  }

  .apply-btn:hover {
    background: var(--accent, #4a9eff);
    border-color: var(--accent, #4a9eff);
    color: var(--text-on-accent, #fff);
  }
</style>
