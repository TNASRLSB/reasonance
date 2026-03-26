use crate::permission_engine::{
    DecisionScope, PermissionDecision, PermissionMemory, StoredDecision,
    DEFAULT_PERMISSION_TIMEOUT_SECS,
};
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

/// Wait for a permission decision on (session_id, tool_name), polling every 100ms.
///
/// If no decision is recorded within `timeout_secs` (default: 300s), the request
/// is auto-denied and `ReasonanceError::Timeout` is returned.
/// If a decision arrives before the deadline, returns `"allow"`, `"deny"`, or `"confirm"`.
#[tauri::command]
pub async fn wait_for_permission_decision(
    session_id: String,
    tool_name: String,
    timeout_secs: Option<u64>,
    memory: State<'_, PermissionMemory>,
) -> Result<String, crate::error::ReasonanceError> {
    let timeout = timeout_secs.unwrap_or(DEFAULT_PERMISSION_TIMEOUT_SECS);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout);

    loop {
        if let Some(decision) = memory.lookup(&session_id, &tool_name) {
            return match decision {
                PermissionDecision::Allow => Ok("allow".to_string()),
                PermissionDecision::Deny { .. } => Ok("deny".to_string()),
                PermissionDecision::Confirm => Ok("confirm".to_string()),
            };
        }

        if std::time::Instant::now() >= deadline {
            // Auto-deny on timeout and record the decision so callers can inspect it.
            memory.record(
                &session_id,
                &tool_name,
                PermissionDecision::Deny {
                    reason: format!("Permission timeout after {}s", timeout),
                },
                DecisionScope::Once,
            );
            return Err(crate::error::ReasonanceError::Timeout {
                operation: format!("permission request for tool '{}'", tool_name),
                duration_ms: timeout * 1000,
            });
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use crate::permission_engine::{DecisionScope, PermissionDecision, PermissionMemory};
    use crate::permission_engine::{PermissionTimeoutConfig, DEFAULT_PERMISSION_TIMEOUT_SECS};
    use std::sync::Arc;

    #[test]
    fn test_timeout_config_default() {
        let config = PermissionTimeoutConfig::default();
        assert_eq!(config.timeout_secs, DEFAULT_PERMISSION_TIMEOUT_SECS);
        assert_eq!(config.timeout_secs, 300);
        assert!(config.auto_deny_on_timeout);
    }

    #[tokio::test]
    async fn test_decision_before_timeout_allow() {
        let memory = Arc::new(PermissionMemory::new());
        // Record allow before waiting.
        memory.record("sess1", "Write", PermissionDecision::Allow, DecisionScope::Once);

        // Inline the wait logic so we don't need Tauri State in unit tests.
        let timeout: u64 = 5;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout);
        let result = loop {
            if let Some(decision) = memory.lookup("sess1", "Write") {
                break match decision {
                    PermissionDecision::Allow => Ok("allow".to_string()),
                    PermissionDecision::Deny { .. } => Ok("deny".to_string()),
                    PermissionDecision::Confirm => Ok("confirm".to_string()),
                };
            }
            if std::time::Instant::now() >= deadline {
                break Err("timeout");
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        };

        assert_eq!(result, Ok("allow".to_string()));
    }

    #[tokio::test]
    async fn test_timeout_auto_denies() {
        let memory = PermissionMemory::new();
        // Use a 1-second timeout; no decision will be recorded.
        let timeout: u64 = 1;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout);

        let timed_out = loop {
            if memory.lookup("sess2", "Bash").is_some() {
                break false;
            }
            if std::time::Instant::now() >= deadline {
                // Simulate the auto-deny.
                memory.record(
                    "sess2",
                    "Bash",
                    PermissionDecision::Deny {
                        reason: format!("Permission timeout after {}s", timeout),
                    },
                    DecisionScope::Once,
                );
                break true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        };

        assert!(timed_out, "Expected timeout but got a decision");
        // The auto-deny should now be recorded (Once scope — consumed on lookup).
        let stored = memory.lookup("sess2", "Bash");
        assert!(
            matches!(stored, Some(PermissionDecision::Deny { ref reason }) if reason.contains("timeout")),
            "Expected a timeout-deny decision, got {:?}",
            stored
        );
    }
}
