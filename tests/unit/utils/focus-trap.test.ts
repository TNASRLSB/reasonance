import { describe, it, expect } from 'vitest';
import { findFocusFallback } from '$lib/utils/focus-trap';

function makeVisible(el: HTMLElement) {
  Object.defineProperty(el, 'offsetParent', {
    configurable: true,
    get: () => document.body,
  });
}

describe('findFocusFallback', () => {
  it('returns element if still in DOM and visible', () => {
    const el = document.createElement('button');
    makeVisible(el);
    document.body.appendChild(el);
    expect(findFocusFallback(el)).toBe(el);
    el.remove();
  });

  it('returns nearest focusable sibling if element removed', () => {
    const parent = document.createElement('div');
    const btn1 = document.createElement('button');
    btn1.id = 'sibling';
    const btn2 = document.createElement('button');
    btn2.id = 'target';
    makeVisible(btn1);
    makeVisible(btn2);
    parent.appendChild(btn1);
    parent.appendChild(btn2);
    document.body.appendChild(parent);

    btn2.remove();
    const fallback = findFocusFallback(btn2, parent);
    expect(fallback?.id).toBe('sibling');
    parent.remove();
  });

  it('returns document.body as last resort', () => {
    const el = document.createElement('button');
    expect(findFocusFallback(el)).toBe(document.body);
  });
});
