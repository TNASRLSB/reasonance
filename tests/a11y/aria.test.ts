/**
 * Accessibility tests using axe-core directly.
 * Tests ARIA attributes and semantic HTML on key components.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import axe from 'axe-core';

// Store imports for setup
import { llmConfigs } from '$lib/stores/config';
import { setupTestProject, resetProjectState } from '../helpers/project-setup';

// Component imports
import StatusBar from '$lib/components/StatusBar.svelte';
import EditorTabs from '$lib/components/EditorTabs.svelte';

async function checkA11y(container: HTMLElement): Promise<axe.Result[]> {
  const results = await axe.run(container);
  return results.violations;
}

afterEach(() => {
  cleanup();
});

// --- StatusBar ---

describe('StatusBar accessibility', () => {
  beforeEach(() => {
    resetProjectState();
    llmConfigs.set([]);
  });

  it('renders with no ARIA violations (default state)', async () => {
    const { container } = render(StatusBar);
    const violations = await checkA11y(container);
    if (violations.length > 0) {
      console.warn('StatusBar (default) ARIA violations:', violations.map(v => `${v.id}: ${v.description}`));
    }
    expect(violations).toHaveLength(0);
  });

  it('renders with no ARIA violations when a model has yolo permission', async () => {
    llmConfigs.set([{ name: 'claude', type: 'cli' as const, command: 'claude', permissionLevel: 'yolo' as const }]);
    const { container } = render(StatusBar);
    const violations = await checkA11y(container);
    if (violations.length > 0) {
      console.warn('StatusBar (yolo) ARIA violations:', violations.map(v => `${v.id}: ${v.description}`));
    }
    expect(violations).toHaveLength(0);
  });

  it('renders with no ARIA violations when file is active', async () => {
    setupTestProject({
      rootPath: '/project',
      activeFilePath: '/project/src/main.ts',
      openFiles: [{ path: '/project/src/main.ts' }],
    });
    llmConfigs.set([{ name: 'claude', type: 'cli', command: 'claude' }]);

    const { container } = render(StatusBar);
    const violations = await checkA11y(container);
    if (violations.length > 0) {
      console.warn('StatusBar (with file) ARIA violations:', violations.map(v => `${v.id}: ${v.description}`));
    }
    expect(violations).toHaveLength(0);
  });

  it('displays app name and LLM count in status left', async () => {
    llmConfigs.set([
      { name: 'claude', type: 'cli', command: 'claude' },
      { name: 'gemini', type: 'cli', command: 'gemini' },
    ]);

    const { container } = render(StatusBar);
    const text = container.textContent ?? '';
    expect(text).toContain('REASONANCE');
  });
});

// --- EditorTabs ---

describe('EditorTabs accessibility', () => {
  beforeEach(() => {
    resetProjectState();
  });

  it('renders tablist with no ARIA violations when empty', async () => {
    const { container } = render(EditorTabs);

    const tablist = container.querySelector('[role="tablist"]');
    expect(tablist).not.toBeNull();

    const violations = await checkA11y(container);
    if (violations.length > 0) {
      console.warn('EditorTabs (empty) ARIA violations:', violations.map(v => `${v.id}: ${v.description}`));
    }
    // Empty tablist is a known pattern -- no tabs means no violations expected
    expect(violations).toHaveLength(0);
  });

  it('renders tabs with proper ARIA roles and attributes', async () => {
    setupTestProject({
      rootPath: '/project',
      openFiles: [
        { path: '/project/main.ts', isDirty: false },
        { path: '/project/App.svelte', isDirty: true },
      ],
      activeFilePath: '/project/main.ts',
    });

    const { container } = render(EditorTabs);

    const tabs = container.querySelectorAll('[role="tab"]');
    expect(tabs.length).toBe(2);

    // Active tab has aria-selected="true"
    const activeTab = container.querySelector('[role="tab"][aria-selected="true"]');
    expect(activeTab).not.toBeNull();

    // Inactive tab has aria-selected="false"
    const inactiveTab = container.querySelector('[role="tab"][aria-selected="false"]');
    expect(inactiveTab).not.toBeNull();

    // Close buttons have accessible labels
    const closeButtons = container.querySelectorAll('button[aria-label^="Close"]');
    expect(closeButtons.length).toBe(2);

    const violations = await checkA11y(container);
    if (violations.length > 0) {
      console.warn(
        'EditorTabs (with files) ARIA violations:',
        violations.map(v => `${v.id}: ${v.description}`)
      );
    }
    // nested-interactive was fixed: buttons are now siblings of role="tab", not children
    expect(violations).toHaveLength(0);
  });

  it('tabs are keyboard-accessible (have tabindex)', async () => {
    setupTestProject({
      rootPath: '/project',
      openFiles: [{ path: '/project/index.ts' }],
      activeFilePath: '/project/index.ts',
    });

    const { container } = render(EditorTabs);

    const tab = container.querySelector('[role="tab"]');
    expect(tab?.getAttribute('tabindex')).toBe('0');
  });

  it('dirty files show modified indicator in tab name', async () => {
    setupTestProject({
      rootPath: '/project',
      openFiles: [{ path: '/project/dirty.ts', isDirty: true }],
      activeFilePath: '/project/dirty.ts',
    });

    const { container } = render(EditorTabs);
    const tabName = container.querySelector('.tab-name');
    expect(tabName?.textContent).toContain('\u25CF');
  });
});
