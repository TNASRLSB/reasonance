use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::agent_comms::{AgentCommsBus, AgentMessage, ChannelType};
use crate::agent_memory_v2::{AgentMemoryV2, MemoryEntryV2, MemoryScope, SortBy};
use crate::analytics::collector::AnalyticsCollector;
use crate::app_state_store::AppStateStore;
use crate::commands::{
    analytics, app_state, config, engine, fs, session, settings, shadow, workflow,
};
use crate::error::ReasonanceError;
use crate::event_bus::{Event, EventBus};
use crate::settings::LayeredSettings;
use crate::shadow_store::ShadowStore;
use crate::transport::session_manager::SessionManager;
use crate::workflow_engine::WorkflowEngine;
use crate::workflow_store::WorkflowStore;

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
                    policy
                        .add_policy_rule(&tool_name, d)
                        .map_err(|e| ReasonanceError::validation("policy", e))?;
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
            let result = crate::commands::pty::reconnect_pty(
                pty_id,
                command,
                cmd_args,
                cwd,
                app.clone(),
                pty_manager,
            )?;
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
