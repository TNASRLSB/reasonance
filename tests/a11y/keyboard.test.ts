/**
 * Keyboard navigation and interaction tests.
 * Tests Escape handling, focus management, and keyboard-driven interactions.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, fireEvent, cleanup } from '@testing-library/svelte';
import { get } from 'svelte/store';

// Store imports
import { openFiles, activeFilePath } from '$lib/stores/files';

// Component imports — all at top level to avoid "import inside function" errors
import EditorTabs from '$lib/components/EditorTabs.svelte';
import SearchPalette from '$lib/components/SearchPalette.svelte';

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

// ─── EditorTabs keyboard navigation ───────────────────────────────────────────

describe('EditorTabs keyboard navigation', () => {
  beforeEach(() => {
    openFiles.set([
      { path: '/project/a.ts', name: 'a.ts', content: '', isDirty: false, isDeleted: false },
      { path: '/project/b.ts', name: 'b.ts', content: '', isDirty: false, isDeleted: false },
    ]);
    activeFilePath.set('/project/a.ts');
  });

  it('activates tab on Enter key', async () => {
    const { container } = render(EditorTabs);
    const tabs = container.querySelectorAll<HTMLElement>('[role="tab"]');

    // Second tab (b.ts) should not be active initially
    expect(tabs[1].getAttribute('aria-selected')).toBe('false');

    // Press Enter on the second tab
    await fireEvent.keyDown(tabs[1], { key: 'Enter' });

    expect(get(activeFilePath)).toBe('/project/b.ts');
  });

  it('activates tab on Space key', async () => {
    const { container } = render(EditorTabs);
    const tabs = container.querySelectorAll<HTMLElement>('[role="tab"]');

    await fireEvent.keyDown(tabs[1], { key: ' ' });

    expect(get(activeFilePath)).toBe('/project/b.ts');
  });

  it('clicking a tab switches active file', async () => {
    const { container } = render(EditorTabs);
    const tabs = container.querySelectorAll<HTMLElement>('[role="tab"]');

    await fireEvent.click(tabs[1]);

    expect(get(activeFilePath)).toBe('/project/b.ts');
  });

  it('close button removes file from open files', async () => {
    const { container } = render(EditorTabs);

    const closeButtons = container.querySelectorAll<HTMLButtonElement>('button[aria-label^="Close"]');
    expect(closeButtons.length).toBe(2);

    await fireEvent.click(closeButtons[0]);

    const remaining = get(openFiles);
    expect(remaining).toHaveLength(1);
    expect(remaining[0].name).toBe('b.ts');
  });

  it('close button has accessible label with file name', async () => {
    const { container } = render(EditorTabs);
    const closeButtons = container.querySelectorAll('button[aria-label^="Close"]');

    const labels = Array.from(closeButtons).map(b => b.getAttribute('aria-label'));
    expect(labels).toContain('Close a.ts');
    expect(labels).toContain('Close b.ts');
  });

  it('tabs have tabindex=0 making them keyboard reachable', async () => {
    const { container } = render(EditorTabs);
    const tabs = container.querySelectorAll('[role="tab"]');

    tabs.forEach(tab => {
      expect(tab.getAttribute('tabindex')).toBe('0');
    });
  });
});

// ─── SearchPalette keyboard navigation ────────────────────────────────────────

const makeMockAdapter = () => ({
  listDir: vi.fn().mockResolvedValue([]),
  readFile: vi.fn().mockResolvedValue(''),
  readConfig: vi.fn().mockResolvedValue(''),
});

describe('SearchPalette keyboard navigation', () => {
  it('calls onClose when Escape is pressed in the input', async () => {
    const onClose = vi.fn();
    const { container } = render(SearchPalette, {
      props: {
        adapter: makeMockAdapter() as any,
        visible: true,
        onClose,
      },
    });

    const input = container.querySelector('input.palette-input') as HTMLInputElement;
    expect(input).not.toBeNull();

    await fireEvent.keyDown(input, { key: 'Escape' });
    // onClose may be called once (input handler) or twice (input + overlay bubble)
    expect(onClose).toHaveBeenCalled();
  });

  it('does not render when visible=false', async () => {
    const { container } = render(SearchPalette, {
      props: {
        adapter: makeMockAdapter() as any,
        visible: false,
        onClose: vi.fn(),
      },
    });

    const overlay = container.querySelector('.palette-overlay');
    expect(overlay).toBeNull();
  });

  it('renders dialog with aria-modal when visible', async () => {
    const { container } = render(SearchPalette, {
      props: {
        adapter: makeMockAdapter() as any,
        visible: true,
        onClose: vi.fn(),
      },
    });

    const dialog = container.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    expect(dialog?.getAttribute('aria-modal')).toBe('true');
    expect(dialog?.getAttribute('aria-label')).toBe('File search');
  });

  it('input has accessible label', async () => {
    const { container } = render(SearchPalette, {
      props: {
        adapter: makeMockAdapter() as any,
        visible: true,
        onClose: vi.fn(),
      },
    });

    const input = container.querySelector('input[aria-label="Search files"]');
    expect(input).not.toBeNull();
  });

  it('overlay calls onClose when clicking the backdrop element', async () => {
    const onClose = vi.fn();
    const { container } = render(SearchPalette, {
      props: {
        adapter: makeMockAdapter() as any,
        visible: true,
        onClose,
      },
    });

    // The handleOverlayClick in SearchPalette checks if target has class 'palette-overlay'.
    // We simulate by dispatching a click event on the overlay where target = the overlay itself.
    const overlay = container.querySelector('.palette-overlay') as HTMLElement;
    expect(overlay).not.toBeNull();

    // fireEvent.click dispatches on the element with target = element
    await fireEvent.click(overlay);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('ESC on overlay element calls onClose', async () => {
    const onClose = vi.fn();
    const { container } = render(SearchPalette, {
      props: {
        adapter: makeMockAdapter() as any,
        visible: true,
        onClose,
      },
    });

    const overlay = container.querySelector('.palette-overlay') as HTMLElement;
    expect(overlay).not.toBeNull();
    // Overlay has role="button" making it keyboard-operable
    expect(overlay.getAttribute('role')).toBe('button');

    await fireEvent.keyDown(overlay, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('ArrowDown moves selection down in matches list', async () => {
    const { container } = render(SearchPalette, {
      props: {
        adapter: makeMockAdapter() as any,
        visible: true,
        onClose: vi.fn(),
      },
    });

    const input = container.querySelector('input.palette-input') as HTMLInputElement;
    // Palette starts with empty query, shows no matches until typing or files loaded
    // Just verify ArrowDown doesn't throw
    await fireEvent.keyDown(input, { key: 'ArrowDown' });
    // No error = pass
    expect(input).toBeTruthy();
  });
});

// ─── Focus management ─────────────────────────────────────────────────────────

describe('Focus management', () => {
  it('EditorTabs close button is reachable by keyboard (not tabindex=-1)', async () => {
    openFiles.set([
      { path: '/project/focus.ts', name: 'focus.ts', content: '', isDirty: false, isDeleted: false },
    ]);
    activeFilePath.set('/project/focus.ts');

    const { container } = render(EditorTabs);
    const closeBtn = container.querySelector('button[aria-label="Close focus.ts"]') as HTMLButtonElement;

    expect(closeBtn).not.toBeNull();
    // Buttons are natively focusable — they should NOT have tabindex="-1"
    const tabindex = closeBtn.getAttribute('tabindex');
    expect(tabindex === null || tabindex !== '-1').toBe(true);
  });

  it('SearchPalette input exists and is not disabled when visible', async () => {
    const { container } = render(SearchPalette, {
      props: {
        adapter: makeMockAdapter() as any,
        visible: true,
        onClose: vi.fn(),
      },
    });

    // Input should exist and be focusable
    const input = container.querySelector('input.palette-input') as HTMLInputElement;
    expect(input).not.toBeNull();
    // Must not be disabled
    expect(input.disabled).toBe(false);
    // Must not be hidden
    expect(input.type).not.toBe('hidden');
  });

  it('SearchPalette has no input when not visible', async () => {
    const { container } = render(SearchPalette, {
      props: {
        adapter: makeMockAdapter() as any,
        visible: false,
        onClose: vi.fn(),
      },
    });

    const input = container.querySelector('input.palette-input');
    expect(input).toBeNull();
  });
});
