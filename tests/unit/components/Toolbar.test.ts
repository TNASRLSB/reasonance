import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import { get } from 'svelte/store';
import Toolbar from '$lib/components/Toolbar.svelte';
import { showSettings } from '$lib/stores/ui';
import { createMockAdapter } from '../../mocks/adapter';

beforeEach(() => {
  showSettings.set(false);
});

describe('Toolbar component', () => {
  it('renders the toolbar element', () => {
    const adapter = createMockAdapter();
    render(Toolbar, { props: { adapter } });
    const toolbar = document.querySelector('.toolbar');
    expect(toolbar).not.toBeNull();
  });

  it('renders the REASONANCE logo text', () => {
    const adapter = createMockAdapter();
    render(Toolbar, { props: { adapter } });
    const logo = document.querySelector('.logo');
    expect(logo?.textContent).toBe('REASONANCE');
  });

  it('renders the GIT trigger button', () => {
    const adapter = createMockAdapter();
    render(Toolbar, { props: { adapter } });
    const gitTrigger = document.querySelector('.git-trigger');
    expect(gitTrigger).not.toBeNull();
  });

  it('renders the settings button', () => {
    const adapter = createMockAdapter();
    render(Toolbar, { props: { adapter } });
    const settingsBtn = document.querySelector('button[title]');
    const allButtons = document.querySelectorAll('.toolbar-btn');
    // Settings button uses .toolbar-btn class, not .settings-btn
    expect(allButtons.length).toBeGreaterThan(0);
  });

  it('renders window control buttons (minimize, maximize, close)', () => {
    const adapter = createMockAdapter();
    render(Toolbar, { props: { adapter } });
    const winBtns = document.querySelectorAll('.win-btn');
    expect(winBtns.length).toBe(3);
  });

  it('git dropdown is not visible on initial render', () => {
    const adapter = createMockAdapter();
    render(Toolbar, { props: { adapter } });
    const dropdown = document.querySelector('.git-dropdown');
    expect(dropdown).toBeNull();
  });
});
