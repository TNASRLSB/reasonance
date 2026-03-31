use log::{debug, info, trace};
use notify::event::Event;
use notify::{EventHandler, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::event_bus::EventBus;

struct AppEventHandler {
    event_bus: Arc<EventBus>,
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
                self.event_bus.publish(crate::event_bus::Event::new(
                    "fs:change",
                    payload,
                    "fs-watcher",
                ));
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
    event_bus: Arc<EventBus>,
    state: &FsWatcherState,
) -> Result<(), crate::error::ReasonanceError> {
    info!("Starting filesystem watcher on path='{}'", path);
    let handler = AppEventHandler { event_bus };
    let mut watcher = RecommendedWatcher::new(handler, notify::Config::default())
        .map_err(|e| crate::error::ReasonanceError::internal(e.to_string()))?;
    watcher
        .watch(Path::new(path), RecursiveMode::Recursive)
        .map_err(|e| {
            crate::error::ReasonanceError::io(
                format!("watch '{}'", path),
                std::io::Error::other(e.to_string()),
            )
        })?;

    debug!("Filesystem watcher active on path='{}'", path);
    *state.watcher.lock().unwrap_or_else(|e| e.into_inner()) = Some(watcher);
    Ok(())
}
