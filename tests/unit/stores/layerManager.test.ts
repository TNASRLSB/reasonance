import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';

// Will import from implementation
let layerManager: typeof import('$lib/stores/layerManager');

beforeEach(async () => {
  // Fresh import for each test
  vi.resetModules();
  layerManager = await import('$lib/stores/layerManager');
});

describe('layerManager', () => {
  it('starts with empty stack', () => {
    expect(get(layerManager.layerStack)).toEqual([]);
    expect(get(layerManager.topLayer)).toBeNull();
  });

  it('pushes a layer onto the stack', () => {
    const returnFocus = document.createElement('button');
    layerManager.pushLayer({
      id: 'test-modal',
      type: 'modal',
      returnFocus
    });

    const stack = get(layerManager.layerStack);
    expect(stack).toHaveLength(1);
    expect(stack[0].id).toBe('test-modal');
    expect(get(layerManager.topLayer)?.id).toBe('test-modal');
  });

  it('pops the top layer', () => {
    layerManager.pushLayer({ id: 'layer-1', type: 'modal', returnFocus: null });
    layerManager.pushLayer({ id: 'layer-2', type: 'dropdown', returnFocus: null });

    layerManager.popLayer();
    expect(get(layerManager.layerStack)).toHaveLength(1);
    expect(get(layerManager.topLayer)?.id).toBe('layer-1');
  });

  it('pops a specific layer by id', () => {
    layerManager.pushLayer({ id: 'layer-1', type: 'modal', returnFocus: null });
    layerManager.pushLayer({ id: 'layer-2', type: 'dropdown', returnFocus: null });

    layerManager.popLayer('layer-1');
    expect(get(layerManager.layerStack)).toHaveLength(1);
    expect(get(layerManager.topLayer)?.id).toBe('layer-2');
  });

  it('hasOpenModal returns true when modal layer exists', () => {
    expect(get(layerManager.hasOpenModal)).toBe(false);

    layerManager.pushLayer({ id: 'test', type: 'modal', returnFocus: null });
    expect(get(layerManager.hasOpenModal)).toBe(true);
  });

  it('handleGlobalEscape pops top layer on Escape', () => {
    layerManager.pushLayer({ id: 'layer-1', type: 'modal', returnFocus: null });
    layerManager.pushLayer({ id: 'layer-2', type: 'dropdown', returnFocus: null });

    const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true });
    layerManager.handleGlobalEscape(event);

    expect(get(layerManager.layerStack)).toHaveLength(1);
    expect(get(layerManager.topLayer)?.id).toBe('layer-1');
  });

  it('handleGlobalEscape is no-op on empty stack', () => {
    const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true });
    layerManager.handleGlobalEscape(event);
    expect(get(layerManager.layerStack)).toEqual([]);
  });

  it('pushLayer ignores duplicate ids', () => {
    layerManager.pushLayer({ id: 'test', type: 'modal', returnFocus: null });
    layerManager.pushLayer({ id: 'test', type: 'dropdown', returnFocus: null });
    expect(get(layerManager.layerStack)).toHaveLength(1);
  });
});
