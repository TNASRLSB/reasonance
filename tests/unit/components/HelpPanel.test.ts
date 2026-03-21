import { describe, it } from 'vitest';

describe('HelpPanel', () => {
  it.todo('renders the help panel container');
  it.todo('loads and displays documentation for the current locale');
  it.todo('falls back to English docs when locale-specific docs are not found');
  it.todo('shows fallback message when no documentation file can be loaded');
  it.todo('reloads documentation when the locale store changes');
  it.todo('passes loaded markdown content to MarkdownPreview');
  it.todo('unsubscribes from the locale store on destroy');
});
