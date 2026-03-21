use crate::shadow_store::ShadowStore;
use tauri::State;

#[tauri::command]
pub fn store_shadow(
    path: String,
    content: String,
    store: State<'_, ShadowStore>,
) -> Result<(), String> {
    store.store(&path, &content);
    Ok(())
}

#[tauri::command]
pub fn get_shadow(path: String, store: State<'_, ShadowStore>) -> Result<Option<String>, String> {
    Ok(store.get(&path))
}
