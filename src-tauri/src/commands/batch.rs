use serde::{Deserialize, Serialize};

use crate::error::ReasonanceError;

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
