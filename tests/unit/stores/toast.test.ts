import { describe, it } from 'vitest';

describe('toast store', () => {
  it.todo('starts with an empty toast list');
  it.todo('showToast adds a toast with correct type, title, and body');
  it.todo('showToast returns a numeric id');
  it.todo('dismissToast removes the toast with the given id');
  it.todo('showToast with duration > 0 auto-removes the toast after timeout');
  it.todo('showToast with duration 0 keeps the toast indefinitely');
  it.todo('multiple toasts can coexist with unique ids');
});
