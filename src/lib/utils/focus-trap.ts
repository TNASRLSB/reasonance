const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export interface FocusTrapOptions {
  initialFocus?: boolean;
}

export interface FocusTrap {
  destroy: () => void;
}

export function createFocusTrap(container: HTMLElement, options: FocusTrapOptions = {}): FocusTrap {
  const { initialFocus = false } = options;

  function getFocusable(): HTMLElement[] {
    return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE))
      .filter(el => el.offsetParent !== null);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return;

    const focusable = getFocusable();
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }

  container.addEventListener('keydown', handleKeydown);

  if (initialFocus) {
    const focusable = getFocusable();
    if (focusable.length > 0) {
      focusable[0].focus();
    }
  }

  return {
    destroy() {
      container.removeEventListener('keydown', handleKeydown);
    }
  };
}

export function findFocusFallback(
  original: HTMLElement | null,
  container?: HTMLElement
): HTMLElement {
  // 1. Original element still in DOM and visible
  if (original && original.isConnected && original.offsetParent !== null) {
    return original;
  }

  // 2. Nearest focusable in container
  if (container) {
    const focusable = Array.from(
      container.querySelectorAll<HTMLElement>(FOCUSABLE)
    ).filter(el => el.offsetParent !== null);
    if (focusable.length > 0) return focusable[0];
  }

  // 3. Container itself if focusable
  if (container && container.tabIndex >= 0) return container;

  // 4. Last resort
  return document.body;
}
