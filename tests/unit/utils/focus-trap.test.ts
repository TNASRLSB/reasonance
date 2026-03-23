import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createFocusTrap, findFocusFallback } from '$lib/utils/focus-trap';

// jsdom does not compute layout so offsetParent is always null.
// Stub it to return a non-null value so visibility checks pass.
function makeVisible(el: HTMLElement) {
  Object.defineProperty(el, 'offsetParent', {
    configurable: true,
    get: () => document.body,
  });
}

describe('createFocusTrap', () => {
  let container: HTMLElement;

  beforeEach(() => {
    container = document.createElement('div');
    const first = document.createElement('button');
    first.id = 'first';
    first.textContent = 'First';
    const middle = document.createElement('input');
    middle.id = 'middle';
    const last = document.createElement('button');
    last.id = 'last';
    last.textContent = 'Last';
    [first, middle, last].forEach(el => {
      makeVisible(el);
      container.appendChild(el);
    });
    document.body.appendChild(container);
  });

  afterEach(() => {
    container.remove();
  });

  it('traps Tab within container', () => {
    const trap = createFocusTrap(container);
    const last = container.querySelector('#last') as HTMLElement;
    last.focus();

    const event = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true });
    container.dispatchEvent(event);

    // Tab on last element should be prevented (wraps to first)
    expect(event.defaultPrevented).toBe(true);
    trap.destroy();
  });

  it('traps Shift+Tab within container', () => {
    const trap = createFocusTrap(container);
    const first = container.querySelector('#first') as HTMLElement;
    first.focus();

    const event = new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true, cancelable: true });
    container.dispatchEvent(event);

    // Shift+Tab on first element should be prevented (wraps to last)
    expect(event.defaultPrevented).toBe(true);
    trap.destroy();
  });

  it('focuses first focusable element on activate', () => {
    const trap = createFocusTrap(container, { initialFocus: true });
    expect(document.activeElement?.id).toBe('first');
    trap.destroy();
  });

  it('returns destroy function that removes listeners', () => {
    const trap = createFocusTrap(container);
    trap.destroy();
    // Should not throw after destroy
  });
});

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
    // Not in DOM
    expect(findFocusFallback(el)).toBe(document.body);
  });
});
