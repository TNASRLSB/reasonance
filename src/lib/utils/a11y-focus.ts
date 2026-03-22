class FocusManager {
  private stack: HTMLElement[] = [];

  push(target: HTMLElement): void {
    const current = document.activeElement as HTMLElement;
    if (current) this.stack.push(current);
    target.focus();
  }

  pop(): void {
    const prev = this.stack.pop();
    if (!prev) return;
    if (prev.isConnected) {
      prev.focus();
    } else {
      let parent = prev.parentElement;
      while (parent && !parent.isConnected) parent = parent.parentElement;
      parent?.focus();
    }
  }

  reset(): void {
    this.stack = [];
  }

  get depth(): number {
    return this.stack.length;
  }
}

export const focusManager = new FocusManager();
