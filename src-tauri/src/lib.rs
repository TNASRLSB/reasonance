#![allow(dead_code)]
#![allow(
    clippy::too_many_arguments,
    clippy::large_enum_variant,
    clippy::manual_strip,
    clippy::unnecessary_cast,
    clippy::derivable_impls,
    clippy::or_fun_call,
    clippy::unnecessary_map_or,
    clippy::unwrap_or_default,
    clippy::doc_lazy_continuation,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
)]

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
mod resource_lock;
mod agent_memory;
mod logic_eval;
mod cli_updater;
mod normalizer_version;
mod normalizer_health;
mod capability;
mod self_heal;
mod analytics;
mod workspace_trust;
mod theme_manager;
mod theme_watcher;
mod project_manager;

use commands::fs::ProjectRootState;
use fs_watcher::FsWatcherState;
use pty_manager::PtyManager;
use shadow_store::ShadowStore;
use tauri::{Emitter, Manager};
use log::info;

/// Shared state for the resolved normalizers directory path.
pub struct NormalizersDir(pub std::path::PathBuf);

/// Resolve the normalizers directory.
/// Search order:
///   1. `normalizers/` relative to CWD (dev mode)
///   2. Next to the executable (Linux AUR / Windows MSI)
///   3. `../Resources/normalizers/` relative to exe (macOS .app bundle)
///   4. `/usr/share/reasonance/normalizers/` (Linux system install)
fn resolve_normalizers_dir() -> std::path::PathBuf {
    let candidates: Vec<std::path::PathBuf> = {
        let mut v = vec![std::path::PathBuf::from("normalizers")];
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                v.push(dir.join("normalizers"));
                // macOS: exe is in Contents/MacOS/, resources in Contents/Resources/
                v.push(dir.join("../Resources/normalizers"));
            }
        }
        v.push(std::path::PathBuf::from("/usr/share/reasonance/normalizers"));
        v
    };
    for c in &candidates {
        if c.is_dir() {
            info!("Normalizers found at: {}", c.display());
            return c.clone();
        }
    }
    // Return the most likely candidate so the error message is useful
    log::warn!("Normalizers directory not found in any candidate: {:?}", candidates);
    candidates.into_iter().next().unwrap_or_else(|| std::path::PathBuf::from("normalizers"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Fix blurry rendering on Linux with fractional scaling (WebKitGTK)
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    // Dev: DEBUG by default, override with RUST_LOG env var.
    // Release: INFO to avoid log noise.
    let log_level = if cfg!(debug_assertions) {
        std::env::var("RUST_LOG")
            .ok()
            .and_then(|v| v.parse::<log::LevelFilter>().ok())
            .unwrap_or(log::LevelFilter::Debug)
    } else {
        log::LevelFilter::Info
    };

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log_level)
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Webview,
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir { file_name: None },
                ))
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .build(),
        )
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
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // If a path argument was provided, emit event to frontend
            if args.len() > 1 {
                if let Some(path) = args.get(1) {
                    let _ = app.emit("cli-open-project", path.as_str());
                }
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }))
        .manage(PtyManager::new())
        .manage(project_manager::ProjectsState::new())
        .manage(project_manager::ActiveProjectState::new())
        .manage(ShadowStore::new())
        .manage(FsWatcherState::new())
        .manage(ProjectRootState::new())
        .manage(discovery::DiscoveryEngine::new())
        .manage(workflow_store::WorkflowStore::new())
        .manage(agent_runtime::AgentRuntime::new())
        .manage(workflow_engine::WorkflowEngine::new())
        .manage(resource_lock::ResourceLockManager::new())
        .manage(NormalizersDir(resolve_normalizers_dir()))
        .manage({
            let dir = resolve_normalizers_dir();
            match transport::StructuredAgentTransport::new(&dir) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Failed to load normalizers from {}: {}. Starting with empty registry.", dir.display(), e);
                    transport::StructuredAgentTransport::empty()
                }
            }
        })
        .manage({
            let sessions_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance")
                .join("sessions");
            transport::session_manager::SessionManager::new(&sessions_dir)
                .expect("Failed to initialize SessionManager")
        })
        .manage(capability::CapabilityNegotiator::new())
        .manage(std::sync::Arc::new(cli_updater::CliUpdater::new()))
        .manage(normalizer_health::NormalizerHealth::new())
        .manage({
            let analytics_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance")
                .join("analytics");
            let store = std::sync::Arc::new(
                analytics::store::AnalyticsStore::new(&analytics_dir)
                    .expect("Failed to init analytics store")
            );
            std::sync::Arc::new(analytics::collector::AnalyticsCollector::new(store))
        })
        .manage({
            let versions_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance")
                .join("normalizer-versions");
            normalizer_version::NormalizerVersionStore::new(&versions_dir)
        })
        .manage({
            let config_dir = dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance");
            workspace_trust::TrustStore::new(config_dir.join("trusted-workspaces.json"))
        })
        .setup(|app| {
            info!("🚀 Reasonance setup starting");
            let transport: tauri::State<'_, transport::StructuredAgentTransport> = app.state();

            // Wire FrontendEmitter (existing)
            info!("  ✓ Wiring FrontendEmitter to event bus");
            let emitter = transport::event_bus::FrontendEmitter::new(app.handle().clone());
            transport.event_bus().subscribe(Box::new(emitter));

            // Wire SessionHistoryRecorder into event bus
            info!("  ✓ Wiring SessionHistoryRecorder to event bus");
            let session_mgr: tauri::State<'_, transport::session_manager::SessionManager> = app.state();
            let session_recorder = session_mgr.recorder();
            struct SessionRecorderWrapper(std::sync::Arc<transport::event_bus::SessionHistoryRecorder>);
            impl transport::event_bus::AgentEventSubscriber for SessionRecorderWrapper {
                fn on_event(&self, session_id: &str, event: &crate::agent_event::AgentEvent) {
                    self.0.on_event(session_id, event);
                }
            }
            transport.event_bus().subscribe(Box::new(SessionRecorderWrapper(session_recorder)));

            // Wire AnalyticsCollector into event bus
            info!("  ✓ Wiring AnalyticsCollector to event bus");
            let analytics: tauri::State<'_, std::sync::Arc<analytics::collector::AnalyticsCollector>> = app.state();
            struct AnalyticsWrapper(std::sync::Arc<analytics::collector::AnalyticsCollector>);
            impl transport::event_bus::AgentEventSubscriber for AnalyticsWrapper {
                fn on_event(&self, session_id: &str, event: &crate::agent_event::AgentEvent) {
                    self.0.on_event(session_id, event);
                }
                fn filter(&self) -> Option<transport::event_bus::EventFilter> {
                    self.0.filter()
                }
            }
            transport.event_bus().subscribe(Box::new(AnalyticsWrapper(analytics.inner().clone())));

            // Load capability cache
            let negotiator: tauri::State<'_, capability::CapabilityNegotiator> = app.state();
            let cache_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance")
                .join("capabilities");
            let _ = negotiator.load_cache(&cache_dir);

            // Register CLI providers from normalizer configs
            let updater: tauri::State<'_, std::sync::Arc<cli_updater::CliUpdater>> = app.state();
            let registry = transport.registry();
            let registry_guard = registry.lock().unwrap_or_else(|e| e.into_inner());
            let configs: std::collections::HashMap<String, _> = registry_guard.providers()
                .into_iter()
                .filter_map(|p| registry_guard.get_config(&p).map(|c| (p, c.clone())))
                .collect();
            drop(registry_guard);
            updater.register_from_configs(&configs);

            // Register capabilities from TOML configs
            for (provider, config) in &configs {
                let mut features = std::collections::HashMap::new();
                for (key, val) in &config.capabilities {
                    let support = if val.as_bool() == Some(true) {
                        capability::FeatureSupport::Full
                    } else {
                        capability::FeatureSupport::Unsupported { alternative: None }
                    };
                    features.insert(key.clone(), support);
                }
                let caps = capability::NegotiatedCapabilities {
                    provider: provider.clone(),
                    cli_version: String::new(),
                    cli_mode: transport::request::CliMode::Structured,
                    features,
                    negotiated_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                };
                negotiator.set_capabilities(provider, caps);
            }

            // Start theme file watcher
            info!("  ✓ Starting theme file watcher");
            theme_watcher::start_theme_watcher(app.handle().clone());

            // Spawn background CLI update task
            info!("  ✓ Scheduling background CLI updates");
            let updater_arc = updater.inner().clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                cli_updater::run_background_updates(app_handle, updater_arc).await;
            });

            info!("🚀 Reasonance setup complete — all systems wired");
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
            commands::pty::kill_project_ptys,
            commands::shadow::store_shadow,
            commands::shadow::get_shadow,
            commands::config::read_config,
            commands::config::write_config,
            commands::discovery::discover_agents,
            commands::discovery::get_discovered_agents,
            commands::discovery::register_custom_agent,
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
            commands::agent::get_agent_memory,
            commands::engine::play_workflow,
            commands::engine::pause_workflow,
            commands::engine::resume_workflow,
            commands::engine::stop_workflow,
            commands::engine::step_workflow,
            commands::engine::get_run_status,
            commands::engine::approve_node,
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
            commands::capability::get_capabilities,
            commands::capability::get_provider_capabilities,
            commands::capability::get_cli_versions,
            commands::capability::get_normalizer_versions,
            commands::capability::rollback_normalizer,
            commands::capability::get_health_report,
            commands::capability::get_all_health_reports,
            commands::capability::get_normalizer_config,
            commands::analytics::analytics_provider,
            commands::analytics::analytics_compare,
            commands::analytics::analytics_model_breakdown,
            commands::analytics::analytics_session,
            commands::analytics::analytics_daily,
            commands::analytics::analytics_active,
            commands::provider::test_provider_connection,
            commands::provider::reload_normalizers,
            commands::workspace_trust::check_workspace_trust,
            commands::workspace_trust::set_workspace_trust,
            commands::workspace_trust::revoke_workspace_trust,
            commands::workspace_trust::list_workspace_trust,
            theme_manager::list_user_themes,
            theme_manager::load_user_theme,
            theme_manager::save_user_theme,
            theme_manager::delete_user_theme,
            theme_manager::load_theme_preferences,
            theme_manager::save_theme_preferences,
            project_manager::add_project,
            project_manager::remove_project,
            project_manager::set_active_project,
            project_manager::get_project_root,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
