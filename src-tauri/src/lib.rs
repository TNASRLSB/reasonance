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
    clippy::needless_borrows_for_generic_args
)]

pub mod agent_comms;
mod agent_event;
mod agent_memory;
pub mod agent_memory_v2;
mod agent_runtime;
mod analytics;
pub mod app_state_store;
mod capability;
pub mod circuit_breaker;
mod cli_updater;
mod commands;
mod config;
mod discovery;
pub mod error;
pub mod event_bus;
mod file_ops;
mod fs_watcher;
mod logic_eval;
pub mod model_slots;
pub mod node_registry;
mod normalizer;
mod normalizer_health;
mod normalizer_version;
pub mod permission_engine;
pub mod policy_file;
mod project_manager;
mod pty_manager;
mod resource_lock;
mod self_heal;
pub mod settings;
mod shadow_store;
pub mod signal;
pub mod storage;
pub mod subscribers;
mod theme_manager;
mod theme_watcher;
pub mod tracked_map;
mod transport;
mod workflow_engine;
mod workflow_store;
mod workspace_trust;

mod perf;

use commands::fs::ProjectRootState;
use fs_watcher::FsWatcherState;
use log::info;
use pty_manager::PtyManager;
use shadow_store::ShadowStore;
use std::time::Instant;
use tauri::{Emitter, Manager};

/// Shared state for the resolved normalizers directory path.
pub struct NormalizersDir(pub std::path::PathBuf);

/// Holds strong references to EventBus subscribers to keep them alive.
/// The EventBus stores Weak references; these Arcs are the strong owners.
pub struct EventBusSubscribers {
    pub history: std::sync::Arc<subscribers::history::HistoryRecorder>,
    pub session_writer: std::sync::Arc<subscribers::session_writer::SessionHistoryWriter>,
    pub analytics: std::sync::Arc<subscribers::analytics::AnalyticsHandler>,
    pub bridge: std::sync::Arc<event_bus::TauriFrontendBridge>,
}

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
        v.push(std::path::PathBuf::from(
            "/usr/share/reasonance/normalizers",
        ));
        v
    };
    for c in &candidates {
        if c.is_dir() {
            info!("Normalizers found at: {}", c.display());
            return c.clone();
        }
    }
    // Return the most likely candidate so the error message is useful
    log::warn!(
        "Normalizers directory not found in any candidate: {:?}",
        candidates
    );
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| std::path::PathBuf::from("normalizers"))
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
        .manage(permission_engine::PermissionMemory::new())
        .manage(policy_file::PolicyFile::new())
        .manage(PtyManager::new())
        .manage(project_manager::ProjectsState::new())
        .manage(project_manager::ActiveProjectState::new())
        .manage(ShadowStore::new())
        .manage(FsWatcherState::new())
        .manage(ProjectRootState::new())
        .manage(discovery::DiscoveryEngine::new())
        .manage(node_registry::HiveNodeRegistry::new())
        .manage(workflow_store::WorkflowStore::new())
        .manage(agent_runtime::AgentRuntime::new())
        .manage(agent_comms::AgentCommsBus::new(1000))
        .manage(workflow_engine::WorkflowEngine::new())
        .manage(resource_lock::ResourceLockManager::new())
        .manage(NormalizersDir(resolve_normalizers_dir()))
        .manage({
            let dir = resolve_normalizers_dir();
            match transport::StructuredAgentTransport::new(&dir) {
                Ok(t) => t,
                Err(e) => {
                    log::error!(
                        "Failed to load normalizers from {}: {}. Starting with empty registry.",
                        dir.display(),
                        e
                    );
                    transport::StructuredAgentTransport::empty()
                }
            }
        })
        .manage(capability::CapabilityNegotiator::new())
        .manage(std::sync::Arc::new(cli_updater::CliUpdater::new()))
        .manage(normalizer_health::NormalizerHealth::new())
        // NOTE: AnalyticsCollector is managed inside setup() where the tokio runtime is active,
        // because AnalyticsStore::new() is async (it reads from StorageBackend).
        .manage({
            let versions_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance")
                .join("normalizer-versions");
            normalizer_version::NormalizerVersionStore::new(&versions_dir)
        })
        .manage({
            let memory_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance");
            let db_path = memory_dir.join("agent-memory.db");
            agent_memory_v2::AgentMemoryV2::new(&db_path)
                .expect("Failed to initialize agent memory database")
        })
        .manage({
            let config_dir = dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance");
            workspace_trust::TrustStore::new(config_dir.join("trusted-workspaces.json"))
        })
        .manage({
            let state_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("reasonance")
                .join("state");
            app_state_store::AppStateStore::new(&state_dir)
                .expect("Failed to initialize app state store")
        })
        .manage(std::sync::Mutex::new(model_slots::ModelSlotRegistry::new()))
        .manage(commands::file_ops::FileOpsState::new())
        .manage(std::sync::Mutex::new(settings::LayeredSettings::new()))
        .setup(|app| {
            let setup_start = Instant::now();
            info!("🚀 Reasonance setup starting");

            // Build EventBus inside setup() where the tokio runtime is guaranteed active.
            let t_bus = Instant::now();
            let bus =
                std::sync::Arc::new(event_bus::EventBus::new(tokio::runtime::Handle::current()));
            bus.register_channel("transport:event", true);
            bus.register_channel("transport:send", true);
            bus.register_channel("transport:complete", true);
            bus.register_channel("transport:error", true);
            bus.register_channel("session:create", true);
            bus.register_channel("session:delete", true);
            bus.register_channel("file:open", true);
            bus.register_channel("file:close", true);
            bus.register_channel("file:save", true);
            bus.register_channel("workflow:node-state", true);
            bus.register_channel("workflow:run-status", true);
            bus.register_channel("workflow:agent-output", true);
            bus.register_channel("workflow:permission-request", true);
            bus.register_channel("lifecycle:sweep", true);
            bus.register_channel("lifecycle:update-check", true);
            bus.register_channel("permission:decision", false);
            bus.register_channel("comms:message_published", true);
            bus.register_channel("normalizer:health", true);
            bus.register_channel("normalizer:version-created", true);
            bus.register_channel("normalizer:heal-attempted", true);
            bus.register_channel("lifecycle:update-available", true);
            bus.register_channel("transport:circuit-state", true);
            bus.register_channel("fileop:execute", true);
            bus.register_channel("fileop:undo", true);
            info!("  ⏱ EventBus init: {}ms", t_bus.elapsed().as_millis());
            app.manage(bus.clone());

            // Pre-load global policy file. Project-level policy will be loaded
            // when set_project_root is called by the frontend.
            {
                let policy: tauri::State<'_, policy_file::PolicyFile> = app.state();
                let global_config = dirs::config_dir().map(|d| d.join("reasonance"));
                policy.load_optional(None, global_config.as_deref());
            }

            // SessionManager + AnalyticsCollector: parallel async init inside setup()
            // where the tokio runtime is active. These are independent so we use tokio::join!.
            let rt = tokio::runtime::Handle::current();
            let t_parallel = Instant::now();
            let (session_mgr, analytics_collector) = rt
                .block_on(async {
                    let (sm_result, ac_result) = tokio::join!(
                        async {
                            let sessions_dir = dirs::data_dir()
                                .unwrap_or_else(|| std::path::PathBuf::from("."))
                                .join("reasonance")
                                .join("sessions");
                            let backend = std::sync::Arc::new(
                                crate::storage::JsonFileBackend::new(&sessions_dir)
                                    .expect("Failed to init session storage backend"),
                            );
                            transport::session_manager::SessionManager::new(backend).await
                        },
                        async {
                            let analytics_dir = dirs::data_dir()
                                .unwrap_or_else(|| std::path::PathBuf::from("."))
                                .join("reasonance")
                                .join("analytics");
                            let backend = std::sync::Arc::new(
                                crate::storage::JsonFileBackend::new(&analytics_dir)
                                    .expect("Failed to init analytics storage backend"),
                            );
                            let store = std::sync::Arc::new(
                                analytics::store::AnalyticsStore::new(backend)
                                    .await
                                    .expect("Failed to init analytics store"),
                            );
                            Ok::<_, crate::error::ReasonanceError>(std::sync::Arc::new(
                                analytics::collector::AnalyticsCollector::new(store),
                            ))
                        },
                    );
                    Ok::<_, crate::error::ReasonanceError>((
                        sm_result.expect("SessionManager init failed"),
                        ac_result.expect("AnalyticsCollector init failed"),
                    ))
                })
                .expect("Parallel init failed");
            let parallel_init_ms = t_parallel.elapsed().as_millis();
            info!(
                "  ⏱ Parallel init (SessionManager + Analytics): {}ms",
                parallel_init_ms
            );
            app.manage(session_mgr);
            app.manage(analytics_collector);

            let transport: tauri::State<'_, transport::StructuredAgentTransport> = app.state();
            let session_mgr: tauri::State<'_, transport::session_manager::SessionManager> =
                app.state();

            // Wire EventBus subscribers
            info!("  ✓ Wiring EventBus subscribers");
            let history = std::sync::Arc::new(subscribers::history::HistoryRecorder::new());
            let session_writer = std::sync::Arc::new(
                subscribers::session_writer::SessionHistoryWriter::new(session_mgr.store()),
            );
            let analytics_state: tauri::State<
                '_,
                std::sync::Arc<analytics::collector::AnalyticsCollector>,
            > = app.state();
            let analytics_handler = std::sync::Arc::new(
                subscribers::analytics::AnalyticsHandler::new(analytics_state.inner().clone()),
            );

            bus.subscribe("transport:event", history.clone());
            bus.subscribe("transport:complete", history.clone());
            bus.subscribe("transport:error", history.clone());
            bus.subscribe_async("transport:event", session_writer.clone());
            bus.subscribe_async("transport:complete", session_writer.clone());
            bus.subscribe_async("transport:error", session_writer.clone());
            bus.subscribe("transport:event", analytics_handler.clone());
            bus.subscribe("transport:complete", analytics_handler.clone());
            bus.subscribe("transport:error", analytics_handler.clone());

            // Wire the session writer into SessionManager so it can track new sessions
            session_mgr.set_writer(session_writer.clone());

            let bridge = std::sync::Arc::new(event_bus::TauriFrontendBridge::new(
                app.handle().clone(),
                bus.clone(),
            ));
            bus.subscribe_to_visible(bridge.clone());

            // Store strong Arcs so the Weak refs inside EventBus stay alive
            // for the application lifetime.
            app.manage(EventBusSubscribers {
                history,
                session_writer,
                analytics: analytics_handler,
                bridge,
            });

            // Lifecycle signals: periodic timers bridged to EventBus so the
            // frontend can listen instead of polling with setInterval.
            // The sweep timer also runs TrackedMap GC on transport + PTY maps.
            let sweep_signal = signal::Signal::new(());
            sweep_signal.bridge_to_event_bus(bus.clone(), "lifecycle:sweep");
            let transport_sessions = transport.sessions_map().clone();
            let pty_mgr: tauri::State<'_, pty_manager::PtyManager> = app.state();
            let pty_instances = pty_mgr.instances_map().clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                interval.tick().await; // skip initial immediate tick
                loop {
                    interval.tick().await;
                    // GC: sweep TrackedMap entries with no external strong refs
                    let swept_sessions = transport_sessions
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .sweep_exclusive();
                    let swept_ptys = pty_instances
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .sweep_exclusive();
                    if swept_sessions > 0 || swept_ptys > 0 {
                        log::info!(
                            "Lifecycle sweep: removed {} transport sessions, {} PTY instances",
                            swept_sessions,
                            swept_ptys
                        );
                    }
                    // Notify frontend
                    sweep_signal.send(());
                }
            });

            let update_signal = signal::Signal::new(());
            update_signal.bridge_to_event_bus(bus.clone(), "lifecycle:update-check");
            tokio::spawn(async move {
                // Initial delay to let the app settle
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(4 * 60 * 60));
                loop {
                    interval.tick().await;
                    update_signal.send(());
                }
            });

            // Pass bus to the transport
            transport.set_event_bus(bus.clone());

            // W2.11 + W2.12: run structural health checks and create initial version
            // snapshots for all loaded normalizer providers.
            {
                let health: tauri::State<'_, normalizer_health::NormalizerHealth> = app.state();
                let version_store: tauri::State<'_, normalizer_version::NormalizerVersionStore> =
                    app.state();
                commands::provider::run_startup_health_checks(
                    &transport,
                    &health,
                    &version_store,
                    &bus,
                );
            }

            // Pass bus to the workflow engine
            let workflow_engine: tauri::State<'_, workflow_engine::WorkflowEngine> = app.state();
            workflow_engine.set_event_bus(bus.clone());

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
            let configs: std::collections::HashMap<String, _> = registry_guard
                .providers()
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
            let updater_bus = bus.clone();
            let auto_check = {
                let s = app
                    .state::<std::sync::Mutex<settings::LayeredSettings>>()
                    .inner()
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                s.get::<bool>("updates.auto_check").unwrap_or(true)
            };
            tauri::async_runtime::spawn(async move {
                cli_updater::run_background_updates(
                    app_handle,
                    updater_arc,
                    Some(updater_bus),
                    auto_check,
                )
                .await;
            });

            let total_setup_ms = setup_start.elapsed().as_millis() as u64;
            info!("⏱ Total setup: {}ms", total_setup_ms);

            perf::record_startup(&perf::StartupBaseline {
                timestamp: chrono::Utc::now().to_rfc3339(),
                total_setup_ms,
                parallel_init_ms: parallel_init_ms as u64,
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
            commands::fs::get_git_status,
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
            commands::pty::reconnect_pty,
            commands::pty::sweep_ptys,
            commands::pty::kill_all_ptys,
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
            commands::self_heal::heal_normalizer,
            commands::workspace_trust::check_workspace_trust,
            commands::workspace_trust::set_workspace_trust,
            commands::workspace_trust::revoke_workspace_trust,
            commands::workspace_trust::list_workspace_trust,
            commands::permission::record_permission_decision,
            commands::permission::lookup_permission_decision,
            commands::permission::list_permission_decisions,
            commands::permission::clear_permission_session,
            commands::permission::wait_for_permission_decision,
            commands::memory::memory_add_entry,
            commands::memory::memory_search,
            commands::memory::memory_list,
            commands::memory::memory_get,
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
            commands::agent_comms::agent_publish_message,
            commands::agent_comms::agent_get_messages,
            commands::agent_comms::agent_get_topic_messages,
            commands::agent_comms::agent_get_broadcast_messages,
            commands::agent_comms::agent_sweep_messages,
            commands::agent_comms::agent_clear_workflow_messages,
            node_registry::get_node_types,
            commands::app_state::get_app_state,
            commands::app_state::save_app_state,
            commands::app_state::get_project_state,
            commands::app_state::save_project_state,
            model_slots::get_model_for_slot,
            model_slots::set_model_slot,
            model_slots::list_model_slots,
            commands::file_ops::file_ops_set_project,
            commands::file_ops::file_ops_delete,
            commands::file_ops::file_ops_undo,
            commands::file_ops::file_ops_record_create,
            commands::file_ops::file_ops_record_rename,
            commands::file_ops::file_ops_move,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::get_all_settings,
            commands::settings::reload_settings,
            commands::batch::batch_invoke,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                if let Some(pty_manager) = app.try_state::<PtyManager>() {
                    let killed = pty_manager.kill_all();
                    log::info!("Shutdown: killed {} PTY instances", killed);
                }
            }
        });
}
