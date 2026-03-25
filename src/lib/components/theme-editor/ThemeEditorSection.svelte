<script lang="ts">
  import ColorFamilyGroup from './ColorFamilyGroup.svelte';

  let {
    sectionName,
    variables,
    onUpdate,
  }: {
    sectionName: string;
    variables: Record<string, string | number>;
    onUpdate: (key: string, value: string | number) => void;
  } = $props();

  // Color family groupings for the 'colors' section
  const COLOR_FAMILIES: Record<string, string[]> = {
    Background: ['--bg-primary', '--bg-secondary', '--bg-tertiary', '--bg-hover', '--bg-surface'],
    Text: ['--text-primary', '--text-body', '--text-secondary', '--text-muted', '--text-on-accent'],
    Accent: ['--accent', '--accent-text', '--accent-hover', '--accent-btn', '--accent-statusbar'],
    Danger: ['--danger', '--danger-dark', '--danger-text', '--danger-btn'],
    Success: ['--success', '--success-text'],
    Warning: ['--warning', '--warning-text'],
    Other: ['--border', '--border-color', '--code-bg', '--code-accent'],
  };

  function isHexColor(val: string | number): boolean {
    return typeof val === 'string' && /^#[0-9a-fA-F]{3,8}$/.test(val.trim());
  }

  function isDimension(val: string | number): boolean {
    return typeof val === 'string' && /\d+(rem|px|em|vh|vw|%)/.test(val);
  }

  function getInputType(val: string | number): 'color' | 'number' | 'dimension' | 'text' {
    if (isHexColor(val)) return 'color';
    if (typeof val === 'number') return 'number';
    if (isDimension(val)) return 'dimension';
    return 'text';
  }

  // Build family groups from current variables for the colors section
  let colorFamilies = $derived(() => {
    const entries = Object.entries(variables) as [string, string][];
    const assigned = new Set<string>();
    const result: Array<{ name: string; vars: Record<string, string> }> = [];

    for (const [familyName, keys] of Object.entries(COLOR_FAMILIES)) {
      const familyVars: Record<string, string> = {};
      for (const key of keys) {
        const found = entries.find(([k]) => k === key);
        if (found) {
          familyVars[key] = found[1];
          assigned.add(key);
        }
      }
      // Also pick up any unassigned keys that contain the family name hint
      for (const [k, v] of entries) {
        if (!assigned.has(k)) {
          const lk = k.toLowerCase();
          const lf = familyName.toLowerCase();
          if (lk.includes(lf.substring(0, 3))) {
            familyVars[k] = v;
            assigned.add(k);
          }
        }
      }
      if (Object.keys(familyVars).length > 0) {
        result.push({ name: familyName, vars: familyVars });
      }
    }

    // Remaining unassigned variables
    const remaining: Record<string, string> = {};
    for (const [k, v] of entries) {
      if (!assigned.has(k)) remaining[k] = v as string;
    }
    if (Object.keys(remaining).length > 0) {
      result.push({ name: 'Other', vars: remaining });
    }

    return result;
  });

  let bgPrimary = $derived(
    sectionName === 'colors'
      ? ((variables['--bg-primary'] as string) ?? '#000000')
      : '#000000'
  );

  let nonColorEntries = $derived(
    sectionName !== 'colors' ? Object.entries(variables) : []
  );
</script>

<div class="section-content">
  <h2 class="section-title">{sectionName}</h2>

  {#if sectionName === 'colors'}
    {#each colorFamilies() as family}
      <ColorFamilyGroup
        familyName={family.name}
        variables={family.vars}
        bgPrimary={bgPrimary}
        onUpdate={onUpdate}
      />
    {/each}
  {:else}
    <div class="generic-vars">
      {#each nonColorEntries as [key, value]}
        {@const inputType = getInputType(value)}
        <div class="var-row">
          <label class="var-label" for="var-{key}">{key}</label>
          <div class="var-controls">
            {#if inputType === 'color'}
              <input
                type="color"
                value={value as string}
                oninput={(e) => onUpdate(key, (e.target as HTMLInputElement).value)}
                class="color-swatch"
                aria-label="Color picker for {key}"
              />
              <input
                id="var-{key}"
                type="text"
                value={value as string}
                oninput={(e) => onUpdate(key, (e.target as HTMLInputElement).value)}
                class="text-input"
                spellcheck="false"
              />
            {:else if inputType === 'number'}
              <input
                id="var-{key}"
                type="number"
                value={value as number}
                step="1"
                oninput={(e) => onUpdate(key, Number((e.target as HTMLInputElement).value))}
                class="number-input"
              />
            {:else}
              <input
                id="var-{key}"
                type="text"
                value={value as string}
                oninput={(e) => onUpdate(key, (e.target as HTMLInputElement).value)}
                class="text-input"
                spellcheck="false"
              />
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .section-content {
    padding: var(--space-4, 16px);
  }

  .section-title {
    font-size: var(--font-size-small, 12px);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-primary);
    margin: 0 0 var(--space-4, 16px) 0;
    padding-bottom: var(--space-2, 8px);
    border-bottom: 2px solid var(--accent);
  }

  .generic-vars {
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
    color: var(--text-secondary);
    width: 200px;
    flex-shrink: 0;
    white-space: nowrap;
    overflow: auto;
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
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 2px;
    cursor: pointer;
    background: transparent;
    flex-shrink: 0;
  }

  .text-input,
  .number-input {
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 3px 6px;
  }

  .text-input {
    width: 200px;
  }

  .number-input {
    width: 80px;
  }

  .text-input:focus,
  .number-input:focus {
    outline: var(--focus-ring);
    outline-offset: 1px;
  }
</style>
