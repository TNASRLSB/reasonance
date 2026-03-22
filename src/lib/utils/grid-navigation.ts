// src/lib/utils/grid-navigation.ts
import type { ActionReturn } from 'svelte/action';

export function gridNavigation(node: HTMLElement): ActionReturn {
  function getCells(): HTMLElement[] {
    return Array.from(node.querySelectorAll<HTMLElement>('[role="gridcell"], [role="rowheader"]'))
      .filter(el => el.offsetParent !== null);
  }

  function getRows(): HTMLElement[][] {
    const rows: HTMLElement[][] = [];
    node.querySelectorAll<HTMLElement>('[role="row"]').forEach(row => {
      if (row.offsetParent === null) return;
      const cells = Array.from(row.querySelectorAll<HTMLElement>('[role="gridcell"], [role="rowheader"]'))
        .filter(el => el.offsetParent !== null);
      if (cells.length) rows.push(cells);
    });
    return rows;
  }

  function findPosition(cell: HTMLElement): [number, number] {
    const rows = getRows();
    for (let r = 0; r < rows.length; r++) {
      const c = rows[r].indexOf(cell);
      if (c >= 0) return [r, c];
    }
    return [-1, -1];
  }

  function focusCell(row: number, col: number) {
    const rows = getRows();
    if (row < 0 || row >= rows.length) return;
    const clampedCol = Math.min(col, rows[row].length - 1);
    const target = rows[row][clampedCol];
    getCells().forEach(c => c.setAttribute('tabindex', '-1'));
    target.setAttribute('tabindex', '0');
    target.focus();
  }

  function handleKeydown(e: KeyboardEvent) {
    const active = document.activeElement as HTMLElement;
    const [row, col] = findPosition(active);
    if (row < 0) return;
    const rows = getRows();

    switch (e.key) {
      case 'ArrowDown': e.preventDefault(); focusCell(row + 1, col); break;
      case 'ArrowUp': e.preventDefault(); focusCell(row - 1, col); break;
      case 'ArrowRight': e.preventDefault(); focusCell(row, col + 1); break;
      case 'ArrowLeft': e.preventDefault(); focusCell(row, col - 1); break;
      case 'Home':
        e.preventDefault();
        focusCell(e.ctrlKey ? 0 : row, 0);
        break;
      case 'End':
        e.preventDefault();
        focusCell(e.ctrlKey ? rows.length - 1 : row, Infinity);
        break;
      case 'Enter':
        e.preventDefault();
        active.click();
        break;
    }
  }

  const cells = getCells();
  cells.forEach((c, i) => c.setAttribute('tabindex', i === 0 ? '0' : '-1'));

  node.addEventListener('keydown', handleKeydown);
  return { destroy: () => node.removeEventListener('keydown', handleKeydown) };
}
