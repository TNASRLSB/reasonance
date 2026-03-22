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

export function showToast(
  type: ToastMessage['type'],
  title: string,
  body: string = '',
  duration = 5000,
  options?: { actions?: ToastAction[]; persistent?: boolean }
) {
  const id = nextId++;
  const persistent = options?.persistent ?? false;
  toasts.update((t) => [
    ...t,
    { id, type, title, body, actions: options?.actions, persistent },
  ]);
  if (duration > 0 && !persistent) {
    setTimeout(() => {
      toasts.update((t) => t.filter((msg) => msg.id !== id));
    }, duration);
  }
  return id;
}

export function updateToastProgress(id: number, progress: number) {
  toasts.update((t) =>
    t.map((msg) => (msg.id === id ? { ...msg, progress } : msg))
  );
}

export function dismissToast(id: number) {
  toasts.update((t) => t.filter((msg) => msg.id !== id));
}
