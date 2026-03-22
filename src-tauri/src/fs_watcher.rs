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
    let handler = AppEventHandler { app };
    let mut watcher =
        RecommendedWatcher::new(handler, notify::Config::default()).map_err(|e| e.to_string())?;
    watcher
        .watch(Path::new(path), RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    *state.watcher.lock().unwrap() = Some(watcher);
    Ok(())
}
