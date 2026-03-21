import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import Settings from '$lib/components/Settings.svelte';
import { llmConfigs, appSettings } from '$lib/stores/config';
import { createMockAdapter } from '../../mocks/adapter';

beforeEach(() => {
  llmConfigs.set([]);
  appSettings.set({});
});

describe('Settings component', () => {
  it('renders nothing when visible is false', () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(Settings, { props: { adapter, visible: false, onClose } });
    const modal = document.querySelector('.settings-modal, .settings-overlay, [role="dialog"]');
    expect(modal).toBeNull();
  });

  it('renders the settings dialog when visible is true', async () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(Settings, { props: { adapter, visible: true, onClose } });
    await new Promise((r) => setTimeout(r, 20));
    const dialog = document.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
  });

  it('renders a close button when visible', async () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(Settings, { props: { adapter, visible: true, onClose } });
    await new Promise((r) => setTimeout(r, 20));
    // Close button should have an aria-label or be a button with × character
    const closeBtn = document.querySelector('button[aria-label*="lose"], button.settings-close, button.close-btn');
    expect(closeBtn).not.toBeNull();
  });

  it('renders at least one tab or section heading when visible', async () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(Settings, { props: { adapter, visible: true, onClose } });
    await new Promise((r) => setTimeout(r, 20));
    // Settings typically have tabs or headings
    const headings = document.querySelectorAll('h1, h2, h3, [role="tab"], .settings-tab, .tab-label');
    expect(headings.length).toBeGreaterThan(0);
  });

  it('renders an "Add LLM" button or similar action when visible', async () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(Settings, { props: { adapter, visible: true, onClose } });
    await new Promise((r) => setTimeout(r, 20));
    const buttons = document.querySelectorAll('button');
    // Should have at least 2 buttons (close + some action)
    expect(buttons.length).toBeGreaterThanOrEqual(2);
  });

  it('renders locale selector or theme selector when visible', async () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(Settings, { props: { adapter, visible: true, onClose } });
    await new Promise((r) => setTimeout(r, 20));
    // Settings should have at least one select element (theme, locale, etc.)
    const selects = document.querySelectorAll('select');
    expect(selects.length).toBeGreaterThan(0);
  });
});
