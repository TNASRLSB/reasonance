import { describe, it } from 'vitest';

describe('FindInFiles', () => {
  it.todo('is not rendered when visible is false');
  it.todo('renders the overlay and panel when visible is true');
  it.todo('renders the search input and Search button');
  it.todo('Search button is disabled when the input is empty');
  it.todo('Search button is disabled while a search is in progress');
  it.todo('pressing Enter in the input triggers a search');
  it.todo('pressing Escape calls onClose');
  it.todo('clicking the overlay background calls onClose');
  it.todo('shows "no results" message when search completes with zero matches');
  it.todo('shows result count summary when results are returned');
  it.todo('groups results by file path');
  it.todo('clicking a result row opens the file and calls onClose');
  it.todo('shows an error message when the search command fails');
  it.todo('resets results and error state when reopened (visible toggles to true)');
});
