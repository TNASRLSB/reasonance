const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

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
