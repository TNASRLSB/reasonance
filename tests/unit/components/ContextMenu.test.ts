import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import ContextMenu from '$lib/components/ContextMenu.svelte';
import { llmConfigs, appSettings } from '$lib/stores/config';
import { createMockAdapter } from '../../mocks/adapter';
import { resetProjectState } from '../../helpers/project-setup';

beforeEach(() => {
  resetProjectState();
  llmConfigs.set([]);
  appSettings.set({});
});

describe('ContextMenu component', () => {
  it('renders nothing when visible is false', () => {
    const adapter = createMockAdapter();
    render(ContextMenu, {
      props: {
        adapter,
        x: 100,
        y: 200,
        visible: false,
        selectedText: 'some code',
        onResponse: vi.fn(),
        onClose: vi.fn(),
      },
    });
    const menu = document.querySelector('[role="menu"]');
    expect(menu).toBeNull();
  });

  it('renders the context menu when visible is true', () => {
    const adapter = createMockAdapter();
    render(ContextMenu, {
      props: {
        adapter,
        x: 100,
        y: 200,
        visible: true,
        selectedText: 'const x = 1;',
        onResponse: vi.fn(),
        onClose: vi.fn(),
      },
    });
    const menu = document.querySelector('[role="menu"]');
    expect(menu).not.toBeNull();
  });

  it('positions the context menu at the given x/y coordinates', () => {
    const adapter = createMockAdapter();
    render(ContextMenu, {
      props: {
        adapter,
        x: 150,
        y: 300,
        visible: true,
        selectedText: 'code',
        onResponse: vi.fn(),
        onClose: vi.fn(),
      },
    });
    const menu = document.querySelector('.context-menu') as HTMLElement;
    expect(menu?.style.left).toBe('150px');
    expect(menu?.style.top).toBe('300px');
  });

  it('renders 4 action buttons (Explain, Rewrite, Find Bugs, Document)', () => {
    const adapter = createMockAdapter();
    render(ContextMenu, {
      props: {
        adapter,
        x: 0,
        y: 0,
        visible: true,
        selectedText: 'function foo() {}',
        onResponse: vi.fn(),
        onClose: vi.fn(),
      },
    });
    const items = document.querySelectorAll('[role="menuitem"]');
    expect(items.length).toBe(4);
  });

  it('renders disabled buttons when no LLM is configured', () => {
    llmConfigs.set([]);
    const adapter = createMockAdapter();
    render(ContextMenu, {
      props: {
        adapter,
        x: 0,
        y: 0,
        visible: true,
        selectedText: 'some code',
        onResponse: vi.fn(),
        onClose: vi.fn(),
      },
    });
    const disabledItems = document.querySelectorAll('.context-menu-item.disabled');
    expect(disabledItems.length).toBe(4);
  });

  it('shows a hint message when no LLM is configured', () => {
    llmConfigs.set([]);
    const adapter = createMockAdapter();
    render(ContextMenu, {
      props: {
        adapter,
        x: 0,
        y: 0,
        visible: true,
        selectedText: 'code',
        onResponse: vi.fn(),
        onClose: vi.fn(),
      },
    });
    const hint = document.querySelector('.context-menu-hint');
    expect(hint).not.toBeNull();
    expect(hint?.textContent).toContain('Configure an LLM');
  });

  it('renders backdrop element when visible', () => {
    const adapter = createMockAdapter();
    render(ContextMenu, {
      props: {
        adapter,
        x: 10,
        y: 10,
        visible: true,
        selectedText: 'hello',
        onResponse: vi.fn(),
        onClose: vi.fn(),
      },
    });
    const backdrop = document.querySelector('.context-menu-backdrop');
    expect(backdrop).not.toBeNull();
  });

  it('calls onClose when backdrop is clicked', async () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(ContextMenu, {
      props: {
        adapter,
        x: 0,
        y: 0,
        visible: true,
        selectedText: 'code',
        onResponse: vi.fn(),
        onClose,
      },
    });
    const backdrop = document.querySelector('.context-menu-backdrop') as HTMLElement;
    backdrop?.click();
    await new Promise((r) => setTimeout(r, 0));
    expect(onClose).toHaveBeenCalled();
  });
});
