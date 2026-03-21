mod commands;
mod config;
mod discovery;
mod fs_watcher;
mod pty_manager;
mod shadow_store;
mod workflow_store;
mod agent_runtime;
mod workflow_engine;

use fs_watcher::FsWatcherState;
use pty_manager::PtyManager;
use shadow_store::ShadowStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(PtyManager::new())
        .manage(ShadowStore::new())
        .manage(FsWatcherState::new())
        .manage(discovery::DiscoveryEngine::new())
        .manage(workflow_store::WorkflowStore::new())
        .manage(agent_runtime::AgentRuntime::new())
        .manage(workflow_engine::WorkflowEngine::new())
        .invoke_handler(tauri::generate_handler![
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
