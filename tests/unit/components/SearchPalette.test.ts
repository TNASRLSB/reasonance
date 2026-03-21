import { describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import SearchPalette from '$lib/components/SearchPalette.svelte';
import { projectRoot } from '$lib/stores/files';
import { createMockAdapter } from '../../mocks/adapter';

describe('SearchPalette component', () => {
  it('renders nothing when visible is false', () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(SearchPalette, { props: { adapter, visible: false, onClose } });
    const overlay = document.querySelector('.palette-overlay');
    expect(overlay).toBeNull();
  });

  it('renders the overlay when visible is true', () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(SearchPalette, { props: { adapter, visible: true, onClose } });
    const overlay = document.querySelector('.palette-overlay');
    expect(overlay).not.toBeNull();
  });

  it('renders the palette dialog with aria-modal attribute', () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(SearchPalette, { props: { adapter, visible: true, onClose } });
    const dialog = document.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    expect(dialog?.getAttribute('aria-modal')).toBe('true');
  });

  it('renders the search input with correct placeholder', () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(SearchPalette, { props: { adapter, visible: true, onClose } });
    const input = document.querySelector('input.palette-input');
    expect(input).not.toBeNull();
    expect((input as HTMLInputElement)?.placeholder).toBe('Go to file…');
  });

  it('renders ESC hint when visible', () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(SearchPalette, { props: { adapter, visible: true, onClose } });
    const hint = document.querySelector('.palette-hint');
    // hint shows "Loading..." initially or "ESC to close"
    expect(hint).not.toBeNull();
  });

  it('does not render overlay when not visible', () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    const { unmount } = render(SearchPalette, { props: { adapter, visible: false, onClose } });
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    unmount();
  });

  it('renders dialog with aria-label "File search"', () => {
    const adapter = createMockAdapter();
    const onClose = vi.fn();
    render(SearchPalette, { props: { adapter, visible: true, onClose } });
    const dialog = document.querySelector('[aria-label="File search"]');
    expect(dialog).not.toBeNull();
  });
});
