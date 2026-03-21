mod commands;
mod config;
mod fs_watcher;
mod pty_manager;
mod shadow_store;

use fs_watcher::FsWatcherState;
use pty_manager::PtyManager;
use shadow_store::ShadowStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(PtyManager::new())
        .manage(ShadowStore::new())
        .manage(FsWatcherState::new())
        .invoke_handler(tauri::generate_handler![
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::list_dir,
            commands::fs::grep_files,
            commands::fs::start_watching,
            commands::system::open_external,
            commands::system::get_env_var,
            commands::pty::spawn_process,
            commands::pty::write_pty,
            commands::pty::resize_pty,
            commands::pty::kill_process,
            commands::shadow::store_shadow,
            commands::shadow::get_shadow,
            commands::config::read_config,
            commands::config::write_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
