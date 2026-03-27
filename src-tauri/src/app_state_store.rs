//! App state persistence store.
//!
//! Persists app-level and per-project state across restarts using atomic JSON
//! file writes. The frontend controls when to save — no debouncing here.

use crate::error::ReasonanceError;
use crate::storage::atomic_write;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ── Domain types ─────────────────────────────────────────────────────────────

/// App-level state (not per-project).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub last_active_project_id: Option<String>,
    pub recent_projects: Vec<RecentProject>,
    pub window_state: Option<WindowState>,
}

/// An entry in the recent projects list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub id: String,
    pub path: String,
    /// RFC3339 timestamp.
    pub last_opened: String,
}

/// Saved window geometry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub maximized: bool,
}

/// Per-project state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectState {
    pub active_session_id: Option<String>,
    pub open_files: Vec<OpenFileState>,
    pub active_file_path: Option<String>,
    pub panel_layout: Option<PanelLayout>,
    pub last_model_used: Option<String>,
}

/// State for a single open editor tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFileState {
    pub path: String,
    pub cursor_line: Option<u32>,
    pub cursor_column: Option<u32>,
    pub scroll_offset: Option<u32>,
}

/// Saved panel layout geometry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelLayout {
    pub sidebar_visible: bool,
    pub sidebar_width: Option<u32>,
    pub bottom_panel_visible: bool,
    pub bottom_panel_height: Option<u32>,
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// Key-value store for app and per-project state.
///
/// - App state: `{base_dir}/app-state.json`
/// - Project state: `{base_dir}/projects/{project_id}.json`
///
/// All writes are atomic (write to `.tmp`, then rename).
pub struct AppStateStore {
    base_dir: PathBuf,
    app_state: Mutex<AppState>,
    project_states: Mutex<HashMap<String, ProjectState>>,
}

impl AppStateStore {
    /// Create (or open) the store rooted at `base_dir`.
    ///
    /// Loads the existing app state from disk if present; starts with
    /// `Default` otherwise. Project states are loaded lazily on first access.
    pub fn new(base_dir: &Path) -> Result<Self, ReasonanceError> {
        std::fs::create_dir_all(base_dir)
            .map_err(|e| ReasonanceError::io("create app state dir", e))?;
        std::fs::create_dir_all(base_dir.join("projects"))
            .map_err(|e| ReasonanceError::io("create projects state dir", e))?;

        let app_state =
            Self::load_json::<AppState>(&base_dir.join("app-state.json")).unwrap_or_default();

        Ok(Self {
            base_dir: base_dir.to_path_buf(),
            app_state: Mutex::new(app_state),
            project_states: Mutex::new(HashMap::new()),
        })
    }

    // ── App-level ─────────────────────────────────────────────────────────

    /// Return a clone of the current app state.
    pub fn get_app_state(&self) -> AppState {
        self.app_state.lock().unwrap().clone()
    }

    /// Persist `state` to disk and update the in-memory cache.
    pub fn save_app_state(&self, state: &AppState) -> Result<(), ReasonanceError> {
        let path = self.base_dir.join("app-state.json");
        Self::save_json(&path, state)?;
        *self.app_state.lock().unwrap() = state.clone();
        Ok(())
    }

    // ── Per-project ───────────────────────────────────────────────────────

    /// Return the state for `project_id`.
    ///
    /// Tries the in-memory cache first; falls back to disk; returns `Default`
    /// if neither exists.
    pub fn get_project_state(&self, project_id: &str) -> ProjectState {
        let mut guard = self.project_states.lock().unwrap();
        if let Some(s) = guard.get(project_id) {
            return s.clone();
        }
        // Load from disk on first access
        let path = self.project_path(project_id);
        let state = Self::load_json::<ProjectState>(&path).unwrap_or_default();
        guard.insert(project_id.to_string(), state.clone());
        state
    }

    /// Persist `state` for `project_id` and update the in-memory cache.
    pub fn save_project_state(
        &self,
        project_id: &str,
        state: &ProjectState,
    ) -> Result<(), ReasonanceError> {
        let path = self.project_path(project_id);
        Self::save_json(&path, state)?;
        self.project_states
            .lock()
            .unwrap()
            .insert(project_id.to_string(), state.clone());
        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    fn project_path(&self, project_id: &str) -> PathBuf {
        // Sanitize: replace path-unsafe chars with '_'
        let safe_id = project_id.replace(['/', '\\', ':'], "_").replace("..", "_");
        self.base_dir
            .join("projects")
            .join(format!("{}.json", safe_id))
    }

    fn load_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
        let data = std::fs::read(path).ok()?;
        serde_json::from_slice(&data).ok()
    }

    fn save_json<T: Serialize>(path: &Path, data: &T) -> Result<(), ReasonanceError> {
        let bytes =
            serde_json::to_vec_pretty(data).map_err(|e| ReasonanceError::Serialization {
                context: "AppStateStore::save_json".to_string(),
                message: e.to_string(),
            })?;
        atomic_write(path, &bytes)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_store() -> (AppStateStore, TempDir) {
        let tmp = TempDir::new().unwrap();
        let store = AppStateStore::new(tmp.path()).unwrap();
        (store, tmp)
    }

    // 1. App state roundtrip
    #[test]
    fn app_state_save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let state = AppState {
            last_active_project_id: Some("proj-abc".to_string()),
            recent_projects: vec![RecentProject {
                id: "proj-abc".to_string(),
                path: "/home/user/my-project".to_string(),
                last_opened: "2026-03-26T10:00:00Z".to_string(),
            }],
            window_state: Some(WindowState {
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
                maximized: false,
            }),
        };

        let store = AppStateStore::new(tmp.path()).unwrap();
        store.save_app_state(&state).unwrap();

        // Re-open from disk
        let store2 = AppStateStore::new(tmp.path()).unwrap();
        let loaded = store2.get_app_state();
        assert_eq!(loaded.last_active_project_id, state.last_active_project_id);
        assert_eq!(loaded.recent_projects.len(), 1);
        assert_eq!(loaded.recent_projects[0].id, "proj-abc");
        let ws = loaded.window_state.unwrap();
        assert_eq!(ws.width, 1920);
        assert_eq!(ws.maximized, false);
    }

    // 2. Project state roundtrip
    #[test]
    fn project_state_save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let state = ProjectState {
            active_session_id: Some("sess-42".to_string()),
            open_files: vec![OpenFileState {
                path: "/src/main.rs".to_string(),
                cursor_line: Some(10),
                cursor_column: Some(5),
                scroll_offset: Some(0),
            }],
            active_file_path: Some("/src/main.rs".to_string()),
            panel_layout: Some(PanelLayout {
                sidebar_visible: true,
                sidebar_width: Some(240),
                bottom_panel_visible: false,
                bottom_panel_height: None,
            }),
            last_model_used: Some("claude-sonnet".to_string()),
        };

        let store = AppStateStore::new(tmp.path()).unwrap();
        store.save_project_state("proj-1", &state).unwrap();

        // Re-open from disk
        let store2 = AppStateStore::new(tmp.path()).unwrap();
        let loaded = store2.get_project_state("proj-1");
        assert_eq!(loaded.active_session_id, Some("sess-42".to_string()));
        assert_eq!(loaded.open_files.len(), 1);
        assert_eq!(loaded.open_files[0].cursor_line, Some(10));
        let layout = loaded.panel_layout.unwrap();
        assert!(layout.sidebar_visible);
        assert_eq!(layout.sidebar_width, Some(240));
    }

    // 3. Default state when no file exists
    #[test]
    fn default_state_when_no_file_exists() {
        let (store, _tmp) = make_store();
        let app = store.get_app_state();
        assert!(app.last_active_project_id.is_none());
        assert!(app.recent_projects.is_empty());
        assert!(app.window_state.is_none());

        let proj = store.get_project_state("nonexistent");
        assert!(proj.active_session_id.is_none());
        assert!(proj.open_files.is_empty());
    }

    // 4. Recent projects list preserved
    #[test]
    fn recent_projects_list_preserved() {
        let tmp = TempDir::new().unwrap();
        let mut state = AppState::default();
        state.recent_projects.push(RecentProject {
            id: "p1".to_string(),
            path: "/projects/alpha".to_string(),
            last_opened: "2026-03-01T00:00:00Z".to_string(),
        });
        state.recent_projects.push(RecentProject {
            id: "p2".to_string(),
            path: "/projects/beta".to_string(),
            last_opened: "2026-03-26T00:00:00Z".to_string(),
        });

        let store = AppStateStore::new(tmp.path()).unwrap();
        store.save_app_state(&state).unwrap();

        let store2 = AppStateStore::new(tmp.path()).unwrap();
        let loaded = store2.get_app_state();
        assert_eq!(loaded.recent_projects.len(), 2);
        assert_eq!(loaded.recent_projects[0].id, "p1");
        assert_eq!(loaded.recent_projects[1].path, "/projects/beta");
    }

    // 5. Multiple project states are independent
    #[test]
    fn multiple_project_states_independent() {
        let (store, _tmp) = make_store();

        let state_a = ProjectState {
            active_session_id: Some("sess-a".to_string()),
            last_model_used: Some("claude-opus".to_string()),
            ..Default::default()
        };
        let state_b = ProjectState {
            active_session_id: Some("sess-b".to_string()),
            last_model_used: Some("gpt-4o".to_string()),
            ..Default::default()
        };

        store.save_project_state("proj-a", &state_a).unwrap();
        store.save_project_state("proj-b", &state_b).unwrap();

        let loaded_a = store.get_project_state("proj-a");
        let loaded_b = store.get_project_state("proj-b");

        assert_eq!(loaded_a.active_session_id, Some("sess-a".to_string()));
        assert_eq!(loaded_b.active_session_id, Some("sess-b".to_string()));
        assert_ne!(loaded_a.last_model_used, loaded_b.last_model_used);
    }

    // 6. Window state persistence
    #[test]
    fn window_state_persists_across_restart() {
        let tmp = TempDir::new().unwrap();
        let ws = WindowState {
            width: 2560,
            height: 1440,
            x: -100,
            y: 50,
            maximized: true,
        };
        let mut state = AppState::default();
        state.window_state = Some(ws);

        let store = AppStateStore::new(tmp.path()).unwrap();
        store.save_app_state(&state).unwrap();

        let store2 = AppStateStore::new(tmp.path()).unwrap();
        let loaded = store2.get_app_state();
        let ws2 = loaded.window_state.unwrap();
        assert_eq!(ws2.width, 2560);
        assert_eq!(ws2.height, 1440);
        assert_eq!(ws2.x, -100);
        assert_eq!(ws2.y, 50);
        assert!(ws2.maximized);
    }

    // 7. In-memory cache update — get reflects save without disk round-trip
    #[test]
    fn in_memory_cache_updated_after_save() {
        let (store, _tmp) = make_store();

        let mut state = AppState::default();
        state.last_active_project_id = Some("proj-cache".to_string());
        store.save_app_state(&state).unwrap();

        let retrieved = store.get_app_state();
        assert_eq!(
            retrieved.last_active_project_id,
            Some("proj-cache".to_string())
        );
    }

    // 8. Project ID sanitization — unsafe chars don't escape base_dir
    #[test]
    fn project_id_with_path_chars_is_sanitized() {
        let (store, tmp) = make_store();

        let state = ProjectState {
            active_session_id: Some("x".to_string()),
            ..Default::default()
        };
        // This should NOT write outside of tmp.path()
        store.save_project_state("../../etc/evil", &state).unwrap();

        // The file must be inside the projects dir
        let projects_dir = tmp.path().join("projects");
        let entries: Vec<_> = std::fs::read_dir(&projects_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1);
        let filename = entries[0].file_name();
        let name = filename.to_str().unwrap();
        // Must not contain ".." or "/"
        assert!(!name.contains(".."));
        assert!(!name.contains('/'));
    }
}
