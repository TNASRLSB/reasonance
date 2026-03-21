use crate::workflow_store::{Workflow, WorkflowStore};
use tauri::State;

#[tauri::command]
pub fn load_workflow(
    file_path: String,
    store: State<'_, WorkflowStore>,
) -> Result<Workflow, String> {
    store.load(&file_path)
}

#[tauri::command]
pub fn save_workflow(
    file_path: String,
    workflow: Workflow,
    store: State<'_, WorkflowStore>,
) -> Result<(), String> {
    store.save(&file_path, &workflow)
}

#[tauri::command]
pub fn list_workflows(dir: String) -> Result<Vec<String>, String> {
    WorkflowStore::list_workflows(&dir)
}

#[tauri::command]
pub fn delete_workflow(
    file_path: String,
    store: State<'_, WorkflowStore>,
) -> Result<(), String> {
    store.delete(&file_path)
}

#[tauri::command]
pub fn create_workflow(
    name: String,
    file_path: String,
    store: State<'_, WorkflowStore>,
) -> Result<Workflow, String> {
    let workflow = WorkflowStore::create_empty(&name);
    store.save(&file_path, &workflow)?;
    Ok(workflow)
}

#[tauri::command]
pub fn get_workflow(
    file_path: String,
    store: State<'_, WorkflowStore>,
) -> Result<Option<Workflow>, String> {
    Ok(store.get(&file_path))
}
