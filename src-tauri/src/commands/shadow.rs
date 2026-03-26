use crate::error::ReasonanceError;
use crate::shadow_store::ShadowStore;
use log::{info, debug};
use tauri::State;

#[tauri::command]
pub fn store_shadow(
    path: String,
    content: String,
    store: State<'_, ShadowStore>,
) -> Result<(), ReasonanceError> {
    info!("cmd::store_shadow(path={})", path);
    store.store(&path, &content);
    Ok(())
}

#[tauri::command]
pub fn get_shadow(path: String, store: State<'_, ShadowStore>) -> Result<Option<String>, ReasonanceError> {
    debug!("cmd::get_shadow(path={})", path);
    Ok(store.get(&path))
}
