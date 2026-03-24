use crate::capability::{CapabilityNegotiator, NegotiatedCapabilities};
use crate::cli_updater::{CliUpdater, CliVersionInfo};
use crate::normalizer_health::{NormalizerHealth, HealthReport};
use crate::normalizer_version::{NormalizerVersionStore, VersionEntry};
use log::{info, error, debug};
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub fn get_capabilities(
    negotiator: State<'_, CapabilityNegotiator>,
) -> HashMap<String, NegotiatedCapabilities> {
    info!("cmd::get_capabilities called");
    negotiator.all_capabilities()
}

#[tauri::command]
pub fn get_provider_capabilities(
    negotiator: State<'_, CapabilityNegotiator>,
    provider: String,
) -> Result<NegotiatedCapabilities, String> {
    info!("cmd::get_provider_capabilities(provider={})", provider);
    negotiator
        .get_capabilities(&provider)
        .ok_or_else(|| {
            error!("cmd::get_provider_capabilities no capabilities for provider: {}", provider);
            format!("No capabilities for provider: {}", provider)
        })
}

#[tauri::command]
pub fn get_cli_versions(
    updater: State<'_, CliUpdater>,
) -> Vec<CliVersionInfo> {
    info!("cmd::get_cli_versions called");
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
    info!("cmd::get_normalizer_versions(provider={})", provider);
    version_store.list_versions(&provider)
}

#[tauri::command]
pub fn rollback_normalizer(
    version_store: State<'_, NormalizerVersionStore>,
    transport: State<'_, crate::transport::StructuredAgentTransport>,
    provider: String,
    version_id: String,
) -> Result<String, String> {
    info!("cmd::rollback_normalizer(provider={}, version_id={})", provider, version_id);
    let toml_content = version_store.restore(&provider, &version_id)?;

    // Hot-reload the normalizer in the transport's registry
    let registry = transport.registry();
    let mut registry = registry.lock().unwrap_or_else(|e| e.into_inner());
    registry.reload_provider(&provider, &toml_content)?;

    Ok(format!("Rolled back {} to version {}", provider, version_id))
}

#[tauri::command]
pub fn get_health_report(
    health: State<'_, NormalizerHealth>,
    provider: String,
) -> Result<HealthReport, String> {
    debug!("cmd::get_health_report(provider={})", provider);
    health
        .get_report(&provider)
        .ok_or_else(|| format!("No health report for provider: {}", provider))
}

#[tauri::command]
pub fn get_all_health_reports(
    health: State<'_, NormalizerHealth>,
) -> HashMap<String, HealthReport> {
    debug!("cmd::get_all_health_reports called");
    health.all_reports()
}

#[derive(Serialize)]
pub struct NormalizerConfigResponse {
    pub permission_args: Vec<String>,
}

#[tauri::command]
pub fn get_normalizer_config(
    transport: State<'_, crate::transport::StructuredAgentTransport>,
    provider: String,
) -> Option<NormalizerConfigResponse> {
    info!("cmd::get_normalizer_config(provider={})", provider);
    let registry = transport.registry();
    let registry = registry.lock().unwrap_or_else(|e| e.into_inner());
    registry.get_config(&provider).map(|config| NormalizerConfigResponse {
        permission_args: config.cli.permission_args.clone(),
    })
}
