use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::agent_comms::{AgentCommsBus, AgentMessage, ChannelType};
use crate::agent_memory_v2::{AgentMemoryV2, MemoryEntryV2, MemoryScope, SortBy};
use crate::agent_runtime::AgentRuntime;
use crate::analytics::collector::AnalyticsCollector;
use crate::analytics::TimeRange;
use crate::app_state_store::AppStateStore;
use crate::capability::CapabilityNegotiator;
use crate::cli_updater::CliUpdater;
use crate::commands::{
    analytics, app_state, capability, config, discovery, engine, file_ops, fs, pty, self_heal,
    session, settings, shadow, system, workflow, workspace_trust,
};
use crate::discovery::DiscoveryEngine;
use crate::error::ReasonanceError;
use crate::event_bus::{Event, EventBus};
use crate::node_registry::HiveNodeRegistry;
use crate::normalizer_health::NormalizerHealth;
use crate::normalizer_version::NormalizerVersionStore;
use crate::project_manager::{ActiveProjectState, ProjectsState};
use crate::pty_manager::PtyManager;
use crate::resource_lock::ResourceLockManager;
use crate::settings::LayeredSettings;
use crate::shadow_store::ShadowStore;
use crate::transport::session_manager::SessionManager;
use crate::transport::StructuredAgentTransport;
use crate::workflow_engine::WorkflowEngine;
use crate::workflow_store::WorkflowStore;
use crate::workspace_trust::TrustStore;

#[derive(Debug, Deserialize)]
pub struct BatchCall {
    pub command: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct BatchCallResult {
    pub ok: Option<serde_json::Value>,
    pub err: Option<ReasonanceError>,
}

impl BatchCallResult {
    pub fn success(value: serde_json::Value) -> Self {
        Self {
            ok: Some(value),
            err: None,
        }
    }

    pub fn error(err: ReasonanceError) -> Self {
        Self {
            ok: None,
            err: Some(err),
        }
    }
}

/// Extract a typed field from a JSON args object, or return a Validation error.
pub fn extract<T: serde::de::DeserializeOwned>(
    args: &serde_json::Value,
    field: &str,
) -> Result<T, ReasonanceError> {
    serde_json::from_value(args.get(field).cloned().unwrap_or(serde_json::Value::Null)).map_err(
        |e| ReasonanceError::validation(field, format!("failed to deserialize '{}': {}", field, e)),
    )
}

/// Extract an optional field — returns Ok(None) if the field is absent or null.
pub fn extract_opt<T: serde::de::DeserializeOwned>(
    args: &serde_json::Value,
    field: &str,
) -> Result<Option<T>, ReasonanceError> {
    match args.get(field) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => serde_json::from_value(v.clone()).map(Some).map_err(|e| {
            ReasonanceError::validation(field, format!("failed to deserialize '{}': {}", field, e))
        }),
    }
}

// ── Constants ────────────────────────────────────────────────────────────────

const BATCH_CALL_TIMEOUT: Duration = Duration::from_secs(5);

// ── Dispatcher ───────────────────────────────────────────────────────────────

/// Dispatch a single command by name, extracting state from the AppHandle
/// and parsing args from the JSON payload.
async fn dispatch(app: &AppHandle, cmd: &str, args: Value) -> Result<Value, ReasonanceError> {
    match cmd {
        // ── fs (sync, except get_git_status) ─────────────────────────────
        "read_file" => {
            let path: String = extract(&args, "path")?;
            let state = app.state::<fs::ProjectRootState>();
            let result = fs::read_file_inner(&path, &state)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "write_file" => {
            let path: String = extract(&args, "path")?;
            let content: String = extract(&args, "content")?;
            let state = app.state::<fs::ProjectRootState>();
            fs::write_file_inner(&path, &content, &state)?;
            Ok(Value::Null)
        }
        "list_dir" => {
            let path: String = extract(&args, "path")?;
            let respect_gitignore: bool = extract_opt(&args, "respectGitignore")?.unwrap_or(true);
            let state = app.state::<fs::ProjectRootState>();
            let result = fs::list_dir_inner(&path, respect_gitignore, &state)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "grep_files" => {
            let path: String = extract(&args, "path")?;
            let pattern: String = extract(&args, "pattern")?;
            let respect_gitignore: bool = extract_opt(&args, "respectGitignore")?.unwrap_or(true);
            let state = app.state::<fs::ProjectRootState>();
            let result = fs::grep_files_inner(&path, &pattern, respect_gitignore, &state)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_git_status" => {
            let project_root: String = extract(&args, "projectRoot")?;
            let result = fs::get_git_status(project_root).await?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── session (all async) ──────────────────────────────────────────
        "session_create" => {
            let provider: String = extract(&args, "provider")?;
            let model: String = extract(&args, "model")?;
            let sm = app.state::<SessionManager>();
            let result = session::session_create_inner(&provider, &model, &sm).await?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "session_list" => {
            let sm = app.state::<SessionManager>();
            let result = session::session_list_inner(&sm).await?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "session_get_events" => {
            let session_id: String = extract(&args, "sessionId")?;
            let sm = app.state::<SessionManager>();
            let result = session::session_get_events_inner(&session_id, &sm).await?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "session_restore" => {
            let session_id: String = extract(&args, "sessionId")?;
            let sm = app.state::<SessionManager>();
            let result = session::session_restore_inner(&session_id, &sm).await?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── app_state (all sync) ─────────────────────────────────────────
        "get_app_state" => {
            let store = app.state::<AppStateStore>();
            let result = app_state::get_app_state_inner(&store)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_project_state" => {
            let project_id: String = extract(&args, "projectId")?;
            let store = app.state::<AppStateStore>();
            let result = app_state::get_project_state_inner(&store, &project_id)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "save_app_state" => {
            let state: crate::app_state_store::AppState = extract(&args, "state")?;
            let store = app.state::<AppStateStore>();
            app_state::save_app_state_inner(&store, &state)?;
            Ok(Value::Null)
        }
        "save_project_state" => {
            let project_id: String = extract(&args, "projectId")?;
            let state: crate::app_state_store::ProjectState = extract(&args, "state")?;
            let store = app.state::<AppStateStore>();
            app_state::save_project_state_inner(&store, &project_id, &state)?;
            Ok(Value::Null)
        }

        // ── workflow/engine (all sync) ───────────────────────────────────
        "get_run_status" => {
            let run_id: String = extract(&args, "runId")?;
            let eng = app.state::<WorkflowEngine>();
            let result = engine::get_run_status_inner(&run_id, &eng);
            Ok(serde_json::to_value(result).unwrap())
        }
        "load_workflow" => {
            let file_path: String = extract(&args, "filePath")?;
            let store = app.state::<WorkflowStore>();
            let root_state = app.state::<fs::ProjectRootState>();
            let result = workflow::load_workflow_inner(&file_path, &store, &root_state)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "list_workflows" => {
            let dir: String = extract(&args, "dir")?;
            let root_state = app.state::<fs::ProjectRootState>();
            let result = workflow::list_workflows_inner(&dir, &root_state)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── analytics (all sync) ─────────────────────────────────────────
        "analytics_daily" => {
            let provider: Option<String> = extract_opt(&args, "provider")?;
            let days: Option<u32> = extract_opt(&args, "days")?;
            let collector = app.state::<Arc<AnalyticsCollector>>();
            let result = analytics::analytics_daily_inner(provider.as_deref(), days, &collector)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "analytics_compare" => {
            let from: Option<u64> = extract_opt(&args, "from")?;
            let to: Option<u64> = extract_opt(&args, "to")?;
            let collector = app.state::<Arc<AnalyticsCollector>>();
            let result = analytics::analytics_compare_inner(from, to, &collector)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "analytics_model_breakdown" => {
            let provider: String = extract(&args, "provider")?;
            let from: Option<u64> = extract_opt(&args, "from")?;
            let to: Option<u64> = extract_opt(&args, "to")?;
            let collector = app.state::<Arc<AnalyticsCollector>>();
            let result =
                analytics::analytics_model_breakdown_inner(&provider, from, to, &collector)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── shadow (all sync) ────────────────────────────────────────────
        "store_shadow" => {
            let path: String = extract(&args, "path")?;
            let content: String = extract(&args, "content")?;
            let store = app.state::<ShadowStore>();
            shadow::store_shadow_inner(&path, &content, &store)?;
            Ok(Value::Null)
        }
        "get_shadow" => {
            let path: String = extract(&args, "path")?;
            let store = app.state::<ShadowStore>();
            let result = shadow::get_shadow_inner(&path, &store)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── config/settings ──────────────────────────────────────────────
        "read_config" => {
            let result = config::read_config()?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_setting" => {
            let key: String = extract(&args, "key")?;
            let settings_state = app.state::<std::sync::Mutex<LayeredSettings>>();
            let result = settings::get_setting_inner(&settings_state, &key)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── permissions ──────────────────────────────────────────────────
        "record_permission_decision" => {
            let session_id: String = extract(&args, "sessionId")?;
            let tool_name: String = extract(&args, "toolName")?;
            let action: String = extract(&args, "action")?;
            let scope: String = extract(&args, "scope")?;
            let memory = app.state::<crate::permission_engine::PermissionMemory>();
            let policy = app.state::<crate::policy_file::PolicyFile>();
            let decision = match action.as_str() {
                "allow" => crate::permission_engine::PermissionDecision::Allow,
                "deny" => crate::permission_engine::PermissionDecision::Deny {
                    reason: "User denied".into(),
                },
                other => {
                    return Err(ReasonanceError::validation(
                        "action",
                        format!("must be 'allow' or 'deny', got '{}'", other),
                    ))
                }
            };
            match scope.as_str() {
                "once" => memory.record(
                    &session_id,
                    &tool_name,
                    decision,
                    crate::permission_engine::DecisionScope::Once,
                ),
                "session" => memory.record(
                    &session_id,
                    &tool_name,
                    decision,
                    crate::permission_engine::DecisionScope::Session,
                ),
                "project" => {
                    let d = if action == "allow" { "allow" } else { "deny" };
                    policy.add_policy_rule(&tool_name, d)?;
                }
                other => {
                    return Err(ReasonanceError::validation(
                        "scope",
                        format!("must be 'once', 'session', or 'project', got '{}'", other),
                    ))
                }
            }
            Ok(Value::Null)
        }
        "lookup_permission_decision" => {
            let session_id: String = extract(&args, "sessionId")?;
            let tool_name: String = extract(&args, "toolName")?;
            let memory = app.state::<crate::permission_engine::PermissionMemory>();
            let result = memory.lookup(&session_id, &tool_name);
            Ok(serde_json::to_value(result).unwrap())
        }
        "clear_permission_session" => {
            let session_id: String = extract(&args, "sessionId")?;
            let memory = app.state::<crate::permission_engine::PermissionMemory>();
            memory.clear_session(&session_id);
            Ok(Value::Null)
        }

        // ── agent memory v2 ──────────────────────────────────────────────
        "memory_add_entry" => {
            let entry: MemoryEntryV2 = extract(&args, "entry")?;
            let store = app.state::<AgentMemoryV2>();
            let id = store.add_entry(entry)?;
            Ok(serde_json::to_value(id).unwrap())
        }
        "memory_search" => {
            let query: String = extract(&args, "query")?;
            let scope: MemoryScope = extract(&args, "scope")?;
            let limit: u32 = extract_opt(&args, "limit")?.unwrap_or(20);
            let store = app.state::<AgentMemoryV2>();
            let results = store.search(&query, scope, limit)?;
            Ok(serde_json::to_value(results).unwrap())
        }
        "memory_list" => {
            let scope: MemoryScope = extract(&args, "scope")?;
            let sort: SortBy = extract_opt(&args, "sort")?.unwrap_or(SortBy::Recency);
            let limit: u32 = extract_opt(&args, "limit")?.unwrap_or(50);
            let offset: u32 = extract_opt(&args, "offset")?.unwrap_or(0);
            let store = app.state::<AgentMemoryV2>();
            let results = store.list(scope, sort, limit, offset)?;
            Ok(serde_json::to_value(results).unwrap())
        }
        "memory_get" => {
            let id: String = extract(&args, "id")?;
            let store = app.state::<AgentMemoryV2>();
            let result = store.get_entry(&id)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── agent comms ──────────────────────────────────────────────────
        "agent_publish_message" => {
            let from: String = extract(&args, "from")?;
            let channel: ChannelType = extract(&args, "channel")?;
            let payload: serde_json::Value = extract(&args, "payload")?;
            let reply_to: Option<String> = extract_opt(&args, "replyTo")?;
            let ttl_secs: Option<u64> = extract_opt(&args, "ttlSecs")?;
            let bus = app.state::<AgentCommsBus>();
            let msg = AgentMessage {
                id: uuid::Uuid::new_v4().to_string(),
                from,
                channel,
                payload,
                timestamp: chrono::Utc::now().to_rfc3339(),
                reply_to,
                ttl_secs,
            };
            let id = msg.id.clone();
            bus.publish(msg)?;
            Ok(serde_json::to_value(id).unwrap())
        }
        "agent_get_messages" => {
            let node_id: String = extract(&args, "nodeId")?;
            let since_id: Option<String> = extract_opt(&args, "sinceId")?;
            let bus = app.state::<AgentCommsBus>();
            let result = bus.get_messages(&node_id, since_id.as_deref());
            Ok(serde_json::to_value(result).unwrap())
        }
        "agent_get_topic_messages" => {
            let topic: String = extract(&args, "topic")?;
            let since_id: Option<String> = extract_opt(&args, "sinceId")?;
            let bus = app.state::<AgentCommsBus>();
            let result = bus.get_topic_messages(&topic, since_id.as_deref());
            Ok(serde_json::to_value(result).unwrap())
        }
        "agent_get_broadcast_messages" => {
            let workflow_id: String = extract(&args, "workflowId")?;
            let since_id: Option<String> = extract_opt(&args, "sinceId")?;
            let bus = app.state::<AgentCommsBus>();
            let result = bus.get_broadcast_messages(&workflow_id, since_id.as_deref());
            Ok(serde_json::to_value(result).unwrap())
        }
        "agent_sweep_messages" => {
            let bus = app.state::<AgentCommsBus>();
            let result = bus.sweep_expired();
            Ok(serde_json::to_value(result).unwrap())
        }
        "agent_clear_workflow_messages" => {
            let workflow_id: String = extract(&args, "workflowId")?;
            let bus = app.state::<AgentCommsBus>();
            bus.clear_workflow(&workflow_id);
            Ok(Value::Null)
        }

        // ── model slots ──────────────────────────────────────────────────
        "get_model_for_slot" => {
            let provider: String = extract(&args, "provider")?;
            let slot: String = extract(&args, "slot")?;
            let registry = app.state::<std::sync::Mutex<crate::model_slots::ModelSlotRegistry>>();
            let slot_enum = crate::model_slots::parse_slot(&slot)
                .map_err(|e| ReasonanceError::validation("slot", e.to_string()))?;
            let reg = registry.lock().unwrap();
            let result = reg.resolve_model(&provider, &slot_enum);
            Ok(serde_json::to_value(result).unwrap())
        }
        "set_model_slot" => {
            let provider: String = extract(&args, "provider")?;
            let slot: String = extract(&args, "slot")?;
            let model: String = extract(&args, "model")?;
            let registry = app.state::<std::sync::Mutex<crate::model_slots::ModelSlotRegistry>>();
            let slot_enum = crate::model_slots::parse_slot(&slot)
                .map_err(|e| ReasonanceError::validation("slot", e.to_string()))?;
            registry
                .lock()
                .unwrap()
                .set_slot(&provider, slot_enum, model);
            Ok(Value::Null)
        }
        "list_model_slots" => {
            let provider: String = extract(&args, "provider")?;
            let registry = app.state::<std::sync::Mutex<crate::model_slots::ModelSlotRegistry>>();
            let reg = registry.lock().unwrap();
            let result = reg
                .providers
                .get(&provider)
                .map(|c| c.list_resolved())
                .unwrap_or_default();
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── pty ──────────────────────────────────────────────────────────
        "reconnect_pty" => {
            let pty_id: String = extract(&args, "ptyId")?;
            let command: String = extract(&args, "command")?;
            let cmd_args: Vec<String> = extract_opt(&args, "args")?.unwrap_or_default();
            let cwd: String = extract(&args, "cwd")?;
            let pty_manager = app.state::<crate::pty_manager::PtyManager>();
            let bus = app.state::<std::sync::Arc<crate::event_bus::EventBus>>();
            let result = crate::commands::pty::reconnect_pty(
                pty_id,
                command,
                cmd_args,
                cwd,
                pty_manager,
                bus,
            )?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── fs (project root) ────────────────────────────────────────
        "set_project_root" => {
            let path: String = extract(&args, "path")?;
            let state = app.state::<fs::ProjectRootState>();
            let settings_state = app.state::<std::sync::Mutex<LayeredSettings>>();
            let policy = app.state::<crate::policy_file::PolicyFile>();
            info!("batch::set_project_root(path={})", path);
            let canonical = if path.is_empty() {
                None
            } else {
                Some(std::fs::canonicalize(&path).map_err(|e| {
                    ReasonanceError::io(format!("canonicalize project root '{}'", path), e)
                })?)
            };
            *state.0.lock().unwrap_or_else(|e| e.into_inner()) = canonical.clone();
            if let Some(ref root) = canonical {
                settings_state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .set_project_root(root);
            }
            if let Some(ref root) = canonical {
                let global_config = dirs::config_dir().map(|d| d.join("reasonance"));
                policy.load_optional(Some(root), global_config.as_deref());
            }
            if let Some(ref root) = canonical {
                fs::install_commit_hook(root);
            }
            Ok(Value::Null)
        }
        "open_external" => {
            let path: String = extract(&args, "path")?;
            system::open_external(path)?;
            Ok(Value::Null)
        }
        "discover_llms" => {
            let result = system::discover_llms();
            Ok(serde_json::to_value(result).unwrap())
        }
        "write_config" => {
            let content: String = extract(&args, "content")?;
            config::write_config(content)?;
            Ok(Value::Null)
        }

        // ── file_ops ────────────────────────────────────────────────────
        "file_ops_set_project" => {
            let path: String = extract(&args, "path")?;
            let state = app.state::<file_ops::FileOpsState>();
            file_ops::file_ops_set_project(path, state)?;
            Ok(Value::Null)
        }
        "file_ops_delete" => {
            let path: String = extract(&args, "path")?;
            let state = app.state::<file_ops::FileOpsState>();
            let bus = app.state::<Arc<EventBus>>();
            file_ops::file_ops_delete(path, state, bus)?;
            Ok(Value::Null)
        }
        "file_ops_undo" => {
            let state = app.state::<file_ops::FileOpsState>();
            let bus = app.state::<Arc<EventBus>>();
            let result = file_ops::file_ops_undo(state, bus)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "file_ops_move" => {
            let old_path: String = extract(&args, "oldPath")?;
            let new_path: String = extract(&args, "newPath")?;
            let state = app.state::<file_ops::FileOpsState>();
            let bus = app.state::<Arc<EventBus>>();
            file_ops::file_ops_move(old_path, new_path, state, bus)?;
            Ok(Value::Null)
        }
        "file_ops_record_create" => {
            let path: String = extract(&args, "path")?;
            let state = app.state::<file_ops::FileOpsState>();
            let bus = app.state::<Arc<EventBus>>();
            file_ops::file_ops_record_create(path, state, bus)?;
            Ok(Value::Null)
        }
        "file_ops_record_rename" => {
            let old_path: String = extract(&args, "oldPath")?;
            let new_path: String = extract(&args, "newPath")?;
            let state = app.state::<file_ops::FileOpsState>();
            let bus = app.state::<Arc<EventBus>>();
            file_ops::file_ops_record_rename(old_path, new_path, state, bus)?;
            Ok(Value::Null)
        }

        // ── project management ──────────────────────────────────────────
        "add_project" => {
            let id: String = extract(&args, "id")?;
            let root_path: String = extract(&args, "rootPath")?;
            let trust_level: String = extract(&args, "trustLevel")?;
            let state = app.state::<ProjectsState>();
            crate::project_manager::add_project(id, root_path, trust_level, state)?;
            Ok(Value::Null)
        }
        "remove_project" => {
            let id: String = extract(&args, "id")?;
            let state = app.state::<ProjectsState>();
            crate::project_manager::remove_project(id, state)?;
            Ok(Value::Null)
        }
        "set_active_project" => {
            let id: String = extract(&args, "id")?;
            let projects_state = app.state::<ProjectsState>();
            let active_state = app.state::<ActiveProjectState>();
            crate::project_manager::set_active_project(id, projects_state, active_state)?;
            Ok(Value::Null)
        }
        "get_project_root" => {
            let project_id: String = extract(&args, "projectId")?;
            let state = app.state::<ProjectsState>();
            let result = crate::project_manager::get_project_root(project_id, state)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── discovery ───────────────────────────────────────────────────
        "discover_agents" => {
            let engine = app.state::<DiscoveryEngine>();
            let result = discovery::discover_agents(engine).await?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_discovered_agents" => {
            let engine = app.state::<DiscoveryEngine>();
            let result = discovery::get_discovered_agents(engine);
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── pty (additional) ────────────────────────────────────────────
        "spawn_process" => {
            let command: String = extract(&args, "command")?;
            let cmd_args: Vec<String> = extract_opt(&args, "args")?.unwrap_or_default();
            let cwd: String = extract(&args, "cwd")?;
            let pty_manager = app.state::<PtyManager>();
            let bus = app.state::<Arc<EventBus>>();
            let result = pty::spawn_process(command, cmd_args, cwd, pty_manager, bus)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "kill_process" => {
            let id: String = extract(&args, "id")?;
            let pty_manager = app.state::<PtyManager>();
            pty::kill_process(id, pty_manager)?;
            Ok(Value::Null)
        }
        "resize_pty" => {
            let id: String = extract(&args, "id")?;
            let cols: u16 = extract(&args, "cols")?;
            let rows: u16 = extract(&args, "rows")?;
            let pty_manager = app.state::<PtyManager>();
            pty::resize_pty(id, cols, rows, pty_manager)?;
            Ok(Value::Null)
        }
        "write_pty" => {
            let id: String = extract(&args, "id")?;
            let data: String = extract(&args, "data")?;
            let pty_manager = app.state::<PtyManager>();
            pty::write_pty(id, data, pty_manager)?;
            Ok(Value::Null)
        }
        "sweep_ptys" => {
            let pty_manager = app.state::<PtyManager>();
            let result = pty::sweep_ptys(pty_manager);
            Ok(serde_json::to_value(result).unwrap())
        }
        "kill_all_ptys" => {
            let pty_manager = app.state::<PtyManager>();
            let result = pty::kill_all_ptys(pty_manager);
            Ok(serde_json::to_value(result).unwrap())
        }
        "kill_project_ptys" => {
            let project_id: String = extract(&args, "projectId")?;
            let pty_manager = app.state::<PtyManager>();
            let result = pty::kill_project_ptys(project_id, pty_manager)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── workflow (additional) ───────────────────────────────────────
        "save_workflow" => {
            let file_path: String = extract(&args, "filePath")?;
            let wf: crate::workflow_store::Workflow = extract(&args, "workflow")?;
            let store = app.state::<WorkflowStore>();
            let root_state = app.state::<fs::ProjectRootState>();
            workflow::save_workflow(file_path, wf, store, root_state)?;
            Ok(Value::Null)
        }
        "delete_workflow" => {
            let file_path: String = extract(&args, "filePath")?;
            let store = app.state::<WorkflowStore>();
            let root_state = app.state::<fs::ProjectRootState>();
            workflow::delete_workflow(file_path, store, root_state)?;
            Ok(Value::Null)
        }
        "create_workflow" => {
            let name: String = extract(&args, "name")?;
            let file_path: String = extract(&args, "filePath")?;
            let store = app.state::<WorkflowStore>();
            let root_state = app.state::<fs::ProjectRootState>();
            let result = workflow::create_workflow(name, file_path, store, root_state)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_workflow" => {
            let file_path: String = extract(&args, "filePath")?;
            let store = app.state::<WorkflowStore>();
            let root_state = app.state::<fs::ProjectRootState>();
            let result = workflow::get_workflow(file_path, store, root_state)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "duplicate_workflow" => {
            let source_path: String = extract(&args, "sourcePath")?;
            let dest_path: String = extract(&args, "destPath")?;
            let store = app.state::<WorkflowStore>();
            let root_state = app.state::<fs::ProjectRootState>();
            let result = workflow::duplicate_workflow(store, source_path, dest_path, root_state)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "save_to_global" => {
            let workflow_path: String = extract(&args, "workflowPath")?;
            let store = app.state::<WorkflowStore>();
            let root_state = app.state::<fs::ProjectRootState>();
            let result = workflow::save_to_global(store, workflow_path, root_state)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "list_global_workflows" => {
            let result = workflow::list_global_workflows()?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── agent runtime ───────────────────────────────────────────────
        "create_agent" => {
            let node_id: String = extract(&args, "nodeId")?;
            let workflow_path: String = extract(&args, "workflowPath")?;
            let max_retries: u32 = extract(&args, "maxRetries")?;
            let fallback_agent: Option<String> = extract_opt(&args, "fallbackAgent")?;
            let runtime = app.state::<AgentRuntime>();
            let result: String = crate::commands::agent::create_agent(
                node_id,
                workflow_path,
                max_retries,
                fallback_agent,
                runtime,
            );
            Ok(serde_json::to_value(result).unwrap())
        }
        "transition_agent" => {
            let agent_id: String = extract(&args, "agentId")?;
            let new_state: crate::agent_runtime::AgentState = extract(&args, "newState")?;
            let runtime = app.state::<AgentRuntime>();
            let result = crate::commands::agent::transition_agent(agent_id, new_state, runtime)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "set_agent_pty" => {
            let agent_id: String = extract(&args, "agentId")?;
            let pty_id: String = extract(&args, "ptyId")?;
            let runtime = app.state::<AgentRuntime>();
            crate::commands::agent::set_agent_pty(agent_id, pty_id, runtime)?;
            Ok(Value::Null)
        }
        "set_agent_error" => {
            let agent_id: String = extract(&args, "agentId")?;
            let message: String = extract(&args, "message")?;
            let runtime = app.state::<AgentRuntime>();
            crate::commands::agent::set_agent_error(agent_id, message, runtime)?;
            Ok(Value::Null)
        }
        "get_agent" => {
            let agent_id: String = extract(&args, "agentId")?;
            let runtime = app.state::<AgentRuntime>();
            let result = crate::commands::agent::get_agent(agent_id, runtime);
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_workflow_agents" => {
            let workflow_path: String = extract(&args, "workflowPath")?;
            let runtime = app.state::<AgentRuntime>();
            let result = crate::commands::agent::get_workflow_agents(workflow_path, runtime);
            Ok(serde_json::to_value(result).unwrap())
        }
        "remove_agent" => {
            let agent_id: String = extract(&args, "agentId")?;
            let runtime = app.state::<AgentRuntime>();
            crate::commands::agent::remove_agent(agent_id, runtime)?;
            Ok(Value::Null)
        }
        "stop_workflow_agents" => {
            let workflow_path: String = extract(&args, "workflowPath")?;
            let runtime = app.state::<AgentRuntime>();
            crate::commands::agent::stop_workflow_agents(workflow_path, runtime);
            Ok(Value::Null)
        }
        "send_agent_message" => {
            let from: String = extract(&args, "from")?;
            let to: String = extract(&args, "to")?;
            let payload: serde_json::Value = extract(&args, "payload")?;
            let runtime = app.state::<AgentRuntime>();
            crate::commands::agent::send_agent_message(from, to, payload, runtime);
            Ok(Value::Null)
        }
        "get_agent_messages" => {
            let agent_id: String = extract(&args, "agentId")?;
            let runtime = app.state::<AgentRuntime>();
            let result = crate::commands::agent::get_agent_messages(agent_id, runtime);
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_agent_memory" => {
            let node_id: String = extract(&args, "nodeId")?;
            let workflow_path: String = extract(&args, "workflowPath")?;
            let persist: String =
                extract_opt(&args, "persist")?.unwrap_or_else(|| "workflow".to_string());
            let result = crate::commands::agent::get_agent_memory(node_id, workflow_path, persist)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── workflow engine ─────────────────────────────────────────────
        "play_workflow" => {
            let workflow_path: String = extract(&args, "workflowPath")?;
            let cwd: String = extract(&args, "cwd")?;
            let eng = app.state::<WorkflowEngine>();
            let store = app.state::<WorkflowStore>();
            let runtime = app.state::<AgentRuntime>();
            let pty_manager = app.state::<PtyManager>();
            let locks = app.state::<ResourceLockManager>();
            let comms = app.state::<AgentCommsBus>();
            let bus = app.state::<Arc<EventBus>>();
            let result = engine::play_workflow(
                workflow_path,
                cwd,
                app.clone(),
                eng,
                store,
                runtime,
                pty_manager,
                locks,
                comms,
                bus,
            )?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "pause_workflow" => {
            let run_id: String = extract(&args, "runId")?;
            let eng = app.state::<WorkflowEngine>();
            engine::pause_workflow(run_id, eng)?;
            Ok(Value::Null)
        }
        "resume_workflow" => {
            let run_id: String = extract(&args, "runId")?;
            let workflow_path: String = extract(&args, "workflowPath")?;
            let cwd: String = extract(&args, "cwd")?;
            let eng = app.state::<WorkflowEngine>();
            let store = app.state::<WorkflowStore>();
            let runtime = app.state::<AgentRuntime>();
            let pty_manager = app.state::<PtyManager>();
            let locks = app.state::<ResourceLockManager>();
            let comms = app.state::<AgentCommsBus>();
            engine::resume_workflow(
                run_id,
                workflow_path,
                cwd,
                app.clone(),
                eng,
                store,
                runtime,
                pty_manager,
                locks,
                comms,
            )?;
            Ok(Value::Null)
        }
        "stop_workflow" => {
            let run_id: String = extract(&args, "runId")?;
            let eng = app.state::<WorkflowEngine>();
            let runtime = app.state::<AgentRuntime>();
            let pty_manager = app.state::<PtyManager>();
            let locks = app.state::<ResourceLockManager>();
            let comms = app.state::<AgentCommsBus>();
            let bus = app.state::<Arc<EventBus>>();
            engine::stop_workflow(run_id, eng, runtime, pty_manager, locks, comms, bus)?;
            Ok(Value::Null)
        }
        "step_workflow" => {
            let run_id: String = extract(&args, "runId")?;
            let workflow_path: String = extract(&args, "workflowPath")?;
            let cwd: String = extract(&args, "cwd")?;
            let eng = app.state::<WorkflowEngine>();
            let store = app.state::<WorkflowStore>();
            let runtime = app.state::<AgentRuntime>();
            let pty_manager = app.state::<PtyManager>();
            let locks = app.state::<ResourceLockManager>();
            let comms = app.state::<AgentCommsBus>();
            let result = engine::step_workflow(
                run_id,
                workflow_path,
                cwd,
                app.clone(),
                eng,
                store,
                runtime,
                pty_manager,
                locks,
                comms,
            )?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "notify_node_completed" => {
            let run_id: String = extract(&args, "runId")?;
            let node_id: String = extract(&args, "nodeId")?;
            let success: bool = extract(&args, "success")?;
            let workflow_path: String = extract(&args, "workflowPath")?;
            let cwd: String = extract(&args, "cwd")?;
            let eng = app.state::<WorkflowEngine>();
            let store = app.state::<WorkflowStore>();
            let runtime = app.state::<AgentRuntime>();
            let pty_manager = app.state::<PtyManager>();
            let locks = app.state::<ResourceLockManager>();
            let comms = app.state::<AgentCommsBus>();
            engine::notify_node_completed(
                run_id,
                node_id,
                success,
                workflow_path,
                cwd,
                app.clone(),
                eng,
                store,
                runtime,
                pty_manager,
                locks,
                comms,
            )?;
            Ok(Value::Null)
        }

        // ── session (additional) ────────────────────────────────────────
        "session_delete" => {
            let session_id: String = extract(&args, "sessionId")?;
            let sm = app.state::<SessionManager>();
            session::session_delete_inner(&session_id, &sm).await?;
            Ok(Value::Null)
        }
        "session_rename" => {
            let session_id: String = extract(&args, "sessionId")?;
            let title: String = extract(&args, "title")?;
            let sm = app.state::<SessionManager>();
            session::session_rename_inner(&session_id, &title, &sm).await?;
            Ok(Value::Null)
        }
        "session_fork" => {
            let session_id: String = extract(&args, "sessionId")?;
            let fork_event_index: u32 = extract(&args, "forkEventIndex")?;
            let sm = app.state::<SessionManager>();
            let result = session::session_fork_inner(&session_id, fork_event_index, &sm).await?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "session_set_view_mode" => {
            let session_id: String = extract(&args, "sessionId")?;
            let mode: crate::transport::session_handle::ViewMode = extract(&args, "mode")?;
            let sm = app.state::<SessionManager>();
            session::session_set_view_mode_inner(&session_id, mode, &sm).await?;
            Ok(Value::Null)
        }

        // ── capability & health ─────────────────────────────────────────
        "get_capabilities" => {
            let negotiator = app.state::<CapabilityNegotiator>();
            let result = capability::get_capabilities(negotiator);
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_provider_capabilities" => {
            let provider: String = extract(&args, "provider")?;
            let negotiator = app.state::<CapabilityNegotiator>();
            let result = capability::get_provider_capabilities(negotiator, provider)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_cli_versions" => {
            let updater = app.state::<Arc<CliUpdater>>();
            let result = capability::get_cli_versions(updater);
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_normalizer_versions" => {
            let provider: String = extract(&args, "provider")?;
            let version_store = app.state::<NormalizerVersionStore>();
            let result = capability::get_normalizer_versions(version_store, provider);
            Ok(serde_json::to_value(result).unwrap())
        }
        "rollback_normalizer" => {
            let provider: String = extract(&args, "provider")?;
            let version_id: String = extract(&args, "versionId")?;
            let version_store = app.state::<NormalizerVersionStore>();
            let transport = app.state::<StructuredAgentTransport>();
            let result =
                capability::rollback_normalizer(version_store, transport, provider, version_id)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_health_report" => {
            let provider: String = extract(&args, "provider")?;
            let health = app.state::<NormalizerHealth>();
            let result = capability::get_health_report(health, provider)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_all_health_reports" => {
            let health = app.state::<NormalizerHealth>();
            let result = capability::get_all_health_reports(health);
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_normalizer_config" => {
            let provider: String = extract(&args, "provider")?;
            let transport = app.state::<StructuredAgentTransport>();
            let result = capability::get_normalizer_config(transport, provider);
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── analytics (additional) ──────────────────────────────────────
        "analytics_provider" => {
            let provider: String = extract(&args, "provider")?;
            let from: Option<u64> = extract_opt(&args, "from")?;
            let to: Option<u64> = extract_opt(&args, "to")?;
            let collector = app.state::<Arc<AnalyticsCollector>>();
            let range = if from.is_some() || to.is_some() {
                Some(TimeRange { from, to })
            } else {
                None
            };
            let result = collector.get_provider_analytics(&provider, range);
            Ok(serde_json::to_value(result).unwrap())
        }
        "analytics_session" => {
            let session_id: String = extract(&args, "sessionId")?;
            let collector = app.state::<Arc<AnalyticsCollector>>();
            let result = collector.get_session_metrics(&session_id);
            Ok(serde_json::to_value(result).unwrap())
        }
        "analytics_active" => {
            let collector = app.state::<Arc<AnalyticsCollector>>();
            let result = collector.get_active_sessions();
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── workspace trust ─────────────────────────────────────────────
        "check_workspace_trust" => {
            let path: String = extract(&args, "path")?;
            let store = app.state::<TrustStore>();
            let result = workspace_trust::check_workspace_trust(path, store)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "set_workspace_trust" => {
            let path: String = extract(&args, "path")?;
            let level: crate::workspace_trust::TrustLevel = extract(&args, "level")?;
            let store = app.state::<TrustStore>();
            workspace_trust::set_workspace_trust(path, level, store)?;
            Ok(Value::Null)
        }
        "revoke_workspace_trust" => {
            let hash: String = extract(&args, "hash")?;
            let store = app.state::<TrustStore>();
            workspace_trust::revoke_workspace_trust(hash, store)?;
            Ok(Value::Null)
        }
        "list_workspace_trust" => {
            let store = app.state::<TrustStore>();
            let result = workspace_trust::list_workspace_trust(store)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── settings (additional) ───────────────────────────────────────
        "set_setting" => {
            let key: String = extract(&args, "key")?;
            let value: serde_json::Value = extract(&args, "value")?;
            let layer: Option<String> = extract_opt(&args, "layer")?;
            let settings_state = app.state::<std::sync::Mutex<LayeredSettings>>();
            settings::set_setting(settings_state, key, value, layer)?;
            Ok(Value::Null)
        }
        "get_all_settings" => {
            let settings_state = app.state::<std::sync::Mutex<LayeredSettings>>();
            let result = settings::get_all_settings(settings_state)?;
            Ok(result)
        }
        "reload_settings" => {
            let settings_state = app.state::<std::sync::Mutex<LayeredSettings>>();
            settings::reload_settings(settings_state)?;
            Ok(Value::Null)
        }

        // ── node registry ───────────────────────────────────────────────
        "get_node_types" => {
            let registry = app.state::<HiveNodeRegistry>();
            let result = crate::node_registry::get_node_types(registry).await?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── self-heal ───────────────────────────────────────────────────
        "heal_normalizer" => {
            let provider: String = extract(&args, "provider")?;
            let transport = app.state::<StructuredAgentTransport>();
            let health = app.state::<NormalizerHealth>();
            let version_store = app.state::<NormalizerVersionStore>();
            let slots = app.state::<std::sync::Mutex<crate::model_slots::ModelSlotRegistry>>();
            let bus = app.state::<Arc<EventBus>>();
            let result =
                self_heal::heal_normalizer(provider, transport, health, version_store, slots, bus)
                    .await?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // ── unknown command ──────────────────────────────────────────────
        other => Err(ReasonanceError::validation(
            "command",
            format!("not batchable: {}", other),
        )),
    }
}

// ── batch_invoke command ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn batch_invoke(calls: Vec<BatchCall>, app: AppHandle) -> Vec<BatchCallResult> {
    let call_count = calls.len();
    let call_names: Vec<String> = calls.iter().map(|c| c.command.clone()).collect();
    let start = std::time::Instant::now();

    info!(
        "batch_invoke: dispatching {} calls: {:?}",
        call_count, call_names
    );

    let futures: Vec<_> = calls
        .into_iter()
        .map(|call| {
            let app = app.clone();
            let cmd = call.command;
            let args = call.args;
            async move {
                let cmd_name = cmd.clone();
                match tokio::time::timeout(BATCH_CALL_TIMEOUT, dispatch(&app, &cmd, args)).await {
                    Ok(Ok(value)) => BatchCallResult::success(value),
                    Ok(Err(e)) => BatchCallResult::error(e),
                    Err(_elapsed) => BatchCallResult::error(ReasonanceError::timeout(
                        cmd_name,
                        BATCH_CALL_TIMEOUT.as_millis() as u64,
                    )),
                }
            }
        })
        .collect();

    let results = join_all(futures).await;

    let elapsed = start.elapsed();
    let error_count = results.iter().filter(|r| r.err.is_some()).count();

    info!(
        "batch_invoke: completed {} calls in {}ms ({} errors)",
        call_count,
        elapsed.as_millis(),
        error_count
    );

    // Publish telemetry to EventBus (may not be available during tests)
    if let Some(event_bus) = app.try_state::<Arc<EventBus>>() {
        event_bus.publish(Event::new(
            "ipc:batch_executed",
            serde_json::json!({
                "batch_size": call_count,
                "duration_ms": elapsed.as_millis() as u64,
                "commands": call_names,
                "errors": error_count,
            }),
            "batch_invoke",
        ));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_string() {
        let args = serde_json::json!({"path": "/test/file.txt"});
        let path: String = extract(&args, "path").unwrap();
        assert_eq!(path, "/test/file.txt");
    }

    #[test]
    fn test_extract_missing_field() {
        let args = serde_json::json!({});
        let result: Result<String, _> = extract(&args, "path");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_opt_present() {
        let args = serde_json::json!({"provider": "claude"});
        let val: Option<String> = extract_opt(&args, "provider").unwrap();
        assert_eq!(val, Some("claude".to_string()));
    }

    #[test]
    fn test_extract_opt_absent() {
        let args = serde_json::json!({});
        let val: Option<String> = extract_opt(&args, "provider").unwrap();
        assert_eq!(val, None);
    }

    #[test]
    fn test_batch_call_result_success() {
        let r = BatchCallResult::success(serde_json::json!("hello"));
        assert!(r.ok.is_some());
        assert!(r.err.is_none());
    }

    #[test]
    fn test_batch_call_result_error() {
        let r = BatchCallResult::error(ReasonanceError::validation("test", "fail"));
        assert!(r.ok.is_none());
        assert!(r.err.is_some());
    }
}
