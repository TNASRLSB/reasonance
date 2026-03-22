use crate::capability::{CapabilityNegotiator, NegotiatedCapabilities};
use crate::cli_updater::{CliUpdater, CliVersionInfo};
use crate::normalizer_health::{NormalizerHealth, HealthReport};
use crate::normalizer_version::{NormalizerVersionStore, VersionEntry};
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub fn get_capabilities(
    negotiator: State<'_, CapabilityNegotiator>,
) -> HashMap<String, NegotiatedCapabilities> {
    negotiator.all_capabilities()
}

#[tauri::command]
pub fn get_provider_capabilities(
    negotiator: State<'_, CapabilityNegotiator>,
    provider: String,
) -> Result<NegotiatedCapabilities, String> {
    negotiator
        .get_capabilities(&provider)
        .ok_or_else(|| format!("No capabilities for provider: {}", provider))
}

#[tauri::command]
pub fn get_cli_versions(
    updater: State<'_, CliUpdater>,
) -> Vec<CliVersionInfo> {
    updater
        .providers()
        .iter()
        .filter_map(|p| updater.get_info(p))
        .collect()
}

#[tauri::command]
pub fn get_normalizer_versions(
    version_store: State<'_, NormalizerVersionStore>,
    provider: String,
) -> Vec<VersionEntry> {
    version_store.list_versions(&provider)
}

#[tauri::command]
pub fn rollback_normalizer(
    version_store: State<'_, NormalizerVersionStore>,
    transport: State<'_, crate::transport::StructuredAgentTransport>,
    provider: String,
    version_id: String,
) -> Result<String, String> {
    let toml_content = version_store.restore(&provider, &version_id)?;

    // Hot-reload the normalizer in the transport's registry
    let registry = transport.registry();
    let mut registry = registry.lock().unwrap();
    registry.reload_provider(&provider, &toml_content)?;

    Ok(format!("Rolled back {} to version {}", provider, version_id))
}

#[tauri::command]
pub fn get_health_report(
    health: State<'_, NormalizerHealth>,
    provider: String,
) -> Result<HealthReport, String> {
    health
        .get_report(&provider)
        .ok_or_else(|| format!("No health report for provider: {}", provider))
}

#[tauri::command]
pub fn get_all_health_reports(
    health: State<'_, NormalizerHealth>,
) -> HashMap<String, HealthReport> {
    health.all_reports()
}
