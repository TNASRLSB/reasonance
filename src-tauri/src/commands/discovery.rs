use crate::discovery::DiscoveryEngine;
use tauri::State;

#[tauri::command]
pub async fn discover_agents(
    engine: State<'_, DiscoveryEngine>,
) -> Result<Vec<crate::discovery::DiscoveredAgent>, String> {
    Ok(engine.discover_all().await)
}

#[tauri::command]
pub fn get_discovered_agents(
    engine: State<'_, DiscoveryEngine>,
) -> Vec<crate::discovery::DiscoveredAgent> {
    engine.get_agents()
}
