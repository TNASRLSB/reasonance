import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import { get } from 'svelte/store';
import Toolbar from '$lib/components/Toolbar.svelte';
import { yoloMode, showSettings } from '$lib/stores/ui';
import { createMockAdapter } from '../../mocks/adapter';

beforeEach(() => {
  yoloMode.set(false);
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

  it('renders the YOLO button', () => {
    const adapter = createMockAdapter();
    render(Toolbar, { props: { adapter } });
    const yoloBtn = document.querySelector('.yolo-btn');
    expect(yoloBtn).not.toBeNull();
    expect(yoloBtn?.textContent).toBe('YOLO');
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
    const settingsBtn = document.querySelector('.settings-btn');
    expect(settingsBtn).not.toBeNull();
  });

  it('renders window control buttons (minimize, maximize, close)', () => {
    const adapter = createMockAdapter();
    render(Toolbar, { props: { adapter } });
    const winBtns = document.querySelectorAll('.win-btn');
    expect(winBtns.length).toBe(3);
  });

  it('reflects yoloMode=true state with active class on YOLO button', async () => {
    yoloMode.set(true);
    const adapter = createMockAdapter();
    render(Toolbar, { props: { adapter } });
    await new Promise((r) => setTimeout(r, 0));
    const yoloBtn = document.querySelector('.yolo-btn');
    expect(yoloBtn?.classList.contains('active')).toBe(true);
  });

  it('git dropdown is not visible on initial render', () => {
    const adapter = createMockAdapter();
    render(Toolbar, { props: { adapter } });
    const dropdown = document.querySelector('.git-dropdown');
    expect(dropdown).toBeNull();
  });
});
