type Handler = () => void;
const bindings = new Map<string, Handler>();

export function registerKeybinding(key: string, handler: Handler) {
  bindings.set(key.toLowerCase(), handler);
}

export function initKeybindings() {
  window.addEventListener('keydown', (e) => {
    const parts: string[] = [];
    if (e.ctrlKey || e.metaKey) parts.push('ctrl');
    if (e.shiftKey) parts.push('shift');
    if (e.altKey) parts.push('alt');
    parts.push(e.key.toLowerCase());
    const combo = parts.join('+');
    const handler = bindings.get(combo);
    if (handler) {
      e.preventDefault();
      handler();
    }
  });
}
