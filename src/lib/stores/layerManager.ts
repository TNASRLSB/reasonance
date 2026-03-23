import { writable, derived, get } from 'svelte/store';
import { findFocusFallback } from '$lib/utils/focus-trap';

export interface LayerEntry {
  id: string;
  type: 'dropdown' | 'overlay' | 'modal' | 'toast';
  returnFocus: HTMLElement | null;
  onClose?: () => void;
}

const _stack = writable<LayerEntry[]>([]);

export const layerStack = { subscribe: _stack.subscribe };

export const topLayer = derived(_stack, ($stack) =>
  $stack.length > 0 ? $stack[$stack.length - 1] : null
);

export const hasOpenModal = derived(_stack, ($stack) =>
  $stack.some(l => l.type === 'modal')
);

export function pushLayer(entry: LayerEntry) {
  _stack.update(stack => {
    if (stack.some(l => l.id === entry.id)) return stack;
    return [...stack, entry];
  });
  updateInert();
  updateScrollLock();
}

export function popLayer(id?: string) {
  const stack = get(_stack);
  const idx = id ? stack.findIndex(l => l.id === id) : stack.length - 1;
  if (idx === -1 || stack.length === 0) return;
  const popped = stack[idx];
  _stack.set([...stack.slice(0, idx), ...stack.slice(idx + 1)]);

  const target = findFocusFallback(popped.returnFocus);
  target.focus();
  popped.onClose?.();

  updateInert();
  updateScrollLock();
}

export function handleGlobalEscape(e: KeyboardEvent) {
  if (e.key !== 'Escape') return;

  const stack = get(_stack);
  if (stack.length === 0) return;

  e.preventDefault();
  e.stopPropagation();
  popLayer();
}

function updateInert() {
  const stack = get(_stack);
  const mainContent = document.querySelector('[data-main-content]');

  if (!mainContent) return;

  const hasModal = stack.some(l => l.type === 'modal' || l.type === 'overlay');
  if (hasModal) {
    mainContent.setAttribute('inert', '');
  } else {
    mainContent.removeAttribute('inert');
  }
}

function updateScrollLock() {
  const stack = get(_stack);
  const hasModal = stack.some(l => l.type === 'modal');
  document.body.classList.toggle('layer-modal-open', hasModal);
}

export function initLayerManager() {
  document.addEventListener('keydown', handleGlobalEscape);

  return () => {
    document.removeEventListener('keydown', handleGlobalEscape);
  };
}
