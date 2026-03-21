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

#[tauri::command]
pub fn duplicate_workflow(
    store: State<'_, WorkflowStore>,
    source_path: String,
    dest_path: String,
) -> Result<Workflow, String> {
    let mut wf = store.load(&source_path)?;
    let now = chrono::Utc::now().format("%Y-%m-%d").to_string();
    wf.name = format!("{} (copy)", wf.name);
    wf.created = Some(now.clone());
    wf.modified = Some(now);
    store.save(&dest_path, &wf)?;
    Ok(wf)
}

#[tauri::command]
pub fn save_to_global(
    store: State<'_, WorkflowStore>,
    workflow_path: String,
) -> Result<String, String> {
    let wf = store.load(&workflow_path)?;
    let global_dir = WorkflowStore::global_dir();
    std::fs::create_dir_all(&global_dir)
        .map_err(|e| format!("Failed to create global dir: {}", e))?;
    let filename = std::path::Path::new(&workflow_path)
        .file_name()
        .ok_or("Invalid path")?
        .to_str()
        .ok_or("Invalid filename")?;
    let dest = global_dir.join(filename);
    let dest_str = dest.to_str().ok_or("Invalid destination path")?.to_string();
    store.save(&dest_str, &wf)?;
    Ok(dest_str)
}

#[tauri::command]
pub fn list_global_workflows() -> Result<Vec<String>, String> {
    let global_dir = WorkflowStore::global_dir();
    let dir_str = global_dir.to_str().ok_or("Invalid global dir path")?;
    WorkflowStore::list_workflows(dir_str)
}
