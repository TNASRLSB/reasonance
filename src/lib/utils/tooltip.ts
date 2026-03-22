// src/lib/utils/tooltip.ts
import type { ActionReturn } from 'svelte/action';

interface TooltipOptions {
  text: string;
  position?: 'top' | 'bottom' | 'left' | 'right';
  delay?: number;
}

let activeTooltip: HTMLElement | null = null;
let uid = 0;

export function tooltip(node: HTMLElement, param: string | TooltipOptions): ActionReturn<string | TooltipOptions> {
  let opts: TooltipOptions = typeof param === 'string' ? { text: param } : param;
  const id = `tooltip-${++uid}`;
  let el: HTMLElement | null = null;
  let showTimer: ReturnType<typeof setTimeout> | null = null;

  function show() {
    if (activeTooltip) activeTooltip.remove();
    el = document.createElement('div');
    el.id = id;
    el.role = 'tooltip';
    el.textContent = opts.text;
    Object.assign(el.style, {
      position: 'fixed', zIndex: '9999',
      padding: '4px 8px', maxWidth: '250px',
      background: 'var(--bg-tertiary, #2a2a2a)',
      color: 'var(--text-primary, #f0f0f0)',
      border: '2px solid var(--border, #5a5a5a)',
      fontSize: 'var(--font-size-small, 12px)',
      fontFamily: 'var(--font-ui)',
      pointerEvents: 'none',
    });
    document.body.appendChild(el);
    activeTooltip = el;
    node.setAttribute('aria-describedby', id);
    position(el);
  }

  function position(tip: HTMLElement) {
    const rect = node.getBoundingClientRect();
    const tipRect = tip.getBoundingClientRect();
    const pos = opts.position ?? 'top';
    let top = 0, left = 0;
    if (pos === 'top') {
      top = rect.top - tipRect.height - 6;
      left = rect.left + (rect.width - tipRect.width) / 2;
      if (top < 4) { top = rect.bottom + 6; }
    } else if (pos === 'bottom') {
      top = rect.bottom + 6;
      left = rect.left + (rect.width - tipRect.width) / 2;
    }
    left = Math.max(4, Math.min(left, window.innerWidth - tipRect.width - 4));
    tip.style.top = `${top}px`;
    tip.style.left = `${left}px`;
  }

  function hide() {
    if (showTimer) { clearTimeout(showTimer); showTimer = null; }
    if (el) { el.remove(); el = null; activeTooltip = null; }
    node.removeAttribute('aria-describedby');
  }

  function onEnter() { showTimer = setTimeout(show, opts.delay ?? 300); }
  function onLeave() { hide(); }
  function onFocus() { show(); }
  function onBlur() { hide(); }
  function onKeydown(e: KeyboardEvent) { if (e.key === 'Escape') hide(); }

  node.addEventListener('mouseenter', onEnter);
  node.addEventListener('mouseleave', onLeave);
  node.addEventListener('focus', onFocus);
  node.addEventListener('blur', onBlur);
  node.addEventListener('keydown', onKeydown);

  return {
    update(newParam: string | TooltipOptions) {
      opts = typeof newParam === 'string' ? { text: newParam } : newParam;
      if (el) el.textContent = opts.text;
    },
    destroy() {
      hide();
      node.removeEventListener('mouseenter', onEnter);
      node.removeEventListener('mouseleave', onLeave);
      node.removeEventListener('focus', onFocus);
      node.removeEventListener('blur', onBlur);
      node.removeEventListener('keydown', onKeydown);
    },
  };
}
