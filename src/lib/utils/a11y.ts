/**
 * Creates a focus trap within a container element.
 * Returns a destroy function to remove event listeners.
 */
export function trapFocus(container: HTMLElement): () => void {
  const focusableSelectors = 'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

  function handleKeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return;

    const focusable = Array.from(container.querySelectorAll<HTMLElement>(focusableSelectors)).filter(el => el.offsetParent !== null);
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.shiftKey) {
      if (document.activeElement === first) {
        e.preventDefault();
        last.focus();
      }
    } else {
      if (document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  container.addEventListener('keydown', handleKeydown);

  // Focus first focusable element
  const focusable = container.querySelectorAll<HTMLElement>(focusableSelectors);
  if (focusable.length > 0) focusable[0].focus();

  return () => container.removeEventListener('keydown', handleKeydown);
}

/**
 * Handles arrow key navigation within a menu/list container.
 * Manages roving tabindex on items matching the selector.
 */
export function menuKeyHandler(e: KeyboardEvent, container: HTMLElement, itemSelector: string = '[role="menuitem"]'): void {
  const items = Array.from(container.querySelectorAll<HTMLElement>(itemSelector)).filter(el => el.offsetParent !== null);
  if (items.length === 0) return;

  const currentIndex = items.indexOf(document.activeElement as HTMLElement);
  let nextIndex = -1;

  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault();
      nextIndex = currentIndex < items.length - 1 ? currentIndex + 1 : 0;
      break;
    case 'ArrowUp':
      e.preventDefault();
      nextIndex = currentIndex > 0 ? currentIndex - 1 : items.length - 1;
      break;
    case 'Home':
      e.preventDefault();
      nextIndex = 0;
      break;
    case 'End':
      e.preventDefault();
      nextIndex = items.length - 1;
      break;
  }

  if (nextIndex >= 0) {
    items[nextIndex].focus();
  }
}

/**
 * Handles arrow key navigation within a toolbar container.
 * Uses left/right arrows (horizontal navigation) with roving tabindex.
 */
export function toolbarKeyHandler(e: KeyboardEvent, container: HTMLElement, selector = 'button:not([disabled])') {
  const items = Array.from(container.querySelectorAll<HTMLElement>(selector));
  const current = items.indexOf(document.activeElement as HTMLElement);
  if (current === -1) return;

  let next = -1;
  switch (e.key) {
    case 'ArrowRight': next = (current + 1) % items.length; break;
    case 'ArrowLeft': next = (current - 1 + items.length) % items.length; break;
    case 'Home': next = 0; break;
    case 'End': next = items.length - 1; break;
    default: return;
  }

  e.preventDefault();
  items.forEach((item, i) => item.tabIndex = i === next ? 0 : -1);
  items[next].focus();
}

/** Sanitize a string for use as an HTML id attribute */
export function sanitizeId(path: string): string {
  return path.replace(/[^a-zA-Z0-9-_]/g, '-');
}
