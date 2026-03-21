import { describe, it } from 'vitest';

describe('TerminalToolbar', () => {
  it.todo('renders the add-file (+) button');
  it.todo('renders the slash-command (/) button');
  it.todo('renders the mode selector button showing the active mode or "Default"');
  it.todo('slash command dropdown is hidden by default');
  it.todo('clicking the slash button opens the slash command dropdown');
  it.todo('slash command dropdown is not shown when slashCommands list is empty');
  it.todo('clicking a slash command sends it to the terminal and closes the dropdown');
  it.todo('mode dropdown is hidden by default');
  it.todo('clicking the mode button opens the mode dropdown');
  it.todo('mode dropdown is not shown when modes list is empty');
  it.todo('clicking outside closes both dropdowns');
  it.todo('clicking the add-file button calls adapter.writePty with "/file "');
  it.todo('active mode item in the dropdown has the active class');
});
