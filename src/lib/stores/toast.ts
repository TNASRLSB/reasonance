import { writable } from 'svelte/store';

export interface ToastMessage {
  id: number;
  type: 'info' | 'warning' | 'error' | 'success';
  title: string;
  body: string;
}

let nextId = 0;
export const toasts = writable<ToastMessage[]>([]);

export function showToast(type: ToastMessage['type'], title: string, body: string = '', duration = 5000) {
  const id = nextId++;
  toasts.update((t) => [...t, { id, type, title, body }]);
  if (duration > 0) {
    setTimeout(() => {
      toasts.update((t) => t.filter((msg) => msg.id !== id));
    }, duration);
  }
  return id;
}

export function dismissToast(id: number) {
  toasts.update((t) => t.filter((msg) => msg.id !== id));
}
