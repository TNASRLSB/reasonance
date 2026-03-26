import { writable } from 'svelte/store';

export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface ToastMessage {
  id: number;
  type: 'info' | 'warning' | 'error' | 'success' | 'update';
  title: string;
  body: string;
  actions?: ToastAction[];
  persistent?: boolean;
  progress?: number; // 0-100, undefined = no progress bar
}

let nextId = 0;
export const toasts = writable<ToastMessage[]>([]);

// Track auto-dismiss timers so they can be paused/resumed
const toastTimers = new Map<number, { timer: ReturnType<typeof setTimeout>; remaining: number; start: number }>();

export function showToast(
  type: ToastMessage['type'],
  title: string,
  body: string = '',
  duration = 5000,
  options?: { actions?: ToastAction[]; persistent?: boolean }
) {
  const id = nextId++;
  // Error and warning toasts never auto-dismiss — require explicit user action
  const persistent = options?.persistent ?? (type === 'error' || type === 'warning');
  toasts.update((t) => [
    ...t,
    { id, type, title, body, actions: options?.actions, persistent },
  ]);
  if (duration > 0 && !persistent) {
    const timer = setTimeout(() => {
      toastTimers.delete(id);
      toasts.update((t) => t.filter((msg) => msg.id !== id));
    }, duration);
    toastTimers.set(id, { timer, remaining: duration, start: Date.now() });
  }
  return id;
}

export function pauseToastTimer(id: number) {
  const entry = toastTimers.get(id);
  if (!entry) return;
  clearTimeout(entry.timer);
  entry.remaining -= (Date.now() - entry.start);
  if (entry.remaining <= 0) entry.remaining = 500;
}

export function resumeToastTimer(id: number) {
  const entry = toastTimers.get(id);
  if (!entry) return;
  entry.start = Date.now();
  entry.timer = setTimeout(() => {
    toastTimers.delete(id);
    toasts.update((t) => t.filter((msg) => msg.id !== id));
  }, entry.remaining);
}

export function dismissToast(id: number) {
  toasts.update((t) => t.filter((msg) => msg.id !== id));
}
