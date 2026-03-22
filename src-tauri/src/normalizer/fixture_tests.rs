//! Integration tests that replay JSON fixtures through the full normalizer pipeline.

use crate::normalizer::NormalizerRegistry;
use crate::agent_event::{AgentEvent, AgentEventType, ErrorSeverity, EventContent};
use std::path::Path;
use serde_json::Value;

fn event_type_str(event_type: &AgentEventType) -> String {
    // Use serde to get the snake_case string (e.g., ToolUse → "tool_use")
    serde_json::to_value(event_type)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{:?}", event_type).to_lowercase())
}

fn run_fixture_test(provider: &str, fixture_name: &str) {
    let base = env!("CARGO_MANIFEST_DIR");
    let fixture_path = format!("{}/normalizers/fixtures/{}/{}.jsonl", base, provider, fixture_name);
    let expected_path = format!("{}/normalizers/fixtures/{}/{}.expected.json", base, provider, fixture_name);

    let fixture_data = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", fixture_path, e));
    let expected_data = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("Failed to read expected {}: {}", expected_path, e));

    let expected: Vec<Value> = serde_json::from_str(&expected_data)
        .unwrap_or_else(|e| panic!("Failed to parse expected JSON {}: {}", expected_path, e));

    // Load registry with all TOMLs
    let normalizers_dir = format!("{}/normalizers", base);
    let mut registry = NormalizerRegistry::load_from_dir(Path::new(&normalizers_dir))
        .expect("Failed to load normalizers");

    // Process each line through the pipeline
    let mut all_events: Vec<AgentEvent> = Vec::new();
    for line in fixture_data.lines() {
        if line.trim().is_empty() { continue; }
        let events = registry.process(provider, line);
        all_events.extend(events);
    }

    // Verify against expected
    let mut event_idx = 0;
    for (i, exp) in expected.iter().enumerate() {
        let exp_type = exp.get("event_type").and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("Expected entry {} missing event_type", i));

        // Find matching event (scan forward, skip non-matching types)
        let matching_event = loop {
            if event_idx >= all_events.len() {
                panic!(
                    "Expected event_type '{}' (entry {}) but no more events. Got {} events total: {:?}",
                    exp_type, i, all_events.len(),
                    all_events.iter().map(|e| event_type_str(&e.event_type)).collect::<Vec<_>>()
                );
            }
            let ev = &all_events[event_idx];
            event_idx += 1;
            if event_type_str(&ev.event_type) == exp_type {
                break ev;
            }
        };

        // Validate optional assertions
        if let Some(pattern) = exp.get("content_contains").and_then(|v| v.as_str()) {
            let content_str = match &matching_event.content {
                EventContent::Text { value } => value.clone(),
                EventContent::Json { value } => value.to_string(),
                EventContent::Code { source, .. } => source.clone(),
                _ => String::new(),
            };
            assert!(
                content_str.contains(pattern),
                "Event {} content '{}' doesn't contain '{}'",
                i, content_str, pattern
            );
        }

        if exp.get("has_input_tokens") == Some(&Value::Bool(true)) {
            assert!(
                matching_event.metadata.input_tokens.is_some(),
                "Event {} expected input_tokens", i
            );
        }

        if exp.get("has_tool_name") == Some(&Value::Bool(true)) {
            assert!(
                matching_event.metadata.tool_name.is_some(),
                "Event {} expected tool_name", i
            );
        }

        if let Some(severity_str) = exp.get("severity").and_then(|v| v.as_str()) {
            let expected_severity = match severity_str {
                "recoverable" => Some(ErrorSeverity::Recoverable),
                "fatal" => Some(ErrorSeverity::Fatal),
                "degraded" => Some(ErrorSeverity::Degraded),
                _ => None,
            };
            assert_eq!(
                matching_event.metadata.error_severity, expected_severity,
                "Event {} expected severity {:?}", i, severity_str
            );
        }

        if let Some(code) = exp.get("error_code").and_then(|v| v.as_str()) {
            assert_eq!(
                matching_event.metadata.error_code.as_deref(), Some(code),
                "Event {} expected error_code {}", i, code
            );
        }
    }
}

// --- Gemini ---
#[test]
fn test_gemini_basic_text_fixture() { run_fixture_test("gemini", "basic_text"); }
#[test]
fn test_gemini_tool_use_fixture() { run_fixture_test("gemini", "tool_use"); }
#[test]
fn test_gemini_error_fixture() { run_fixture_test("gemini", "error"); }

// --- Kimi ---
#[test]
fn test_kimi_basic_text_fixture() { run_fixture_test("kimi", "basic_text"); }
#[test]
fn test_kimi_thinking_fixture() { run_fixture_test("kimi", "thinking"); }
#[test]
fn test_kimi_tool_use_fixture() { run_fixture_test("kimi", "tool_use"); }
#[test]
fn test_kimi_error_fixture() { run_fixture_test("kimi", "error"); }

// --- Qwen ---
#[test]
fn test_qwen_basic_text_fixture() { run_fixture_test("qwen", "basic_text"); }
#[test]
fn test_qwen_tool_use_fixture() { run_fixture_test("qwen", "tool_use"); }
#[test]
fn test_qwen_error_fixture() { run_fixture_test("qwen", "error"); }

// --- Codex ---
#[test]
fn test_codex_basic_text_fixture() { run_fixture_test("codex", "basic_text"); }
#[test]
fn test_codex_reasoning_fixture() { run_fixture_test("codex", "reasoning"); }
#[test]
fn test_codex_tool_use_fixture() { run_fixture_test("codex", "tool_use"); }
#[test]
fn test_codex_error_fixture() { run_fixture_test("codex", "error"); }
