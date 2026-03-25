class ScreenReaderAnnouncer {
  private queue: string[] = [];
  private throttleMs: number;
  private lastAnnounce = 0;
  private politeEl: HTMLElement | null = null;
  private assertiveEl: HTMLElement | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;

  constructor(throttleMs = 5000) {
    this.throttleMs = throttleMs;
  }

  mount(container: HTMLElement): void {
    this.politeEl = this.createLiveRegion(container, 'polite');
    this.assertiveEl = this.createLiveRegion(container, 'assertive');
  }

  private createLiveRegion(container: HTMLElement, politeness: string): HTMLElement {
    const el = document.createElement('div');
    el.setAttribute('aria-live', politeness);
    el.setAttribute('aria-atomic', 'true');
    el.setAttribute('role', 'status');
    Object.assign(el.style, {
      position: 'absolute', width: '1px', height: '1px',
      padding: '0', margin: '-1px', overflow: 'hidden',
      clip: 'rect(0,0,0,0)', whiteSpace: 'nowrap', border: '0',
    });
    container.appendChild(el);
    return el;
  }

  announce(message: string): void {
    const now = Date.now();
    if (now - this.lastAnnounce < this.throttleMs) {
      this.queue.push(message);
      if (!this.timer) {
        this.timer = setTimeout(() => this.flush(), this.throttleMs - (now - this.lastAnnounce));
      }
      return;
    }
    this.doAnnounce(message, this.politeEl);
  }

  announceUrgent(message: string): void {
    this.doAnnounce(message, this.assertiveEl);
  }

  private doAnnounce(message: string, el: HTMLElement | null): void {
    if (!el) return;
    el.textContent = '';
    requestAnimationFrame(() => { el.textContent = message; });
    this.lastAnnounce = Date.now();
  }

  private flush(): void {
    this.timer = null;
    if (this.queue.length === 0) return;
    const fused = this.queue.join('. ');
    this.queue = [];
    this.doAnnounce(fused, this.politeEl);
  }

  destroy(): void {
    if (this.timer) clearTimeout(this.timer);
    this.politeEl?.remove();
    this.assertiveEl?.remove();
  }
}

export const analyticsAnnouncer = new ScreenReaderAnnouncer(5000);
export const appAnnouncer = new ScreenReaderAnnouncer(2000);
