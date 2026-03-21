import { describe, it, expect, beforeEach, vi } from 'vitest';
import { registerKeybinding, initKeybindings } from '$lib/utils/keybindings';

describe('keybindings', () => {
  beforeEach(() => {
    // Reset event listeners between tests by replacing addEventListener
    vi.restoreAllMocks();
  });

  describe('registerKeybinding', () => {
    it('does not throw when registering a simple key', () => {
      expect(() => registerKeybinding('ctrl+s', () => {})).not.toThrow();
    });

    it('does not throw when registering a multi-modifier combo', () => {
      expect(() => registerKeybinding('ctrl+shift+p', () => {})).not.toThrow();
    });

    it('does not throw when registering a single key', () => {
      expect(() => registerKeybinding('f1', () => {})).not.toThrow();
    });

    it('normalises key to lowercase', () => {
      // Should not throw even with mixed case (lowercased internally)
      expect(() => registerKeybinding('Ctrl+S', () => {})).not.toThrow();
    });

    it('allows overwriting a previously registered key', () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();
      expect(() => {
        registerKeybinding('ctrl+k', handler1);
        registerKeybinding('ctrl+k', handler2);
      }).not.toThrow();
    });
  });

  describe('initKeybindings', () => {
    it('does not throw when called', () => {
      expect(() => initKeybindings()).not.toThrow();
    });

    it('attaches a keydown listener to window', () => {
      const spy = vi.spyOn(window, 'addEventListener');
      initKeybindings();
      expect(spy).toHaveBeenCalledWith('keydown', expect.any(Function));
    });

    it('invokes registered handler on matching keydown event', () => {
      const handler = vi.fn();
      registerKeybinding('ctrl+z', handler);
      initKeybindings();

      const event = new KeyboardEvent('keydown', {
        key: 'z',
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      });
      window.dispatchEvent(event);

      expect(handler).toHaveBeenCalled();
    });

    it('does not invoke handler on non-matching keydown event', () => {
      const handler = vi.fn();
      registerKeybinding('ctrl+q', handler);
      initKeybindings();

      const event = new KeyboardEvent('keydown', {
        key: 'w',
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      });
      window.dispatchEvent(event);

      expect(handler).not.toHaveBeenCalled();
    });

    it('prevents default on matching keydown event', () => {
      registerKeybinding('alt+f4', () => {});
      initKeybindings();

      const event = new KeyboardEvent('keydown', {
        key: 'F4',
        altKey: true,
        bubbles: true,
        cancelable: true,
      });
      const preventDefaultSpy = vi.spyOn(event, 'preventDefault');
      window.dispatchEvent(event);

      expect(preventDefaultSpy).toHaveBeenCalled();
    });
  });
});
