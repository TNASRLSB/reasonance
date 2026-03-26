use crate::error::ReasonanceError;
use crate::workspace_trust::{TrustLevel, TrustStore, TrustCheckResult, TrustEntry};
use tauri::State;

#[tauri::command]
pub fn check_workspace_trust(path: String, store: State<'_, TrustStore>) -> Result<TrustCheckResult, ReasonanceError> {
    Ok(store.check_trust(&path))
}

#[tauri::command]
pub fn set_workspace_trust(path: String, level: TrustLevel, store: State<'_, TrustStore>) -> Result<(), ReasonanceError> {
    store.set_trust(&path, level)
}

#[tauri::command]
pub fn revoke_workspace_trust(hash: String, store: State<'_, TrustStore>) -> Result<(), ReasonanceError> {
    store.revoke_trust(&hash)
}

#[tauri::command]
pub fn list_workspace_trust(store: State<'_, TrustStore>) -> Result<Vec<TrustEntry>, ReasonanceError> {
    Ok(store.list_trusted())
}
