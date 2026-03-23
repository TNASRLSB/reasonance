use log::{debug, info, trace};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, EventHandler};
use notify::event::Event;
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

struct AppEventHandler {
    app: AppHandle,
}

impl EventHandler for AppEventHandler {
    fn handle_event(&mut self, event: notify::Result<Event>) {
        if let Ok(event) = event {
            let kind = match event.kind {
                notify::EventKind::Create(_) => "create",
                notify::EventKind::Modify(_) => "modify",
                notify::EventKind::Remove(_) => "remove",
                _ => return,
            };
            for path in &event.paths {
                trace!("FS event: type={}, path={}", kind, path.display());
                let payload = serde_json::json!({
                    "type": kind,
                    "path": path.to_string_lossy(),
                });
                let _ = self.app.emit("fs-change", payload);
            }
        }
    }
}

pub struct FsWatcherState {
    watcher: Mutex<Option<RecommendedWatcher>>,
}

impl FsWatcherState {
    pub fn new() -> Self {
        Self {
            watcher: Mutex::new(None),
        }
    }
}

pub fn start_watching(
    path: &str,
    app: AppHandle,
    state: &FsWatcherState,
) -> Result<(), String> {
    info!("Starting filesystem watcher on path='{}'", path);
    let handler = AppEventHandler { app };
    let mut watcher =
        RecommendedWatcher::new(handler, notify::Config::default()).map_err(|e| e.to_string())?;
    watcher
        .watch(Path::new(path), RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    debug!("Filesystem watcher active on path='{}'", path);
    *state.watcher.lock().unwrap_or_else(|e| e.into_inner()) = Some(watcher);
    Ok(())
}
