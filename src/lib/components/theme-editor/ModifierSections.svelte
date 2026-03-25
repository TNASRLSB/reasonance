<script lang="ts">
  import type { ThemeFile, ThemeSectionKey } from '$lib/engine/theme-types';
  import { THEME_SECTIONS } from '$lib/engine/theme-types';
  import ThemeEditorSection from './ThemeEditorSection.svelte';

  let {
    theme,
    onUpdate,
  }: {
    theme: ThemeFile;
    onUpdate: (theme: ThemeFile) => void;
  } = $props();

  type ConditionalTab = 'when-dark' | 'when-light';

  let activeConditionalTab = $state<ConditionalTab>('when-dark');
  let activeConditionalSection = $state<string>('colors');

  function isSectionEnabled(section: ThemeSectionKey): boolean {
    return theme[section] !== undefined && theme[section] !== null;
  }

  function toggleSection(section: ThemeSectionKey) {
    const updated = structuredClone(theme);
    if (isSectionEnabled(section)) {
      delete updated[section];
    } else {
      (updated as any)[section] = {};
    }
    onUpdate(updated);
  }

  function updateSectionVariable(section: ThemeSectionKey, key: string, value: string | number) {
    const updated = structuredClone(theme);
    if (!updated[section]) (updated as any)[section] = {};
    (updated[section] as Record<string, string | number>)[key] = value;
    onUpdate(updated);
  }

  function updateConditionalVariable(
    conditional: ConditionalTab,
    section: string,
    key: string,
    value: string | number
  ) {
    const updated = structuredClone(theme);
    if (!updated[conditional]) (updated as any)[conditional] = {};
    const cond = updated[conditional] as Record<string, Record<string, string | number>>;
    if (!cond[section]) cond[section] = {};
    cond[section][key] = value;
    onUpdate(updated);
  }

  function handleTriggerChange(e: Event) {
    const val = (e.target as HTMLSelectElement).value;
    const updated = structuredClone(theme);
    updated.meta.trigger = val;
    onUpdate(updated);
  }

  let conditionalSections = $derived<string[]>(
    Object.keys((theme[activeConditionalTab] as Record<string, unknown>) ?? {})
  );

  let conditionalVariables = $derived<Record<string, string | number>>(
    activeConditionalTab && activeConditionalSection
      ? (((theme[activeConditionalTab] as any)?.[activeConditionalSection]) ?? {})
      : {}
  );
</script>

<div class="modifier-sections">
  <!-- Trigger selector -->
  <div class="trigger-row">
    <label class="trigger-label" for="mod-trigger">Trigger</label>
    <select
      id="mod-trigger"
      class="trigger-select"
      value={theme.meta.trigger ?? 'manual'}
      onchange={handleTriggerChange}
    >
      <option value="manual">Manual</option>
      <option value="(prefers-color-scheme: dark)">Media: prefers-color-scheme dark</option>
      <option value="(prefers-color-scheme: light)">Media: prefers-color-scheme light</option>
      <option value="(prefers-contrast: more)">Media: prefers-contrast more</option>
      <option value="(prefers-reduced-motion: reduce)">Media: prefers-reduced-motion reduce</option>
    </select>
  </div>

  <!-- Section enable/disable checkboxes -->
  <div class="section-toggles">
    <h3 class="group-title">Active Sections</h3>
    <div class="toggles-grid">
      {#each THEME_SECTIONS as section}
        <label class="toggle-row">
          <input
            type="checkbox"
            checked={isSectionEnabled(section)}
            onchange={() => toggleSection(section)}
            class="toggle-checkbox"
          />
          <span class="toggle-label">{section}</span>
        </label>
      {/each}
    </div>
  </div>

  <!-- Enabled sections' variables -->
  {#each THEME_SECTIONS as section}
    {#if isSectionEnabled(section)}
      <ThemeEditorSection
        sectionName={section}
        variables={(theme[section] as Record<string, string | number>) ?? {}}
        onUpdate={(key, value) => updateSectionVariable(section, key, value)}
      />
    {/if}
  {/each}

  <!-- Conditional overrides (when-dark / when-light) -->
  <div class="conditionals">
    <h3 class="group-title">Conditional Overrides</h3>

    <div class="conditional-tabs" role="tablist">
      {#each (['when-dark', 'when-light'] as ConditionalTab[]) as tab}
        <button
          class="cond-tab"
          class:active={activeConditionalTab === tab}
          onclick={() => { activeConditionalTab = tab; }}
          role="tab"
          aria-selected={activeConditionalTab === tab}
        >{tab}</button>
      {/each}
    </div>

    {#if conditionalSections.length > 0}
      <div class="cond-section-tabs">
        {#each conditionalSections as sec}
          <button
            class="cond-section-tab"
            class:active={activeConditionalSection === sec}
            onclick={() => { activeConditionalSection = sec; }}
          >{sec}</button>
        {/each}
      </div>
      <ThemeEditorSection
        sectionName={activeConditionalSection}
        variables={conditionalVariables}
        onUpdate={(key, value) => updateConditionalVariable(activeConditionalTab, activeConditionalSection, key, value)}
      />
    {:else}
      <p class="empty-cond">No {activeConditionalTab} overrides defined.</p>
    {/if}
  </div>
</div>

<style>
  .modifier-sections {
    padding: var(--space-4, 16px);
    display: flex;
    flex-direction: column;
    gap: var(--space-4, 16px);
    overflow-y: auto;
  }

  .trigger-row {
    display: flex;
    align-items: center;
    gap: var(--space-3, 12px);
  }

  .trigger-label {
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
    width: 70px;
    flex-shrink: 0;
  }

  .trigger-select {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-ui, sans-serif);
    padding: 4px 8px;
    flex: 1;
  }

  .group-title {
    font-size: 11px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    margin: 0 0 var(--space-2, 8px) 0;
  }

  .toggles-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: var(--space-1, 4px);
  }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
    cursor: pointer;
    padding: 4px;
  }

  .toggle-checkbox {
    accent-color: var(--accent);
    cursor: pointer;
  }

  .toggle-label {
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    color: var(--text-secondary);
  }

  .conditionals {
    border-top: 1px solid var(--border);
    padding-top: var(--space-4, 16px);
  }

  .conditional-tabs {
    display: flex;
    gap: 2px;
    margin-bottom: var(--space-3, 12px);
  }

  .cond-tab {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    padding: 4px 12px;
    cursor: pointer;
  }

  .cond-tab.active {
    background: var(--bg-hover);
    border-color: var(--accent);
    color: var(--accent-text);
  }

  .cond-section-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    margin-bottom: var(--space-2, 8px);
  }

  .cond-section-tab {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    padding: 2px 8px;
    cursor: pointer;
  }

  .cond-section-tab.active {
    border-color: var(--accent);
    color: var(--accent-text);
  }

  .empty-cond {
    font-size: 12px;
    color: var(--text-muted);
    font-style: italic;
    padding: var(--space-3, 12px) 0;
  }
</style>
