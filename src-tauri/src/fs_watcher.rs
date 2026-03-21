use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::{mpsc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

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
    let (tx, rx) = mpsc::channel();
    let mut watcher =
        RecommendedWatcher::new(tx, notify::Config::default()).map_err(|e| e.to_string())?;
    watcher
        .watch(Path::new(path), RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    thread::spawn(move || {
        for result in rx {
            if let Ok(event) = result {
                let kind = match event.kind {
                    notify::EventKind::Create(_) => "create",
                    notify::EventKind::Modify(_) => "modify",
                    notify::EventKind::Remove(_) => "remove",
                    _ => continue,
                };
                for path in &event.paths {
                    let payload = serde_json::json!({
                        "type": kind,
                        "path": path.to_string_lossy(),
                    });
                    let _ = app.emit("fs-change", payload);
                }
            }
        }
    });

    *state.watcher.lock().unwrap() = Some(watcher);
    Ok(())
}
