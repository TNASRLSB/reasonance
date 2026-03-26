export interface ShortcutEntry {
  keys: string[];
  descriptionKey: string;
  context: string;
}

export const shortcuts: ShortcutEntry[] = [
  { keys: ['Ctrl', 'P'], descriptionKey: 'shortcuts.search', context: 'shortcuts.ctx.global' },
  { keys: ['Ctrl', 'Shift', 'F'], descriptionKey: 'shortcuts.findInFiles', context: 'shortcuts.ctx.global' },
  { keys: ['Ctrl', ','], descriptionKey: 'shortcuts.settings', context: 'shortcuts.ctx.global' },
  { keys: ['F1'], descriptionKey: 'shortcuts.docs', context: 'shortcuts.ctx.global' },
  { keys: ['Ctrl', '/'], descriptionKey: 'shortcuts.shortcutsDialog', context: 'shortcuts.ctx.global' },
  { keys: ['Escape'], descriptionKey: 'shortcuts.closeDialog', context: 'shortcuts.ctx.dialog' },
  { keys: ['Ctrl', 'S'], descriptionKey: 'shortcuts.saveFile', context: 'shortcuts.ctx.editor' },
  { keys: ['Ctrl', 'Z'], descriptionKey: 'shortcuts.undo', context: 'shortcuts.ctx.editor' },
  { keys: ['Ctrl', 'Y'], descriptionKey: 'shortcuts.redo', context: 'shortcuts.ctx.editor' },
  { keys: ['Ctrl', 'F'], descriptionKey: 'shortcuts.findInFile', context: 'shortcuts.ctx.terminal' },
  { keys: ['Ctrl', 'Shift', 'H'], descriptionKey: 'shortcuts.sessionHistory', context: 'shortcuts.ctx.global' },
];
