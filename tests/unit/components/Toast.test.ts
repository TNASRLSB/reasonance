import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import { get } from 'svelte/store';
import Toast from '$lib/components/Toast.svelte';
import { toasts, showToast, dismissToast } from '$lib/stores/toast';

beforeEach(() => {
  toasts.set([]);
});

describe('Toast component', () => {
  it('renders the toast container with aria-live attribute', () => {
    render(Toast);
    const container = document.querySelector('.toast-container');
    expect(container).not.toBeNull();
    expect(container?.getAttribute('aria-live')).toBe('polite');
  });

  it('renders nothing when there are no toasts', () => {
    render(Toast);
    const alerts = document.querySelectorAll('[role="alert"]');
    expect(alerts.length).toBe(0);
  });

  it('renders a toast when showToast is called', async () => {
    render(Toast);
    showToast('info', 'Hello', 'World', 0);
    // Wait for reactive update
    await new Promise((r) => setTimeout(r, 0));
    const alerts = document.querySelectorAll('[role="alert"]');
    expect(alerts.length).toBe(1);
  });

  it('renders toast title and body text', async () => {
    render(Toast);
    showToast('error', 'Something failed', 'Details here', 0);
    await new Promise((r) => setTimeout(r, 0));
    expect(document.querySelector('.toast-title')?.textContent).toBe('Something failed');
    expect(document.querySelector('.toast-body')?.textContent).toBe('Details here');
  });

  it('renders correct label for each toast type', async () => {
    render(Toast);
    showToast('success', 'Done', '', 0);
    await new Promise((r) => setTimeout(r, 0));
    const label = document.querySelector('.toast-label');
    expect(label?.textContent).toBe('SUCCESS');
  });

  it('renders multiple toasts when multiple are added', async () => {
    render(Toast);
    showToast('info', 'First', '', 0);
    showToast('warning', 'Second', '', 0);
    await new Promise((r) => setTimeout(r, 0));
    const alerts = document.querySelectorAll('[role="alert"]');
    expect(alerts.length).toBe(2);
  });

  it('removes a toast from the store when dismiss is called', () => {
    const id = showToast('info', 'Dismiss me', '', 0);
    expect(get(toasts)).toHaveLength(1);
    dismissToast(id);
    expect(get(toasts)).toHaveLength(0);
  });

  it('renders a dismiss button with accessible label', async () => {
    render(Toast);
    showToast('warning', 'Warn', '', 0);
    await new Promise((r) => setTimeout(r, 0));
    const btn = document.querySelector('button.toast-dismiss');
    expect(btn).not.toBeNull();
    expect(btn?.getAttribute('aria-label')).toBe('Dismiss notification');
  });
});
