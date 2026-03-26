pub mod defaults;

use log::{debug, warn};
use std::path::{Path, PathBuf};

/// Deep merge two TOML values. `overlay` overrides `base`.
/// Tables merge recursively; scalars and arrays: overlay replaces base.
pub fn deep_merge(base: &mut toml::Value, overlay: &toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (key, overlay_val) in overlay_table {
                if let Some(base_val) = base_table.get_mut(key) {
                    deep_merge(base_val, overlay_val);
                } else {
                    base_table.insert(key.clone(), overlay_val.clone());
                }
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}

/// 4-layer settings: builtin → user → project → workspace.
/// Each layer is optional except builtin. Layers are merged with `deep_merge`
/// so that higher-priority layers override lower ones at the key level.
pub struct LayeredSettings {
    builtin: toml::Value,
    user: Option<toml::Value>,
    project: Option<toml::Value>,
    workspace: Option<toml::Value>,
    resolved: toml::Value,
    user_path: PathBuf,
    project_path: Option<PathBuf>,
    workspace_path: Option<PathBuf>,
}

impl LayeredSettings {
    /// Create a new LayeredSettings with builtin defaults and user layer loaded.
    pub fn new() -> Self {
        let builtin = defaults::builtin_defaults();
        let user_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("reasonance")
            .join("settings.toml");

        let mut settings = Self {
            builtin: builtin.clone(),
            user: None,
            project: None,
            workspace: None,
            resolved: builtin,
            user_path,
            project_path: None,
            workspace_path: None,
        };
        settings.load_user_layer();
        settings.resolve();
        settings
    }

    /// Create from explicit builtin defaults (useful for testing).
    #[cfg(test)]
    fn from_builtin(builtin: toml::Value) -> Self {
        Self {
            builtin: builtin.clone(),
            user: None,
            project: None,
            workspace: None,
            resolved: builtin,
            user_path: PathBuf::from("/nonexistent/settings.toml"),
            project_path: None,
            workspace_path: None,
        }
    }

    /// Set the project root, loading project and workspace layers from it.
    pub fn set_project_root(&mut self, root: &Path) {
        self.project_path = Some(root.join(".reasonance").join("settings.toml"));
        self.workspace_path = Some(root.join(".reasonance").join("workspace-settings.toml"));
        self.load_project_layer();
        self.load_workspace_layer();
        self.resolve();
    }

    fn load_layer(path: &Path) -> Option<toml::Value> {
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(val) => {
                    debug!("Settings: loaded layer from {}", path.display());
                    Some(val)
                }
                Err(e) => {
                    warn!("Settings: invalid TOML in {}: {}", path.display(), e);
                    None
                }
            },
            Err(_) => None, // File doesn't exist — skip silently
        }
    }

    fn load_user_layer(&mut self) {
        self.user = Self::load_layer(&self.user_path);
    }

    fn load_project_layer(&mut self) {
        self.project = self.project_path.as_ref().and_then(|p| Self::load_layer(p));
    }

    fn load_workspace_layer(&mut self) {
        self.workspace = self
            .workspace_path
            .as_ref()
            .and_then(|p| Self::load_layer(p));
    }

    /// Recompute the resolved settings from all layers.
    pub fn resolve(&mut self) {
        let mut resolved = self.builtin.clone();
        if let Some(ref user) = self.user {
            deep_merge(&mut resolved, user);
        }
        if let Some(ref project) = self.project {
            deep_merge(&mut resolved, project);
        }
        if let Some(ref workspace) = self.workspace {
            deep_merge(&mut resolved, workspace);
        }
        self.resolved = resolved;
    }

    /// Reload all layers from disk and re-resolve.
    pub fn reload(&mut self) {
        self.load_user_layer();
        self.load_project_layer();
        self.load_workspace_layer();
        self.resolve();
    }

    /// Get a raw TOML value by dotted path (e.g., "editor.font_size").
    pub fn get_value(&self, path: &str) -> Option<&toml::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &self.resolved;
        for part in parts {
            match current.get(part) {
                Some(val) => current = val,
                None => return None,
            }
        }
        Some(current)
    }

    /// Get a typed value by dotted path, deserializing from TOML.
    pub fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Option<T> {
        self.get_value(path).and_then(|v| v.clone().try_into().ok())
    }

    /// Access the fully resolved settings tree.
    pub fn resolved(&self) -> &toml::Value {
        &self.resolved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_toml(s: &str) -> toml::Value {
        toml::from_str(s).unwrap()
    }

    // ── Test 1: Builtin defaults resolve correctly ──────────────────────

    #[test]
    fn builtin_defaults_resolve_correctly() {
        let settings = LayeredSettings::from_builtin(defaults::builtin_defaults());
        assert_eq!(settings.get::<i64>("editor.font_size"), Some(14));
        assert_eq!(settings.get::<i64>("editor.tab_size"), Some(2));
        assert_eq!(
            settings.get::<String>("editor.font_family"),
            Some("Atkinson Hyperlegible Mono".to_string())
        );
        assert_eq!(settings.get::<i64>("terminal.font_size"), Some(13));
        assert_eq!(settings.get::<bool>("analytics.enabled"), Some(true));
        assert_eq!(settings.get::<bool>("filetree.auto_fold"), Some(false));
        assert_eq!(settings.get::<bool>("filetree.show_git_status"), Some(true));
    }

    // ── Test 2: User layer overrides builtin scalar ─────────────────────

    #[test]
    fn user_layer_overrides_builtin_scalar() {
        let mut settings = LayeredSettings::from_builtin(defaults::builtin_defaults());
        settings.user = Some(make_toml(
            r#"
[editor]
font_size = 18
"#,
        ));
        settings.resolve();
        assert_eq!(settings.get::<i64>("editor.font_size"), Some(18));
    }

    // ── Test 3: Deep merge preserves sibling keys ───────────────────────

    #[test]
    fn deep_merge_preserves_sibling_keys() {
        let mut settings = LayeredSettings::from_builtin(defaults::builtin_defaults());
        // Override only font_size — tab_size and font_family must survive
        settings.user = Some(make_toml(
            r#"
[editor]
font_size = 20
"#,
        ));
        settings.resolve();
        assert_eq!(settings.get::<i64>("editor.font_size"), Some(20));
        assert_eq!(settings.get::<i64>("editor.tab_size"), Some(2));
        assert_eq!(
            settings.get::<String>("editor.font_family"),
            Some("Atkinson Hyperlegible Mono".to_string())
        );
    }

    // ── Test 4: Project overrides user ──────────────────────────────────

    #[test]
    fn project_overrides_user() {
        let mut settings = LayeredSettings::from_builtin(defaults::builtin_defaults());
        settings.user = Some(make_toml(
            r#"
[editor]
font_size = 18
tab_size = 4
"#,
        ));
        settings.project = Some(make_toml(
            r#"
[editor]
tab_size = 8
"#,
        ));
        settings.resolve();
        // User set font_size=18, project didn't touch it → 18
        assert_eq!(settings.get::<i64>("editor.font_size"), Some(18));
        // Project overrides user's tab_size → 8
        assert_eq!(settings.get::<i64>("editor.tab_size"), Some(8));
    }

    // ── Test 5: Workspace overrides project ─────────────────────────────

    #[test]
    fn workspace_overrides_project() {
        let mut settings = LayeredSettings::from_builtin(defaults::builtin_defaults());
        settings.user = Some(make_toml(
            r#"
[editor]
font_size = 18
"#,
        ));
        settings.project = Some(make_toml(
            r#"
[editor]
font_size = 16
tab_size = 4
"#,
        ));
        settings.workspace = Some(make_toml(
            r#"
[editor]
font_size = 12
"#,
        ));
        settings.resolve();
        // Workspace wins for font_size
        assert_eq!(settings.get::<i64>("editor.font_size"), Some(12));
        // Project's tab_size survives (workspace didn't touch it)
        assert_eq!(settings.get::<i64>("editor.tab_size"), Some(4));
    }

    // ── Test 6: Missing layer gracefully skipped ────────────────────────

    #[test]
    fn missing_layer_gracefully_skipped() {
        let mut settings = LayeredSettings::from_builtin(defaults::builtin_defaults());
        // No user/project/workspace layers set — only builtin
        settings.resolve();
        // Should still work with just builtin
        assert_eq!(settings.get::<i64>("editor.font_size"), Some(14));
        assert_eq!(settings.get::<bool>("analytics.enabled"), Some(true));
    }

    // ── Test 7: Invalid TOML file → layer skipped, no crash ─────────────

    #[test]
    fn invalid_toml_file_skipped_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        let bad_path = dir.path().join("bad.toml");
        std::fs::write(&bad_path, "this is [[[not valid toml!!!").unwrap();

        let result = LayeredSettings::load_layer(&bad_path);
        assert!(result.is_none(), "Invalid TOML should return None");
    }

    // ── Test 8: Dotted path access works ────────────────────────────────

    #[test]
    fn dotted_path_access_works() {
        let mut settings = LayeredSettings::from_builtin(defaults::builtin_defaults());
        settings.user = Some(make_toml(
            r#"
[custom]
nested.ignored = true

[custom.deeply]
value = 42
"#,
        ));
        settings.resolve();

        // Standard dotted paths
        assert_eq!(settings.get::<i64>("editor.font_size"), Some(14));
        assert_eq!(settings.get::<i64>("custom.deeply.value"), Some(42));

        // Non-existent path returns None
        assert_eq!(settings.get_value("nonexistent.path"), None);
        assert_eq!(settings.get_value("editor.nonexistent"), None);
    }

    // ── Test: deep_merge function directly ──────────────────────────────

    #[test]
    fn deep_merge_nested_tables() {
        let mut base = make_toml(
            r#"
[a]
x = 1
[a.b]
y = 2
z = 3
"#,
        );
        let overlay = make_toml(
            r#"
[a.b]
z = 99
w = 100
"#,
        );
        deep_merge(&mut base, &overlay);
        // a.x untouched
        assert_eq!(base["a"]["x"].as_integer(), Some(1));
        // a.b.y untouched
        assert_eq!(base["a"]["b"]["y"].as_integer(), Some(2));
        // a.b.z overridden
        assert_eq!(base["a"]["b"]["z"].as_integer(), Some(99));
        // a.b.w added
        assert_eq!(base["a"]["b"]["w"].as_integer(), Some(100));
    }

    #[test]
    fn deep_merge_overlay_replaces_scalar_with_scalar() {
        let mut base = make_toml("x = 1");
        let overlay = make_toml("x = 2");
        deep_merge(&mut base, &overlay);
        assert_eq!(base["x"].as_integer(), Some(2));
    }

    #[test]
    fn deep_merge_overlay_adds_new_section() {
        let mut base = make_toml(
            r#"
[editor]
font_size = 14
"#,
        );
        let overlay = make_toml(
            r#"
[theme]
name = "dark"
"#,
        );
        deep_merge(&mut base, &overlay);
        assert_eq!(base["editor"]["font_size"].as_integer(), Some(14));
        assert_eq!(base["theme"]["name"].as_str(), Some("dark"));
    }
}
