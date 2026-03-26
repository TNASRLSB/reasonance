export interface AppState {
  last_active_project_id: string | null;
  recent_projects: RecentProjectEntry[];
  window_state: WindowState | null;
}

export interface WindowState {
  width: number;
  height: number;
  x: number;
  y: number;
  maximized: boolean;
}

export interface RecentProjectEntry {
  path: string;
  label: string;
  last_opened: number;
}

export interface ProjectState {
  active_session_id: string | null;
  open_files: OpenFileState[];
  active_file_path: string | null;
  panel_layout: PanelLayout | null;
  last_model_used: string | null;
}

export interface PanelLayout {
  sidebar_visible: boolean;
  sidebar_width: number;
  bottom_panel_visible: boolean;
  bottom_panel_height: number;
}

export interface OpenFileState {
  path: string;
  cursor_line: number;
  cursor_column: number;
  scroll_offset: number;
}
