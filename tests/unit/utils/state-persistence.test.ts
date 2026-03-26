import { describe, it, expect } from 'vitest';
import { gatherProjectState, gatherAppState } from '$lib/utils/state-persistence';

describe('gatherAppState', () => {
  it('returns default state when no active project', () => {
    const state = gatherAppState(null, []);
    expect(state.last_active_project_id).toBeNull();
    expect(state.recent_projects).toEqual([]);
    expect(state.window_state).toBeNull();
  });

  it('includes active project id', () => {
    const state = gatherAppState('proj-1', []);
    expect(state.last_active_project_id).toBe('proj-1');
  });

  it('maps recent projects with snake_case fields', () => {
    const state = gatherAppState(null, [
      { path: '/home/user/proj', label: 'My Project', lastOpened: 1711234567890 },
    ]);
    expect(state.recent_projects).toEqual([
      { path: '/home/user/proj', label: 'My Project', last_opened: 1711234567890 },
    ]);
  });
});

describe('gatherProjectState', () => {
  it('returns default state with empty files', () => {
    const state = gatherProjectState([], null, null, null, null);
    expect(state.open_files).toEqual([]);
    expect(state.active_file_path).toBeNull();
    expect(state.panel_layout).toBeNull();
    expect(state.last_model_used).toBeNull();
    expect(state.active_session_id).toBeNull();
  });

  it('maps open files with cursor positions', () => {
    const state = gatherProjectState(
      [{ path: '/src/main.ts', cursorLine: 42, cursorCol: 10, scrollOffset: 150.5 }],
      '/src/main.ts',
      null,
      'claude-opus-4-6',
      'session-123',
    );
    expect(state.open_files).toEqual([
      { path: '/src/main.ts', cursor_line: 42, cursor_column: 10, scroll_offset: 150.5 },
    ]);
    expect(state.active_file_path).toBe('/src/main.ts');
    expect(state.last_model_used).toBe('claude-opus-4-6');
    expect(state.active_session_id).toBe('session-123');
  });

  it('defaults cursor values to 0 when missing', () => {
    const state = gatherProjectState(
      [{ path: '/src/lib.ts' }],
      null, null, null, null,
    );
    expect(state.open_files[0]).toEqual({
      path: '/src/lib.ts',
      cursor_line: 0,
      cursor_column: 0,
      scroll_offset: 0,
    });
  });

  it('includes panel layout when provided', () => {
    const layout = { sidebar_visible: true, sidebar_width: 250, bottom_panel_visible: true, bottom_panel_height: 300 };
    const state = gatherProjectState([], null, layout, null, null);
    expect(state.panel_layout).toEqual(layout);
  });
});
