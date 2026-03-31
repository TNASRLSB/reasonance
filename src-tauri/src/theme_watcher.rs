use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;

use crate::event_bus::EventBus;

pub fn start_theme_watcher(event_bus: Arc<EventBus>) {
    let themes_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("reasonance")
        .join("themes");

    // Create dir if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&themes_dir) {
        eprintln!("Failed to create themes directory: {}", e);
        return;
    }

    std::thread::spawn(move || {
        let bus = event_bus.clone();
        let mut watcher = match RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if res.is_ok() {
                    bus.publish(crate::event_bus::Event::new(
                        "theme:changed",
                        serde_json::json!(null),
                        "theme-watcher",
                    ));
                }
            },
            Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create file watcher: {}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(&themes_dir, RecursiveMode::NonRecursive) {
            eprintln!("Failed to watch themes directory: {}", e);
            return;
        }

        // Keep the thread alive so the watcher stays active
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
}
