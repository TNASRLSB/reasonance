/**
 * Global vitest setup — provides a working localStorage mock for jsdom/forks pool,
 * where Node's built-in localStorage may be partially initialized.
 */
import { beforeEach } from 'vitest';

const store = new Map<string, string>();

const localStorageMock: Storage = {
  getItem: (key: string) => store.get(key) ?? null,
  setItem: (key: string, value: string) => { store.set(key, String(value)); },
  removeItem: (key: string) => { store.delete(key); },
  clear: () => { store.clear(); },
  get length() { return store.size; },
  key: (index: number) => [...store.keys()][index] ?? null,
};

Object.defineProperty(globalThis, 'localStorage', {
  value: localStorageMock,
  writable: true,
  configurable: true,
});

globalThis.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
} as unknown as typeof ResizeObserver;

if (typeof CSS === 'undefined' || !CSS.escape) {
  (globalThis as Record<string, unknown>).CSS = {
    escape: (s: string) => s.replace(/([^\w-])/g, '\\$1'),
  };
}

Element.prototype.scrollIntoView = Element.prototype.scrollIntoView || function () {};

beforeEach(() => {
  store.clear();
});
