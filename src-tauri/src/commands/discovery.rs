use crate::discovery::DiscoveryEngine;
use log::{info, debug};
use tauri::State;

#[tauri::command]
pub async fn discover_agents(
    engine: State<'_, DiscoveryEngine>,
) -> Result<Vec<crate::discovery::DiscoveredAgent>, String> {
    info!("cmd::discover_agents called");
    let result = engine.discover_all().await;
    debug!("cmd::discover_agents found {} agents", result.len());
    Ok(result)
}

#[tauri::command]
pub fn get_discovered_agents(
    engine: State<'_, DiscoveryEngine>,
) -> Vec<crate::discovery::DiscoveredAgent> {
    info!("cmd::get_discovered_agents called");
    engine.get_agents()
}

#[tauri::command]
pub fn register_custom_agent(
    agent: crate::discovery::DiscoveredAgent,
    engine: State<'_, DiscoveryEngine>,
) -> Result<(), String> {
    info!("cmd::register_custom_agent called for '{}'", agent.name);
    engine.register_custom_agent(agent);
    Ok(())
}
