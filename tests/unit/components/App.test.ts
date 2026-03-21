import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import App from '$lib/components/App.svelte';
import { fileTreeWidth, terminalWidth } from '$lib/stores/ui';
import { createMockAdapter } from '../../mocks/adapter';

beforeEach(() => {
  fileTreeWidth.set(200);
  terminalWidth.set(300);
});

describe('App component', () => {
  it('renders the app-root container', () => {
    const adapter = createMockAdapter();
    render(App, { props: { adapter } });
    const root = document.querySelector('.app-root');
    expect(root).not.toBeNull();
  });

  it('renders the main-content area', () => {
    const adapter = createMockAdapter();
    render(App, { props: { adapter } });
    const main = document.querySelector('.main-content');
    expect(main).not.toBeNull();
  });

  it('renders the file-tree panel', () => {
    const adapter = createMockAdapter();
    render(App, { props: { adapter } });
    const panel = document.querySelector('.panel.file-tree');
    expect(panel).not.toBeNull();
  });

  it('renders the editor panel', () => {
    const adapter = createMockAdapter();
    render(App, { props: { adapter } });
    const panel = document.querySelector('.panel.editor');
    expect(panel).not.toBeNull();
  });

  it('renders dividers for panel resizing', () => {
    const adapter = createMockAdapter();
    render(App, { props: { adapter } });
    const dividers = document.querySelectorAll('[role="separator"]');
    expect(dividers.length).toBeGreaterThanOrEqual(2);
  });

  it('applies file tree width from store as inline style', () => {
    fileTreeWidth.set(250);
    const adapter = createMockAdapter();
    render(App, { props: { adapter } });
    const fileTreePanel = document.querySelector('.panel.file-tree') as HTMLElement;
    expect(fileTreePanel?.style.width).toBe('250px');
  });

  it('renders placeholder text in file tree panel when no snippet provided', () => {
    const adapter = createMockAdapter();
    render(App, { props: { adapter } });
    // Without snippets, placeholders should render
    const placeholder = document.querySelector('.placeholder');
    expect(placeholder).not.toBeNull();
  });

  it('renders the Toolbar inside app-root', () => {
    const adapter = createMockAdapter();
    render(App, { props: { adapter } });
    const toolbar = document.querySelector('.toolbar');
    expect(toolbar).not.toBeNull();
  });
});
