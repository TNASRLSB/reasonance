use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub expected: Vec<ExpectedEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedEvent {
    pub event_type: String,
    pub required: bool,
    pub validate: Validation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Validation {
    Exists,
    ContentNotEmpty,
    ContentMatches(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    pub name: String,
    pub passed: bool,
    pub failure_reason: Option<String>,
}

// Note: Spec uses `missing: Vec<String>` for Degraded; we use `failing_tests`
// which is more descriptive. This is a deliberate deviation from spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded { failing_tests: Vec<String> },
    Broken { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub provider: String,
    pub status: HealthStatus,
    pub results: Vec<TestCaseResult>,
    pub capabilities_confirmed: Vec<String>,
    pub capabilities_missing: Vec<String>,
    pub capabilities_broken: Vec<String>,
    pub tested_at: u64,
    pub cli_version: String,
}

/// Container for health reports per provider. Registered as Tauri managed state.
pub struct NormalizerHealth {
    reports: Mutex<HashMap<String, HealthReport>>,
}

impl NormalizerHealth {
    pub fn new() -> Self {
        Self {
            reports: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_report(&self, provider: &str, report: HealthReport) {
        self.reports.lock().unwrap().insert(provider.to_string(), report);
    }

    pub fn get_report(&self, provider: &str) -> Option<HealthReport> {
        self.reports.lock().unwrap().get(provider).cloned()
    }

    pub fn all_reports(&self) -> HashMap<String, HealthReport> {
        self.reports.lock().unwrap().clone()
    }
}

use crate::agent_event::{AgentEvent, AgentEventType, EventContent};

pub fn evaluate_test_case(test_case: &TestCase, events: &[AgentEvent]) -> TestCaseResult {
    for expected in &test_case.expected {
        let matching_events: Vec<&AgentEvent> = events
            .iter()
            .filter(|e| event_type_matches(&e.event_type, &expected.event_type))
            .collect();

        if matching_events.is_empty() {
            if expected.required {
                return TestCaseResult {
                    name: test_case.name.clone(),
                    passed: false,
                    failure_reason: Some(format!(
                        "Required event '{}' not found",
                        expected.event_type
                    )),
                };
            }
            continue;
        }

        let validation_passed = matching_events.iter().any(|e| validate_event(e, &expected.validate));
        if !validation_passed && expected.required {
            return TestCaseResult {
                name: test_case.name.clone(),
                passed: false,
                failure_reason: Some(format!(
                    "Event '{}' found but validation '{}' failed",
                    expected.event_type,
                    validation_label(&expected.validate),
                )),
            };
        }
    }

    TestCaseResult {
        name: test_case.name.clone(),
        passed: true,
        failure_reason: None,
    }
}

pub fn health_status_from_results(results: &[TestCaseResult]) -> HealthStatus {
    let failing: Vec<String> = results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| r.name.clone())
        .collect();

    if failing.is_empty() {
        HealthStatus::Healthy
    } else if failing.len() == results.len() {
        HealthStatus::Broken {
            error: format!("All {} tests failed", results.len()),
        }
    } else {
        HealthStatus::Degraded {
            failing_tests: failing,
        }
    }
}

fn event_type_matches(actual: &AgentEventType, expected: &str) -> bool {
    match (actual, expected) {
        (AgentEventType::Text, "text") => true,
        (AgentEventType::Thinking, "thinking") => true,
        (AgentEventType::ToolUse, "tool_use") => true,
        (AgentEventType::ToolResult, "tool_result") => true,
        (AgentEventType::Error, "error") => true,
        (AgentEventType::Usage, "usage") => true,
        (AgentEventType::Done, "done") => true,
        (AgentEventType::Status, "status") => true,
        (AgentEventType::Metrics, "metrics") => true,
        _ => false,
    }
}

fn validate_event(event: &AgentEvent, validation: &Validation) -> bool {
    match validation {
        Validation::Exists => true,
        Validation::ContentNotEmpty => {
            match &event.content {
                EventContent::Text { value } => !value.is_empty(),
                EventContent::Code { source, .. } => !source.is_empty(),
                _ => true,
            }
        }
        Validation::ContentMatches(pattern) => {
            match &event.content {
                EventContent::Text { value } => value.contains(pattern),
                EventContent::Code { source, .. } => source.contains(pattern),
                _ => false,
            }
        }
    }
}

fn validation_label(v: &Validation) -> &str {
    match v {
        Validation::Exists => "exists",
        Validation::ContentNotEmpty => "content_not_empty",
        Validation::ContentMatches(_) => "content_matches",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::{AgentEvent, AgentEventType, EventContent, AgentEventMetadata};

    fn test_metadata() -> AgentEventMetadata {
        AgentEventMetadata {
            session_id: Some("test".to_string()),
            input_tokens: None,
            output_tokens: None,
            tool_name: None,
            model: None,
            provider: "test".to_string(),
            error_severity: None,
            error_code: None,
            stream_metrics: None,
        }
    }

    fn make_text_event(text: &str) -> AgentEvent {
        AgentEvent {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::Text,
            content: EventContent::Text { value: text.to_string() },
            timestamp: 0,
            metadata: test_metadata(),
        }
    }

    fn make_done_event() -> AgentEvent {
        AgentEvent {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            event_type: AgentEventType::Done,
            content: EventContent::Text { value: String::new() },
            timestamp: 0,
            metadata: test_metadata(),
        }
    }

    #[test]
    fn test_evaluate_basic_text_pass() {
        let test_case = TestCase {
            name: "basic_text".to_string(),
            expected: vec![
                ExpectedEvent {
                    event_type: "text".to_string(),
                    required: true,
                    validate: Validation::ContentMatches("REASONANCE_TEST_OK".to_string()),
                },
                ExpectedEvent {
                    event_type: "done".to_string(),
                    required: true,
                    validate: Validation::Exists,
                },
            ],
        };

        let events = vec![
            make_text_event("REASONANCE_TEST_OK"),
            make_done_event(),
        ];

        let result = evaluate_test_case(&test_case, &events);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_missing_required_event() {
        let test_case = TestCase {
            name: "basic_text".to_string(),
            expected: vec![
                ExpectedEvent {
                    event_type: "text".to_string(),
                    required: true,
                    validate: Validation::ContentMatches("REASONANCE_TEST_OK".to_string()),
                },
                ExpectedEvent {
                    event_type: "done".to_string(),
                    required: true,
                    validate: Validation::Exists,
                },
            ],
        };

        let events = vec![make_text_event("REASONANCE_TEST_OK")];

        let result = evaluate_test_case(&test_case, &events);
        assert!(!result.passed);
        assert!(result.failure_reason.is_some());
    }

    #[test]
    fn test_evaluate_content_mismatch() {
        let test_case = TestCase {
            name: "basic_text".to_string(),
            expected: vec![
                ExpectedEvent {
                    event_type: "text".to_string(),
                    required: true,
                    validate: Validation::ContentMatches("REASONANCE_TEST_OK".to_string()),
                },
            ],
        };

        let events = vec![make_text_event("wrong output")];

        let result = evaluate_test_case(&test_case, &events);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_optional_missing_still_passes() {
        let test_case = TestCase {
            name: "thinking".to_string(),
            expected: vec![
                ExpectedEvent {
                    event_type: "thinking".to_string(),
                    required: false,
                    validate: Validation::ContentNotEmpty,
                },
            ],
        };

        let events = vec![make_text_event("answer")];

        let result = evaluate_test_case(&test_case, &events);
        assert!(result.passed);
    }

    #[test]
    fn test_health_status_from_results() {
        let all_pass = vec![
            TestCaseResult { name: "basic_text".to_string(), passed: true, failure_reason: None },
            TestCaseResult { name: "thinking".to_string(), passed: true, failure_reason: None },
        ];
        assert!(matches!(health_status_from_results(&all_pass), HealthStatus::Healthy));

        let some_fail = vec![
            TestCaseResult { name: "basic_text".to_string(), passed: true, failure_reason: None },
            TestCaseResult { name: "thinking".to_string(), passed: false, failure_reason: Some("missing".into()) },
        ];
        assert!(matches!(health_status_from_results(&some_fail), HealthStatus::Degraded { .. }));

        let all_fail = vec![
            TestCaseResult { name: "basic_text".to_string(), passed: false, failure_reason: Some("no text".into()) },
        ];
        assert!(matches!(health_status_from_results(&all_fail), HealthStatus::Broken { .. }));
    }
}
