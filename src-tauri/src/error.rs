use serde::Serialize;

/// Security error classification codes.
#[derive(Debug, Clone, Serialize)]
pub enum SecurityErrorCode {
    PathTraversal,
    UnauthorizedAccess,
    BlockedWorkspace,
    InvalidScheme,
    DisallowedCommand,
    DisallowedEnvVar,
}

/// Error severity levels for operational classification.
#[derive(Debug, Clone, Serialize)]
pub enum ErrorSeverity {
    Recoverable,
    Degraded,
    Fatal,
}

/// Structured error type for the entire Reasonance application.
///
/// Replaces all `Result<T, String>` returns with typed, serializable errors.
/// Each variant captures domain-specific context for debugging and frontend display.
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "type", content = "details")]
pub enum ReasonanceError {
    #[error("I/O error ({context}): {source}")]
    Io {
        context: String,
        #[serde(serialize_with = "serialize_io_error")]
        source: std::io::Error,
    },

    #[error("Serialization error ({context}): {message}")]
    Serialization { context: String, message: String },

    #[error("Transport error ({provider}): {message}")]
    Transport {
        provider: String,
        message: String,
        retryable: bool,
    },

    #[error("Permission denied: {action}")]
    PermissionDenied {
        action: String,
        tool: Option<String>,
    },

    #[error("{resource_type} not found: {identifier}")]
    NotFound {
        resource_type: String,
        identifier: String,
    },

    #[error("Validation error on {field}: {message}")]
    Validation { field: String, message: String },

    #[error("Workflow error [{workflow_id}/{node_id}]: {message}")]
    Workflow {
        workflow_id: String,
        node_id: String,
        message: String,
    },

    #[error("Config error: {message}")]
    Config { message: String },

    #[error("Security violation: {message}")]
    Security {
        message: String,
        code: SecurityErrorCode,
    },

    #[error("Operation timed out ({operation}): {duration_ms}ms")]
    Timeout { operation: String, duration_ms: u64 },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

/// Serialize `std::io::Error` as its display string for JSON output.
fn serialize_io_error<S>(err: &std::io::Error, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&err.to_string())
}

impl ReasonanceError {
    /// Whether the error is transient and the operation can be retried.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Transport { retryable, .. } => *retryable,
            Self::Timeout { .. } => true,
            Self::Io { .. } => true,
            _ => false,
        }
    }

    /// Classify the operational severity of this error.
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Validation { .. } | Self::NotFound { .. } => ErrorSeverity::Recoverable,
            Self::Io { .. } | Self::Timeout { .. } | Self::Config { .. } => ErrorSeverity::Degraded,
            Self::Transport { retryable, .. } => {
                if *retryable {
                    ErrorSeverity::Degraded
                } else {
                    ErrorSeverity::Fatal
                }
            }
            Self::Security { .. } | Self::Internal { .. } => ErrorSeverity::Fatal,
            Self::Serialization { .. } => ErrorSeverity::Degraded,
            Self::PermissionDenied { .. } => ErrorSeverity::Fatal,
            Self::Workflow { .. } => ErrorSeverity::Degraded,
        }
    }

    // ── Convenience constructors ────────────────────────────────────────

    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    pub fn not_found(resource_type: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self::NotFound {
            resource_type: resource_type.into(),
            identifier: identifier.into(),
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn workflow(
        workflow_id: impl Into<String>,
        node_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Workflow {
            workflow_id: workflow_id.into(),
            node_id: node_id.into(),
            message: message.into(),
        }
    }

    pub fn serialization(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Serialization {
            context: context.into(),
            message: message.into(),
        }
    }

    pub fn transport(
        provider: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self::Transport {
            provider: provider.into(),
            message: message.into(),
            retryable,
        }
    }

    pub fn timeout(operation: impl Into<String>, duration_ms: u64) -> Self {
        Self::Timeout {
            operation: operation.into(),
            duration_ms,
        }
    }
}

// ── From impls ──────────────────────────────────────────────────────────────

impl From<std::io::Error> for ReasonanceError {
    fn from(e: std::io::Error) -> Self {
        Self::Io {
            context: String::new(),
            source: e,
        }
    }
}

impl From<serde_json::Error> for ReasonanceError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization {
            context: "JSON".to_string(),
            message: e.to_string(),
        }
    }
}

impl From<toml::de::Error> for ReasonanceError {
    fn from(e: toml::de::Error) -> Self {
        Self::Serialization {
            context: "TOML deserialization".to_string(),
            message: e.to_string(),
        }
    }
}

impl From<toml::ser::Error> for ReasonanceError {
    fn from(e: toml::ser::Error) -> Self {
        Self::Serialization {
            context: "TOML serialization".to_string(),
            message: e.to_string(),
        }
    }
}

// Tauri 2 has a blanket impl `From<T: Serialize> for InvokeError`, so
// ReasonanceError (which derives Serialize) works directly as a command error type.
// The frontend receives the serialized JSON with `type` and `details` fields.

impl From<String> for ReasonanceError {
    fn from(s: String) -> Self {
        Self::Internal { message: s }
    }
}

// Keep From<ReasonanceError> for String so modules can propagate
// errors through String-based boundaries without explicit conversion.
impl From<ReasonanceError> for String {
    fn from(e: ReasonanceError) -> Self {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_is_retryable() {
        let err = ReasonanceError::io(
            "reading config",
            std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
        );
        assert!(err.is_retryable());
    }

    #[test]
    fn test_timeout_is_retryable() {
        let err = ReasonanceError::Timeout {
            operation: "llm call".to_string(),
            duration_ms: 30000,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_transport_retryable_flag() {
        let retryable = ReasonanceError::Transport {
            provider: "claude".to_string(),
            message: "rate limited".to_string(),
            retryable: true,
        };
        assert!(retryable.is_retryable());

        let fatal = ReasonanceError::Transport {
            provider: "claude".to_string(),
            message: "auth failed".to_string(),
            retryable: false,
        };
        assert!(!fatal.is_retryable());
    }

    #[test]
    fn test_validation_not_retryable() {
        let err = ReasonanceError::validation("title", "too long");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_severity_classification() {
        let not_found = ReasonanceError::not_found("session", "abc123");
        assert!(matches!(not_found.severity(), ErrorSeverity::Recoverable));

        let security = ReasonanceError::Security {
            message: "path traversal".to_string(),
            code: SecurityErrorCode::PathTraversal,
        };
        assert!(matches!(security.severity(), ErrorSeverity::Fatal));

        let io = ReasonanceError::io(
            "read file",
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        );
        assert!(matches!(io.severity(), ErrorSeverity::Degraded));
    }

    #[test]
    fn test_serialization_to_json() {
        let err = ReasonanceError::not_found("workflow", "test.json");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"type\":\"NotFound\""));
        assert!(json.contains("\"resource_type\":\"workflow\""));
        assert!(json.contains("\"identifier\":\"test.json\""));
    }

    #[test]
    fn test_display_formatting() {
        let err = ReasonanceError::config("missing required field 'name'");
        assert_eq!(
            err.to_string(),
            "Config error: missing required field 'name'"
        );

        let err = ReasonanceError::not_found("agent", "agent-42");
        assert_eq!(err.to_string(), "agent not found: agent-42");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err: ReasonanceError = io_err.into();
        assert!(matches!(err, ReasonanceError::Io { .. }));
        assert!(err.to_string().contains("file missing"));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("not valid json").unwrap_err();
        let err: ReasonanceError = json_err.into();
        assert!(matches!(err, ReasonanceError::Serialization { .. }));
        if let ReasonanceError::Serialization { context, .. } = &err {
            assert_eq!(context, "JSON");
        }
    }

    #[test]
    fn test_from_toml_de_error() {
        let toml_err = toml::from_str::<toml::Value>("= invalid").unwrap_err();
        let err: ReasonanceError = toml_err.into();
        assert!(matches!(err, ReasonanceError::Serialization { .. }));
        if let ReasonanceError::Serialization { context, .. } = &err {
            assert_eq!(context, "TOML deserialization");
        }
    }

    #[test]
    fn test_into_string_for_tauri() {
        let err = ReasonanceError::internal("something broke");
        let s: String = err.into();
        assert!(s.contains("something broke"));
    }

    #[test]
    fn test_from_string() {
        let err: ReasonanceError = "something failed".to_string().into();
        assert!(matches!(err, ReasonanceError::Internal { .. }));
        assert!(err.to_string().contains("something failed"));
    }

    #[test]
    fn test_workflow_constructor() {
        let err = ReasonanceError::workflow("wf-1", "node-a", "cycle detected");
        assert!(matches!(err, ReasonanceError::Workflow { .. }));
    }

    #[test]
    fn test_serialization_constructor() {
        let err = ReasonanceError::serialization("JSON", "parse error");
        assert!(matches!(err, ReasonanceError::Serialization { .. }));
    }

    #[test]
    fn test_transport_constructor() {
        let err = ReasonanceError::transport("claude", "rate limited", true);
        assert!(err.is_retryable());
    }
}
