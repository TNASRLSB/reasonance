use std::sync::{Arc, Mutex};

use log::{debug, error, info, warn};
use serde::Serialize;
use tauri::State;

use crate::error::ReasonanceError;
use crate::event_bus::{Event, EventBus};
use crate::model_slots::{ModelSlot, ModelSlotRegistry};
use crate::normalizer_health::{HealthStatus, NormalizerHealth, TestCaseResult};
use crate::normalizer_version::NormalizerVersionStore;
use crate::self_heal::{build_heal_prompt, extract_toml_from_response, SelfHealConfig};
use crate::transport::StructuredAgentTransport;

#[derive(Debug, Clone, Serialize)]
pub struct HealNormalizerResponse {
    pub status: String,
    pub message: String,
    pub iterations: u32,
}

/// Resolve LLM provider/model/endpoint/api_key_env for the self-heal call.
/// Uses the "quick" model slot, falling back through the slot chain.
/// Returns (provider_api, model, api_key_env, endpoint).
fn resolve_llm_config(
    transport: &StructuredAgentTransport,
    slots: &ModelSlotRegistry,
) -> Result<(String, String, String, String), ReasonanceError> {
    // Walk registered providers looking for one with a quick slot model configured.
    let registry = transport.registry();
    let reg = registry.lock().unwrap_or_else(|e| e.into_inner());
    let providers = reg.providers();

    for provider in &providers {
        if let Some(model) = slots.resolve_model(provider, &ModelSlot::Quick) {
            let config = reg.get_config(provider);
            let api_key_env = config
                .and_then(|c| c.cli.api_key_env.clone())
                .unwrap_or_default();
            // Determine the API provider type and endpoint
            let (api_provider, endpoint) = match provider.as_str() {
                "claude" | "anthropic" => ("anthropic".to_string(), String::new()),
                "gemini" | "google" => ("google".to_string(), String::new()),
                _ => {
                    let ep = config
                        .and_then(|c| c.direct_api.as_ref())
                        .and_then(|d| d.endpoint.clone())
                        .unwrap_or_default();
                    ("openai".to_string(), ep)
                }
            };
            debug!(
                "self-heal LLM config resolved: api_provider={}, model={}, provider={}",
                api_provider, model, provider
            );
            return Ok((api_provider, model, api_key_env, endpoint));
        }
    }

    Err(ReasonanceError::config(
        "No model slot configured for self-heal. Configure a 'quick' (or 'chat') model slot for at least one provider.",
    ))
}

/// Call the LLM API to get a fix suggestion for a broken normalizer TOML.
async fn call_llm(
    prompt: &str,
    api_provider: &str,
    model: &str,
    api_key_env: &str,
    endpoint: &str,
) -> Result<String, ReasonanceError> {
    crate::commands::llm::call_llm_api(
        api_provider.to_string(),
        model.to_string(),
        prompt.to_string(),
        api_key_env.to_string(),
        endpoint.to_string(),
    )
    .await
}

/// Parse the LLM response JSON wrapper to extract the content string.
/// The call_llm_api command returns JSON like `{"content":"...","error":null}`.
fn parse_llm_response(raw: &str) -> Result<String, ReasonanceError> {
    let parsed: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| ReasonanceError::internal(format!("Failed to parse LLM response: {}", e)))?;

    if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
        if !err.is_empty() {
            return Err(ReasonanceError::internal(format!("LLM API error: {}", err)));
        }
    }

    parsed
        .get("content")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ReasonanceError::internal("LLM response had no content"))
}

/// Get failing test case results from the health report for the given provider.
fn get_failures(health: &NormalizerHealth, provider: &str) -> Vec<TestCaseResult> {
    match health.get_report(provider) {
        Some(report) => match &report.status {
            HealthStatus::Healthy => vec![],
            HealthStatus::Degraded { failing_tests } => {
                // Synthesize TestCaseResults from the failing test names
                failing_tests
                    .iter()
                    .map(|name| TestCaseResult {
                        name: name.clone(),
                        passed: false,
                        failure_reason: Some(format!("Test '{}' is failing", name)),
                    })
                    .collect()
            }
            HealthStatus::Broken { error } => vec![TestCaseResult {
                name: "structural".to_string(),
                passed: false,
                failure_reason: Some(error.clone()),
            }],
        },
        None => vec![TestCaseResult {
            name: "no_report".to_string(),
            passed: false,
            failure_reason: Some(format!(
                "No health report available for provider '{}'",
                provider
            )),
        }],
    }
}

/// Attempt to heal a normalizer TOML using an iterative LLM feedback loop.
///
/// Flow: read TOML -> get failures -> build prompt -> call LLM -> extract fix
///       -> backup old TOML -> write fix -> reload -> re-check -> repeat if needed
#[tauri::command]
pub async fn heal_normalizer(
    provider: String,
    transport: State<'_, StructuredAgentTransport>,
    health: State<'_, NormalizerHealth>,
    version_store: State<'_, NormalizerVersionStore>,
    slots: State<'_, Mutex<ModelSlotRegistry>>,
    bus: State<'_, Arc<EventBus>>,
) -> Result<HealNormalizerResponse, ReasonanceError> {
    info!("cmd::heal_normalizer(provider={})", provider);

    let config = SelfHealConfig::default();
    let max_iterations = config.max_iterations;

    // 1. Read the current TOML source
    let current_toml = {
        let registry = transport.registry();
        let reg = registry.lock().unwrap_or_else(|e| e.into_inner());
        reg.get_toml_source(&provider)
            .ok_or_else(|| ReasonanceError::not_found("normalizer TOML source", &provider))?
    };

    // 2. Get failures from the health report
    let failures = get_failures(&health, &provider);
    if failures.is_empty() {
        info!(
            "cmd::heal_normalizer provider='{}' is already healthy, nothing to heal",
            provider
        );
        return Ok(HealNormalizerResponse {
            status: "fixed".to_string(),
            message: "Provider is already healthy — no healing needed.".to_string(),
            iterations: 0,
        });
    }

    // 3. Resolve LLM configuration
    let (api_provider, model, api_key_env, endpoint) = {
        let slot_reg = slots.lock().unwrap_or_else(|e| e.into_inner());
        resolve_llm_config(&transport, &slot_reg)?
    };

    // 4. Backup the original TOML before any modifications
    match version_store.backup_with_retention(&provider, &current_toml, 20) {
        Ok(version_id) => {
            info!(
                "cmd::heal_normalizer backed up original TOML for provider='{}' version_id={}",
                provider, version_id
            );
        }
        Err(e) => {
            warn!(
                "cmd::heal_normalizer backup failed for provider='{}': {}",
                provider, e
            );
        }
    }

    // 5. Iterative heal loop
    let mut previous_attempt: Option<String> = None;
    let mut current_failures = failures;

    for iteration in 1..=max_iterations {
        info!(
            "cmd::heal_normalizer iteration {}/{} for provider='{}'",
            iteration, max_iterations, provider
        );

        // Build the prompt
        let prompt = build_heal_prompt(
            &current_toml,
            &current_failures,
            previous_attempt.as_deref(),
        );

        // Call the LLM
        let llm_response = call_llm(&prompt, &api_provider, &model, &api_key_env, &endpoint)
            .await
            .map_err(|e| {
                error!(
                    "cmd::heal_normalizer LLM call failed on iteration {}: {}",
                    iteration, e
                );
                ReasonanceError::transport(&api_provider, format!("LLM call failed: {}", e), true)
            })?;

        // Parse the wrapper JSON to get the content
        let content = parse_llm_response(&llm_response).map_err(|e| {
            error!("cmd::heal_normalizer failed to parse LLM response: {}", e);
            e
        })?;

        // Extract TOML from the LLM response
        let fixed_toml = match extract_toml_from_response(&content) {
            Some(toml) => toml,
            None => {
                warn!(
                    "cmd::heal_normalizer iteration {} — LLM response contained no TOML block",
                    iteration
                );
                previous_attempt = Some(content);
                continue;
            }
        };

        // SEC: Validate that the LLM did not alter the [cli] identity fields.
        {
            let original_parsed: toml::Value =
                toml::from_str(&current_toml).unwrap_or(toml::Value::Table(Default::default()));
            let fixed_parsed: toml::Value = toml::from_str(&fixed_toml).map_err(|e| {
                ReasonanceError::internal(format!(
                    "LLM produced invalid TOML on iteration {}: {}",
                    iteration, e
                ))
            })?;
            let orig_cli = original_parsed.get("cli");
            let fix_cli = fixed_parsed.get("cli");
            if let (Some(orig), Some(fix)) = (orig_cli, fix_cli) {
                if orig.get("name") != fix.get("name") || orig.get("binary") != fix.get("binary") {
                    warn!(
                        "cmd::heal_normalizer iteration {} — LLM altered [cli] identity fields (name/binary), rejecting fix",
                        iteration
                    );
                    previous_attempt = Some(fixed_toml);
                    continue;
                }
            }
        }

        // Try to reload the normalizer with the fixed TOML
        let reload_result = {
            let registry = transport.registry();
            let mut reg = registry.lock().unwrap_or_else(|e| e.into_inner());
            reg.reload_provider(&provider, &fixed_toml)
        };

        if let Err(e) = reload_result {
            warn!(
                "cmd::heal_normalizer iteration {} — fixed TOML failed to parse: {}",
                iteration, e
            );
            previous_attempt = Some(fixed_toml);
            continue;
        }

        // Re-run structural health check to verify the fix
        let binary = {
            let registry = transport.registry();
            let reg = registry.lock().unwrap_or_else(|e| e.into_inner());
            reg.get_config(&provider)
                .map(|c| c.cli.binary.clone())
                .unwrap_or_default()
        };
        let report = crate::normalizer_health::run_structural_check(&provider, &binary, "");
        health.set_report(&provider, report.clone());

        let new_failures = get_failures(&health, &provider);

        if new_failures.is_empty() {
            // Success! The fix worked.
            info!(
                "cmd::heal_normalizer provider='{}' healed after {} iteration(s)",
                provider, iteration
            );

            // Backup the successful fix
            let _ = version_store.backup_with_retention(&provider, &fixed_toml, 20);

            // Publish event
            bus.publish(Event::new(
                "normalizer:heal-attempted",
                serde_json::json!({
                    "provider": provider,
                    "status": "fixed",
                    "iterations": iteration,
                }),
                "heal_normalizer",
            ));

            return Ok(HealNormalizerResponse {
                status: "fixed".to_string(),
                message: format!(
                    "Normalizer for '{}' healed after {} iteration(s).",
                    provider, iteration
                ),
                iterations: iteration,
            });
        }

        // Not fully fixed yet — feed failures back for next iteration
        info!(
            "cmd::heal_normalizer iteration {} — {} failures remain",
            iteration,
            new_failures.len()
        );
        current_failures = new_failures;
        previous_attempt = Some(fixed_toml);
    }

    // All iterations exhausted — restore the original TOML
    warn!(
        "cmd::heal_normalizer exhausted {} iterations for provider='{}', restoring original",
        max_iterations, provider
    );
    {
        let registry = transport.registry();
        let mut reg = registry.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = reg.reload_provider(&provider, &current_toml) {
            error!(
                "cmd::heal_normalizer failed to restore original TOML for provider='{}': {}",
                provider, e
            );
        }
    }

    // Publish event
    bus.publish(Event::new(
        "normalizer:heal-attempted",
        serde_json::json!({
            "provider": provider,
            "status": "failed",
            "iterations": max_iterations,
        }),
        "heal_normalizer",
    ));

    Ok(HealNormalizerResponse {
        status: "failed".to_string(),
        message: format!(
            "Failed to heal normalizer for '{}' after {} iterations. Original TOML restored.",
            provider, max_iterations
        ),
        iterations: max_iterations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalizer_health::HealthReport;

    #[test]
    fn test_parse_llm_response_success() {
        let raw = r#"{"content":"Here is the fixed TOML:\n\n```toml\n[cli]\nname = \"test\"\n```","error":null}"#;
        let content = parse_llm_response(raw).unwrap();
        assert!(content.contains("fixed TOML"));
    }

    #[test]
    fn test_parse_llm_response_error() {
        let raw = r#"{"content":null,"error":"401: Unauthorized"}"#;
        let result = parse_llm_response(raw);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("401"));
    }

    #[test]
    fn test_parse_llm_response_invalid_json() {
        let result = parse_llm_response("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_failures_healthy() {
        let nh = NormalizerHealth::new();
        let report = HealthReport {
            provider: "test".to_string(),
            status: HealthStatus::Healthy,
            results: vec![],
            capabilities_confirmed: vec![],
            capabilities_missing: vec![],
            capabilities_broken: vec![],
            tested_at: 0,
            cli_version: String::new(),
        };
        nh.set_report("test", report);
        assert!(get_failures(&nh, "test").is_empty());
    }

    #[test]
    fn test_get_failures_degraded() {
        let nh = NormalizerHealth::new();
        let report = HealthReport {
            provider: "test".to_string(),
            status: HealthStatus::Degraded {
                failing_tests: vec!["basic_text".to_string()],
            },
            results: vec![],
            capabilities_confirmed: vec![],
            capabilities_missing: vec![],
            capabilities_broken: vec![],
            tested_at: 0,
            cli_version: String::new(),
        };
        nh.set_report("test", report);
        let failures = get_failures(&nh, "test");
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].name, "basic_text");
    }

    #[test]
    fn test_get_failures_broken() {
        let nh = NormalizerHealth::new();
        let report = HealthReport {
            provider: "test".to_string(),
            status: HealthStatus::Broken {
                error: "binary not found".to_string(),
            },
            results: vec![],
            capabilities_confirmed: vec![],
            capabilities_missing: vec![],
            capabilities_broken: vec![],
            tested_at: 0,
            cli_version: String::new(),
        };
        nh.set_report("test", report);
        let failures = get_failures(&nh, "test");
        assert_eq!(failures.len(), 1);
        assert!(failures[0]
            .failure_reason
            .as_ref()
            .unwrap()
            .contains("binary not found"));
    }

    #[test]
    fn test_get_failures_no_report() {
        let nh = NormalizerHealth::new();
        let failures = get_failures(&nh, "nonexistent");
        assert_eq!(failures.len(), 1);
        assert!(failures[0]
            .failure_reason
            .as_ref()
            .unwrap()
            .contains("No health report"));
    }
}
