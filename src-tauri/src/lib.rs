mod commands;
mod config;
mod discovery;
mod fs_watcher;
mod pty_manager;
mod shadow_store;
mod workflow_store;
mod agent_runtime;
mod workflow_engine;

use commands::fs::ProjectRootState;
use fs_watcher::FsWatcherState;
use pty_manager::PtyManager;
use shadow_store::ShadowStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Fix blurry rendering on Linux with fractional scaling (WebKitGTK)
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus the existing window when a second instance is launched
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }))
        .manage(PtyManager::new())
        .manage(ShadowStore::new())
        .manage(FsWatcherState::new())
        .manage(ProjectRootState::new())
        .manage(discovery::DiscoveryEngine::new())
        .manage(workflow_store::WorkflowStore::new())
        .manage(agent_runtime::AgentRuntime::new())
        .manage(workflow_engine::WorkflowEngine::new())
        .invoke_handler(tauri::generate_handler![
            commands::fs::set_project_root,
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::list_dir,
            commands::fs::grep_files,
            commands::fs::start_watching,
            commands::system::open_external,
            commands::system::get_env_var,
            commands::system::get_system_colors,
            commands::system::discover_llms,
            commands::pty::spawn_process,
            commands::pty::write_pty,
            commands::pty::resize_pty,
            commands::pty::kill_process,
            commands::shadow::store_shadow,
            commands::shadow::get_shadow,
            commands::config::read_config,
            commands::config::write_config,
            commands::discovery::discover_agents,
            commands::discovery::get_discovered_agents,
            commands::workflow::load_workflow,
            commands::workflow::save_workflow,
            commands::workflow::list_workflows,
            commands::workflow::delete_workflow,
            commands::workflow::create_workflow,
            commands::workflow::get_workflow,
            commands::workflow::duplicate_workflow,
            commands::workflow::save_to_global,
            commands::workflow::list_global_workflows,
            commands::agent::create_agent,
            commands::agent::transition_agent,
            commands::agent::set_agent_pty,
            commands::agent::set_agent_error,
            commands::agent::get_agent,
            commands::agent::get_workflow_agents,
            commands::agent::remove_agent,
            commands::agent::stop_workflow_agents,
            commands::agent::send_agent_message,
            commands::agent::get_agent_messages,
            commands::engine::play_workflow,
            commands::engine::pause_workflow,
            commands::engine::resume_workflow,
            commands::engine::stop_workflow,
            commands::engine::step_workflow,
            commands::engine::get_run_status,
            commands::engine::notify_node_completed,
            commands::llm::call_llm_api,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
