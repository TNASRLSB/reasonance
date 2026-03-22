mod agent_event;
mod normalizer;
mod transport;
mod commands;
mod config;
mod discovery;
mod fs_watcher;
mod pty_manager;
mod shadow_store;
mod workflow_store;
mod agent_runtime;
mod workflow_engine;
mod cli_updater;
mod normalizer_version;
mod normalizer_health;
mod capability;
mod self_heal;

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
        .manage(
            transport::StructuredAgentTransport::new(
                std::path::Path::new("normalizers")
            ).expect("Failed to load normalizers")
        )
        .manage({
            let sessions_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance")
                .join("sessions");
            transport::session_manager::SessionManager::new(&sessions_dir)
                .expect("Failed to initialize SessionManager")
        })
        .setup(|app| {
            let transport: tauri::State<'_, transport::StructuredAgentTransport> = app.state();

            // Wire FrontendEmitter (existing)
            let emitter = transport::event_bus::FrontendEmitter::new(app.handle().clone());
            transport.event_bus().subscribe(Box::new(emitter));

            // Wire SessionHistoryRecorder into event bus
            let session_mgr: tauri::State<'_, transport::session_manager::SessionManager> = app.state();
            let session_recorder = session_mgr.recorder();
            struct SessionRecorderWrapper(std::sync::Arc<transport::event_bus::SessionHistoryRecorder>);
            impl transport::event_bus::AgentEventSubscriber for SessionRecorderWrapper {
                fn on_event(&self, session_id: &str, event: &crate::agent_event::AgentEvent) {
                    self.0.on_event(session_id, event);
                }
            }
            transport.event_bus().subscribe(Box::new(SessionRecorderWrapper(session_recorder)));

            Ok(())
        })
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
            commands::transport::agent_send,
            commands::transport::agent_stop,
            commands::transport::agent_get_events,
            commands::transport::agent_get_session_status,
            commands::transport::agent_list_sessions,
            commands::session::session_create,
            commands::session::session_restore,
            commands::session::session_get_events,
            commands::session::session_list,
            commands::session::session_delete,
            commands::session::session_rename,
            commands::session::session_fork,
            commands::session::session_set_view_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
