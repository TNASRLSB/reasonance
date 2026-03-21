import { describe, it } from 'vitest';

describe('MenuItem', () => {
  it.todo('renders the trigger button with the provided label');
  it.todo('dropdown is not rendered when open is false');
  it.todo('dropdown is rendered when open is true');
  it.todo('clicking the trigger button calls onOpen');
  it.todo('hovering the trigger button calls onHover');
  it.todo('renders divider items as horizontal rules');
  it.todo('renders normal items as clickable buttons with label');
  it.todo('renders shortcut text when item has a shortcut');
  it.todo('clicking a normal item calls its action and then calls onClose');
  it.todo('items with a submenu show the submenu arrow and reveal submenu on hover');
  it.todo('pressing Escape calls onClose');
  it.todo('trigger button has active class when open is true');
});
