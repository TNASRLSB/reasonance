use crate::permission_engine::{DecisionScope, PermissionDecision, PermissionMemory, StoredDecision};
use tauri::State;

#[tauri::command]
pub async fn record_permission_decision(
    session_id: String,
    tool_name: String,
    action: String,
    scope: String,
    memory: State<'_, PermissionMemory>,
) -> Result<(), crate::error::ReasonanceError> {
    let decision = match action.as_str() {
        "allow" => PermissionDecision::Allow,
        "deny" => PermissionDecision::Deny {
            reason: "User denied".to_string(),
        },
        _ => {
            return Err(crate::error::ReasonanceError::validation(
                "action",
                "must be 'allow' or 'deny'",
            ))
        }
    };
    let scope = match scope.as_str() {
        "once" => DecisionScope::Once,
        "session" => DecisionScope::Session,
        "project" => DecisionScope::Project,
        _ => {
            return Err(crate::error::ReasonanceError::validation(
                "scope",
                "must be 'once', 'session', or 'project'",
            ))
        }
    };
    memory.record(&session_id, &tool_name, decision, scope);
    Ok(())
}

#[tauri::command]
pub async fn lookup_permission_decision(
    session_id: String,
    tool_name: String,
    memory: State<'_, PermissionMemory>,
) -> Result<Option<PermissionDecision>, crate::error::ReasonanceError> {
    Ok(memory.lookup(&session_id, &tool_name))
}

#[tauri::command]
pub async fn list_permission_decisions(
    session_id: String,
    memory: State<'_, PermissionMemory>,
) -> Result<Vec<(String, StoredDecision)>, crate::error::ReasonanceError> {
    Ok(memory.list_decisions(&session_id))
}

#[tauri::command]
pub async fn clear_permission_session(
    session_id: String,
    memory: State<'_, PermissionMemory>,
) -> Result<(), crate::error::ReasonanceError> {
    memory.clear_session(&session_id);
    Ok(())
}
