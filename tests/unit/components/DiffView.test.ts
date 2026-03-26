import { describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import DiffView from '$lib/components/DiffView.svelte';
import { createMockAdapter } from '../../mocks/adapter';

// DiffView uses CodeMirror MergeView which requires DOM measurement.
// jsdom cannot run the full CM6 layout, so we test mount/unmount and prop acceptance.
// The component should at minimum render its outer container without throwing.

describe('DiffView component', () => {
  it('mounts without throwing an error', () => {
    const adapter = createMockAdapter();
    const onAccept = vi.fn();
    const onReject = vi.fn();

    expect(() => {
      render(DiffView, {
        props: {
          adapter,
          original: 'const x = 1;',
          modified: 'const x = 2;',
          filename: 'example.ts',
          filePath: '/proj/example.ts',
          onAccept,
          onReject,
        },
      });
    }).not.toThrow();
  });

  it.skip('renders the diff container element (requires CodeMirror DOM — run via Playwright)', () => {});

  it('renders accept and reject buttons', () => {
    const adapter = createMockAdapter();
    const onAccept = vi.fn();
    const onReject = vi.fn();

    render(DiffView, {
      props: {
        adapter,
        original: 'old',
        modified: 'new',
        filename: 'file.ts',
        filePath: '/proj/file.ts',
        onAccept,
        onReject,
      },
    });

    const buttons = document.querySelectorAll('button');
    // At minimum accept and reject buttons should be present
    expect(buttons.length).toBeGreaterThanOrEqual(2);
  });

  it('renders the filename in the header', () => {
    const adapter = createMockAdapter();
    const onAccept = vi.fn();
    const onReject = vi.fn();

    render(DiffView, {
      props: {
        adapter,
        original: 'a',
        modified: 'b',
        filename: 'myfile.ts',
        filePath: '/proj/myfile.ts',
        onAccept,
        onReject,
      },
    });

    const bodyText = document.body.textContent ?? '';
    expect(bodyText).toContain('myfile.ts');
  });

  it.skip('calls onAccept when accept button is clicked (requires CodeMirror DOM — run via Playwright)', () => {});
});
