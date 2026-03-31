use crate::error::ReasonanceError;
use crate::event_bus::{Event, EventBus};
use crate::normalizer_health::{run_structural_check, NormalizerHealth};
use crate::normalizer_version::NormalizerVersionStore;
use crate::transport::StructuredAgentTransport;
use crate::NormalizersDir;
use log::{debug, error, info, warn};
use std::sync::Arc;
use tauri::State;

#[derive(Clone, serde::Serialize)]
struct ConnectionTestStep {
    step: String,
    status: String,
    detail: Option<String>,
}

#[tauri::command]
pub async fn test_provider_connection(
    provider: String,
    transport: State<'_, StructuredAgentTransport>,
    bus: State<'_, Arc<EventBus>>,
) -> Result<(), ReasonanceError> {
    info!("cmd::test_provider_connection(provider={})", provider);
    let (binary, api_key_env, version_cmd) = {
        let registry = transport.registry();
        let reg = registry.lock().unwrap_or_else(|e| e.into_inner());
        let config = reg
            .get_config(&provider)
            .ok_or_else(|| ReasonanceError::not_found("provider", &provider))?;
        (
            config.cli.binary.clone(),
            config.cli.api_key_env.clone(),
            config.cli.version_command.clone(),
        )
    };

    // Helper to emit a connection test step via EventBus
    let emit_step = |step: &str, status: &str, detail: Option<String>| {
        bus.publish(Event::new(
            "provider:connection-test",
            serde_json::json!(ConnectionTestStep {
                step: step.into(),
                status: status.into(),
                detail,
            }),
            "test_provider_connection",
        ));
    };

    // Step 1: Binary check
    emit_step("binary", "checking", None);

    let binary_path = which::which(&binary).ok();
    emit_step(
        "binary",
        if binary_path.is_some() {
            "ok"
        } else {
            "failed"
        },
        binary_path.as_ref().map(|p| p.display().to_string()),
    );

    if binary_path.is_none() {
        debug!(
            "cmd::test_provider_connection binary not found for provider={}",
            provider
        );
        return Ok(());
    }

    // Step 2: API key check
    emit_step("api_key", "checking", None);

    let api_key_set = api_key_env
        .as_ref()
        .map(|env| std::env::var(env).is_ok())
        .unwrap_or(true);

    emit_step(
        "api_key",
        if api_key_set { "ok" } else { "failed" },
        api_key_env.clone(),
    );

    if !api_key_set {
        debug!(
            "cmd::test_provider_connection API key not set for provider={}",
            provider
        );
        return Ok(());
    }

    // Step 3: Connection test (use version_command from TOML if available)
    emit_step("connection", "checking", None);

    let start = std::time::Instant::now();
    let output = if !version_cmd.is_empty() {
        tokio::process::Command::new(&version_cmd[0])
            .args(&version_cmd[1..])
            .output()
            .await
    } else {
        tokio::process::Command::new(&binary)
            .args(["--version"])
            .output()
            .await
    };

    match output {
        Ok(o) if o.status.success() => {
            let latency = start.elapsed().as_millis();
            emit_step("connection", "ok", Some(format!("{}ms", latency)));
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            emit_step("connection", "failed", Some(stderr.to_string()));
        }
        Err(e) => {
            emit_step("connection", "failed", Some(e.to_string()));
        }
    }

    Ok(())
}

#[tauri::command]
pub fn reload_normalizers(
    transport: State<'_, StructuredAgentTransport>,
    norm_dir: State<'_, NormalizersDir>,
    health: State<'_, NormalizerHealth>,
    version_store: State<'_, NormalizerVersionStore>,
    bus: State<'_, Arc<EventBus>>,
) -> Result<(), ReasonanceError> {
    info!("cmd::reload_normalizers called");
    let new_registry =
        crate::normalizer::NormalizerRegistry::load_from_dir(&norm_dir.0).map_err(|e| {
            error!("cmd::reload_normalizers failed to load from dir: {}", e);
            ReasonanceError::config(e)
        })?;

    // Collect provider info before swapping in the new registry.
    // We need (provider_name, binary, toml_source) for health + version work.
    let provider_info: Vec<(String, String, Option<String>)> = new_registry
        .providers()
        .into_iter()
        .filter_map(|p| {
            new_registry.get_config(&p).map(|c| {
                (
                    p.clone(),
                    c.cli.binary.clone(),
                    new_registry.get_toml_source(&p),
                )
            })
        })
        .collect();

    let registry = transport.registry();
    *registry.lock().unwrap_or_else(|e| e.into_inner()) = new_registry;
    info!(
        "cmd::reload_normalizers swapped registry with {} providers",
        provider_info.len()
    );

    // W2.11: run structural health check for each provider and publish results.
    // W2.12: create versioned backup for each provider with retention of 20.
    for (provider, binary, toml_source) in &provider_info {
        // --- W2.12: version backup ---
        if let Some(src) = toml_source {
            match version_store.backup_with_retention(provider, src, 20) {
                Ok(version_id) => {
                    info!(
                        "cmd::reload_normalizers backed up provider='{}' version_id={}",
                        provider, version_id
                    );
                    bus.publish(Event::new(
                        "normalizer:version-created",
                        serde_json::json!({
                            "provider": provider,
                            "version_id": version_id,
                        }),
                        "reload_normalizers",
                    ));
                }
                Err(e) => {
                    warn!(
                        "cmd::reload_normalizers version backup failed for provider='{}': {}",
                        provider, e
                    );
                }
            }
        }

        // --- W2.11: structural health check ---
        let report = run_structural_check(provider, binary, "");
        let status_str = match &report.status {
            crate::normalizer_health::HealthStatus::Healthy => "healthy",
            crate::normalizer_health::HealthStatus::Degraded { .. } => "degraded",
            crate::normalizer_health::HealthStatus::Broken { .. } => "broken",
        };
        info!(
            "cmd::reload_normalizers health check provider='{}': {}",
            provider, status_str
        );
        bus.publish(Event::new(
            "normalizer:health",
            serde_json::json!({
                "provider": provider,
                "status": status_str,
                "report": serde_json::to_value(&report).unwrap_or_default(),
            }),
            "reload_normalizers",
        ));
        health.set_report(provider, report);
    }

    Ok(())
}

/// Run startup health checks for all registered providers.
/// Called from `lib.rs` setup() after the transport is wired.
pub fn run_startup_health_checks(
    transport: &StructuredAgentTransport,
    health: &NormalizerHealth,
    version_store: &NormalizerVersionStore,
    bus: &Arc<EventBus>,
) {
    let registry = transport.registry();
    let registry_guard = registry.lock().unwrap_or_else(|e| e.into_inner());
    let provider_info: Vec<(String, String, Option<String>)> = registry_guard
        .providers()
        .into_iter()
        .filter_map(|p| {
            registry_guard.get_config(&p).map(|c| {
                (
                    p.clone(),
                    c.cli.binary.clone(),
                    registry_guard.get_toml_source(&p),
                )
            })
        })
        .collect();
    drop(registry_guard);

    info!(
        "startup health checks: running for {} providers",
        provider_info.len()
    );

    for (provider, binary, toml_source) in &provider_info {
        // W2.12: initial backup on startup so there is always at least one version snapshot.
        if let Some(src) = toml_source {
            match version_store.backup_with_retention(provider, src, 20) {
                Ok(version_id) => {
                    debug!(
                        "startup: backed up provider='{}' version_id={}",
                        provider, version_id
                    );
                    bus.publish(Event::new(
                        "normalizer:version-created",
                        serde_json::json!({
                            "provider": provider,
                            "version_id": version_id,
                        }),
                        "startup",
                    ));
                }
                Err(e) => {
                    warn!(
                        "startup: version backup failed for provider='{}': {}",
                        provider, e
                    );
                }
            }
        }

        // W2.11: structural health check.
        let report = run_structural_check(provider, binary, "");
        let status_str = match &report.status {
            crate::normalizer_health::HealthStatus::Healthy => "healthy",
            crate::normalizer_health::HealthStatus::Degraded { .. } => "degraded",
            crate::normalizer_health::HealthStatus::Broken { .. } => "broken",
        };
        debug!(
            "startup health check provider='{}': {}",
            provider, status_str
        );
        bus.publish(Event::new(
            "normalizer:health",
            serde_json::json!({
                "provider": provider,
                "status": status_str,
                "report": serde_json::to_value(&report).unwrap_or_default(),
            }),
            "startup",
        ));
        health.set_report(provider, report);
    }
}
